use std::fmt::Display;

use num_traits::FromPrimitive as _;
use serde::{Deserialize, Serialize};

use crate::{
    BinDeserialize, TextDeserialize, bin_deserialize::BinError, bin_err,
    text_deserialize::TextError, text_lexer::TextToken,
};

pub use crate::eu4_date::Month;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Serialize, Deserialize)]
pub struct EU5Date {
    pub year: u16,
    pub month: Month,
    /// starts at 1, goes to [`Month::length`]
    pub day: u8,
    /// goes from 0 to 23
    pub hour: u8,
}
impl EU5Date {
    /// `day_of_year` should be passed in the range `0..365`
    fn handle_day_of_year(day_of_year: i32) -> Result<(Month, u8), BinError> {
        const START_JAN: i32 = 0;
        const START_FEB: i32 = START_JAN + Month::JAN.length() as i32;
        const START_MAR: i32 = START_FEB + Month::FEB.length() as i32;
        const START_APR: i32 = START_MAR + Month::MAR.length() as i32;
        const START_MAY: i32 = START_APR + Month::APR.length() as i32;
        const START_JUN: i32 = START_MAY + Month::MAY.length() as i32;
        const START_JUL: i32 = START_JUN + Month::JUN.length() as i32;
        const START_AUG: i32 = START_JUL + Month::JUL.length() as i32;
        const START_SEP: i32 = START_AUG + Month::AUG.length() as i32;
        const START_OCT: i32 = START_SEP + Month::SEP.length() as i32;
        const START_NOV: i32 = START_OCT + Month::OCT.length() as i32;
        const START_DEC: i32 = START_NOV + Month::NOV.length() as i32;

        return match day_of_year {
            START_JAN..START_FEB => Ok((Month::JAN, (day_of_year - START_JAN) as u8)),
            START_FEB..START_MAR => Ok((Month::FEB, (day_of_year - START_FEB) as u8)),
            START_MAR..START_APR => Ok((Month::MAR, (day_of_year - START_MAR) as u8)),
            START_APR..START_MAY => Ok((Month::APR, (day_of_year - START_APR) as u8)),
            START_MAY..START_JUN => Ok((Month::MAY, (day_of_year - START_MAY) as u8)),
            START_JUN..START_JUL => Ok((Month::JUN, (day_of_year - START_JUN) as u8)),
            START_JUL..START_AUG => Ok((Month::JUL, (day_of_year - START_JUL) as u8)),
            START_AUG..START_SEP => Ok((Month::AUG, (day_of_year - START_AUG) as u8)),
            START_SEP..START_OCT => Ok((Month::SEP, (day_of_year - START_SEP) as u8)),
            START_OCT..START_NOV => Ok((Month::OCT, (day_of_year - START_OCT) as u8)),
            START_NOV..START_DEC => Ok((Month::NOV, (day_of_year - START_NOV) as u8)),
            START_DEC..365 => Ok((Month::DEC, (day_of_year - START_DEC) as u8)),
            _ => Err(bin_err!("Invalid day of year {day_of_year}")),
        };
    }
}
impl BinDeserialize<'_> for EU5Date {
    fn take(
        mut stream: crate::bin_deserialize::BinDeserializer<'_>,
    ) -> ::std::result::Result<
        (Self, crate::bin_deserialize::BinDeserializer<'_>),
        crate::bin_deserialize::BinError,
    > {
        let mut as_num: i32 = stream
            .parse()
            .map_err(|err| err.context("While parsing an EU5 date"))?;

        let hour = (as_num % 24) as u8;
        // silly encoding format
        if hour < 8 || 19 < hour {
            let real_hour = ((hour as i32) - 8) * 2;
            return Err(bin_err!("Invalid EU5 date hour '{hour}' ({real_hour})"));
        }
        let hour = (hour - 8) * 2;
        as_num /= 24;

        let day_of_year = as_num % 365;
        let (month, day) = Self::handle_day_of_year(day_of_year)?;

        let Some(year) = (as_num / 365)
            .checked_sub(5000)
            .and_then(|year| year.try_into().ok())
        else {
            return Err(bin_err!("Invalid EU5 date year (was out of range)"));
        };
        return Ok((
            EU5Date {
                year,
                month,
                day,
                hour,
            },
            stream,
        ));
    }
}
impl EU5Date {
    fn parse_year(text: &str) -> Result<u16, TextError> {
        let Ok(year) = text.parse::<u16>() else {
            return Err(TextError::Custom(format!(
                "Failed to parse an EU5 date year '{text}'"
            )));
        };
        return Ok(year);
    }
    fn parse_month(text: &str) -> Result<Month, TextError> {
        let Ok(month) = text.parse::<u8>() else {
            return Err(TextError::Custom(format!(
                "Failed to parse an EU5 date month '{text}'"
            )));
        };
        let Some(month) = Month::from_u8(month) else {
            return Err(TextError::Custom(format!(
                "Invalid EU5 date month '{month}'"
            )));
        };
        return Ok(month);
    }
    fn parse_day(text: &str, month: Month) -> Result<u8, TextError> {
        let Ok(day) = text.parse::<u8>() else {
            return Err(TextError::Custom(format!(
                "Failed to parse an EU5 date day '{text}'"
            )));
        };
        if day == 0 || day > month.length() {
            return Err(TextError::Custom(format!(
                "Invalid EU5 date day '{day}' for month {month} with length {}",
                month.length()
            )));
        }
        return Ok(day);
    }
    fn parse_hour(text: &str) -> Result<u8, TextError> {
        let Ok(hour) = text.parse::<u8>() else {
            return Err(TextError::Custom(format!(
                "Failed to parse an EU5 date hour '{text}'"
            )));
        };
        // silly encoding format
        if hour < 8 || 19 < hour {
            let real_hour = ((hour as i32) - 8) * 2;
            return Err(TextError::Custom(format!(
                "Invalid EU5 date hour '{hour}' ({real_hour})"
            )));
        }
        let hour = (hour - 8) * 2;
        return Ok(hour);
    }
}
impl TextDeserialize<'_> for EU5Date {
    fn take_text(
        mut stream: crate::text_deserialize::TextDeserializer<'_>,
    ) -> ::std::result::Result<
        (Self, crate::text_deserialize::TextDeserializer<'_>),
        crate::text_deserialize::TextError,
    > {
        let TextToken::StringUnquoted(text) = stream.peek_token().ok_or(TextError::EOF)? else {
            return Err(TextError::UnexpectedToken.context("While parsing an EU5 date"));
        };

        let (year, text) = text.split_once('.').ok_or(TextError::EOF)?;
        let year = Self::parse_year(year)?;

        let (month, text) = text.split_once('.').ok_or(TextError::EOF)?;
        let month = Self::parse_month(month)?;

        let Some((day, text)) = text.split_once('.') else {
            // no hour, assume it's 0
            let day = Self::parse_day(text, month)?;
            stream.eat_token();
            return Ok((
                EU5Date {
                    year,
                    month,
                    day,
                    hour: 0,
                },
                stream,
            ));
        };

        // with hour
        let day = Self::parse_day(day, month)?;
        let hour = Self::parse_hour(text)?;
        stream.eat_token();
        return Ok((
            EU5Date {
                year,
                month,
                day,
                hour,
            },
            stream,
        ));
    }
}
impl Display for EU5Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            return f.write_fmt(format_args!(
                "{:02}:00, {} {}, {}",
                self.hour,
                self.day,
                self.month.month_name(),
                self.year
            ));
        }
        return f.write_fmt(format_args!(
            "{}.{}.{}.{}",
            self.year, self.month, self.day, self.hour
        ));
    }
}
