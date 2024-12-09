use serde::Deserialize;
use poise::serenity_prelude::Timestamp;
use super::geocoding::Place;
use super::types::{ApiError, ClimateApiError};

const BASE_URL: &str = "https://archive-api.open-meteo.com/v1/archive";

#[derive(Deserialize, Debug)]
struct HistoricalTemperature {
    #[serde(alias = "hourly")]
    series: TemperatureSeries,
}

#[derive(Deserialize, Debug, Clone)]
struct TemperatureSeries {
    #[serde(alias = "time")]
    pub times: Vec<u32>,
    #[serde(alias = "temperature_2m")]
    pub temperatures: Vec<f32>,
}
impl TemperatureSeries {
    fn flatten(&self) -> Vec<TemperatureDataPoint> {
        self.times.iter()
            .zip(self.temperatures.iter())
            .map(|(&time, &temp)| TemperatureDataPoint {
                time,
                value: temp,
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct TemperatureDataPoint {
    pub time: u32,
    pub value: f32,
}


pub async fn get_temperature_series(client: &reqwest::Client, place: &Place,
                                    start_date: Timestamp, end_date: Timestamp)
    -> Result<Vec<TemperatureDataPoint>, ApiError>
{
    let params = [
        ("latitude", place.latitude.to_string()),
        ("longitude", place.longitude.to_string()),
        ("hourly", "temperature_2m".to_string()),
        ("start_date", start_date.date_naive().to_string()),
        ("end_date", end_date.date_naive().to_string()),
        ("timeformat", "unixtime".to_string()),
    ];
    let url = reqwest::Url::parse_with_params(BASE_URL, &params)?;

    let response = client.get(url).send().await?;
    let payload = response.text().await?;

    match serde_json::from_str::<HistoricalTemperature>(&payload) {
        Ok(hist_temp) => Ok(hist_temp.series.flatten()),
        Err(e) => {
            // If it fails, attempt to parse as ClimateApiError
            match serde_json::from_str::<ClimateApiError>(&payload) {
                Ok(api_error) => Err(ApiError::BadRequest { reason: api_error.reason }),
                Err(_) => Err(ApiError::Parsing(e)), // Return the error if both parsing attempts fail
            }
        }
    }
}