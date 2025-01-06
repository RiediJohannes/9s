use super::common::*;
use crate::sources::common;
use bot_macros::collect_fields;
use cached::proc_macro::cached;
use cached::SizedCache;
use codes_iso_639::part_1::LanguageCode;
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt;
use AddressLevel::*;


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
}
impl Place {
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

        // if the local place name differs from its name in the app language, add the latter in parentheses
        let summary = if let Some(expected) = self.name.get_lang(crate::LANGUAGE) {
            if !self.name.local.contains(expected) {
                let replacement = format!("{} ({})", self.name, expected);
                self.address_details().replacen(&self.name.to_string(), &replacement, 1)
            } else {
                self.address_details()
            }
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
#[collect_fields({
   neighbourhood = [neighbourhood, allotments, quarter],
   district = [city_district, suburb, subdivision, borough],
   hamlet = [hamlet, croft, isolated_dwelling],
   municipality = [village, town, municipality, city],
   county = [county, state_district],
   state = [state, province]
})]
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
    #[serde(rename = "neighbourhood", alias = "quarter", alias = "allotments")]
    Neighbourhood,
    #[serde(rename = "city_district", alias = "borough", alias = "subdivision", alias = "suburb")]
    District,
    #[serde(rename = "hamlet", alias = "isolated_dwelling", alias = "croft")]
    Hamlet,
    #[serde(rename = "municipality", alias = "city", alias = "town", alias = "village", alias = "locality")]
    Municipality,
    #[serde(rename = "county", alias = "state_district")]
    County,
    #[serde(rename = "state", alias = "province")] // region is purposely left out
    State,
    #[serde(rename = "country")]
    Country,
    #[serde(rename = "continent")]
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

    #[serde(flatten)]
    global: HashMap<String, String>,
}
impl PlaceName {
    const NAME_PREFIX: &'static str = "name:";

    pub fn get_lang(&self, lang: LanguageCode) -> Option<&String> {
        let name_key = |code: &str| format!("{}{}", Self::NAME_PREFIX, code);

        self.global.get(name_key(lang.code()).as_str())
    }

    pub fn get_lang_or(&self, lang: LanguageCode, default_lang: LanguageCode) -> Option<&String> {
        self.get_lang(lang).or(self.get_lang(default_lang))
    }

    pub fn get_lang_or_default(&self, lang: LanguageCode) -> Option<&String> {
        self.get_lang_or(lang, LanguageCode::En)
    }
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
    pub capital: Option<String>,
    pub population: Option<String>,
    pub population_date: Option<String>,
}

type NominatimResult = Vec<Place>;

#[derive(Deserialize, Debug, Clone)]
pub struct NominatimError {
    pub code: u16,
    pub message: String,
}
impl From<NominatimError> for ApiError {
    fn from(nomi_error: NominatimError) -> Self {
        ApiError::BadRequest { reason: nomi_error.message }
    }
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
    let params = [("city", name.to_string())];

    common::query_api::<Vec<Place>, NominatimResult, ClimateApiError>
        (client, BASE_URL, params).await
}