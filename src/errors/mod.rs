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