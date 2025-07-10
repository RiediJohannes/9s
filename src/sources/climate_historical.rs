use crate::geo_time::Tz;
use crate::sources::common;
use crate::sources::common::{ApiError, ClimateApiError, Coordinates, SingleTemperature};
use chrono::{DateTime, DurationRound, NaiveDate, TimeDelta, Utc};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Bound::Included;

const BASE_URL: &str = "https://archive-api.open-meteo.com/v1/archive";
static EARLIEST_DATA: NaiveDate = NaiveDate::from_ymd_opt(1940, 1, 1)
    .expect("Invalid start date entered for 'EARLIEST_DATA'!");

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

// private type alias to make IntoIterator for TemperatureSeries more readable
type TemperatureSeriesIter<'a> = std::iter::Map<
    std::iter::Zip<
        std::slice::Iter<'a, u32>,
        std::slice::Iter<'a, f32>
    >,
    fn((&u32, &f32)) -> TemperatureDataPoint
>;

impl<'a> IntoIterator for &'a TemperatureSeries {
    type Item = TemperatureDataPoint;
    type IntoIter = TemperatureSeriesIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.times.iter()
            .zip(self.temperatures.iter())
            .map(|(&time, &temp)| TemperatureDataPoint {
                time: time as i64,
                value: temp,
            })
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
        historical_temp.series.into_iter().collect()       
    }
}
impl From<HistoricalTemperature> for BTreeSet<TemperatureDataPoint> {
    fn from(historical_temp: HistoricalTemperature) -> Self {
        historical_temp.series.into_iter().collect()       
    }
}
impl From<HistoricalTemperature> for BTreeMap<i64, TemperatureDataPoint> {
    fn from(historical_temp: HistoricalTemperature) -> Self {
        historical_temp.series.into_iter()
            .map(|point| (point.time, point))
            .collect()
    }
}
impl FromIterator<TemperatureDataPoint> for BTreeMap<i64, TemperatureDataPoint> {
    fn from_iter<T: std::iter::IntoIterator<Item = TemperatureDataPoint>>(iter: T) -> Self
    {
        iter.into_iter().map(|point| (point.time, point)).collect()
    }
}

// ----------------------- Public Functions --------------------------

pub async fn get_past_temperature(client: &reqwest::Client, location: &Coordinates, timestamp: &DateTime<Tz>)
                                  -> Result<SingleTemperature, ApiError>
{
    // round the timestamp to the nearest hour, since the API only stores temperature data every hour
    let rounded_timestamp = timestamp.duration_round(TimeDelta::hours(1))
        .map_err(|_| ApiError::BadRequest { reason: "Rounding the timestamp exceeded its possible value space".to_string()})?;
    
    let temperature_series = get_temperature_series(client, location, &rounded_timestamp, &rounded_timestamp).await?;

    let data = temperature_series.get(&rounded_timestamp.timestamp());

    match data {
        Some(temperature_point) => Ok(temperature_point.clone().into()),
        None => Err(ApiError::NotFound)
    }
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
                                    -> Result<BTreeMap<i64, TemperatureDataPoint>, ApiError>
{
    // check for incorrect datetime ranges
    if start_time > end_time {
        return Err(ApiError::BadRequest {
            reason: "Start date must be less than or equal to end date".to_string(),
        })
    }

    if start_time.date_naive() < EARLIEST_DATA {
        return Err(ApiError::BadRequest {
            reason: format!("There is no climate data available before {}", EARLIEST_DATA)
        })
    }

    if end_time.date_naive() > Utc::now().date_naive() {
        return Err(ApiError::BadRequest {
            reason: "Cannot predict climate data in the future!".to_string()
        })
    }


    // execute the request
    let params = [
        ("latitude", location.latitude.to_string()),
        ("longitude", location.longitude.to_string()),
        ("hourly", "temperature_2m".to_string()),
        ("start_date", start_time.date_naive().to_string()),
        ("end_date", end_time.date_naive().to_string()),
        ("timezone", start_time.timezone().name().to_string()), // W.l.o.g we take the timezone of the start date
        ("timeformat", "unixtime".to_string()),
    ];

    let temperature_series =
        common::query_api::<BTreeMap<i64, TemperatureDataPoint>, HistoricalTemperature, ClimateApiError>
        (client, BASE_URL, params).await;

    let start_point = start_time.timestamp();
    let end_point = end_time.timestamp();

    // return a subset of the temperature series that falls within the given time interval
    temperature_series.map(|series| {
        series.range((Included(&start_point), Included(&end_point)))
            .map(|(&time, value)| (time, value.clone()))
            .collect::<BTreeMap<i64, TemperatureDataPoint>>()
    })
}