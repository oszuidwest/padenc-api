//! End-to-end HTTP tests for the track/program handlers, the route table in
//! `server::configure`, and the multipart upload parsing in `utils::multipart`.

use std::sync::Mutex;

use actix_web::{http::StatusCode, test, web, App};
use padenc_api::config::Config;
use padenc_api::models::AppState;
use padenc_api::server;
use tempfile::TempDir;

const BOUNDARY: &str = "TESTBOUNDARY123";

fn test_config(image_dir: &std::path::Path) -> Config {
    Config {
        station_name: "TestStation".into(),
        api_key: "test-key".into(),
        default_station_image: None,
        image_dir: image_dir.to_string_lossy().to_string(),
        mot_dir: "/tmp/padenc-test-mot".into(),
        dls_file: "/tmp/padenc-test-dls.txt".into(),
    }
}

/// Build a multipart/form-data body. Each part is (name, optional filename +
/// content-type, value bytes).
struct Part {
    name: &'static str,
    filename_and_ct: Option<(&'static str, &'static str)>,
    value: Vec<u8>,
}

fn build_multipart(parts: &[Part]) -> (String, Vec<u8>) {
    let mut body: Vec<u8> = Vec::new();
    for part in parts {
        body.extend_from_slice(format!("--{}\r\n", BOUNDARY).as_bytes());
        match part.filename_and_ct {
            Some((filename, ct)) => {
                body.extend_from_slice(
                    format!(
                        "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n",
                        part.name, filename
                    )
                    .as_bytes(),
                );
                body.extend_from_slice(format!("Content-Type: {}\r\n\r\n", ct).as_bytes());
            }
            None => {
                body.extend_from_slice(
                    format!("Content-Disposition: form-data; name=\"{}\"\r\n\r\n", part.name)
                        .as_bytes(),
                );
            }
        }
        body.extend_from_slice(&part.value);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{}--\r\n", BOUNDARY).as_bytes());
    (
        format!("multipart/form-data; boundary={}", BOUNDARY),
        body,
    )
}

/// Shared test harness: holds the state handle (for post-call inspection), the
/// config, and the temp image directory. Kept out of the app builder so tests
/// don't have to name actix's opaque `Service` type.
struct Harness {
    state: web::Data<Mutex<AppState>>,
    config: web::Data<Config>,
    _image_dir: TempDir,
    image_dir_path: std::path::PathBuf,
}

fn harness() -> Harness {
    let image_dir = TempDir::new().unwrap();
    let image_dir_path = image_dir.path().to_path_buf();
    let state = web::Data::new(Mutex::new(AppState::default()));
    let config = web::Data::new(test_config(image_dir.path()));
    Harness {
        state,
        config,
        _image_dir: image_dir,
        image_dir_path,
    }
}

/// Build the production-configured app for a harness. Returned as an owned value
/// so each test drives it with `test::call_service`.
macro_rules! app_for {
    ($h:expr) => {
        test::init_service(
            App::new()
                .app_data($h.state.clone())
                .app_data($h.config.clone())
                .configure(server::configure),
        )
        .await
    };
}

fn count_images(dir: &std::path::Path) -> usize {
    std::fs::read_dir(dir)
        .map(|entries| entries.filter_map(Result::ok).count())
        .unwrap_or(0)
}

/// Drive the service and return the resulting HTTP status. Handlers reject bad
/// input by returning an `Err`, which only becomes a response at the server
/// boundary, so `try_call_service` is used and the error is mapped to the
/// status it would produce.
async fn status_of<S, B>(app: &S, req: actix_http::Request) -> StatusCode
where
    S: actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse<B>,
        Error = actix_web::Error,
    >,
{
    match test::try_call_service(app, req).await {
        Ok(resp) => resp.status(),
        Err(e) => e.error_response().status(),
    }
}

// --- Track: JSON -------------------------------------------------------------

#[actix_web::test]
async fn post_track_json_sets_state() {
    let h = harness();
    let app = app_for!(h);

    let req = test::TestRequest::post()
        .uri("/track")
        .set_json(serde_json::json!({ "item": { "title": "Song", "artist": "Band" } }))
        .to_request();
        assert_eq!(status_of(&app, req).await, StatusCode::OK);

    let state = h.state.lock().unwrap();
    let track = state.track.as_ref().expect("track should be set");
    assert_eq!(track.item.title, "Song");
    assert_eq!(track.item.artist.as_deref(), Some("Band"));
}

#[actix_web::test]
async fn delete_track_clears_state() {
    let h = harness();
    let app = app_for!(h);

    // First set a track.
    let req = test::TestRequest::post()
        .uri("/track")
        .set_json(serde_json::json!({ "item": { "title": "Song" } }))
        .to_request();
    assert_eq!(status_of(&app, req).await, StatusCode::OK);

    // Then delete it.
    let req = test::TestRequest::delete().uri("/track").to_request();
    assert_eq!(status_of(&app, req).await, StatusCode::OK);

    assert!(h.state.lock().unwrap().track.is_none());
}

#[actix_web::test]
async fn post_track_without_body_is_bad_request() {
    let h = harness();
    let app = app_for!(h);
    // No multipart and no JSON -> handler returns a Validation error (400).
    let req = test::TestRequest::post()
        .uri("/track")
        .insert_header(("content-type", "text/plain"))
        .set_payload("nonsense")
        .to_request();
        assert_eq!(status_of(&app, req).await, StatusCode::BAD_REQUEST);
}

// --- Track: multipart --------------------------------------------------------

#[actix_web::test]
async fn post_track_multipart_with_image_stores_file() {
    let h = harness();
    let app = app_for!(h);

    let (ct, body) = build_multipart(&[
        Part {
            name: "track_info",
            filename_and_ct: None,
            value: br#"{"item":{"title":"MP Song","artist":"MP Band"}}"#.to_vec(),
        },
        Part {
            name: "image",
            filename_and_ct: Some(("pic.jpg", "image/jpeg")),
            value: b"\xff\xd8\xff fake jpeg".to_vec(),
        },
    ]);

    let req = test::TestRequest::post()
        .uri("/track")
        .insert_header(("content-type", ct))
        .set_payload(body)
        .to_request();
        assert_eq!(status_of(&app, req).await, StatusCode::OK);

    let state = h.state.lock().unwrap();
    let track = state.track.as_ref().expect("track set");
    assert_eq!(track.item.title, "MP Song");
    let image = track.image.as_ref().expect("image set");
    assert_eq!(image.content_type.as_deref(), Some("image/jpeg"));
    assert!(image.path.as_ref().unwrap().exists());
    assert_eq!(count_images(&h.image_dir_path), 1);
}

#[actix_web::test]
async fn post_track_multipart_without_image() {
    let h = harness();
    let app = app_for!(h);

    let (ct, body) = build_multipart(&[Part {
        name: "track_info",
        filename_and_ct: None,
        value: br#"{"item":{"title":"NoImg"}}"#.to_vec(),
    }]);

    let req = test::TestRequest::post()
        .uri("/track")
        .insert_header(("content-type", ct))
        .set_payload(body)
        .to_request();
    assert_eq!(status_of(&app, req).await, StatusCode::OK);

    let state = h.state.lock().unwrap();
    assert!(state.track.as_ref().unwrap().image.is_none());
    assert_eq!(count_images(&h.image_dir_path), 0);
}

#[actix_web::test]
async fn post_track_multipart_empty_image_part_is_ignored() {
    let h = harness();
    let app = app_for!(h);

    // An image field with an empty body must not produce a stored image
    // (exercises the empty-data guard).
    let (ct, body) = build_multipart(&[
        Part {
            name: "track_info",
            filename_and_ct: None,
            value: br#"{"item":{"title":"EmptyImg"}}"#.to_vec(),
        },
        Part {
            name: "image",
            filename_and_ct: Some(("empty.jpg", "image/jpeg")),
            value: Vec::new(),
        },
    ]);

    let req = test::TestRequest::post()
        .uri("/track")
        .insert_header(("content-type", ct))
        .set_payload(body)
        .to_request();
    assert_eq!(status_of(&app, req).await, StatusCode::OK);

    let state = h.state.lock().unwrap();
    assert!(state.track.as_ref().unwrap().image.is_none());
    assert_eq!(count_images(&h.image_dir_path), 0);
}

#[actix_web::test]
async fn post_track_multipart_invalid_json_is_bad_request() {
    let h = harness();
    let app = app_for!(h);

    let (ct, body) = build_multipart(&[Part {
        name: "track_info",
        filename_and_ct: None,
        value: b"{ this is not json".to_vec(),
    }]);

    let req = test::TestRequest::post()
        .uri("/track")
        .insert_header(("content-type", ct))
        .set_payload(body)
        .to_request();
        assert_eq!(status_of(&app, req).await, StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn post_track_multipart_invalid_utf8_is_bad_request() {
    let h = harness();
    let app = app_for!(h);

    // Invalid UTF-8 bytes in the track_info field exercise the UTF-8 error
    // branch of `extract_json`.
    let (ct, body) = build_multipart(&[Part {
        name: "track_info",
        filename_and_ct: None,
        value: vec![0xff, 0xfe, 0xfd],
    }]);

    let req = test::TestRequest::post()
        .uri("/track")
        .insert_header(("content-type", ct))
        .set_payload(body)
        .to_request();
    assert_eq!(status_of(&app, req).await, StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn post_track_multipart_invalid_image_type_is_bad_request() {
    let h = harness();
    let app = app_for!(h);

    let (ct, body) = build_multipart(&[
        Part {
            name: "track_info",
            filename_and_ct: None,
            value: br#"{"item":{"title":"X"}}"#.to_vec(),
        },
        Part {
            name: "image",
            filename_and_ct: Some(("pic.gif", "image/gif")),
            value: b"GIF89a fake".to_vec(),
        },
    ]);

    let req = test::TestRequest::post()
        .uri("/track")
        .insert_header(("content-type", ct))
        .set_payload(body)
        .to_request();
        assert_eq!(status_of(&app, req).await, StatusCode::BAD_REQUEST);
    // No image should have been persisted.
    assert_eq!(count_images(&h.image_dir_path), 0);
}

#[actix_web::test]
async fn post_track_multipart_ignores_unknown_fields() {
    let h = harness();
    let app = app_for!(h);

    let (ct, body) = build_multipart(&[
        Part {
            name: "extra",
            filename_and_ct: None,
            value: b"ignored".to_vec(),
        },
        Part {
            name: "track_info",
            filename_and_ct: None,
            value: br#"{"item":{"title":"WithExtra"}}"#.to_vec(),
        },
    ]);

    let req = test::TestRequest::post()
        .uri("/track")
        .insert_header(("content-type", ct))
        .set_payload(body)
        .to_request();
    assert_eq!(status_of(&app, req).await, StatusCode::OK);
    assert_eq!(
        h.state.lock().unwrap().track.as_ref().unwrap().item.title,
        "WithExtra"
    );
}

#[actix_web::test]
async fn overwriting_track_cleans_up_previous_image() {
    let h = harness();
    let app = app_for!(h);

    // First upload with image A.
    let (ct, body) = build_multipart(&[
        Part {
            name: "track_info",
            filename_and_ct: None,
            value: br#"{"item":{"title":"First"}}"#.to_vec(),
        },
        Part {
            name: "image",
            filename_and_ct: Some(("a.jpg", "image/jpeg")),
            value: b"image A".to_vec(),
        },
    ]);
    let req = test::TestRequest::post()
        .uri("/track")
        .insert_header(("content-type", ct))
        .set_payload(body)
        .to_request();
    assert_eq!(status_of(&app, req).await, StatusCode::OK);
    let first_path = h
        .state
        .lock()
        .unwrap()
        .track
        .as_ref()
        .unwrap()
        .image
        .as_ref()
        .unwrap()
        .path
        .clone()
        .unwrap();
    assert!(first_path.exists());
    assert_eq!(count_images(&h.image_dir_path), 1);

    // Second upload with image B should clean up image A.
    let (ct, body) = build_multipart(&[
        Part {
            name: "track_info",
            filename_and_ct: None,
            value: br#"{"item":{"title":"Second"}}"#.to_vec(),
        },
        Part {
            name: "image",
            filename_and_ct: Some(("b.jpg", "image/jpeg")),
            value: b"image B".to_vec(),
        },
    ]);
    let req = test::TestRequest::post()
        .uri("/track")
        .insert_header(("content-type", ct))
        .set_payload(body)
        .to_request();
    assert_eq!(status_of(&app, req).await, StatusCode::OK);

    assert!(!first_path.exists(), "previous image should be cleaned up");
    assert_eq!(count_images(&h.image_dir_path), 1);
}

// --- Program -----------------------------------------------------------------

#[actix_web::test]
async fn post_program_json_sets_state() {
    let h = harness();
    let app = app_for!(h);

    let req = test::TestRequest::post()
        .uri("/program")
        .set_json(serde_json::json!({ "name": "Morning Show" }))
        .to_request();
    assert_eq!(status_of(&app, req).await, StatusCode::OK);
    assert_eq!(
        h.state.lock().unwrap().program.as_ref().unwrap().name,
        "Morning Show"
    );
}

#[actix_web::test]
async fn post_program_multipart_with_image() {
    let h = harness();
    let app = app_for!(h);

    let (ct, body) = build_multipart(&[
        Part {
            name: "program_info",
            filename_and_ct: None,
            value: br#"{"name":"MP Program"}"#.to_vec(),
        },
        Part {
            name: "image",
            filename_and_ct: Some(("p.png", "image/png")),
            value: b"\x89PNG fake".to_vec(),
        },
    ]);

    let req = test::TestRequest::post()
        .uri("/program")
        .insert_header(("content-type", ct))
        .set_payload(body)
        .to_request();
    assert_eq!(status_of(&app, req).await, StatusCode::OK);

    let state = h.state.lock().unwrap();
    let program = state.program.as_ref().unwrap();
    assert_eq!(program.name, "MP Program");
    assert_eq!(
        program.image.as_ref().unwrap().content_type.as_deref(),
        Some("image/png")
    );
    assert_eq!(count_images(&h.image_dir_path), 1);
}

#[actix_web::test]
async fn delete_program_clears_state() {
    let h = harness();
    let app = app_for!(h);

    let req = test::TestRequest::post()
        .uri("/program")
        .set_json(serde_json::json!({ "name": "Show" }))
        .to_request();
    assert_eq!(status_of(&app, req).await, StatusCode::OK);

    let req = test::TestRequest::delete().uri("/program").to_request();
    assert_eq!(status_of(&app, req).await, StatusCode::OK);
    assert!(h.state.lock().unwrap().program.is_none());
}

#[actix_web::test]
async fn post_program_without_body_is_bad_request() {
    let h = harness();
    let app = app_for!(h);
    let req = test::TestRequest::post()
        .uri("/program")
        .insert_header(("content-type", "text/plain"))
        .set_payload("nonsense")
        .to_request();
    assert_eq!(
        status_of(&app, req).await,
        StatusCode::BAD_REQUEST
    );
}
