use super::common::{ApiError, ClimateApiError, Coordinates, SingleTemperature};
use crate::sources::common;
use cached::proc_macro::cached;
use cached::TimedCache;
use serde::Deserialize;

const BASE_URL: &str = "https://api.open-meteo.com/v1/forecast";
const CACHE_TTL_SECONDS: u64 = 120;

#[derive(Deserialize, Debug)]
struct CurrentTempResult {
    current: SingleTemperature,
}
impl From<CurrentTempResult> for SingleTemperature {
    fn from(result: CurrentTempResult) -> Self {
        result.current
    }
}


#[cached(
    ty = "TimedCache<Coordinates, SingleTemperature>",
    create = "{ TimedCache::with_lifespan(CACHE_TTL_SECONDS) }",
    convert = r#"{ location.clone() }"#,
    result = true
)]
pub async fn get_current_temperature(client: &reqwest::Client, location: &Coordinates)
                                     -> Result<SingleTemperature, ApiError>
{
    let params = [
        ("latitude", location.latitude.to_string()),
        ("longitude", location.longitude.to_string()),
        ("current", "temperature_2m".to_string()),
        ("timeformat", "unixtime".to_string()),
    ];

    common::query_api::<SingleTemperature, CurrentTempResult, ClimateApiError>
        (client, BASE_URL, params).await
}