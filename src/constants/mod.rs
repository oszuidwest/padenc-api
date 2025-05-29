pub mod api {
    pub const DEFAULT_SERVER_PORT: &str = "8080";
    pub const AUTH_HEADER: &str = "authorization";
    pub const BEARER_PREFIX: &str = "Bearer ";
}

pub mod fs {
    pub const IMAGE_DIR: &str = "/tmp/padenc/images";
    pub const MOT_OUTPUT_DIR: &str = "/data/mot";
    pub const DLS_OUTPUT_FILE: &str = "/data/dls.txt";

    pub const SUPPORTED_MIME_TYPES: [&str; 2] = ["image/jpeg", "image/png"];

    pub mod extensions {
        pub const JPEG: &str = "jpg";
        pub const PNG: &str = "png";
    }
}

pub mod form {
    pub const TRACK_INFO_FIELD: &str = "track_info";
    pub const PROGRAM_INFO_FIELD: &str = "program_info";
    pub const IMAGE_FIELD: &str = "image";
}

pub mod ticker {
    pub const INTERVAL_MS: u64 = 50;
    pub const CLEANUP_INTERVAL_TICKS: i64 = 20;
}
