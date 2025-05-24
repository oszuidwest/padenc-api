use actix_web::web;
use chrono::Utc;
use log::{debug, error, info};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::interval;

use crate::config::Config;
use crate::models::AppState;
use crate::services::{FileService, OutputType};

pub struct TickerService;

impl TickerService {
    pub async fn start(app_state: Arc<web::Data<Mutex<AppState>>>, config: Arc<Config>) {
        info!("Starting ticker service with 50-millisecond interval");
        let mut interval_timer = interval(Duration::from_millis(50));
        let mut previous_output_type: Option<OutputType> = None;

        loop {
            interval_timer.tick().await;

            // Get a lock on the app state and update the output file
            match app_state.lock() {
                Ok(state) => {
                    debug!("Ticker: Checking content expiration");
                    
                    let now = Utc::now();
                    let current_output_type = FileService::determine_output_type(&state, &config, now);
                    
                    let has_changed = match (&previous_output_type, &current_output_type) {
                        (None, _) => true,
                        (Some(prev), curr) => !Self::output_types_equal(prev, curr)
                    };
                    
                    if has_changed {
                        info!("Ticker: Content changed at {}", now);
                        match &current_output_type {
                            OutputType::Track(artist, title) => {
                                info!("New content: Track \"{}\" by \"{}\"", title, artist);
                            },
                            OutputType::Program(name) => {
                                info!("New content: Program \"{}\"", name);
                            },
                            OutputType::Station(name) => {
                                info!("New content: Station \"{}\"", name);
                            },
                        }
                        
                        if let Err(e) = FileService::update_file_with_content(&state, &current_output_type) {
                            error!("Ticker: Failed to update output file: {}", e);
                        }
                        
                        previous_output_type = Some(current_output_type);
                    }
                }
                Err(e) => {
                    error!("Ticker: Failed to acquire lock on app state: {}", e);
                }
            }
        }
    }

    fn output_types_equal(prev: &OutputType, curr: &OutputType) -> bool {
        match (prev, curr) {
            (OutputType::Track(prev_artist, prev_title), 
             OutputType::Track(curr_artist, curr_title)) => {
                prev_artist == curr_artist && prev_title == curr_title
            },
            (OutputType::Program(prev_name), OutputType::Program(curr_name)) => {
                prev_name == curr_name
            },
            (OutputType::Station(prev_name), OutputType::Station(curr_name)) => {
                prev_name == curr_name
            },
            _ => false,
        }
    }
}
