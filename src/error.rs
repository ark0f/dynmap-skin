use crate::api;
use actix_web::{http::StatusCode, ResponseError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Player not found")]
    NoPlayer,
    #[error("No skin available")]
    NoSkin,
    #[error("{0}")]
    Base64(
        #[from]
        #[source]
        base64::DecodeError,
    ),
    #[error("{0}")]
    Json(
        #[from]
        #[source]
        serde_json::Error,
    ),
    #[error("{0}")]
    Api(
        #[from]
        #[source]
        api::Error,
    ),
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NoPlayer | Self::NoSkin => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
