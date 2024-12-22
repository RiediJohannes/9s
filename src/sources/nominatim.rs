use serde::Deserialize;
use std::fmt;
use std::fmt::Debug;
use super::types::*;
use AddressLevel::*;
use cached::SizedCache;
use cached::proc_macro::cached;


const BASE_URL: &str = "https://nominatim.openstreetmap.org/search?format=jsonv2&limit=10&\
                        addressdetails=1&namedetails=1&extratags=1&\
                        featureType=settlement&\
                        viewbox=55.030541,5.324132,45.850230,17.435780";
const CACHED_ITEMS: usize = 200;


#[derive(Deserialize, Clone)]
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
    pub address_type: AddressLevel,
    pub address: Address,
    #[serde(rename = "display_name")]
    pub full_name: String,
    pub extratags: Option<Extratags>,
    pub place_rank: i16,
    pub importance: f32,

    expected_name: Option<String>
}
impl Place {
    pub fn set_expected_name(&mut self, name: String) {
        self.expected_name = Some(name);
    }

    pub fn has_unexpected_name(&self) -> bool {
        match self.expected_name.as_ref() {
            Some(expected) => expected != &self.name.to_string(),
            None => true
        }
    }

    pub fn address_details(&self) -> String {
        AddressLevel::HIERARCHY.iter()
            .map(|level| self.address.get_address_level(level))
            .filter_map(|opt| opt.to_owned()) // filter out Nones and dereference Somes
            .collect::<Vec<_>>()
            .concat()
            .join(", ")
    }

    pub fn address_summary(&self) -> String {
        self.address_type.related_address_levels().iter()
            .map(|level| self.address.get_address_level(level))
            .filter_map(|option| { // filter out Nones but take the first string of Somes
                if let Some(list) = option {
                    if let Some(first) = list.first() { // only takes the first string to keep it short
                        return Some(first.to_string());
                    }
                }

                None
            })
            .collect::<Vec<String>>()
            .join(", ")
    }

    pub fn country_indicator(&self) -> String {
        const COUNTRY_LETTERS_OFFSET: u32 = ('🇦' as u32) - ('a' as u32);
        const COUNTRY_FALLBACK: &str = "??";

        match self.address.country_code.as_deref() {
            Some(code) => {
                // offset the alpha-2 country codes to get the "regional indicator symbols" (country flags) in Unicode
                code.to_lowercase().chars()
                    .map(|c| char::from_u32((c as u32) + COUNTRY_LETTERS_OFFSET)
                        .unwrap_or(c.to_ascii_uppercase()))
                    .collect()
            }
            None => COUNTRY_FALLBACK.to_string()
        }
    }
}
impl fmt::Display for Place {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let country_letters = self.country_indicator();

        // if the expected place name is not its default name, add it in parentheses
        let summary = if self.has_unexpected_name() {
            let replacement = format!("{} ({})", self.name, self.expected_name.clone().unwrap());
            self.address_details().replacen(&self.name.to_string(), &replacement, 1)
        } else {
            self.address_details()
        };

        write!(f, "{} | {}", country_letters, summary)
    }
}
impl fmt::Debug for Place {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} [lat: {}, lon: {}]", &self.name, &self.lat, &self.lon)
    }
}
impl From<&Place> for Option<Coordinates> {
    fn from(place: &Place) -> Option<Coordinates> {
        match (place.lat.parse::<f64>(), place.lon.parse::<f64>()) {
            (Ok(lat), Ok(lon)) => Some(Coordinates::new(lat, lon)),
            _ => None,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Address {
    hamlet: Option<String>,
    croft: Option<String>,
    isolated_dwelling: Option<String>,

    municipality: Option<String>,
    village: Option<String>,
    town: Option<String>,
    city: Option<String>,

    district: Option<String>,
    city_district: Option<String>,
    borough: Option<String>,
    suburb: Option<String>,
    subdivision: Option<String>,

    pub postcode: Option<String>,

    neighbourhood: Option<String>,
    allotments: Option<String>,
    quarter: Option<String>,

    county: Option<String>,
    state_district: Option<String>,
    //region: Option<String>,

    state: Option<String>,
    province: Option<String>,

    pub country: Option<String>,
    pub country_code: Option<String>,
    pub continent: Option<String>,

    #[serde(rename = "iso3166_2_lvl4")]
    pub iso3166_l4: Option<String>,
    #[serde(rename = "iso3166_2_lvl6")]
    pub iso3166_l6: Option<String>,
}
impl Address {
    pub fn neighbourhood(&self) -> Option<Vec<String>> {
        let neighbourhood_levels = [
            self.neighbourhood.as_ref(),
            self.allotments.as_ref(),
            self.quarter.as_ref(),
        ];

        collect_somes(&neighbourhood_levels)
    }

    pub fn district(&self) -> Option<Vec<String>> {
        let district_levels = [
            self.city_district.as_ref(),
            self.suburb.as_ref(),
            self.subdivision.as_ref(),
            self.borough.as_ref(),
        ];

        collect_somes(&district_levels)
    }

    pub fn hamlet(&self) -> Option<Vec<String>> {
        let hamlet_levels = [
            self.hamlet.as_ref(),
            self.croft.as_ref(),
            self.isolated_dwelling.as_ref(),
        ];

        collect_somes(&hamlet_levels)
    }

    pub fn municipality(&self) -> Option<Vec<String>> {
        let municipality_levels = [
            self.village.as_ref(),
            self.town.as_ref(),
            self.municipality.as_ref(),
            self.city.as_ref(),
        ];

        collect_somes(&municipality_levels)
    }

    pub fn county(&self) -> Option<Vec<String>> {
        let county_levels = [
            self.county.as_ref(),
            self.state_district.as_ref(),
        ];

        collect_somes(&county_levels)
    }

    pub fn state(&self) -> Option<Vec<String>> {
        let state_levels = [
            self.state.as_ref(),
            self.province.as_ref(),
        ];

        collect_somes(&state_levels)
    }

    #[inline]
    pub fn get_address_level(&self, level: &AddressLevel) -> Option<Vec<String>> {
        match level {
            Neighbourhood => self.neighbourhood(),
            District => self.district(),
            Hamlet => self.hamlet(),
            Municipality => self.municipality(),
            County => self.county(),
            State => self.state(),
            Country => self.country.as_ref().map(|c| vec![c.to_string()]),
            Continent => self.continent.as_ref().map(|c| vec![c.to_string()]),
            Other(_) => None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Hash)]
pub enum AddressLevel {
    #[serde(alias = "neighbourhood", alias = "quarter", alias = "allotments")]
    Neighbourhood,
    #[serde(alias = "city_district", alias = "borough", alias = "subdivision", alias = "suburb")]
    District,
    #[serde(alias = "hamlet", alias = "isolated_dwelling", alias = "croft")]
    Hamlet,
    #[serde(alias = "village", alias = "city", alias = "town", alias = "municipality", alias = "locality")]
    Municipality,
    #[serde(alias = "state_district", alias = "county")]
    County,
    #[serde(alias = "state", alias = "province")] // region is purposely left out
    State,
    #[serde(alias = "country")]
    Country,
    #[serde(alias = "continent")]
    Continent,
    #[serde(untagged)]
    Other(String)
}
impl AddressLevel {
    const HIERARCHY: [AddressLevel; 6] = [Neighbourhood, District, Hamlet, Municipality, County, State];

    pub fn related_address_levels(&self) -> &'static [AddressLevel] {
        match self {
            Neighbourhood => &[Neighbourhood, District, Municipality, State],
            District => &[District, Municipality, State],
            Hamlet => &[Hamlet, Municipality, County, State],
            Municipality => &[Municipality, County, State],
            County => &[County, State],
            State => &[State, Country],
            Country => &[Country, Continent],
            Continent => &[Continent],
            _ => &[]
        }
    }
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
    pub capital: Option<String>,
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

// --------------------- functions --------------------

// Cache up to 200 place requests and their responses (result = true -> only cache Ok variants)
#[cached(
    ty = "SizedCache<String, Vec<Place>>",
    create = "{ SizedCache::with_size(CACHED_ITEMS) }",
    convert = r#"{ name.to_string() }"#,
    result = true
)]
pub async fn query_place(client: &reqwest::Client, name: &str) -> Result<Vec<Place>, ApiError> {
    let parameters = format!("&city={name}", name = name);
    let url = format!("{}{}", BASE_URL, parameters);

    let response = client.get(&url).send().await?;
    let payload = response.text().await?;

    match serde_json::from_str::<Vec<Place>>(&payload) {
        Ok(mut place_list) => {
            for place in place_list.iter_mut() {
                place.expected_name = Some(name.to_string());
            }

            Ok(place_list)
        },
        Err(e) => {
            // If it fails, attempt to parse as GeoError
            match serde_json::from_str::<NominatimError>(&payload) {
                Ok(nomi_error) => Err(ApiError::BadRequest { reason: nomi_error.message }),
                Err(_) => Err(ApiError::Parsing(e)), // Return the error if both parsing attempts fail
            }
        }
    }
}

fn collect_somes<'a, I>(s: I) -> Option<Vec<String>>
    where I: IntoIterator<Item = &'a Option<&'a String>>
{
    let string_list = s.into_iter()
        .filter(|f| f.is_some())
        .map(|s| s.unwrap().to_string())
        .collect::<Vec<String>>();

    if string_list.is_empty() {
        None
    } else {
        Some(string_list)
    }
}