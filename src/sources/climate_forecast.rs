use serde::Deserialize;
use super::types::{ApiError, ClimateApiError, Coordinates};

const BASE_URL: &str = "https://api.open-meteo.com/v1/forecast";


#[derive(Deserialize, Debug)]
struct CurrentTempResult {
    current: CurrentTemp,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CurrentTemp {
    #[serde(rename = "time")]
    pub epoch: u32,
    pub temperature_2m: f32,
}


pub async fn get_current_temperature(client: &reqwest::Client, point: Coordinates)
    -> Result<CurrentTemp, ApiError> 
{
    let params = [
        ("latitude", point.latitude.to_string()),
        ("longitude", point.longitude.to_string()),
        ("current", "temperature_2m".to_string()),
        ("timeformat", "unixtime".to_string()),
    ];
    let url = reqwest::Url::parse_with_params(BASE_URL, &params)?;

    let response = client.get(url).send().await?;
    let payload = response.text().await?;

    match serde_json::from_str::<CurrentTempResult>(&payload) {
        Ok(current_temp_result) => Ok(current_temp_result.current),
        Err(e) => {
            // If it fails, attempt to parse as ClimateApiError
            match serde_json::from_str::<ClimateApiError>(&payload) {
                Ok(api_error) => Err(ApiError::BadRequest { reason: api_error.reason }),
                Err(_) => Err(ApiError::Parsing(e)), // Return the error if both parsing attempts fail
            }
        }
    }
}