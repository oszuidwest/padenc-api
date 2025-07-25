use chrono::Utc;
use log::debug;
use std::fs;
use std::io::Write;

use crate::constants::fs::DLS_OUTPUT_FILE;
use crate::errors::{ServiceError, ServiceResult};
use crate::models::tags::{ARTIST_TAG, PROGRAM_TAG, STATION_TAG, TITLE_TAG};
use crate::models::AppState;
use crate::services::ContentService;
use crate::services::content_service::OutputType;

pub struct DlsService;

impl DlsService {
    pub fn update_output_file(app_state: &mut AppState) -> ServiceResult<()> {
        let now = Utc::now();
        debug!("Checking content states at {}", now);

        let output_type = ContentService::get_active_output_type(app_state, now);
        
        let content = match output_type {
            OutputType::Track => {
                if let Some(track) = &app_state.track {
                    let title = &track.item.title;
                    let artist = track.item.artist.as_deref().unwrap_or("");
                    debug!("Writing track info: {} - {}", 
                        if artist.is_empty() { "(no artist)" } else { artist }, 
                        title
                    );
                    Self::generate_track_content(artist, title)
                } else {
                    let station_name = &app_state.station.as_ref().unwrap().name;
                    Self::generate_station_content(station_name)
                }
            },
            OutputType::Program => {
                if let Some(program) = &app_state.program {
                    let program_name = &program.name;
                    debug!("Writing program info: {}", program_name);
                    Self::generate_program_content(program_name)
                } else {
                    let station_name = &app_state.station.as_ref().unwrap().name;
                    Self::generate_station_content(station_name)
                }
            },
            OutputType::Station => {
                let station_name = &app_state.station.as_ref().unwrap().name;
                debug!("Writing station info: {}", station_name);
                Self::generate_station_content(station_name)
            }
        };
        
        Self::write_content_to_file(&content)
    }
    
    fn write_content_to_file(content: &str) -> ServiceResult<()> {
        let mut file = fs::File::create(DLS_OUTPUT_FILE).map_err(|e| {
            ServiceError::FileProcessing(format!("Failed to create output file: {}", e))
        })?;
        file.write_all(content.as_bytes()).map_err(|e| {
            ServiceError::FileProcessing(format!("Failed to write to output file: {}", e))
        })?;
        debug!(
            "Successfully wrote {} bytes to {}",
            content.len(),
            DLS_OUTPUT_FILE
        );

        Ok(())
    }

    pub fn generate_track_content(artist: &str, title: &str) -> String {
        // Handle empty artist case
        if artist.is_empty() {
            return format!(
                "##### parameters {{ #####\n\
                 DL_PLUS=1\n\
                 DL_PLUS_TAG={} 0 {}\n\
                 ##### parameters }} #####\n\
                 {}",
                TITLE_TAG,
                title.len(),
                title
            );
        }

        let separator = " - ";
        let display_text = format!("{}{}{}", artist, separator, title);

        let artist_start = 0;
        let artist_length = artist.len() as u32;
        let title_start = artist_length + separator.len() as u32;
        let title_length = title.len() as u32;

        format!(
            "##### parameters {{ #####\n\
             DL_PLUS=1\n\
             DL_PLUS_TAG={} {} {}\n\
             DL_PLUS_TAG={} {} {}\n\
             ##### parameters }} #####\n\
             {}",
            ARTIST_TAG,
            artist_start,
            artist_length,
            TITLE_TAG,
            title_start,
            title_length,
            display_text
        )
    }

    pub fn generate_program_content(program_name: &str) -> String {
        format!(
            "##### parameters {{ #####\n\
             DL_PLUS=1\n\
             DL_PLUS_TAG={} 0 {}\n\
             ##### parameters }} #####\n\
             {}",
            PROGRAM_TAG,
            program_name.len(),
            program_name
        )
    }

    pub fn generate_station_content(station_name: &str) -> String {
        format!(
            "##### parameters {{ #####\n\
             DL_PLUS=1\n\
             DL_PLUS_TAG={} 0 {}\n\
             ##### parameters }} #####\n\
             {}",
            STATION_TAG,
            station_name.len(),
            station_name
        )
    }
}
