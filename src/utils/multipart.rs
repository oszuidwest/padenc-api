use actix_multipart::Multipart;
use actix_web::{error::ErrorBadRequest, Error};
use futures::{StreamExt, TryStreamExt};
use log::{debug, error};
use serde::de::DeserializeOwned;
use serde_json;
use std::path::Path;

use crate::constants::form::IMAGE_FIELD;
use crate::constants::fs::IMAGE_DIR;
use crate::models::data::Image;
use crate::services::MotService;

pub async fn extract_json<T: DeserializeOwned>(
    field_name: &str,
    field: &mut actix_multipart::Field,
) -> Result<T, Error> {
    let mut data = Vec::new();
    
    while let Some(chunk) = field.next().await {
        data.extend_from_slice(&chunk.map_err(|_| {
            ErrorBadRequest(format!("Failed to read {}", field_name))
        })?);
    }
    
    let info_str = String::from_utf8(data).map_err(|_| 
        ErrorBadRequest(format!("Invalid UTF-8 in {}", field_name))
    )?;
    
    serde_json::from_str(&info_str).map_err(|_| 
        ErrorBadRequest(format!("Invalid {} format", field_name))
    )
}

pub async fn handle_multipart_upload<T: DeserializeOwned>(
    payload: Multipart,
    image_dir: Option<&Path>,
    info_field_name: &str,
) -> Result<(Option<T>, Option<Image>), Error> {
    let mut info: Option<T> = None;
    let mut image: Option<Image> = None;
    let mut payload = payload;
    
    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();
        let field_name = content_disposition.get_name().unwrap_or("");
        
        debug!("Processing multipart field: {}", field_name);
        
        if field_name == info_field_name {
            info = Some(extract_json(info_field_name, &mut field).await?);
        } else if field_name == IMAGE_FIELD {
            let mut image_data = Vec::new();
            let content_type = field.content_type().map(|ct| ct.to_string());
            
            while let Some(chunk) = field.next().await {
                let data = chunk.map_err(|e| 
                    ErrorBadRequest(format!("Failed to read image data: {:?}", e))
                )?;
                image_data.extend_from_slice(&data);
            }
            
            if !image_data.is_empty() && content_type.is_some() {
                let content_type_str = content_type.unwrap();
                if let Ok((path, filename)) = MotService::store_image(&image_data, &content_type_str, image_dir.unwrap_or_else(|| Path::new(IMAGE_DIR))).await {
                    image = Some(Image {
                        content_type: Some(content_type_str),
                        path: Some(path),
                        filename: Some(filename),
                    });
                } else {
                    return Err(ErrorBadRequest("Failed to process image upload"));
                }
            }
        } else {
            // Skip other form fields
            while let Some(chunk) = field.next().await {
                let _ = chunk?;
            }
        }
    }
    
    Ok((info, image))
}

pub fn cleanup_image(image_path: &Option<std::path::PathBuf>) {
    if let Some(path) = image_path {
        if path.exists() {
            if let Err(e) = std::fs::remove_file(path) {
                error!("Failed to remove image: {}", e);
            }
        }
    }
}