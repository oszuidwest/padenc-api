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
            // If expires_at is None, the track never expires
            if let Some(expires) = track.expires_at {
                if expires > now {
                    debug!("Track valid until {:?}", expires);
                    return OutputType::Track;
                }
                debug!("Track expired at {:?}, unsetting track data", expires);
                app_state.track = None;
            } else {
                debug!("Track has no expiration date, using track info");
                return OutputType::Track;
            }
        }

        // Then try program info
        if let Some(program) = &app_state.program {
            // If expires_at is None, the program never expires
            if let Some(expires) = program.expires_at {
                if expires > now {
                    debug!("Program valid until {:?}", expires);
                    return OutputType::Program;
                }
                debug!("Program expired at {:?}, unsetting program data", expires);
                app_state.program = None;
            } else {
                debug!("Program has no expiration date, using program info");
                return OutputType::Program;
            }
        }

        // Fall back to station info
        debug!("Using station info");
        OutputType::Station
    }
}
