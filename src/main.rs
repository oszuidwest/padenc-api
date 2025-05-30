// Import all modules from lib.rs
use odr_metadata_server::{config, constants, errors, handlers, middleware, models, services};

use actix_multipart::Multipart;
use actix_web::{web, App, HttpServer};
use log::{error, info};
use middleware::auth::Auth;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use config::Config;
use constants::api::DEFAULT_SERVER_PORT;
use constants::fs::{DLS_OUTPUT_FILE, IMAGE_DIR, MOT_OUTPUT_DIR};
use errors::{ServiceError, ServiceResult};
use models::data::{Program, Station, Track};
use models::AppState;
use services::{DlsService, MotService, TickerService};

#[actix_web::main]
async fn main() -> ServiceResult<()> {
    env_logger::init();
    info!("Starting DAB metadata service");

    dotenv::dotenv().ok();

    // Load configuration - will panic with error message if required values are missing
    let config = match Config::from_env() {
        Ok(cfg) => {
            info!(
                "Configuration loaded successfully for station: {}",
                cfg.station_name
            );
            cfg
        }
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            return Err(ServiceError::Configuration(format!(
                "Failed to load configuration: {}",
                e
            )));
        }
    };

    let server_port = DEFAULT_SERVER_PORT.to_string();

    // Create directories for images and MOT output
    let image_dir = PathBuf::from(IMAGE_DIR);
    let mot_dir = PathBuf::from(MOT_OUTPUT_DIR);
    info!("Initializing image directory at: {:?}", image_dir);
    if let Err(e) = MotService::init(&image_dir) {
        error!("Failed to initialize image directory: {}", e);
        return Err(ServiceError::FileProcessing(
            "Image directory initialization error".to_string(),
        ));
    }

    // Initialize MOT directory
    info!("Initializing MOT directory at: {:?}", mot_dir);
    if let Err(e) = MotService::init_mot_dir(&mot_dir) {
        error!("Failed to initialize MOT directory: {}", e);
        return Err(ServiceError::FileProcessing(
            "MOT directory initialization error".to_string(),
        ));
    }

    let has_station_image;
    let station_image = match MotService::load_station_image(&config.default_station_image).await {
        Ok(img) => {
            has_station_image = img.is_some();
            img
        }
        Err(e) => {
            error!("Failed to load default station image: {}", e);
            has_station_image = false;
            None
        }
    };

    // Create shared application state
    let station_name = config.station_name.clone();
    let state = web::Data::new(Mutex::new(AppState {
        track: None,
        program: None,
        station: Some(Station {
            id: uuid::Uuid::new_v4(),
            name: station_name,
            image: station_image,
        }),
    }));

    // Create Arc reference for the ticker service
    let state_for_ticker = state.clone();

    // Configuration for routes
    let config_data = web::Data::new(config);

    // Create default files
    {
        let mut app_state = state.lock().unwrap();
        if let Err(e) = DlsService::update_output_file(&mut app_state) {
            error!("Failed to create initial output file: {}", e);
            return Err(ServiceError::FileProcessing(
                "File creation error".to_string(),
            ));
        }

        // Initialize MOT images
        if let Err(e) = MotService::update_mot_output(&mut app_state, &mot_dir) {
            error!("Failed to initialize MOT images: {}", e);
            return Err(ServiceError::FileProcessing(
                "MOT initialization error".to_string(),
            ));
        }
    }

    // Start the ticker service in a background task
    info!("Starting background ticker service");
    let state_arc = Arc::new(state_for_ticker);
    tokio::spawn(async move {
        TickerService::start(state_arc).await;
    });

    info!("MOT slideshow using station image: {}", has_station_image);

    // Start HTTP server
    let bind_address = format!("0.0.0.0:{}", server_port);
    info!("Starting HTTP server at {}", bind_address);
    info!(
        "Using fixed paths: DLS output={}, Images={}, MOT={}",
        DLS_OUTPUT_FILE, IMAGE_DIR, MOT_OUTPUT_DIR
    );

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .app_data(config_data.clone())
            .wrap(Auth)
            .route(
                "/track",
                web::post().to(
                    |payload: Option<Multipart>,
                     json: Option<web::Json<Track>>,
                     state: web::Data<Mutex<AppState>>| {
                        handlers::track::post_track(payload, json, state)
                    },
                ),
            )
            .route("/track", web::delete().to(handlers::track::delete_track))
            .route(
                "/program",
                web::post().to(
                    |payload: Option<Multipart>,
                     json: Option<web::Json<Program>>,
                     state: web::Data<Mutex<AppState>>| {
                        handlers::program::post_program(payload, json, state)
                    },
                ),
            )
            .route(
                "/program",
                web::delete().to(handlers::program::delete_program),
            )
    })
    .bind(bind_address)?
    .run()
    .await
    .map_err(|e| ServiceError::Server(format!("HTTP server error: {}", e)))
}
