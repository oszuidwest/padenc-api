use actix_multipart::Multipart;
use actix_web::web;
use std::sync::Mutex;

use crate::config::Config;
use crate::handlers;
use crate::models::{AppState, data::{Track, Program}};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg
        .route(
            "/track",
            web::post().to(|payload: Option<Multipart>,
                             json: Option<web::Json<Track>>,
                             state: web::Data<Mutex<AppState>>,
                             config: web::Data<Config>| {
                handlers::track::post_track(payload, json, state, config)
            }),
        )
        .route("/track", web::delete().to(handlers::track::delete_track))
        .route(
            "/program",
            web::post().to(|payload: Option<Multipart>,
                             json: Option<web::Json<Program>>,
                             state: web::Data<Mutex<AppState>>,
                             config: web::Data<Config>| {
                handlers::program::post_program(payload, json, state, config)
            }),
        )
        .route("/program", web::delete().to(handlers::program::delete_program));
}

