use anyhow::{Error, Result};
use std::env;

#[derive(Clone)]
pub struct Config {
    pub station_name: String,
    pub api_key: Option<String>,
    pub default_station_image: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let station_name = env::var("STATION_NAME")
            .map_err(|_| Error::msg("STATION_NAME environment variable is required"))?;

        let api_key = env::var("API_KEY")
            .map_err(|_| Error::msg("API_KEY environment variable is required"))?;

        let default_station_image = env::var("DEFAULT_STATION_IMAGE").ok();

        Ok(Config {
            station_name,
            api_key: Some(api_key),
            default_station_image,
        })
    }
}
