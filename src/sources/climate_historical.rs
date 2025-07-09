use crate::geo_time::Tz;
use crate::sources::common;
use crate::sources::common::{ApiError, ClimateApiError, Coordinates, SingleTemperature};
use chrono::DateTime;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::ops::Bound::Included;

const BASE_URL: &str = "https://archive-api.open-meteo.com/v1/archive";

// ----------------------- Type Definitions --------------------------

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
    pub fn to_vec(&self) -> Vec<TemperatureDataPoint> {
        self.times.iter()
            .zip(self.temperatures.iter())
            .map(|(&time, &temp)| TemperatureDataPoint {
                time: time as i64,
                value: temp,
            })
            .collect()
    }

    pub fn to_ordered(&self) -> BTreeSet<TemperatureDataPoint> {
        self.times.iter()
            .zip(self.temperatures.iter())
            .map(|(&time, &temp)| TemperatureDataPoint {
                time: time as i64,
                value: temp,
            })
            .collect()
    }
}

#[derive(Debug, Clone, Default)]
pub struct TemperatureDataPoint {
    pub time: i64,
    pub value: f32,
}
impl PartialEq for TemperatureDataPoint {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}
impl Eq for TemperatureDataPoint {}
impl PartialOrd for TemperatureDataPoint {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for TemperatureDataPoint {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time)
    }
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
        historical_temp.series.to_vec()
    }
}
impl From<HistoricalTemperature> for BTreeSet<TemperatureDataPoint> {
    fn from(historical_temp: HistoricalTemperature) -> Self {
        historical_temp.series.to_ordered()
    }
}

// ----------------------- Public Functions --------------------------

pub async fn get_past_temperature(client: &reqwest::Client, point: &Coordinates, timestamp: &DateTime<Tz>)
                                  -> Result<SingleTemperature, ApiError>
{
    let temperature_series = get_temperature_series(client, point, timestamp, timestamp).await?;

    // TODO Grab the correct temperature point from the series according to the time of the time stamp
    //      Note, the series only has hourly data. Also, it might make sense to change the Vec to a BTree or something similar
    Ok(temperature_series.first().unwrap().clone().into())
}

/// Queries the historic weather API for a series of hourly temperature data at the given geographic
/// `location` between the `start_time` and `end_time`.
/// # Arguments
/// - `location` - the geographic coordinates of the location of interest
/// - `start_time` - the start time of the data series (inclusive)
/// - `end_time` - the end time of the query (inclusive)
///
/// Note that the API only stores temperature data on an hourly basis. Thus, a short time-interval
/// that does not cross the border between two hours might return an empty set of data.
/// # Returns
/// A [BTreeSet](BTreeSet)` of [TemperatureDataPoint](TemperatureDataPoint)s ordered by the data
/// points' `time` attribute.
/// # Errors
/// - [ApiError::BadRequest](ApiError::BadRequest) if the request contains contradicting information,
///   e.g. `start_time`is greater than `end_time`.
/// - [ApiError::Parsing](ApiError::Parsing) if the API response contained unexpected date and could
///   thus not be parsed.
/// ```
pub async fn get_temperature_series(client: &reqwest::Client, location: &Coordinates,
                                    start_time: &DateTime<Tz>, end_time: &DateTime<Tz>)
                                    -> Result<BTreeSet<TemperatureDataPoint>, ApiError>
{
    if start_time > end_time {
        return Err(ApiError::BadRequest {
            reason: "Start date must be less than or equal to end date".to_string(),
        })
    }

    let params = [
        ("latitude", location.latitude.to_string()),
        ("longitude", location.longitude.to_string()),
        ("hourly", "temperature_2m".to_string()),
        ("start_date", start_time.date_naive().to_string()),
        ("end_date", end_time.date_naive().to_string()),
        ("timezone", start_time.timezone().name().to_string()), // W.l.o.g we take the timezone of the start date
        ("timeformat", "unixtime".to_string()),
    ];

    let temperature_series = common::query_api::<BTreeSet<TemperatureDataPoint>, HistoricalTemperature, ClimateApiError>
        (client, BASE_URL, params).await;

    let start_point = TemperatureDataPoint {
        time: start_time.timestamp(),
        ..Default::default()
    };
    let end_point = TemperatureDataPoint {
        time: end_time.timestamp(),
        ..Default::default()
    };

    // return a subset of the temperature series that falls within the given time interval
    temperature_series.map(|series| {
        series.range((Included(&start_point), Included(&end_point)))
            .cloned()
            .collect::<BTreeSet<_>>()
    })
}