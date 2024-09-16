use std::fmt::Display;

use serde::Deserialize;
use sqlx::prelude::FromRow;

use crate::TAGS;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, FromRow)]
pub struct Reservation {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub tag: String,
    pub user_id: u64,
}
impl Display for Reservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(
            f,
            "<@{}>: {} <t:{}>",
            self.user_id,
            TAGS.get(&self.tag).map_or(&self.tag, |names| &names[0]),
            self.timestamp.timestamp()
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservationsData {
    pub reservations: Vec<Reservation>,
}
impl ReservationsData {
    pub fn new() -> ReservationsData {
        return ReservationsData {
            reservations: Vec::new(),
        };
    }

    pub fn len(&self) -> usize {
        return self.reservations.len();
    }

    pub fn reserved_tags<'a>(&'a self) -> impl Iterator<Item = &String> + 'a {
        return self.reservations.iter().map(|res| &res.tag);
    }

    /// Returns `Ok(())` if it succeeds, or `Err(())` if already taken.
    pub fn try_add(&mut self, reservation: Reservation) -> Result<(), ()> {
        if self
            .reservations
            .iter()
            .any(|existing| existing.tag == reservation.tag)
        {
            return Err(());
        }

        match self
            .reservations
            .iter_mut()
            .find(|existing| existing.user_id == reservation.user_id)
        {
            None => self.reservations.push(reservation),
            Some(user_res) => {
                *user_res = reservation;
                self.reservations.sort();
            }
        }
        return Ok(());
    }

    pub fn remove(&mut self, user_id: u64) {
        let Some(index) = self
            .reservations
            .iter()
            .position(|res| res.user_id == user_id)
        else {
            return;
        };
        self.reservations.remove(index);
    }

    pub fn make_map(&self) -> anyhow::Result<image::RgbaImage> {
        let mut img =
            image::load_from_memory_with_format(crate::PNG_MAP_1444, image::ImageFormat::Png)?
                .into_rgba8();
        let icon_x =
            image::load_from_memory_with_format(crate::PNG_ICON_X, image::ImageFormat::Png)?;
        for reservation in &self.reservations {
            let Some((x, y)) = crate::CAPITAL_LOCATIONS.get(&reservation.tag) else {
                continue;
            };
            let x = x.round() - icon_x.width() as f64 / 2.0;
            let y = 2048.0 - y.round() - icon_x.height() as f64 / 2.0;
            image::imageops::overlay(&mut img, &icon_x, x as i64, y as i64);
        }
        return Ok(img);
    }

    pub fn make_map_png(&self) -> anyhow::Result<Vec<u8>> {
        let img = self.make_map()?;
        let mut img_vec: Vec<u8> = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut img_vec),
            image::ImageFormat::Png,
        )?;
        return Ok(img_vec);
    }
}
impl Display for ReservationsData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "EU4 Game Reservations\n")?;
        for res in &self.reservations {
            writeln!(f, "{res}")?;
        }
        if self.reservations.is_empty() {
            writeln!(f, "*none*")?;
        }

        return Ok(());
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test123() {
        let mut res = ReservationsData::new();
        res.try_add(Reservation {
            tag: "ENG".to_string(),
            timestamp: chrono::Utc::now(),
            user_id: 123,
        })
        .unwrap();
        let img = res.make_map().unwrap();
        img.save("./output.png").unwrap();
    }
}
