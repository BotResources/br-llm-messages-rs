use serde::{Deserialize, Serialize};

use crate::error::{Error, ErrorCode};

const MAX_FRACTION_DIGITS: usize = 9;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct ObservedAt(String);

impl ObservedAt {
    pub fn new(value: impl Into<String>) -> Result<Self, Error> {
        let value = value.into();
        if !is_utc_timestamp(&value) {
            return Err(Error::new(ErrorCode::InvalidContent));
        }
        Ok(ObservedAt(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ObservedAt {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Error> {
        ObservedAt::new(value)
    }
}

impl AsRef<str> for ObservedAt {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

fn is_utc_timestamp(text: &str) -> bool {
    let Some(body) = text.as_bytes().strip_suffix(b"Z") else {
        return false;
    };
    let (Some(date), Some(separator), Some(time)) = (body.get(..10), body.get(10), body.get(11..))
    else {
        return false;
    };
    *separator == b'T' && is_date(date) && is_time(time)
}

fn number(bytes: &[u8], from: usize, to: usize) -> Option<u32> {
    bytes.get(from..to)?.iter().try_fold(0u32, |total, byte| {
        byte.is_ascii_digit()
            .then(|| total * 10 + u32::from(byte - b'0'))
    })
}

fn is_date(date: &[u8]) -> bool {
    if date.get(4) != Some(&b'-') || date.get(7) != Some(&b'-') {
        return false;
    }
    let (Some(year), Some(month), Some(day)) =
        (number(date, 0, 4), number(date, 5, 7), number(date, 8, 10))
    else {
        return false;
    };
    (1..=days_in_month(year, month)).contains(&day)
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: u32) -> bool {
    let divisible = |by: u32| year.checked_rem(by) == Some(0);
    divisible(4) && (!divisible(100) || divisible(400))
}

fn is_time(time: &[u8]) -> bool {
    if time.get(2) != Some(&b':') || time.get(5) != Some(&b':') {
        return false;
    }
    let (Some(hour), Some(minute), Some(second)) =
        (number(time, 0, 2), number(time, 3, 5), number(time, 6, 8))
    else {
        return false;
    };
    if hour > 23 || minute > 59 || second > 59 {
        return false;
    }
    match time.get(8..) {
        Some([]) => true,
        Some([b'.', fraction @ ..]) => {
            (1..=MAX_FRACTION_DIGITS).contains(&fraction.len())
                && fraction.iter().all(u8::is_ascii_digit)
        }
        _ => false,
    }
}
