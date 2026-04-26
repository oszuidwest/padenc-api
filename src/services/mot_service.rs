use chrono::Utc;
use log::{debug, error, info};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::constants::mime::{extensions, SUPPORTED_MIME_TYPES};
use crate::errors::{ServiceError, ServiceResult};
use crate::models::data::Image;
use crate::models::AppState;
use crate::services::content_service::OutputType;
use crate::services::ContentService;

pub struct MotService;

impl MotService {
    pub fn init(image_dir: &Path) -> ServiceResult<()> {
        fs::create_dir_all(image_dir)
            .map_err(|e| ServiceError::FileProcessing(format!("Failed to create image directory: {}", e)))?;
        Ok(())
    }

    pub async fn store_image(
        image_dir: &Path,
        content_type: &str,
        image_data: &[u8],
    ) -> ServiceResult<(PathBuf, String)> {
        if !Self::is_valid_image_type(content_type) {
            return Err(ServiceError::Validation(
                "Invalid image format. Supported formats: JPEG, PNG".into(),
            ));
        }

        let file_extension = match content_type {
            "image/jpeg" => extensions::JPEG,
            "image/png" => extensions::PNG,
            _ => {
                return Err(ServiceError::Validation(
                    "Unsupported image format. Only JPEG and PNG are supported".into(),
                ))
            }
        };

        let filename = format!("{}.{}", Uuid::new_v4(), file_extension);
        let file_path = image_dir.join(&filename);

        let mut file = File::create(&file_path)
            .map_err(|e| ServiceError::FileProcessing(format!("Failed to create image file: {}", e)))?;
        file.write_all(image_data)
            .map_err(|e| ServiceError::FileProcessing(format!("Failed to write image data: {}", e)))?;

        debug!("Stored image at {:?}", file_path);

        Ok((file_path, filename))
    }

    pub fn cleanup_expired_images(image_dir: &Path, app_state: &mut AppState) -> ServiceResult<()> {
        let now = Utc::now();
        let mut active_images = Vec::new();

        // Call get_active_output_type to auto-unset expired tracks/programs
        ContentService::get_active_output_type(app_state, now);
        
        // Collect paths of active images (now we know they're not expired)
        if let Some(track) = &app_state.track {
            if let Some(image) = &track.image {
                if let Some(path) = &image.path {
                    active_images.push(path.clone());
                }
            }
        }

        if let Some(program) = &app_state.program {
            if let Some(image) = &program.image {
                if let Some(path) = &image.path {
                    active_images.push(path.clone());
                }
            }
        }

        if let Some(station) = &app_state.station {
            if let Some(image) = &station.image {
                if let Some(path) = &image.path {
                    active_images.push(path.clone());
                }
            }
        }

        // Read image directory and delete unused images
        if let Ok(entries) = fs::read_dir(image_dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();

                // Skip if not a file or not an image
                if !path.is_file() || !Self::is_image_file(&path) {
                    continue;
                }

                // Check if this image is still active
                if !active_images.contains(&path) {
                    debug!("Removing expired image: {:?}", path);
                    if let Err(e) = fs::remove_file(&path) {
                        error!("Failed to delete expired image {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(())
    }

    pub fn is_valid_image_type(content_type: &str) -> bool {
        SUPPORTED_MIME_TYPES.contains(&content_type)
    }

    fn is_image_file(path: &Path) -> bool {
        if let Some(extension) = path.extension() {
            let ext = extension.to_string_lossy().to_lowercase();
            return ext == "jpg" || ext == "jpeg" || ext == "png";
        }
        false
    }

    pub fn init_mot_dir(mot_dir: &Path) -> ServiceResult<()> {
        fs::create_dir_all(mot_dir)
            .map_err(|e| ServiceError::FileProcessing(format!("Failed to create MOT directory: {}", e)))?;

        if let Ok(entries) = fs::read_dir(mot_dir) {
            for entry in entries.filter_map(Result::ok) {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        if let Err(e) = fs::remove_file(entry.path()) {
                            error!("Failed to clean existing MOT file: {}", e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn get_active_image_with_fallback<'a>(app_state: &'a AppState, output_type: &OutputType) -> Option<&'a Image> {
        match output_type {
            OutputType::Track => {
                Self::get_active_image(app_state, &OutputType::Track)
                    .or_else(|| Self::get_active_image(app_state, &OutputType::Program))
                    .or_else(|| Self::get_active_image(app_state, &OutputType::Station))
            }
            OutputType::Program => {
                Self::get_active_image(app_state, &OutputType::Program)
                    .or_else(|| Self::get_active_image(app_state, &OutputType::Station))
            }
            OutputType::Station => Self::get_active_image(app_state, &OutputType::Station),
        }
    }

    pub fn update_mot_output(mot_dir: &Path, app_state: &mut AppState) -> ServiceResult<()> {
        let now = Utc::now();

        // Get the active output type from ContentService
        let output_type = ContentService::get_active_output_type(app_state, now);

        // Get the active image based on the output type, with fallback
        let image_path =
            Self::get_active_image_with_fallback(app_state, &output_type).and_then(|img| img.path.clone());

        // Clean current MOT directory first
        Self::init_mot_dir(mot_dir)?;

        if let Some(path) = image_path {
            debug!("Using image for MOT: {:?}", path);

            // Get the filename or generate a new one
            let filename = path
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| format!("slide-{}.jpg", Utc::now().timestamp()));

            // Copy the active image to the MOT directory
            let mot_file_path = mot_dir.join(&filename);

            fs::copy(&path, &mot_file_path)
                .map_err(|e| ServiceError::FileProcessing(format!("Failed to update MOT image: {}", e)))?;

            debug!("Updated MOT image at {:?}", mot_file_path);
            Ok(())
        } else {
            debug!("No image available for MOT, MOT directory is empty");
            Ok(())
        }
    }

    pub fn get_active_image<'a>(
        app_state: &'a AppState,
        output_type: &OutputType,
    ) -> Option<&'a Image> {
        match output_type {
            OutputType::Track => app_state
                .track
                .as_ref()
                .and_then(|track| track.image.as_ref()),
            OutputType::Program => app_state
                .program
                .as_ref()
                .and_then(|program| program.image.as_ref()),
            OutputType::Station => app_state
                .station
                .as_ref()
                .and_then(|station| station.image.as_ref()),
        }
    }

    pub async fn load_station_image(
        image_dir: &Path,
        default_station_image: &Option<String>,
    ) -> ServiceResult<Option<Image>> {
        if let Some(image_path) = default_station_image {
            let path = PathBuf::from(image_path);

            if !path.exists() {
                return Err(ServiceError::NotFound(format!(
                    "Default station image not found at {:?}",
                    path
                )));
            }

            let mut file = File::open(&path)
                .map_err(|e| ServiceError::FileProcessing(format!("Failed to open station image: {}", e)))?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)
                .map_err(|e| ServiceError::FileProcessing(format!("Failed to read station image: {}", e)))?;

            let content_type = Self::detect_mime_type(&path)?.to_string();

            // Copy to image directory with new UUID
            let (new_path, filename) = Self::store_image(image_dir, &content_type, &buffer).await?;

            info!("Loaded default station image: {}", filename);

            Ok(Some(Image {
                content_type: Some(content_type),
                path: Some(new_path),
                filename: Some(filename),
            }))
        } else {
            Ok(None)
        }
    }

    fn detect_mime_type(path: &Path) -> ServiceResult<&'static str> {
        if let Some(ext) = path.extension() {
            match ext.to_string_lossy().to_lowercase().as_str() {
                "jpg" | "jpeg" => return Ok("image/jpeg"),
                "png" => return Ok("image/png"),
                _ => {}
            }
        }

        Err(ServiceError::Validation(
            "Unsupported image format. Only JPEG and PNG are supported".into(),
        ))
    }
}

// TODO: AI heeft het onderstaande geschreven. Nog helemaal nalopen!
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;
    use crate::models::data::{Image, Station, Track, Program, Item};
    use crate::models::AppState;
    use chrono::{Utc, Duration};
    use uuid::Uuid;

    #[test]
    fn init_creates_directory() {
        let temp_dir = tempdir().unwrap();
        let sub_dir = temp_dir.path().join("images");
        assert!(!sub_dir.exists());
        MotService::init(&sub_dir).unwrap();
        assert!(sub_dir.exists());
    }

    #[tokio::test]
    async fn store_image_creates_file_with_correct_content_jpeg() {
        let temp_dir = tempdir().unwrap();
        let image_data = b"fake jpeg data";
        let (path, filename) = MotService::store_image(&temp_dir.path(), "image/jpeg", image_data).await.unwrap();
        assert!(path.exists());
        assert_eq!(fs::read(&path).unwrap(), image_data);
        assert!(filename.ends_with(".jpg"));
    }

    #[tokio::test]
    async fn store_image_creates_file_png() {
        let temp_dir = tempdir().unwrap();
        let image_data = b"fake png data";
        let (path, filename) = MotService::store_image(&temp_dir.path(), "image/png", image_data).await.unwrap();
        assert!(path.exists());
        assert_eq!(fs::read(&path).unwrap(), image_data);
        assert!(filename.ends_with(".png"));
    }

    #[tokio::test]
    async fn store_image_invalid_type() {
        let temp_dir = tempdir().unwrap();
        let result = MotService::store_image(&temp_dir.path(), "image/gif", b"data").await;
        assert!(result.is_err());
    }

    #[test]
    fn is_valid_image_type_accepts_supported() {
        assert!(MotService::is_valid_image_type("image/jpeg"));
        assert!(MotService::is_valid_image_type("image/png"));
        assert!(!MotService::is_valid_image_type("image/gif"));
        assert!(!MotService::is_valid_image_type("text/plain"));
    }

    #[test]
    fn cleanup_expired_images_removes_expired_and_keeps_active() {
        let temp_dir = tempdir().unwrap();
        let expired_img_path = temp_dir.path().join("expired.jpg");
        fs::write(&expired_img_path, b"expired").unwrap();
        let active_img_path = temp_dir.path().join("active.jpg");
        fs::write(&active_img_path, b"active").unwrap();
        let non_img_path = temp_dir.path().join("nonimg.txt");
        fs::write(&non_img_path, b"nonimg").unwrap();

        let mut app = AppState::default();
        let expired_track = Track {
            id: Uuid::new_v4(),
            item: Item { title: "Expired".into(), artist: None },
            expires_at: Some(Utc::now() - Duration::seconds(1)),
            image: Some(Image { content_type: Some("image/jpeg".into()), path: Some(expired_img_path.clone()), filename: Some("expired.jpg".into()) }),
        };
        app.track = Some(expired_track);
        let active_program = Program {
            id: Uuid::new_v4(),
            name: "Active".into(),
            expires_at: None,
            image: Some(Image { content_type: Some("image/jpeg".into()), path: Some(active_img_path.clone()), filename: Some("active.jpg".into()) }),
        };
        app.program = Some(active_program);

        MotService::cleanup_expired_images(&temp_dir.path(), &mut app).unwrap();

        assert!(!expired_img_path.exists());
        assert!(active_img_path.exists());
        assert!(non_img_path.exists()); // non-image should remain
        assert!(app.track.is_none()); // expired track unset
    }

    #[test]
    fn init_mot_dir_creates_and_cleans() {
        let temp_dir = tempdir().unwrap();
        let sub_dir = temp_dir.path().join("mot");
        fs::create_dir(&sub_dir).unwrap();
        let file_path = sub_dir.join("old.jpg");
        fs::write(&file_path, b"old").unwrap();
        assert!(file_path.exists());
        MotService::init_mot_dir(&sub_dir).unwrap();
        assert!(!file_path.exists());
    }

    #[test]
    fn update_mot_output_copies_active_image() {
        let temp_dir = tempdir().unwrap();
        let mot_dir = temp_dir.path().join("mot");
        let img_dir = temp_dir.path().join("img");
        fs::create_dir(&img_dir).unwrap();
        let img_path = img_dir.join("test.jpg");
        fs::write(&img_path, b"image").unwrap();

        let mut app = AppState::default();
        let station = Station {
            id: Uuid::new_v4(),
            name: "Station".into(),
            image: Some(Image { content_type: Some("image/jpeg".into()), path: Some(img_path.clone()), filename: Some("test.jpg".into()) }),
        };
        app.station = Some(station);

        MotService::update_mot_output(&mot_dir, &mut app).unwrap();

        let entries: Vec<_> = fs::read_dir(&mot_dir).unwrap().map(|e| e.unwrap().path()).collect();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].file_name().unwrap().to_str().unwrap().contains("test.jpg"));
        assert_eq!(fs::read(&entries[0]).unwrap(), b"image");
    }

    #[test]
    fn update_mot_output_no_image_leaves_empty() {
        let temp_dir = tempdir().unwrap();
        let mot_dir = temp_dir.path().join("mot");
        let mut app = AppState::default();
        let station = Station {
            id: Uuid::new_v4(),
            name: "Station".into(),
            image: None,
        };
        app.station = Some(station);

        MotService::update_mot_output(&mot_dir, &mut app).unwrap();

        let entries: Vec<_> = fs::read_dir(&mot_dir).unwrap().collect();
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn get_active_image_returns_correct() {
        let mut app = AppState::default();
        let img = Image { content_type: None, path: None, filename: None };
        let track = Track {
            id: Uuid::new_v4(),
            item: Item { title: "T".into(), artist: None },
            expires_at: None,
            image: Some(img.clone()),
        };
        app.track = Some(track);
        assert!(MotService::get_active_image(&app, &OutputType::Track).is_some());
        assert!(MotService::get_active_image(&app, &OutputType::Program).is_none());
        assert!(MotService::get_active_image(&app, &OutputType::Station).is_none());
    }

    #[tokio::test]
    async fn load_station_image_loads_and_stores() {
        let temp_dir = tempdir().unwrap();
        let img_dir = temp_dir.path().join("img");
        fs::create_dir(&img_dir).unwrap();
        let default_img_path = temp_dir.path().join("default.jpg");
        fs::write(&default_img_path, b"default image").unwrap();

        let result = MotService::load_station_image(&img_dir, &Some(default_img_path.to_string_lossy().to_string())).await.unwrap();
        assert!(result.is_some());
        let image = result.unwrap();
        assert!(image.path.as_ref().unwrap().exists());
        assert_eq!(fs::read(image.path.unwrap()).unwrap(), b"default image");
    }

    #[tokio::test]
    async fn load_station_image_none_returns_none() {
        let temp_dir = tempdir().unwrap();
        let img_dir = temp_dir.path().join("img");
        fs::create_dir(&img_dir).unwrap();

        let result = MotService::load_station_image(&img_dir, &None).await.unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn detect_mime_type_detects_correctly() {
        let temp_dir = tempdir().unwrap();
        let jpg_path = temp_dir.path().join("test.jpg");
        assert_eq!(MotService::detect_mime_type(&jpg_path).unwrap(), "image/jpeg");
        let jpeg_path = temp_dir.path().join("test.jpeg");
        assert_eq!(MotService::detect_mime_type(&jpeg_path).unwrap(), "image/jpeg");
        let png_path = temp_dir.path().join("test.png");
        assert_eq!(MotService::detect_mime_type(&png_path).unwrap(), "image/png");
        let txt_path = temp_dir.path().join("test.txt");
        assert!(MotService::detect_mime_type(&txt_path).is_err());
    }
}
