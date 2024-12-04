use serde::Deserialize;
use std::fmt;
use super::types::*;


const BASE_URL: &str = "https://nominatim.openstreetmap.org/search?format=jsonv2&limit=10&\
                        addressdetails=1&namedetails=1&extratags=1";

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct Place {
    #[serde(rename = "place_id")]
    pub id: i64,
    pub lat: String,
    pub lon: String,
    pub category: String,
    #[serde(rename = "namedetails")]
    pub name: PlaceName,
    #[serde(rename = "addresstype")]
    pub address_type: String,
    pub address: Address,
    #[serde(rename = "display_name")]
    pub full_name: String,
    pub extratags: Option<Extratags>,
    pub place_rank: i16,
    pub importance: f32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Address {
    pub municipality: Option<String>,
    pub village: Option<String>,
    pub town: Option<String>,
    pub city: Option<String>,

    pub district: Option<String>,
    pub city_district: Option<String>,
    pub borough: Option<String>,
    pub suburb: Option<String>,
    pub subdivision: Option<String>,

    pub postcode: Option<String>,

    pub neighbourhood: Option<String>,
    pub allotments: Option<String>,
    pub quarter: Option<String>,

    pub county: Option<String>,
    pub region: Option<String>,
    pub state: Option<String>,
    pub state_district: Option<String>,

    pub country: Option<String>,
    pub country_code: Option<String>,
    pub continent: Option<String>,

    #[serde(rename = "iso3166_2_lvl4")]
    pub iso3166_l4: Option<String>,
    #[serde(rename = "iso3166_2_lvl6")]
    pub iso3166_l6: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PlaceName {
    #[serde(rename = "name")]
    pub local: String,
    #[serde(rename = "name:de")]
    pub name_de: Option<String>,
    #[serde(rename = "name:en")]
    pub name_en: Option<String>,
}
impl fmt::Display for PlaceName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.local)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Extratags {
    pub wikidata: Option<String>,
    pub wikipedia: Option<String>,
    pub website: Option<String>,
    #[serde(default)]
    pub capital: Option<bool>,
    pub population: Option<String>,
    pub population_date: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct NominatimError {
    pub code: u16,
    pub message: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct NominatimErrorDetails {
    pub reason: String,
}


impl From<&Place> for Option<Coordinates> {
    fn from(place: &Place) -> Option<Coordinates> {
        match (place.lat.parse::<f64>(), place.lon.parse::<f64>()) {
            (Ok(lat), Ok(lon)) => Some(Coordinates::new(lat, lon)),
            _ => None,
        }
    }
}
impl fmt::Display for Place {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const COUNTRY_LETTERS_OFFSET: u32 = ('🇦' as u32) - ('a' as u32);
        // offset the alpha-2 country codes to get the "regional indicator symbols" (country flags) in Unicode
        let country_letters: String = self.address.country_code.clone().unwrap_or("AT".to_string()) // TODO Handle this case more gracefully
            .to_lowercase().chars()
            .map(|c| char::from_u32((c as u32) + COUNTRY_LETTERS_OFFSET)
                .unwrap_or(c.to_ascii_uppercase()))
            .collect();

        write!(f, "{} | {}", country_letters, self.name)
        //write!(f, "{} | {}, {}", country_letters, self.name.local, self.address.district)
    }
}
impl Place {
    /*pub fn get_district(&self) -> Option<&String> {
        let admins = [&self.admin1, &self.admin2, &self.admin3, &self.admin4];
        let present_admins: Vec<&String> = admins.iter().filter_map(|field| field.as_ref()).collect();

        match present_admins.len() {
            0 => None,
            1 => Some(present_admins[0]),
            _ => Some(present_admins[present_admins.len() - 2])
        }
    }*/

    pub fn info(&self) -> String {
        format!("{} [lat: {}, lon: {}]", &self.name.local, &self.lat, &self.lon)
    }
}


pub async fn query_place(client: &reqwest::Client, name: &str) -> Result<Vec<Place>, ApiError> {
    let parameters = format!("&city={name}", name = name);
    let url = format!("{}{}", BASE_URL, parameters);

    let response = client.get(&url).send().await?;
    let payload = response.text().await?;

    match serde_json::from_str::<Vec<Place>>(&payload) {
        Ok(place_list) => Ok(place_list),
        Err(e) => {
            // If it fails, attempt to parse as GeoError
            match serde_json::from_str::<NominatimError>(&payload) {
                Ok(nomi_error) => Err(ApiError::BadRequest { reason: nomi_error.message }),
                Err(_) => Err(ApiError::Parsing(e)), // Return the error if both parsing attempts fail
            }
        }
    }
}