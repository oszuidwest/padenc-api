use crate::constants::form;
use crate::handlers::shared::{self};
use crate::models::{data::Program, AppState};
use crate::utils::cleanup::cleanup_optional_data_image;
use actix_multipart::Multipart;
use actix_web::{web, Error, HttpResponse};
use chrono::{DateTime, Utc};
use log::info;
use std::sync::Mutex;

#[derive(Debug, serde::Deserialize)]
pub struct ProgramInfo {
    pub name: String,
    pub expires_at: DateTime<Utc>,
}

pub async fn post_program(
    payload: Option<Multipart>,
    json: Option<web::Json<Program>>,
    state: web::Data<Mutex<AppState>>,
) -> Result<HttpResponse, Error> {
    let result = shared::process_content_update(
        payload,
        json,
        state,
        form::PROGRAM_INFO_FIELD,
        |info: ProgramInfo, image| Program {
            id: uuid::Uuid::new_v4(),
            name: info.name,
            expires_at: info.expires_at,
            image,
        },
        |app_state, program_data| {
            app_state.program = Some(program_data);
        },
        |app_state| {
            cleanup_optional_data_image(&app_state.program);
        },
    )
    .await?;

    info!("Program state updated successfully");
    Ok(result)
}

pub async fn delete_program(state: web::Data<Mutex<AppState>>) -> Result<HttpResponse, Error> {
    let result = shared::delete_content(
        state,
        |app_state| &app_state.program,
        |app_state| app_state.program = None,
    )
    .await?;

    info!("Program state reset successfully");
    Ok(result)
}
