use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Native(String),
    #[error("{0}")]
    Internal(String),
}

impl AppError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest(message.into())
    }


    pub fn native(message: impl Into<String>) -> Self {
        Self::Native(message.into())
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }



}

