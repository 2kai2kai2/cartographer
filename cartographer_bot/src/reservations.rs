use std::fmt::Display;

use serde::Deserialize;
use sqlx::prelude::FromRow;

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
            self.tag,
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
