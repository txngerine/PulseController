use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProfileError {
    #[error("profile not found: {0}")]
    NotFound(String),

    #[error("profile already exists: {0}")]
    AlreadyExists(String),

    #[error("invalid profile data: {0}")]
    InvalidData(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("profile validation error: {0}")]
    Validation(String),
}

pub type Result<T> = std::result::Result<T, ProfileError>;
