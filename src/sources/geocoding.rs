use serde::Deserialize;
use std::fmt;
use super::common::*;

const BASE_URL: &str = "https://geocoding-api.open-meteo.com/v1/search?count=10&language=de&format=json";

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct Place {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default)]
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
        const COUNTRY_LETTERS_OFFSET: u32 = ('🇦' as u32) - ('a' as u32);
        // offset the alpha-2 country codes to get the "regional indicator symbols" (country flags) in unicode
        let country: String = self.country
            .to_lowercase().chars()
            .map(|c| char::from_u32((c as u32) + COUNTRY_LETTERS_OFFSET)
                .unwrap_or(c.to_ascii_uppercase()))
            .collect();

        match self.get_district() {
            Some(district) => write!(f, "{} | {}, {}", country, self.name, district),
            None => write!(f, "{} | {}", country, self.name)
        }
    }
}
impl From<&Place> for Coordinates {
    fn from(place: &Place) -> Coordinates {
        Coordinates { longitude: place.longitude, latitude: place.latitude }
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


pub async fn query_place(client: &reqwest::Client, name: &str) -> Result<Vec<Place>, ApiError> {
    let parameters = format!("&name={name}", name = name);
    let url = format!("{}{}", BASE_URL, parameters);

    let response = client.get(url).send().await?;
    let payload = response.text().await?;

    match serde_json::from_str::<GeoResult>(&payload) {
        Ok(geo_result) => Ok(geo_result.places),
        Err(e) => {
            // If it fails, attempt to parse as GeoError
            match serde_json::from_str::<ClimateApiError>(&payload) {
                Ok(geo_error) => Err(ApiError::BadRequest { reason: geo_error.reason }),
                Err(_) => Err(ApiError::Parsing(e)), // Return the error if both parsing attempts fail
            }
        }
    }
}