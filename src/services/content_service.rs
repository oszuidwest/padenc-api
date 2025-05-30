use chrono::{DateTime, Utc};
use log::debug;

use crate::models::AppState;

#[derive(Debug, Clone, PartialEq)]
pub enum OutputType {
    Track,
    Program,
    Station,
}

pub struct ContentService;

impl ContentService {
    pub fn get_active_output_type(app_state: &mut AppState, now: DateTime<Utc>) -> OutputType {
        // Try to use valid track info first
        if let Some(track) = &app_state.track {
            if track.expires_at > now {
                debug!("Track valid until {}", track.expires_at);
                return OutputType::Track;
            }
            debug!("Track expired at {}, unsetting track data", track.expires_at);
            app_state.track = None;
        }

        // Then try program info
        if let Some(program) = &app_state.program {
            if program.expires_at > now {
                debug!("Program valid until {}", program.expires_at);
                return OutputType::Program;
            }
            debug!("Program expired at {}, unsetting program data", program.expires_at);
            app_state.program = None;
        }

        // Fall back to station info
        debug!("Using station info");
        OutputType::Station
    }
}
