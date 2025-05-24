use std::fs;
use std::io::{self, Write};
use chrono::Utc;
use log::{warn, debug};

use crate::models::AppState;
use crate::models::tags::{TITLE_TAG, ARTIST_TAG, PROGRAM_TAG, STATION_TAG};
use crate::config::Config;

pub struct FileService;

#[derive(Debug, Clone)]
pub enum OutputType {
    Track(String, String), // artist, title
    Program(String),       // program name
    Station(String),       // station name
}

impl FileService {
    pub fn update_output_file(app_state: &AppState, config: &Config) -> io::Result<()> {
        let now = Utc::now();
        debug!("Checking content states at {}", now);
        
        let output_type = Self::determine_output_type(app_state, config, now);
        Self::update_file_with_content(app_state, &output_type)
    }
    
    pub fn update_file_with_content(app_state: &AppState, output_type: &OutputType) -> io::Result<()> {
        let content = Self::generate_file_content(output_type);
        
        match output_type {
            OutputType::Track(artist, title) => {
                debug!("Writing track info: {} - {}", artist, title);
            },
            OutputType::Program(name) => {
                debug!("Writing program info: {}", name);
            },
            OutputType::Station(name) => {
                debug!("Writing station info: {}", name);
            },
        }
        
        let mut file = fs::File::create(&app_state.output_path)?;
        file.write_all(content.as_bytes())?;
        debug!("Successfully wrote {} bytes to {}", content.len(), app_state.output_path);
        
        Ok(())
    }
    
    pub fn determine_output_type(app_state: &AppState, config: &Config, now: chrono::DateTime<Utc>) -> OutputType {
        // Try to use valid track info first
        if let Some(track) = &app_state.track {
            if track.expires_at > now {
                debug!("Track valid until {}", track.expires_at);
                return OutputType::Track(track.item.artist.clone(), track.item.title.clone());
            }
            warn!("Track expired at {}", track.expires_at);
        }
        
        // Then try program info
        if let Some(program) = &app_state.program {
            if program.expires_at > now {
                debug!("Program valid until {}", program.expires_at);
                return OutputType::Program(program.name.clone());
            }
            warn!("Program expired at {}", program.expires_at);
        }
        
        // Fall back to station info
        debug!("Using station info");
        OutputType::Station(config.station_name.clone())
    }
    
    pub fn generate_file_content(output_type: &OutputType) -> String {
        match output_type {
            OutputType::Track(artist, title) => Self::generate_track_content(artist, title),
            OutputType::Program(name) => Self::generate_program_content(name),
            OutputType::Station(name) => Self::generate_station_content(name),
        }
    }

    fn generate_track_content(artist: &str, title: &str) -> String {
        let separator = " - ";
        let display_text = format!("{}{}{}", artist, separator, title);
        
        // Calculate tag positions
        let artist_start = 0;
        let artist_length = artist.len() as u32;
        let title_start = artist_length as u32 + separator.len() as u32;
        let title_length = title.len() as u32;
        
        format!(
            "##### parameters {{ #####\n\
             DL_PLUS=1\n\
             DL_PLUS_TAG={} {} {}\n\
             DL_PLUS_TAG={} {} {}\n\
             ##### parameters }} #####\n\
             {}",
            ARTIST_TAG, artist_start, artist_length,
            TITLE_TAG, title_start, title_length,
            display_text
        )
    }

    fn generate_program_content(program_name: &str) -> String {
        format!(
            "##### parameters {{ #####\n\
             DL_PLUS=1\n\
             DL_PLUS_TAG={} 0 {}\n\
             ##### parameters }} #####\n\
             {}",
            PROGRAM_TAG, program_name.len(),
            program_name
        )
    }

    fn generate_station_content(station_name: &str) -> String {
        format!(
            "##### parameters {{ #####\n\
             DL_PLUS=1\n\
             DL_PLUS_TAG={} 0 {}\n\
             ##### parameters }} #####\n\
             {}",
            STATION_TAG, station_name.len(),
            station_name
        )
    }
}