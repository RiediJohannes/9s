use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Failed to parse temporal datatype: {0}")]
    ChronoFormat(#[from] chrono::ParseError),
}

pub fn parse_datetime(date_input: Option<String>, time_input: Option<String>) -> Result<NaiveDateTime, ParseError> {
    // default to today if no date is provided
    let date_value: NaiveDate = match date_input {
        Some(date_str) => try_parse_localized_date(&date_str),
        None => Local::now().naive_local().date(),
    };

    // default to current time if no date is provided
    let time_value: NaiveTime = match time_input {
        Some(time_str) => try_parse_localized_time(&time_str),
        None => Local::now().time(),
    };

    Ok(date_value.and_time(time_value))
}

fn try_parse_localized_date(date_str: &str) -> NaiveDate {
    // TODO Get common localized formats for dates

    // TODO Try to parse the date in one of these formats one after another

    todo!("Parse date")
}

fn try_parse_localized_time(date_str: &str) -> NaiveTime {
    // TODO Get common localized, accepted formats for time

    // TODO Try to parse the time in one of these formats one after another

    todo!("Parse time")
}