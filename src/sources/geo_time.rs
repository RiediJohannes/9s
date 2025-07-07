use super::common::Coordinates;
pub use chrono_tz::Tz;
use std::sync::OnceLock;
use tzf_rs::gen::Timezones;
use tzf_rs::Finder;

static FINDER: OnceLock<Finder> = OnceLock::new();

fn get_finder() -> &'static Finder {
    FINDER.get_or_init(|| {
        // Hard-coded relative path is fine here, since this will be evaluated at compile time and thus
        // throw an error during compilation if the file is not found.
        let file_bytes = include_bytes!("../../data/geographic_timezones.bin").to_vec();
        Finder::from_pb(Timezones::try_from(file_bytes).unwrap_or_default())
    })
}

pub fn init() {
    get_finder(); // force initialization of the static FINDER if it hasn't been initialized yet
}

pub fn get_timezone_name(coordinates: &Coordinates) -> String {
    get_finder().get_tz_name(coordinates.longitude, coordinates.latitude).to_string()
}

pub fn get_timezone(coordinates: &Coordinates) -> Option<Tz> {
    get_timezone_name(coordinates).parse().ok()
}