use actix_web::web;
use chrono::Utc;
use log::{debug, error, info};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::interval;
use uuid::Uuid;

use crate::constants::fs::MOT_OUTPUT_DIR;
use crate::constants::ticker::{CLEANUP_INTERVAL_TICKS, INTERVAL_MS};
use crate::models::AppState;
use crate::models::HasId;
use crate::services::content_service::OutputType;
use crate::services::{ContentService, DlsService, MotService};

pub struct TickerService;

impl TickerService {
    pub async fn start(app_state: Arc<web::Data<Mutex<AppState>>>) {
        info!(
            "Starting ticker service with {}-millisecond interval",
            INTERVAL_MS
        );
        let mut interval_timer = interval(Duration::from_millis(INTERVAL_MS));
        let mut previous_output_type: Option<OutputType> = None;
        let mut previous_content_id: Option<Uuid> = None;
        let mot_dir = PathBuf::from(MOT_OUTPUT_DIR);

        loop {
            interval_timer.tick().await;

            // Get a lock on the app state and update the output file
            match app_state.lock() {
                Ok(mut state) => {
                    debug!("Ticker: Checking content expiration");

                    let now = Utc::now();
                    let current_output_type =
                        ContentService::get_active_output_type(&mut state, now);

                    let current_content_id = match current_output_type {
                        OutputType::Track => state.track.as_ref().and_then(|t| t.get_id()),
                        OutputType::Program => state.program.as_ref().and_then(|p| p.get_id()),
                        OutputType::Station => state.station.as_ref().and_then(|s| s.get_id()),
                    };

                    let has_changed = match &previous_output_type {
                        None => true,
                        Some(prev) => {
                            prev != &current_output_type
                                || previous_content_id != current_content_id
                        }
                    };

                    if has_changed {
                        info!("Ticker: Content changed at {}", now);
                        match current_output_type {
                            OutputType::Track => {
                                if let Some(track) = &state.track {
                                    let artist_display = track.item.artist.as_deref().unwrap_or("(no artist)");
                                    info!(
                                        "New content: Track \"{}\" by \"{}\" (ID: {:?})",
                                        track.item.title,
                                        artist_display,
                                        track.get_id()
                                    );
                                }
                            }
                            OutputType::Program => {
                                if let Some(program) = &state.program {
                                    info!(
                                        "New content: Program \"{}\" (ID: {:?})",
                                        program.name,
                                        program.get_id()
                                    );
                                }
                            }
                            OutputType::Station => {
                                let station_name = &state.station.as_ref().unwrap().name;
                                info!("New content: Station \"{}\"", station_name);
                            }
                        }

                        if let Err(e) = DlsService::update_output_file(&mut state) {
                            error!("Ticker: Failed to update output file: {}", e);
                        }

                        if let Err(e) = MotService::update_mot_output(&mut state, &mot_dir) {
                            error!("Ticker: Failed to update MOT output: {}", e);
                        }

                        previous_output_type = Some(current_output_type);
                        previous_content_id = current_content_id;
                    }

                    // Run cleanup for expired images periodically
                    // We'll do this on every Nth tick (according to CLEANUP_INTERVAL_TICKS)
                    if now.timestamp() % CLEANUP_INTERVAL_TICKS == 0 {
                        debug!("Ticker: Running image cleanup");
                        if let Err(e) = MotService::cleanup_expired_images(&mut state) {
                            error!("Ticker: Failed to clean up expired images: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Ticker: Failed to acquire lock on app state: {}", e);
                }
            }
        }
    }
}
