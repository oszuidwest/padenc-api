use actix_web::{web, HttpResponse, Responder};
use crate::models::{AppState, data::Track};
use std::sync::Mutex;

pub async fn post_track(data: web::Json<Track>, state: web::Data<Mutex<AppState>>) -> impl Responder {
    let mut app_state = state.lock().unwrap();
    app_state.track = Some(data.into_inner());
    HttpResponse::Ok().body("Track state updated successfully")
}

pub async fn delete_track(state: web::Data<Mutex<AppState>>) -> impl Responder {
    let mut app_state = state.lock().unwrap();
    app_state.track = None;
    HttpResponse::Ok().body("Track state reset successfully")
}