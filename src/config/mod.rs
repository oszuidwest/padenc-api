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
        Self::from_lookup(|key| env::var(key).ok())
    }

    /// Build a `Config` from an arbitrary key -> value lookup. Keeping the
    /// parsing logic independent of `std::env` makes it deterministically
    /// testable without mutating process-global state.
    fn from_lookup<F>(lookup: F) -> ServiceResult<Self>
    where
        F: Fn(&str) -> Option<String>,
    {
        let station_name = lookup("STATION_NAME").ok_or_else(|| {
            ServiceError::Configuration("STATION_NAME environment variable is required".into())
        })?;

        let api_key = lookup("API_KEY").ok_or_else(|| {
            ServiceError::Configuration("API_KEY environment variable is required".into())
        })?;

        let default_station_image = lookup("DEFAULT_STATION_IMAGE");

        let image_dir = lookup("PADENC_IMAGE_DIR").unwrap_or_else(|| "/tmp/padenc/images".to_string());
        let mot_dir = lookup("PADENC_MOT_DIR").unwrap_or_else(|| "/data/mot".to_string());
        let dls_file = lookup("PADENC_DLS_FILE").unwrap_or_else(|| "/data/dls.txt".to_string());

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
    use serial_test::serial;
    use std::collections::HashMap;
    use std::env;

    /// A lookup backed by a fixed map — deterministic and free of any
    /// process-global state.
    fn map_lookup(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> =
            pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        move |key: &str| map.get(key).cloned()
    }

    #[test]
    fn missing_station_name_is_configuration_error() {
        let err = Config::from_lookup(map_lookup(&[])).unwrap_err();
        match err {
            ServiceError::Configuration(msg) => assert!(msg.contains("STATION_NAME")),
            other => panic!("expected STATION_NAME configuration error, got {:?}", other),
        }
    }

    #[test]
    fn missing_api_key_is_configuration_error() {
        let err = Config::from_lookup(map_lookup(&[("STATION_NAME", "MyStation")])).unwrap_err();
        match err {
            ServiceError::Configuration(msg) => assert!(msg.contains("API_KEY")),
            other => panic!("expected API_KEY configuration error, got {:?}", other),
        }
    }

    #[test]
    fn required_set_with_defaults_for_the_rest() {
        let cfg = Config::from_lookup(map_lookup(&[
            ("STATION_NAME", "MyStation"),
            ("API_KEY", "secret"),
            ("DEFAULT_STATION_IMAGE", "default.png"),
        ]))
        .expect("should build config");
        assert_eq!(cfg.station_name, "MyStation");
        assert_eq!(cfg.api_key, "secret");
        assert_eq!(cfg.default_station_image.as_deref(), Some("default.png"));
        assert_eq!(cfg.image_dir, "/tmp/padenc/images");
        assert_eq!(cfg.mot_dir, "/data/mot");
        assert_eq!(cfg.dls_file, "/data/dls.txt");
    }

    #[test]
    fn optional_image_absent_and_dir_overrides_applied() {
        let cfg = Config::from_lookup(map_lookup(&[
            ("STATION_NAME", "S"),
            ("API_KEY", "k"),
            ("PADENC_IMAGE_DIR", "/custom/img"),
            ("PADENC_MOT_DIR", "/custom/mot"),
            ("PADENC_DLS_FILE", "/custom/dls.txt"),
        ]))
        .expect("should build config");
        assert!(cfg.default_station_image.is_none());
        assert_eq!(cfg.image_dir, "/custom/img");
        assert_eq!(cfg.mot_dir, "/custom/mot");
        assert_eq!(cfg.dls_file, "/custom/dls.txt");
    }

    /// Smoke test for the real `from_env` delegation. `#[serial]` keeps it from
    /// racing with any other test that touches process environment variables.
    #[test]
    #[serial]
    fn from_env_reads_process_environment() {
        for k in ["DEFAULT_STATION_IMAGE", "PADENC_IMAGE_DIR", "PADENC_MOT_DIR", "PADENC_DLS_FILE"] {
            env::remove_var(k);
        }
        env::set_var("STATION_NAME", "EnvStation");
        env::set_var("API_KEY", "env-secret");

        let cfg = Config::from_env().expect("should build config from env");
        assert_eq!(cfg.station_name, "EnvStation");
        assert_eq!(cfg.api_key, "env-secret");
        assert_eq!(cfg.image_dir, "/tmp/padenc/images");

        env::remove_var("STATION_NAME");
        env::remove_var("API_KEY");
    }
}