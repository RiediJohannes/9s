use serde::Deserialize;
use thiserror::Error;

const BASE_URL: &str = "https://geocoding-api.open-meteo.com/v1/search?count=5&language=de&format=json";

#[derive(Deserialize, Debug, Clone)]
pub struct Place {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub elevation: f32,
    /// Alpha-2 country code
    #[serde(rename = "country_code")]
    pub country: String,
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Request failed: {0}")]
    Communication(#[from] reqwest::Error),

    #[error("Failed to parse API response: {0}")]
    Parsing(#[from] serde_json::Error),

    #[error("Bad request: {reason:?}")]
    BadRequest {
        reason: String
    },
}


#[derive(Deserialize, Debug)]
struct GeoResult {
    #[serde(rename = "results", default)]
    places: Vec<Place>,
    // #[serde(rename = "generationtime_ms")]
    // generation_millis: f32,
}

#[derive(Deserialize, Debug, Clone)]
struct GeoError {
    reason: String,
}


pub async fn query_place(name: &str) -> Result<Vec<Place>, ApiError> {
    let parameters = format!("&name={name}", name = name);
    let url = format!("{}{}", BASE_URL, parameters);

    let response = reqwest::get(&url).await?;
    let payload = response.text().await?;

    match serde_json::from_str::<GeoResult>(&payload) {
        Ok(geo_result) => Ok(geo_result.places),
        Err(_) => {
            // If it fails, attempt to parse as GeoError
            match serde_json::from_str::<GeoError>(&payload) {
                Ok(geo_error) => Err(ApiError::BadRequest { reason: geo_error.reason }),
                Err(e) => Err(ApiError::Parsing(e)), // Return the error if both parsing attempts fail
            }
        }
    }
}