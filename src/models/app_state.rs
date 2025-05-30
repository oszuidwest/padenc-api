use super::data::{Program, Station, Track};

#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub track: Option<Track>,
    pub program: Option<Program>,
    pub station: Option<Station>,
}