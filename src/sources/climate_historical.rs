use crate::geo_time::Tz;
use crate::sources::common;
use crate::sources::common::{ApiError, ClimateApiError, Coordinates, SingleTemperature};
use chrono::DateTime;
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


pub async fn get_past_temperature(client: &reqwest::Client, point: &Coordinates, datetime: &DateTime<Tz>)
                                  -> Result<SingleTemperature, ApiError>
{
    let temperature_series = get_temperature_series(client, point, datetime, datetime).await?;

    // TODO Grab the correct temperature point from the series according to the time of the time stamp
    //      Note, the series only has hourly data. Also, it might make sense to change the Vec to a BTree or something similar
    Ok(temperature_series.first().unwrap().clone().into())
}

pub async fn get_temperature_series(client: &reqwest::Client, point: &Coordinates,
                                    start_date: &DateTime<Tz>, end_date: &DateTime<Tz>)
    -> Result<Vec<TemperatureDataPoint>, ApiError>
{
    if start_date > end_date {
        return Err(ApiError::BadRequest {
            reason: "Start date must be less than or equal to end date".to_string(),
        })
    }

    let params = [
        ("latitude", point.latitude.to_string()),
        ("longitude", point.longitude.to_string()),
        ("hourly", "temperature_2m".to_string()),
        ("start_date", start_date.date_naive().to_string()),
        ("end_date", end_date.date_naive().to_string()),
        ("timezone", start_date.timezone().name().to_string()), // W.l.o.g we take the timezone of the start date
        ("timeformat", "unixtime".to_string()),
    ];

    common::query_api::<Vec<TemperatureDataPoint>, HistoricalTemperature, ClimateApiError>
        (client, BASE_URL, params).await
}