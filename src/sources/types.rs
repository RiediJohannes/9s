use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Request failed: {0}")]
    Communication(#[from] reqwest::Error),

    #[error("Failed to construct URL and parameters: {0}")]
    Url(#[from] url::ParseError),
    
    #[error("Failed to parse API response: {0}")]
    Parsing(#[from] serde_json::Error),

    #[error("Bad request: {reason:?}")]
    BadRequest {
        reason: String
    },
}

#[derive(Deserialize, Debug, Clone)]
pub struct ClimateApiError {
    pub reason: String,
}