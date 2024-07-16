use serde::Deserialize;
use std::fmt;
use super::types::{ApiError, ClimateApiError};

const BASE_URL: &str = "https://geocoding-api.open-meteo.com/v1/search?count=10&language=de&format=json";

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct Place {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub elevation: f32,
    pub timezone: String,
    /// Alpha-2 country code
    #[serde(rename = "country_code")]
    pub country: String,
    admin1: Option<String>,
    admin2: Option<String>,
    admin3: Option<String>,
    admin4: Option<String>,
}
impl fmt::Display for Place {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.get_district() {
            Some(district) => write!(f, "{}, {}, {}", self.name, self.country, district),
            None => write!(f, "{}, {}", self.name, self.country)
        }
    }
}
impl Place {
    pub fn get_district(&self) -> Option<&String> {
        let admins = [&self.admin1, &self.admin2, &self.admin3, &self.admin4];
        let present_admins: Vec<&String> = admins.iter().filter_map(|field| field.as_ref()).collect();

        match present_admins.len() {
            0 => None,
            1 => Some(present_admins[0]),
            _ => Some(present_admins[present_admins.len() - 2])
        }
    }

    pub fn info(&self) -> String {
        format!("{} [lat: {}, lon: {}]", &self.name, &self.latitude, &self.longitude)
    }
}

#[derive(Deserialize, Debug)]
struct GeoResult {
    #[serde(alias = "results", default)]
    places: Vec<Place>,
}


pub async fn query_place(name: &str) -> Result<Vec<Place>, ApiError> {
    let parameters = format!("&name={name}", name = name);
    let url = format!("{}{}", BASE_URL, parameters);

    let response = reqwest::get(&url).await?;
    let payload = response.text().await?;

    match serde_json::from_str::<GeoResult>(&payload) {
        Ok(geo_result) => Ok(geo_result.places),
        Err(_) => {
            // If it fails, attempt to parse as GeoError
            match serde_json::from_str::<ClimateApiError>(&payload) {
                Ok(geo_error) => Err(ApiError::BadRequest { reason: geo_error.reason }),
                Err(e) => Err(ApiError::Parsing(e)), // Return the error if both parsing attempts fail
            }
        }
    }
}