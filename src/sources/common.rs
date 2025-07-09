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
        self.latitude.thousandths() == other.latitude.thousandths() && 
            self.longitude.thousandths() == other.longitude.thousandths()
    }
}
impl Eq for Coordinates {} // marker interface to guarantee that PartialEq implementation is reflexive
impl Hash for Coordinates {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.latitude.thousandths().hash(state);
        self.longitude.thousandths().hash(state);
    }
}

trait Thousandth {
    fn thousandths(&self) -> i32;
}
impl Thousandth for f64 {
    fn thousandths(&self) -> i32 {
        (self * 1000.0).round() as i32
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct SingleTemperature {
    #[serde(rename = "time")]
    pub epoch: i64,
    pub temperature_2m: f32,
}


pub async fn query_api<TOutput, TSuccess, TFailure>(
    client: &reqwest::Client,
    url: &str,
    params: impl IntoIterator<Item = (&str, String)>
) -> Result<TOutput, ApiError>
where
    TSuccess: for<'de> Deserialize<'de> + Into<TOutput>,
    TFailure: for<'de> Deserialize<'de> + Into<ApiError>,
{
    let url = reqwest::Url::parse_with_params(url, params)?;

    let response = client.get(url).send().await?;
    let payload = response.text().await?;

    // try to parse the response body as the given success type TSuccess
    match serde_json::from_str::<TSuccess>(&payload) {
        Ok(current_temp_result) => Ok(current_temp_result.into()),
        Err(e) => {
            // If it fails, attempt to parse the body as the given failure type TFailure
            match serde_json::from_str::<TFailure>(&payload) {
                Ok(api_error) => Err(api_error.into()),
                Err(_) => Err(ApiError::Parsing(e)), // Return the error if both parsing attempts fail
            }
        }
    }
}


pub fn truncate_utf8(s: &mut String, max_chars: usize) {
    match s.char_indices().nth(max_chars) {
        None => (),
        Some((idx, _)) => {
            s.truncate(idx);
        }
    }
}

pub fn truncate_ellipsis(s: &mut String, max_chars: usize, ellipsis: &str) {
    let cut_off = max_chars - ellipsis.len();
    
    match s.char_indices().nth(cut_off) {
        None => (),
        Some((idx, _)) => {
            s.truncate(idx);
            s.push_str(ellipsis);
        }
    }
}