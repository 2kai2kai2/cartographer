use sqlx::prelude::*;

use crate::reservations::Reservation;

#[derive(FromRow)]
pub struct RawReservation {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub tag: String,
    pub user_id: i64,
}
impl From<RawReservation> for Reservation {
    fn from(value: RawReservation) -> Self {
        return Reservation {
            timestamp: value.timestamp,
            tag: value.tag,
            user_id: value.user_id as u64,
        };
    }
}
