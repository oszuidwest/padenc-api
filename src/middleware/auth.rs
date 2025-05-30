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