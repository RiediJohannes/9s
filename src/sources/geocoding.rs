use serde::Deserialize;
use reqwest::Error;

const BASE_URL: &str = "https://geocoding-api.open-meteo.com/v1/search?count=5&language=de&format=json";

#[derive(Deserialize, Debug, Clone)]
pub struct Place {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub elevation: f32,
    /// Alpha-2 country code
    #[serde(rename = "country_code")]
    pub country: String,
}

#[derive(Deserialize, Debug)]
struct GeoResult {
    #[serde(rename = "results")]
    places: Vec<Place>,
}

pub async fn query_place(name: &str) -> Result<Option<Place>, Error> {
    let parameters = format!("&name={name}", name = name);
    let url = format!("{}{}", BASE_URL, parameters);

    let response = reqwest::get(&url).await?;
    let mut result: GeoResult = response.json().await?;

    if result.places.is_empty() {
        return Ok(None);
    }
    Ok(Some(result.places.remove(0)))
}