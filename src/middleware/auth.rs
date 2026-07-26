use crate::config::Config;
use crate::constants::api::{AUTH_HEADER, BEARER_PREFIX};
use crate::errors::ServiceError;
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http::header::HeaderName,
    web, Error,
};
use futures_core::future::LocalBoxFuture;
use log::{debug, error};
use std::future::{ready, Ready};

pub struct Auth;

impl<S, B> Transform<S, ServiceRequest> for Auth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddleware { service }))
    }
}

pub struct AuthMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for AuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Get config from application data
        let config = match req.app_data::<web::Data<Config>>() {
            Some(config) => config,
            None => {
                error!("Config not found in application data");
                return Box::pin(ready(Err(
                    ServiceError::Auth("Server authentication configuration error".into()).into()
                )));
            }
        };

        // Check API key from auth header (normalized for security)
        let auth_header = req.headers().get(HeaderName::from_static(AUTH_HEADER));
        
        let auth_result = match auth_header {
            Some(auth_value) => {
                if let Ok(auth_str) = auth_value.to_str() {
                    if auth_str.starts_with(BEARER_PREFIX) {
                        let token = auth_str.trim_start_matches(BEARER_PREFIX).trim();
                        // Constant-time comparison to prevent timing attacks
                        constant_time_compare(token, &config.api_key)
                    } else {
                        debug!("Invalid authorization format");
                        false
                    }
                } else {
                    debug!("Invalid characters in authorization header");
                    false
                }
            }
            None => {
                debug!("Missing Authorization header");
                false
            }
        };

        if auth_result {
            let fut = self.service.call(req);
            Box::pin(async move {
                let res = fut.await?;
                Ok(res)
            })
        } else {
            Box::pin(ready(Err(
                ServiceError::Auth("Invalid or missing API key".into()).into()
            )))
        }
    }
}

fn constant_time_compare(a: &str, b: &str) -> bool {
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    let max_len = std::cmp::max(a_bytes.len(), b_bytes.len());

    let mut result = a_bytes.len() ^ b_bytes.len(); // Will be non-zero if lengths differ

    for i in 0..max_len {
        let byte_a = if i < a_bytes.len() { a_bytes[i] as usize } else { 0 };
        let byte_b = if i < b_bytes.len() { b_bytes[i] as usize } else { 0 };
        result |= byte_a ^ byte_b;
    }

    result == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test as actix_test;
    use actix_web::{http::StatusCode, web, App, HttpResponse};

    // --- constant_time_compare -------------------------------------------

    #[test]
    fn constant_time_compare_equal_strings() {
        assert!(constant_time_compare("secret-token", "secret-token"));
    }

    #[test]
    fn constant_time_compare_different_content_same_length() {
        assert!(!constant_time_compare("secret-token", "secret-tokeX"));
    }

    #[test]
    fn constant_time_compare_different_length() {
        assert!(!constant_time_compare("short", "short-and-then-some"));
        assert!(!constant_time_compare("short-and-then-some", "short"));
    }

    #[test]
    fn constant_time_compare_empty_strings() {
        assert!(constant_time_compare("", ""));
        assert!(!constant_time_compare("", "x"));
        assert!(!constant_time_compare("x", ""));
    }

    #[test]
    fn constant_time_compare_unicode() {
        assert!(constant_time_compare("wachtwoord-é", "wachtwoord-é"));
        assert!(!constant_time_compare("wachtwoord-é", "wachtwoord-e"));
    }

    // --- Auth middleware integration -------------------------------------

    fn test_config(api_key: &str) -> Config {
        Config {
            station_name: "TestStation".into(),
            api_key: api_key.into(),
            default_station_image: None,
            image_dir: "/tmp".into(),
            mot_dir: "/tmp".into(),
            dls_file: "/tmp/dls.txt".into(),
        }
    }

    async fn ok_handler() -> HttpResponse {
        HttpResponse::Ok().body("reached handler")
    }

    async fn build_app_and_call(
        api_key: &str,
        header: Option<(&'static str, String)>,
        include_config: bool,
    ) -> StatusCode {
        // The Auth middleware rejects by returning an `Err`, which only becomes
        // an HTTP response at the server boundary. `try_call_service` surfaces
        // that error so we can read the status it would map to.
        let build_req = |header: Option<(&'static str, String)>| {
            let mut req = actix_test::TestRequest::get().uri("/protected");
            if let Some((name, value)) = header {
                req = req.insert_header((name, value));
            }
            req.to_request()
        };
        let result = if include_config {
            let app = actix_test::init_service(
                App::new()
                    .app_data(web::Data::new(test_config(api_key)))
                    .wrap(Auth)
                    .route("/protected", web::get().to(ok_handler)),
            )
            .await;
            actix_test::try_call_service(&app, build_req(header)).await
        } else {
            let app = actix_test::init_service(
                App::new()
                    .wrap(Auth)
                    .route("/protected", web::get().to(ok_handler)),
            )
            .await;
            actix_test::try_call_service(&app, build_req(header)).await
        };
        match result {
            Ok(resp) => resp.status(),
            Err(e) => e.error_response().status(),
        }
    }

    #[actix_web::test]
    async fn valid_bearer_token_reaches_handler() {
        let status = build_app_and_call(
            "my-secret",
            Some(("authorization", "Bearer my-secret".into())),
            true,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    #[actix_web::test]
    async fn wrong_token_is_unauthorized() {
        let status = build_app_and_call(
            "my-secret",
            Some(("authorization", "Bearer wrong".into())),
            true,
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn missing_header_is_unauthorized() {
        let status = build_app_and_call("my-secret", None, true).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn non_bearer_scheme_is_unauthorized() {
        let status = build_app_and_call(
            "my-secret",
            Some(("authorization", "Basic my-secret".into())),
            true,
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn bearer_with_surrounding_whitespace_is_trimmed() {
        // token is trimmed after the "Bearer " prefix
        let status = build_app_and_call(
            "my-secret",
            Some(("authorization", "Bearer   my-secret  ".into())),
            true,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    #[actix_web::test]
    async fn missing_config_is_error() {
        // No Config in app_data -> middleware returns an auth/server error, never 200.
        let status = build_app_and_call(
            "my-secret",
            Some(("authorization", "Bearer my-secret".into())),
            false,
        )
        .await;
        assert_ne!(status, StatusCode::OK);
    }

    #[actix_web::test]
    async fn non_utf8_header_value_is_unauthorized() {
        use actix_web::http::header::{HeaderName, HeaderValue};

        // A header whose bytes are not valid UTF-8 exercises the
        // `to_str()` error branch.
        let app = actix_test::init_service(
            App::new()
                .app_data(web::Data::new(test_config("my-secret")))
                .wrap(Auth)
                .route("/protected", web::get().to(ok_handler)),
        )
        .await;
        let name = HeaderName::from_static("authorization");
        let value = HeaderValue::from_bytes(&[0xff, 0xfe, 0xfd]).unwrap();
        let req = actix_test::TestRequest::get()
            .uri("/protected")
            .insert_header((name, value))
            .to_request();
        let status = match actix_test::try_call_service(&app, req).await {
            Ok(resp) => resp.status(),
            Err(e) => e.error_response().status(),
        };
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
}