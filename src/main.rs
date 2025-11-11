// Import all modules from lib.rs
use padenc_api::{config, constants, errors, handlers, middleware, models, services};

use actix_multipart::Multipart;
use actix_web::{web, App, HttpServer};
use log::{error, info};
use middleware::auth::Auth;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use config::Config;
use constants::api::DEFAULT_SERVER_PORT;
use errors::{ServiceError, ServiceResult};
use models::data::{Program, Station, Track};
use models::AppState;
use services::{DlsService, MotService, TickerService};

#[actix_web::main]
async fn main() -> ServiceResult<()> {
    env_logger::init();
    info!("Starting DAB metadata service");

    dotenv::dotenv().ok();

    // Load configuration
    let config = Config::from_env().map_err(|e| {
        error!("Failed to load configuration: {}", e);
        ServiceError::Configuration(format!("Failed to load configuration: {}", e))
    })?;

    info!(
        "Configuration loaded successfully for station: {}",
        config.station_name
    );
    let server_port = DEFAULT_SERVER_PORT.to_string();

    // Create directories for images and MOT output
    let dls_path = PathBuf::from(config.dls_file.clone());
    let image_dir = PathBuf::from(config.image_dir.clone());
    let mot_dir = PathBuf::from(config.mot_dir.clone());

    info!("Initializing image directory at: {:?}", image_dir);
    MotService::init(&image_dir).map_err(|e| {
        error!("Failed to initialize image directory: {}", e);
        ServiceError::FileProcessing("Image directory initialization error".into())
    })?;

    // Initialize MOT directory
    info!("Initializing MOT directory at: {:?}", mot_dir);
    MotService::init_mot_dir(&mot_dir).map_err(|e| {
        error!("Failed to initialize MOT directory: {}", e);
        ServiceError::FileProcessing("MOT directory initialization error".into())
    })?;

    let has_station_image;
    let station_image = match MotService::load_station_image(&image_dir, &config.default_station_image).await {
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

    let station_name = config.station_name.clone();
    let state = web::Data::new(Mutex::new(AppState {
        track: None,
        program: None,
        station: Some(Station {
            id: uuid::Uuid::new_v4(),
            name: station_name,
            image: station_image,
        }),
        dl_plus_item_toggle: false,
    }));

    let state_for_ticker = state.clone();
    let config_data = web::Data::new(config);

    {
        let mut mut_guard = state.lock().map_err(|_| {
            ServiceError::Server("Failed to acquire lock on application state".into())
        })?;

        DlsService::update_output_file(&dls_path, &mut mut_guard).map_err(|e| {
            error!("Failed to create initial output file: {}", e);
            ServiceError::FileProcessing("File creation error".into())
        })?;

        MotService::update_mot_output(&mot_dir, &mut mut_guard).map_err(|e| {
            error!("Failed to initialize MOT images: {}", e);
            ServiceError::FileProcessing("MOT initialization error".into())
        })?;
    }

    info!("Starting background ticker service");
    let state_arc = Arc::new(state_for_ticker);
    let mot_dir_clone = mot_dir.clone();
    let dls_path_clone = dls_path.clone();
    let image_dir_clone = image_dir.clone();
    tokio::spawn(async move {
        TickerService::start(state_arc, mot_dir_clone, dls_path_clone, image_dir_clone).await;
    });

    info!("MOT slideshow using station image: {}", has_station_image);

    let bind_address = format!("0.0.0.0:{}", server_port);
    info!("Starting HTTP server at {}", bind_address);
    info!(
        "Using fixed paths: DLS output={}, Images={}, MOT={}",
        config_data.dls_file,
        config_data.image_dir,
        config_data.mot_dir,
    );

    HttpServer::new(move || {
        let cfg = config_data.clone();
        App::new()
            .app_data(state.clone())
            .app_data(cfg.clone())
            .wrap(Auth)
            .route(
                "/track",
                web::post().to(
                    |payload: Option<Multipart>,
                     json: Option<web::Json<Track>>,
                     state: web::Data<Mutex<AppState>>,
                     config: web::Data<Config>| {
                        handlers::track::post_track(payload, json, state, config)
                    },
                ),
            )
            .route("/track", web::delete().to(handlers::track::delete_track))
            .route(
                "/program",
                web::post().to(
                    |payload: Option<Multipart>,
                     json: Option<web::Json<Program>>,
                     state: web::Data<Mutex<AppState>>,
                     config: web::Data<Config>| {
                        handlers::program::post_program(payload, json, state, config)
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
