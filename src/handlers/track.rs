use crate::constants::form;
use crate::handlers::shared;
use crate::models::{data::Track, AppState};
use crate::utils::cleanup::cleanup_optional_data_image;
use actix_multipart::Multipart;
use actix_web::{web, Error, HttpResponse};
use chrono::{DateTime, Utc};
use log::info;
use std::sync::Mutex;

#[derive(Debug, serde::Deserialize)]
pub struct TrackInfo {
    pub item: TrackItem,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, serde::Deserialize)]
pub struct TrackItem {
    pub title: String,
    pub artist: String,
}

pub async fn post_track(
    payload: Option<Multipart>,
    json: Option<web::Json<Track>>,
    state: web::Data<Mutex<AppState>>,
) -> Result<HttpResponse, Error> {
    let result = shared::process_content_update(
        payload,
        json,
        state,
        form::TRACK_INFO_FIELD,
        |info: TrackInfo, image| Track {
            id: uuid::Uuid::new_v4(),
            item: crate::models::data::Item {
                title: info.item.title,
                artist: info.item.artist,
            },
            expires_at: info.expires_at,
            image,
        },
        |app_state, track_data| {
            app_state.track = Some(track_data);
        },
        |app_state| {
            cleanup_optional_data_image(&app_state.track);
        },
    )
    .await?;
    info!("Track state updated successfully");
    Ok(result)
}

pub async fn delete_track(state: web::Data<Mutex<AppState>>) -> Result<HttpResponse, Error> {
    let result = shared::delete_content(
        state,
        |app_state| &app_state.track,
        |app_state| app_state.track = None,
    )
    .await?;

    info!("Track state reset successfully");
    Ok(result)
}
