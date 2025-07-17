use actix_multipart::Multipart;
use chrono::Utc;
use futures::{StreamExt, TryStreamExt};
use log::{debug, error, info};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::constants::fs::{extensions, IMAGE_DIR, SUPPORTED_MIME_TYPES};
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
        image_data: &[u8],
        content_type: &str,
        image_dir: &Path,
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

    pub async fn process_upload(
        mut payload: Multipart,
        image_dir_path: Option<&Path>,
    ) -> ServiceResult<Image> {
        let image_dir = image_dir_path.unwrap_or_else(|| Path::new(IMAGE_DIR));
        let mut image_data = Vec::new();
        let mut content_type = None;

        while let Ok(Some(mut field)) = payload.try_next().await {
            let content_disposition = field.content_disposition();
            
            let field_name = content_disposition.get_name().unwrap_or("");
            if field_name == "image" {
                content_type = field.content_type().map(|ct| ct.to_string());

                while let Some(chunk) = field.next().await {
                    let data = chunk.map_err(|e| {
                        ServiceError::FileProcessing(format!("Upload error: {:?}", e))
                    })?;
                    image_data.extend_from_slice(&data);
                }
            }
        }

        if image_data.is_empty() || content_type.is_none() {
            return Err(ServiceError::Validation("Missing image data".into()));
        }

        let content_type_str = content_type.unwrap();
        let (path, filename) = Self::store_image(&image_data, &content_type_str, image_dir).await?;

        Ok(Image {
            content_type: Some(content_type_str),
            path: Some(path),
            filename: Some(filename),
        })
    }

    pub fn cleanup_expired_images(app_state: &mut AppState) -> ServiceResult<()> {
        let now = Utc::now();
        let image_dir = Path::new(IMAGE_DIR);
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

    pub fn update_mot_output(app_state: &mut AppState, mot_dir: &Path) -> ServiceResult<()> {
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
        default_station_image: &Option<String>,
    ) -> ServiceResult<Option<Image>> {
        let image_dir = Path::new(IMAGE_DIR);
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
            let (new_path, filename) = Self::store_image(&buffer, &content_type, image_dir).await?;

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