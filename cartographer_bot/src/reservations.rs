use std::{fmt::Display, io::Cursor, str::FromStr};

use serde::Deserialize;
use sqlx::prelude::FromRow;

use crate::assets::{CapitalLocations, Tags};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, FromRow)]
pub struct Reservation {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub tag: String,
    pub user_id: u64,
}
impl Reservation {
    pub fn format_with_tags(
        &self,
        tags: &Tags,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        return write!(
            f,
            "<@{}>: {} <t:{}>",
            self.user_id,
            tags.get_name_for_tag(&self.tag).unwrap_or(&self.tag),
            self.timestamp.timestamp()
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservationsData {
    pub reservations: Vec<Reservation>,
    pub game_type: GameType,
}
impl ReservationsData {
    pub fn new(game_type: GameType) -> ReservationsData {
        return ReservationsData {
            reservations: Vec::new(),
            game_type,
        };
    }

    pub fn len(&self) -> usize {
        return self.reservations.len();
    }

    pub fn reserved_tags<'a>(&'a self) -> impl Iterator<Item = &'a String> + 'a {
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

    pub async fn make_map(&self) -> anyhow::Result<image::RgbaImage> {
        const BASE_PATH: &str = "/app/assets"; // TODO: make this configurable

        let base_img_path = self.game_type.get_base_image_path(BASE_PATH);
        let icon_img_path = self.game_type.get_icon_path(BASE_PATH);
        let capital_locations_path = self.game_type.get_capital_locations_path(BASE_PATH);

        let capital_locations = tokio::fs::read_to_string(capital_locations_path).await?;
        let capital_locations = CapitalLocations::parse_new(capital_locations)?;
        let icon_locations: Vec<(f64, f64)> = self
            .reservations
            .iter()
            .filter_map(|res| capital_locations.get(&res.tag))
            .collect();
        drop(capital_locations);

        let res = tokio::try_join!(
            tokio::fs::read(base_img_path),
            tokio::fs::read(icon_img_path),
        );
        let (base_img, icon_x) = match res {
            Ok((base_img, icon_x)) => (base_img, icon_x),
            Err(err) => return Err(err.into()),
        };
        let mut img =
            image::ImageReader::with_format(Cursor::new(base_img), image::ImageFormat::Png)
                .decode()?
                .into_rgba8();
        let icon_x = image::ImageReader::with_format(Cursor::new(icon_x), image::ImageFormat::Png)
            .decode()?;

        for (x, y) in icon_locations {
            let x = x - icon_x.width() as f64 / 2.0;
            let y = img.height() as f64 - y - icon_x.height() as f64 / 2.0;
            image::imageops::overlay(&mut img, &icon_x, x.round() as i64, y.round() as i64);
        }
        return Ok(img);
    }

    pub async fn make_map_png(&self) -> anyhow::Result<Vec<u8>> {
        let img = self.make_map().await?;
        let mut img_vec: Vec<u8> = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut img_vec),
            image::ImageFormat::Png,
        )?;
        return Ok(img_vec);
    }

    pub fn format_with_game(
        &self,
        game_type: GameType,
        tags: &Tags,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        writeln!(f, "{game_type} Game Reservations\n")?;
        for res in &self.reservations {
            res.format_with_tags(tags, f)?;
            writeln!(f, "")?;
        }
        if self.reservations.is_empty() {
            writeln!(f, "*none*")?;
        }

        return Ok(());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "game_type")]
pub enum GameType {
    EU4,
    EU5,
}
impl GameType {
    pub fn get_base_image_path(
        &self,
        base_path: impl AsRef<std::path::Path>,
    ) -> std::path::PathBuf {
        return match self {
            GameType::EU4 => base_path.as_ref().join("eu4/vanilla/1444.png"),
            GameType::EU5 => base_path.as_ref().join("eu5/vanilla/1337.png"),
        };
    }
    pub fn get_icon_path(&self, base_path: impl AsRef<std::path::Path>) -> std::path::PathBuf {
        return match self {
            GameType::EU4 => base_path.as_ref().join("eu4/xIcon.png"),
            GameType::EU5 => base_path.as_ref().join("eu5/player.png"),
        };
    }
    pub fn get_capital_locations_path(
        &self,
        base_path: impl AsRef<std::path::Path>,
    ) -> std::path::PathBuf {
        return match self {
            GameType::EU4 => base_path.as_ref().join("eu4/vanilla/capitals.txt"),
            GameType::EU5 => base_path.as_ref().join("eu5/vanilla/capitals.txt"),
        };
    }
    pub fn get_tags_path(&self, base_path: impl AsRef<std::path::Path>) -> std::path::PathBuf {
        return match self {
            GameType::EU4 => base_path.as_ref().join("eu4/vanilla/tags.txt"),
            GameType::EU5 => base_path.as_ref().join("eu5/vanilla/tags.txt"),
        };
    }
}
impl FromStr for GameType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        return match s {
            "EU4" => Ok(GameType::EU4),
            "EU5" => Ok(GameType::EU5),
            _ => Err(()),
        };
    }
}
impl Display for GameType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            GameType::EU4 => write!(f, "EU4"),
            GameType::EU5 => write!(f, "EU5"),
        };
    }
}
