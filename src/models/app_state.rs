use super::data::{Track, Program, Image};

#[derive(Debug, Clone)]
pub struct AppState {
    pub track: Option<Track>,
    pub program: Option<Program>,
    pub station_image: Option<Image>,
}