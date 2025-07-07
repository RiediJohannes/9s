use crate::geo_time::Tz;
use crate::sources::common;
use crate::sources::common::{ApiError, ClimateApiError, Coordinates, SingleTemperature};
use crate::sources::nominatim::Place;
use chrono::DateTime;
use poise::serenity_prelude::Timestamp;
use serde::Deserialize;

const BASE_URL: &str = "https://archive-api.open-meteo.com/v1/archive";
// const CACHE_TTL_SECONDS: u64 = 120;

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
impl From<TemperatureDataPoint> for SingleTemperature {
    fn from(data_point: TemperatureDataPoint) -> Self {
        SingleTemperature {
            epoch: data_point.time,
            temperature_2m: data_point.value
        }
    }
}
impl From<HistoricalTemperature> for Vec<TemperatureDataPoint> {
    fn from(historical_temp: HistoricalTemperature) -> Self {
        historical_temp.series.flatten()
    }
}


// TODO Think thoroughly about the caching strategy (add date (without time!) to the key, consider non-timed cache
// #[cached(
//     ty = "TimedCache<Coordinates, SingleTemperature>",
//     create = "{ TimedCache::with_lifespan(CACHE_TTL_SECONDS) }",
//     convert = r#"{ point.clone() }"#,
//     result = true
// )]
pub async fn get_past_temperature(client: &reqwest::Client, point: &Coordinates, datetime: &DateTime<Tz>)
                                  -> Result<SingleTemperature, ApiError>
{
    let requested_date_iso = datetime.format("%Y-%m-%d").to_string();
    let naive_date = datetime.date_naive().to_string();

    println!("Formatted datetime: {}", requested_date_iso);
    println!("Naive date: {}", naive_date);

    let params = [
        ("latitude", point.latitude.to_string()),
        ("longitude", point.longitude.to_string()),
        ("hourly", "temperature_2m".to_string()),
        ("start_date", requested_date_iso.clone()),
        ("end_date", requested_date_iso),
        ("timezone", "Europe/Berlin".to_string()),
        ("timeformat", "unixtime".to_string()),
    ];

    let temperature_series = common::query_api::<Vec<TemperatureDataPoint>, HistoricalTemperature, ClimateApiError>
        (client, BASE_URL, params).await?;

    Ok(temperature_series.first().unwrap().clone().into())
}

pub async fn get_temperature_series(client: &reqwest::Client, place: &Place,
                                    start_date: Timestamp, end_date: Timestamp)
    -> Result<Vec<TemperatureDataPoint>, ApiError>
{
    let params = [
        ("latitude", place.lat.to_string()),
        ("longitude", place.lon.to_string()),
        ("hourly", "temperature_2m".to_string()),
        ("start_date", start_date.date_naive().to_string()),
        ("end_date", end_date.date_naive().to_string()),
        ("timeformat", "unixtime".to_string()),
    ];

    common::query_api::<Vec<TemperatureDataPoint>, HistoricalTemperature, ClimateApiError>
        (client, BASE_URL, params).await
}