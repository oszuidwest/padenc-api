use super::data::{Track, Program};

#[derive(Debug, Clone)]
pub struct AppState {
    pub track: Option<Track>,
    pub program: Option<Program>,
    pub output_path: String,
}