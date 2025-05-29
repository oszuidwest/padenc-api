mod config;
mod handlers;
mod middleware;
mod models;
mod services;

use actix_web::{web, App, HttpServer};
use log::{error, info};
use std::io;
use std::sync::{Arc, Mutex};
use middleware::auth::Auth;

use crate::config::Config;
use crate::models::AppState;
use crate::services::{FileService, TickerService};

#[actix_web::main]
async fn main() -> io::Result<()> {
    env_logger::init();
    info!("Starting DAB metadata service");

    dotenv::dotenv().ok();

    let config = match Config::from_env() {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            return Err(io::Error::new(io::ErrorKind::Other, "Configuration error"));
        }
    };

    let output_path = config.output_file_path.clone();
    let server_port = "8080".to_string();

    // Create shared application state
    let state = web::Data::new(Mutex::new(AppState {
        track: None,
        program: None,
        output_path,
    }));

    // Create Arc reference for the ticker service
    let state_for_ticker = state.clone();
    let config_for_ticker = Arc::new(config.clone());

    // Configuration for routes
    let config_data = web::Data::new(config);

    // Create default file
    {
        let app_state = state.lock().unwrap();
        if let Err(e) = FileService::update_output_file(&app_state, &config_data) {
            error!("Failed to create initial output file: {}", e);
            return Err(io::Error::new(io::ErrorKind::Other, "File creation error"));
        }
    }

    // Start the ticker service in a background task
    info!("Starting background ticker service");
    let state_arc = Arc::new(state_for_ticker);
    tokio::spawn(async move {
        TickerService::start(state_arc, config_for_ticker).await;
    });

    // Start HTTP server
    let bind_address = format!("0.0.0.0:{}", server_port);
    info!("Starting HTTP server at {}", bind_address);

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .app_data(config_data.clone())
            .wrap(Auth)
            .route("/track", web::post().to(handlers::track::post_track))
            .route("/track", web::delete().to(handlers::track::delete_track))
            .route("/program", web::post().to(handlers::program::post_program))
            .route(
                "/program",
                web::delete().to(handlers::program::delete_program),
            )
    })
    .bind(bind_address)?
    .run()
    .await
}
