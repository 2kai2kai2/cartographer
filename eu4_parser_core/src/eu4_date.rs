use anyhow::Error;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};

#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    PartialOrd,
    Ord,
    FromPrimitive,
    ToPrimitive,
    Serialize,
    Deserialize,
)]
pub enum Month {
    JAN = 1,
    FEB,
    MAR,
    APR,
    MAY,
    JUN,
    JUL,
    AUG,
    SEP,
    OCT,
    NOV,
    DEC,
}

impl Month {
    pub fn length(&self) -> u8 {
        return [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][*self as usize];
    }
    pub fn month_num(&self) -> u8 {
        return *self as u8;
    }
    pub fn month_name(&self) -> &'static str {
        return match self {
            Month::JAN => "January",
            Month::FEB => "February",
            Month::MAR => "March",
            Month::APR => "April",
            Month::MAY => "May",
            Month::JUN => "June",
            Month::JUL => "July",
            Month::AUG => "August",
            Month::SEP => "September",
            Month::OCT => "October",
            Month::NOV => "November",
            Month::DEC => "December",
        };
    }
}
impl Display for Month {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            return f.write_str(self.month_name());
        }
        return f.write_fmt(format_args!("{}", self.month_num()));
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Serialize, Deserialize)]
pub struct EU4Date {
    pub year: u64,
    pub month: Month,
    pub day: u8,
}

impl FromStr for EU4Date {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let parts = text.trim().split('.').collect::<Vec<&str>>();
        let [y, m, d] = parts.as_slice() else {
            return Err(Error::msg(format!(
                "Date string '{}' did not have a proper three parts",
                text
            )));
        };
        let year = y.parse::<u64>()?;
        let month = Month::from_u8(m.parse::<u8>()?)
            .ok_or(Error::msg(format!("Invalid month {}", text)))?;
        let day = d.parse::<u8>()?;

        if day == 0 || day > month.length() {
            return Err(Error::msg(format!("Invalid day of month {}", text)));
        }

        return Ok(EU4Date { year, month, day });
    }
}

impl Display for EU4Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            return f.write_fmt(format_args!("{} {:#} {}", self.day, self.month, self.year));
        }
        return f.write_fmt(format_args!(
            "{}.{}.{}",
            self.year, self.month as u8, self.day
        ));
    }
}
