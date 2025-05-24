use actix_web::{web, HttpResponse, Responder};
use crate::models::{AppState, data::Program};
use std::sync::Mutex;

pub async fn post_program(data: web::Json<Program>, state: web::Data<Mutex<AppState>>) -> impl Responder {
    let mut app_state = state.lock().unwrap();
    app_state.program = Some(data.into_inner());
    HttpResponse::Ok().body("Program state updated successfully")
}

pub async fn delete_program(state: web::Data<Mutex<AppState>>) -> impl Responder {
    let mut app_state = state.lock().unwrap();
    app_state.program = None;
    HttpResponse::Ok().body("Program state reset successfully")
}