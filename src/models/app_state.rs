use super::data::{Track, Program, Station};

#[derive(Debug, Clone)]
pub struct AppState {
    pub track: Option<Track>,
    pub program: Option<Program>,
    pub station: Option<Station>,
}