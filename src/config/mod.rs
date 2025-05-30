use crate::errors::{ServiceError, ServiceResult};
use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub station_name: String,
    pub api_key: String,
    pub default_station_image: Option<String>,
}

impl Config {
    pub fn from_env() -> ServiceResult<Self> {
        let station_name = env::var("STATION_NAME").map_err(|_| {
            ServiceError::Configuration("STATION_NAME environment variable is required".into())
        })?;

        let api_key = env::var("API_KEY").map_err(|_| {
            ServiceError::Configuration("API_KEY environment variable is required".into())
        })?;

        let default_station_image = env::var("DEFAULT_STATION_IMAGE").ok();

        Ok(Config {
            station_name,
            api_key,
            default_station_image,
        })
    }
}
