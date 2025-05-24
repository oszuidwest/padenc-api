use std::env;
use anyhow::Result;

#[derive(Clone)]
pub struct Config {
    pub station_name: String,
    pub output_file_path: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let station_name = env::var("STATION_NAME")
            .unwrap_or_else(|_| "Default Station".to_string());
        
        let output_file_path = env::var("OUTPUT_FILE_PATH")
            .unwrap_or_else(|_| "track.txt".to_string());
        
        Ok(Config {
            station_name,
            output_file_path,
        })
    }
}