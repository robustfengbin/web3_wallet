use actix_web::{HttpResponse, ResponseError};
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    // Authentication errors
    Unauthorized(String),
    InvalidCredentials,
    TokenExpired,

    // Authorization errors
    Forbidden(String),

    // Resource errors
    NotFound(String),
    AlreadyExists(String),

    // Validation errors
    ValidationError(String),

    // Blockchain errors
    BlockchainError(String),
    InsufficientBalance(String),

    // Encryption errors
    EncryptionError(String),

    // Database errors
    DatabaseError(String),

    // Configuration errors
    ConfigError(String),

    // Internal errors
    InternalError(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            AppError::InvalidCredentials => write!(f, "Invalid username or password"),
            AppError::TokenExpired => write!(f, "Token has expired"),
            AppError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::AlreadyExists(msg) => write!(f, "Already exists: {}", msg),
            AppError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            AppError::BlockchainError(msg) => write!(f, "Blockchain error: {}", msg),
            AppError::InsufficientBalance(msg) => write!(f, "Insufficient balance: {}", msg),
            AppError::EncryptionError(msg) => write!(f, "Encryption error: {}", msg),
            AppError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            AppError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            AppError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        let error_message = serde_json::json!({
            "error": self.to_string()
        });

        match self {
            AppError::Unauthorized(_) | AppError::InvalidCredentials | AppError::TokenExpired => {
                HttpResponse::Unauthorized().json(error_message)
            }
            AppError::Forbidden(_) => {
                HttpResponse::Forbidden().json(error_message)
            }
            AppError::NotFound(_) => {
                HttpResponse::NotFound().json(error_message)
            }
            AppError::AlreadyExists(_) | AppError::ValidationError(_) => {
                HttpResponse::BadRequest().json(error_message)
            }
            AppError::InsufficientBalance(_) => {
                HttpResponse::BadRequest().json(error_message)
            }
            AppError::BlockchainError(_) | AppError::EncryptionError(_)
            | AppError::DatabaseError(_) | AppError::ConfigError(_)
            | AppError::InternalError(_) => {
                HttpResponse::InternalServerError().json(error_message)
            }
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        tracing::error!("Database error: {:?}", err);
        AppError::DatabaseError(err.to_string())
    }
}

impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        match err.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => AppError::TokenExpired,
            _ => AppError::Unauthorized(err.to_string()),
        }
    }
}

impl From<ethers::providers::ProviderError> for AppError {
    fn from(err: ethers::providers::ProviderError) -> Self {
        tracing::error!("Blockchain provider error: {:?}", err);
        AppError::BlockchainError(err.to_string())
    }
}

impl From<config::ConfigError> for AppError {
    fn from(err: config::ConfigError) -> Self {
        AppError::ConfigError(err.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
