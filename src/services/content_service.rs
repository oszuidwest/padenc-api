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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::data::{Item, Program, Station, Track};
    use crate::models::AppState;
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    // Test helpers
    fn mk_station(name: &str) -> Station {
        Station {
            id: Uuid::new_v4(),
            name: name.into(),
            image: None,
        }
    }

    fn mk_track(title: &str, artist: Option<&str>, expires_at: Option<chrono::DateTime<Utc>>) -> Track {
        Track {
            id: Uuid::new_v4(),
            item: Item {
                title: title.into(),
                artist: artist.map(|s| s.into()),
            },
            expires_at,
            image: None,
        }
    }

    fn mk_program(name: &str, expires_at: Option<chrono::DateTime<Utc>>) -> Program {
        Program {
            id: Uuid::new_v4(),
            name: name.into(),
            expires_at,
            image: None,
        }
    }

    #[test]
    fn returns_track_when_track_has_no_expiration() {
        let now = Utc::now();

        let mut app = AppState::default();
        app.track = Some(mk_track("T1", None, None));
        app.program = None;
        app.station = Some(mk_station("S"));

        let out = ContentService::get_active_output_type(&mut app, now);
        assert_eq!(out, OutputType::Track);
        assert!(app.track.is_some(), "track should not be cleared when no expiration");
    }

    #[test]
    fn expired_track_cleared_and_program_chosen_if_valid() {
        let now = Utc::now();
        let expired_track = mk_track("T2", None, Some(now - Duration::seconds(10)));
        let program = mk_program("P1", Some(now + Duration::seconds(60)));

        let mut app = AppState::default();
        app.track = Some(expired_track);
        app.program = Some(program.clone());
        app.station = Some(mk_station("S"));

        let out = ContentService::get_active_output_type(&mut app, now);
        assert_eq!(out, OutputType::Program);
        assert!(app.track.is_none(), "expired track should be cleared");
        assert!(app.program.is_some(), "valid program should remain");
    }

    #[test]
    fn expired_program_cleared_and_station_chosen() {
        let now = Utc::now();
        let program = mk_program("P2", Some(now - Duration::seconds(5)));

        let mut app = AppState::default();
        app.track = None;
        app.program = Some(program);
        app.station = Some(mk_station("StationX"));

        let out = ContentService::get_active_output_type(&mut app, now);
        assert_eq!(out, OutputType::Station);
        assert!(app.program.is_none(), "expired program should be cleared");
    }

    #[test]
    fn returns_program_when_program_has_no_expiration() {
        let now = Utc::now();
        let program = mk_program("P3", None);

        let mut app = AppState::default();
        app.track = None;
        app.program = Some(program);
        app.station = Some(mk_station("StationY"));

        let out = ContentService::get_active_output_type(&mut app, now);
        assert_eq!(out, OutputType::Program);
        assert!(app.program.is_some(), "program without expiration should remain");
    }

    #[test]
    fn falls_back_to_station_when_no_track_or_program() {
        let now = Utc::now();
        let mut app = AppState::default();
        app.track = None;
        app.program = None;
        app.station = Some(mk_station("OnlyStation"));

        let out = ContentService::get_active_output_type(&mut app, now);
        assert_eq!(out, OutputType::Station);
    }
}
