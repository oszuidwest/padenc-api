use crate::config::Config;
use crate::constants::api::{AUTH_HEADER, BEARER_PREFIX};
use crate::errors::ServiceError;
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http::header::{HeaderName, HeaderValue},
    web, Error,
};
use futures_core::future::LocalBoxFuture;
use log::{debug, error};

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
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(AuthMiddleware { service }))
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
        let config = match req.app_data::<web::Data<Config>>() {
            Some(config) => config,
            None => {
                error!("Config not found in application data");
                return Box::pin(std::future::ready(Err(Error::from(
                    ServiceError::Auth("Server authentication configuration error".to_string())
                ))));
            }
        };

        let api_key = match &config.api_key {
            Some(key) => key,
            None => {
                error!("API_KEY is not configured");
                return Box::pin(std::future::ready(Err(Error::from(
                    ServiceError::Auth("Server authentication configuration error".to_string())
                ))));
            }
        };

        let auth_header = req.headers().get(HeaderName::from_static(AUTH_HEADER));
        let auth_result = match auth_header {
            Some(auth_value) => validate_bearer_token(auth_value, api_key),
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
            Box::pin(std::future::ready(Err(Error::from(
                ServiceError::Auth("Invalid or missing API key".to_string())
            ))))
        }
    }
}

fn validate_bearer_token(auth_header: &HeaderValue, api_key: &str) -> bool {
    let auth_str = match auth_header.to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };

    if !auth_str.starts_with(BEARER_PREFIX) {
        return false;
    }

    let token = auth_str.trim_start_matches(BEARER_PREFIX).trim();
    token == api_key
}
