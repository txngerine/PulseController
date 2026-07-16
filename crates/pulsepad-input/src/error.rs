use thiserror::Error;

#[derive(Error, Debug)]
pub enum InputError {
    #[error("invalid input data: {0}")]
    InvalidData(String),

    #[error("mapping not found: {0}")]
    MappingNotFound(String),

    #[error("deadzone error: {0}")]
    DeadzoneError(String),

    #[error("profile error: {0}")]
    ProfileError(String),

    #[error("platform error: {0}")]
    Platform(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, InputError>;
