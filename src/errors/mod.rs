use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Invalid configuration: {0}")]
    Configuration(String),

    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Invalid input: {0}")]
    Validation(String),

    #[error("File processing error: {0}")]
    FileProcessing(String),

    #[error("Image error: {0}")]
    Image(String),

    #[error("Content error: {0}")]
    Content(String),

    #[error("Expired content")]
    ExpiredContent,

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Server error: {0}")]
    Server(String),
}

impl From<ServiceError> for io::Error {
    fn from(err: ServiceError) -> Self {
        match err {
            ServiceError::Io(io_err) => io_err,
            ServiceError::Configuration(msg) => io::Error::new(io::ErrorKind::InvalidInput, msg),
            ServiceError::Auth(msg) => io::Error::new(io::ErrorKind::PermissionDenied, msg),
            ServiceError::Validation(msg) => io::Error::new(io::ErrorKind::InvalidInput, msg),
            ServiceError::FileProcessing(msg) => io::Error::new(io::ErrorKind::Other, msg),
            ServiceError::Image(msg) => io::Error::new(io::ErrorKind::InvalidData, msg),
            ServiceError::Content(msg) => io::Error::new(io::ErrorKind::InvalidData, msg),
            ServiceError::ExpiredContent => io::Error::new(io::ErrorKind::TimedOut, "Content expired"),
            ServiceError::NotFound(msg) => io::Error::new(io::ErrorKind::NotFound, msg),
            ServiceError::Server(msg) => io::Error::new(io::ErrorKind::Other, msg),
        }
    }
}

impl From<ServiceError> for actix_web::Error {
    fn from(err: ServiceError) -> Self {
        match err {
            ServiceError::Auth(_) => 
                actix_web::error::ErrorUnauthorized(err.to_string()),
            ServiceError::Validation(_) => 
                actix_web::error::ErrorBadRequest(err.to_string()),
            ServiceError::NotFound(_) => 
                actix_web::error::ErrorNotFound(err.to_string()),
            ServiceError::ExpiredContent => 
                actix_web::error::ErrorGone(err.to_string()),
            _ => 
                actix_web::error::ErrorInternalServerError(err.to_string()),
        }
    }
}

pub type ServiceResult<T> = Result<T, ServiceError>;

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http::StatusCode;

    #[test]
    fn display_messages_include_context() {
        assert_eq!(
            ServiceError::Validation("bad".into()).to_string(),
            "Invalid input: bad"
        );
        assert_eq!(ServiceError::ExpiredContent.to_string(), "Expired content");
        assert_eq!(
            ServiceError::NotFound("missing".into()).to_string(),
            "Not found: missing"
        );
    }

    #[test]
    fn into_io_error_maps_kinds() {
        let kind = |e: ServiceError| io::Error::from(e).kind();
        assert_eq!(kind(ServiceError::Configuration("c".into())), io::ErrorKind::InvalidInput);
        assert_eq!(kind(ServiceError::Auth("a".into())), io::ErrorKind::PermissionDenied);
        assert_eq!(kind(ServiceError::Validation("v".into())), io::ErrorKind::InvalidInput);
        assert_eq!(kind(ServiceError::FileProcessing("f".into())), io::ErrorKind::Other);
        assert_eq!(kind(ServiceError::Image("i".into())), io::ErrorKind::InvalidData);
        assert_eq!(kind(ServiceError::Content("c".into())), io::ErrorKind::InvalidData);
        assert_eq!(kind(ServiceError::ExpiredContent), io::ErrorKind::TimedOut);
        assert_eq!(kind(ServiceError::NotFound("n".into())), io::ErrorKind::NotFound);
        assert_eq!(kind(ServiceError::Server("s".into())), io::ErrorKind::Other);
    }

    #[test]
    fn io_error_roundtrips_through_from() {
        let original = io::Error::new(io::ErrorKind::UnexpectedEof, "boom");
        let service_err: ServiceError = original.into();
        // Io variant should preserve the underlying io::Error, including its kind.
        let back: io::Error = service_err.into();
        assert_eq!(back.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn into_actix_error_maps_status_codes() {
        let status = |e: ServiceError| {
            let actix_err: actix_web::Error = e.into();
            actix_err.as_response_error().status_code()
        };
        assert_eq!(status(ServiceError::Auth("a".into())), StatusCode::UNAUTHORIZED);
        assert_eq!(status(ServiceError::Validation("v".into())), StatusCode::BAD_REQUEST);
        assert_eq!(status(ServiceError::NotFound("n".into())), StatusCode::NOT_FOUND);
        assert_eq!(status(ServiceError::ExpiredContent), StatusCode::GONE);
        // Catch-all -> 500
        assert_eq!(status(ServiceError::Server("s".into())), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(status(ServiceError::Io(io::Error::new(io::ErrorKind::Other, "x"))), StatusCode::INTERNAL_SERVER_ERROR);
    }
}