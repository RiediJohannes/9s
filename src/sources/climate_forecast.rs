use super::common::{ApiError, ClimateApiError, Coordinates};
use crate::sources::common;
use cached::proc_macro::cached;
use cached::TimedCache;
use serde::Deserialize;

const BASE_URL: &str = "https://api.open-meteo.com/v1/forecast";
const CACHE_TTL_SECONDS: u64 = 120;

#[derive(Deserialize, Debug)]
struct CurrentTempResult {
    current: CurrentTemp,
}
impl From<CurrentTempResult> for CurrentTemp {
    fn from(result: CurrentTempResult) -> Self {
        result.current
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct CurrentTemp {
    #[serde(rename = "time")]
    pub epoch: u32,
    pub temperature_2m: f32,
}


#[cached(
    ty = "TimedCache<Coordinates, CurrentTemp>",
    create = "{ TimedCache::with_lifespan(CACHE_TTL_SECONDS) }",
    convert = r#"{ point.clone() }"#,
    result = true
)]
pub async fn get_current_temperature(client: &reqwest::Client, point: &Coordinates)
    -> Result<CurrentTemp, ApiError> 
{
    let params = [
        ("latitude", point.latitude.to_string()),
        ("longitude", point.longitude.to_string()),
        ("current", "temperature_2m".to_string()),
        ("timeformat", "unixtime".to_string()),
    ];

    common::query_api::<CurrentTemp, CurrentTempResult, ClimateApiError>
        (client, BASE_URL, params).await
}