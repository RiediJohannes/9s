use super::common::{ApiError, ClimateApiError};
use super::geocoding::Place;
use crate::sources::common;
use poise::serenity_prelude::Timestamp;
use serde::Deserialize;

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
impl From<HistoricalTemperature> for Vec<TemperatureDataPoint> {
    fn from(historical_temp: HistoricalTemperature) -> Self {
        historical_temp.series.flatten()
    }
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

    common::query_api::<Vec<TemperatureDataPoint>, HistoricalTemperature, ClimateApiError>
        (client, BASE_URL, params).await
}