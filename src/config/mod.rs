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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn from_env_reads_required_and_defaults() {
        // Set required vars
        env::set_var("STATION_NAME", "MyStation");
        env::set_var("API_KEY", "secret");

        // Optional var
        env::set_var("DEFAULT_STATION_IMAGE", "default.png");

        // Ensure optional/defaulted vars are not set
        env::remove_var("PADENC_IMAGE_DIR");
        env::remove_var("PADENC_MOT_DIR");
        env::remove_var("PADENC_DLS_FILE");

        let cfg = Config::from_env().expect("should build config from env");

        assert_eq!(cfg.station_name, "MyStation");
        assert_eq!(cfg.api_key, "secret");
        assert_eq!(cfg.default_station_image.as_deref(), Some("default.png"));
        assert_eq!(cfg.image_dir, "/tmp/padenc/images");
        assert_eq!(cfg.mot_dir, "/data/mot");
        assert_eq!(cfg.dls_file, "/data/dls.txt");

        // Clean up
        env::remove_var("STATION_NAME");
        env::remove_var("API_KEY");
        env::remove_var("DEFAULT_STATION_IMAGE");
    }
}