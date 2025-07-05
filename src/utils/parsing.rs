use chrono::{DateTime, Utc};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    // #[error("Failed to parse API response: {0}")]
    // Parsing(#[from] serde_json::Error),
}

pub fn parse_datetime(date_input: Option<String>, time_input: Option<String>) -> Result<DateTime<Utc>, ParseError> {
    // default to today if no date is provided

    // default to current time if no date is provided

    Ok(Utc::now())
}