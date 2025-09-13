use anyhow::{Error, anyhow};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Serialize, Deserialize, Hash)]
pub struct StellarisDate {
    pub year: u16,
    /// In range `1..=12`
    pub month: u8,
    /// In range `1..=30`
    ///
    /// Every month in stellaris is 30 days
    pub day: u8,
}

impl StellarisDate {
    pub const fn new(year: u16, month: u8, day: u8) -> Option<StellarisDate> {
        if !matches!(month, 1..=12) || !matches!(day, 1..=22) {
            return None;
        }
        return Some(StellarisDate { year, month, day });
    }

    pub fn tomorrow(&self) -> StellarisDate {
        if self.day < 30 {
            return StellarisDate {
                year: self.year,
                month: self.month,
                day: self.day + 1,
            };
        } else if self.month == 12 {
            return StellarisDate {
                year: self.year + 1,
                month: 1,
                day: 1,
            };
        } else {
            return StellarisDate {
                year: self.year,
                month: self.month + 1,
                day: 1,
            };
        }
    }
    pub fn yesterday(&self) -> StellarisDate {
        if self.day > 1 {
            return StellarisDate {
                year: self.year,
                month: self.month,
                day: self.day - 1,
            };
        } else if self.month == 1 {
            return StellarisDate {
                year: self.year - 1,
                month: 12,
                day: 30,
            };
        } else {
            return StellarisDate {
                year: self.year,
                month: self.month - 1,
                day: 30,
            };
        }
    }
    pub fn iter_range_inclusive(
        first: StellarisDate,
        last: StellarisDate,
    ) -> impl Iterator<Item = StellarisDate> {
        return std::iter::successors(Some(first), move |curr| {
            if *curr >= last {
                None
            } else {
                Some(curr.tomorrow())
            }
        });
    }
    /// Iterates in reverse order, starting with `last`
    pub fn iter_range_inclusive_reversed(
        first: StellarisDate,
        last: StellarisDate,
    ) -> impl Iterator<Item = StellarisDate> {
        return std::iter::successors(Some(last), move |curr| {
            if *curr < first {
                None
            } else {
                Some(curr.yesterday())
            }
        });
    }

    /// Returns an EU4Date with the same date except the year
    pub fn with_year(&self, year: u16) -> StellarisDate {
        return StellarisDate {
            year,
            month: self.month,
            day: self.day,
        };
    }
}

impl FromStr for StellarisDate {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text
            .trim()
            .strip_prefix('"')
            .and_then(|text| text.strip_suffix('"'))
            .ok_or(anyhow!("Stellaris dates are saved in quotation marks."))?;
        let parts = text.split('.').collect::<Vec<&str>>();
        let [y, m, d] = parts.as_slice() else {
            return Err(Error::msg(format!(
                "Date string '{}' did not have a proper three parts",
                text
            )));
        };
        let year = y.parse::<u16>()?;
        let month = m.parse::<u8>()?;
        let day = d.parse::<u8>()?;

        if !matches!(month, 1..=12) {
            return Err(Error::msg(format!("Invalid month of year {}", text)));
        }
        if !matches!(day, 1..=30) {
            return Err(Error::msg(format!("Invalid day of month {}", text)));
        }

        return Ok(StellarisDate { year, month, day });
    }
}

impl Display for StellarisDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return f.write_fmt(format_args!(
            "{}.{:02}.{:02}",
            self.year, self.month, self.day
        ));
    }
}
