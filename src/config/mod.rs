use crate::errors::{ServiceError, ServiceResult};
use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub station_name: String,
    pub api_key: String,
    pub default_station_image: Option<String>,
    pub image_dir: String,
    pub mot_dir: String,
    pub dls_file: String,
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

        let image_dir = env::var("PADENC_IMAGE_DIR").unwrap_or_else(|_| "/tmp/padenc/images".to_string());
        let mot_dir = env::var("PADENC_MOT_DIR").unwrap_or_else(|_| "/data/mot".to_string());
        let dls_file = env::var("PADENC_DLS_FILE").unwrap_or_else(|_| "/data/dls.txt".to_string());

        Ok(Config {
            station_name,
            api_key,
            default_station_image,
            image_dir,
            mot_dir,
            dls_file,
        })
    }
}
