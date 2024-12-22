use serde::Deserialize;
use std::hash::Hash;
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
impl From<ClimateApiError> for ApiError {
    fn from(error: ClimateApiError) -> Self {
        ApiError::BadRequest { reason: error.reason }
    }
}

#[derive(Clone)]
pub struct Coordinates {
    pub latitude: f64,
    pub longitude: f64
}
impl Coordinates {
    pub fn new(latitude: f64, longitude: f64) -> Coordinates {
        Coordinates {latitude, longitude}
    }
}
impl PartialEq for Coordinates {
    fn eq(&self, other: &Self) -> bool {
        // this is fine because we never do math with coordinates, we just use parsed string coordinates
        self.latitude == other.latitude && self.longitude == other.longitude
    }
}
impl Eq for Coordinates {} // marker interface to guarantee that PartialEq implementation is reflexive
impl Hash for Coordinates {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.latitude.to_bits().hash(state);
        self.longitude.to_bits().hash(state);
    }
}


pub async fn query_api<O, S, F>(client: &reqwest::Client, url: &str, params: impl IntoIterator<Item = (&str, String)>)
                                -> Result<O, ApiError>
where
    S: for<'de> Deserialize<'de> + Into<O>,
    F: for<'de> Deserialize<'de> + Into<ApiError>,
{
    let url = reqwest::Url::parse_with_params(url, params)?;

    let response = client.get(url).send().await?;
    let payload = response.text().await?;

    // try to parse the response body as the given success type S
    match serde_json::from_str::<S>(&payload) {
        Ok(current_temp_result) => Ok(current_temp_result.into()),
        Err(e) => {
            // If it fails, attempt to parse the body as the given failure type F
            match serde_json::from_str::<F>(&payload) {
                Ok(api_error) => Err(api_error.into()),
                Err(_) => Err(ApiError::Parsing(e)), // Return the error if both parsing attempts fail
            }
        }
    }
}