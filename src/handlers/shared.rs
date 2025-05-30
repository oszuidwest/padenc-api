use actix_multipart::Multipart;
use actix_web::{web, Error, HttpResponse};
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::sync::Mutex;

use crate::constants::fs::IMAGE_DIR;
use crate::errors::ServiceError;
use crate::models::data::Image;
use crate::models::AppState;
use crate::utils::cleanup::{cleanup_optional_data_image, HasImage};
use crate::utils::multipart::handle_multipart_upload;
use std::path::Path;

pub async fn process_content_update<T, D>(
    payload: Option<Multipart>,
    json: Option<web::Json<D>>,
    state: web::Data<Mutex<AppState>>,
    field_name: &str,
    build_data_fn: impl FnOnce(T, Option<Image>) -> D,
    update_state_fn: impl FnOnce(&mut AppState, D),
    cleanup_fn: impl FnOnce(&AppState),
) -> Result<HttpResponse, Error>
where
    T: DeserializeOwned + Debug,
    D: Clone + Debug + HasImage,
{
    let image_dir = Path::new(IMAGE_DIR);

    let (content_info, content_image) = if let Some(mp_payload) = payload {
        handle_multipart_upload::<T>(mp_payload, Some(image_dir), field_name).await?
    } else {
        (None, None)
    };

    let content_data = if let Some(info) = content_info {
        build_data_fn(info, content_image)
    } else if let Some(json_data) = json {
        json_data.into_inner()
    } else {
        return Err(ServiceError::Validation("Missing content information".to_string()).into());
    };

    let mut app_state = state.lock().unwrap();
    cleanup_fn(&app_state);
    update_state_fn(&mut app_state, content_data);

    Ok(HttpResponse::Ok().body("Content updated successfully"))
}

pub async fn delete_content<T: HasImage>(
    state: web::Data<Mutex<AppState>>,
    get_content: impl FnOnce(&AppState) -> &Option<T>,
    update_state: impl FnOnce(&mut AppState),
) -> Result<HttpResponse, Error> {
    let mut app_state = state.lock().unwrap();

    let content = get_content(&app_state);
    cleanup_optional_data_image(content);

    update_state(&mut app_state);

    Ok(HttpResponse::Ok().body("Content reset successfully"))
}
