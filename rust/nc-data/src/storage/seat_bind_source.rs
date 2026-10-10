use super::{CampaignStore, CampaignStoreError};

/// How a seat was bound to its caller. A DOOR.SYS caller binds by BBS handle
/// first and retries with the real name; the retry may only match seats
/// bound by real name. A seat with no stored source counts as bound by real
/// name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeatBindSource {
    Handle,
    RealName,
}

impl SeatBindSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::Handle => "handle",
            Self::RealName => "real_name",
        }
    }
}

impl CampaignStore {
    /// Seats bound by handle, in ascending order.
    pub fn handle_bound_seats(&self) -> Result<Vec<usize>, CampaignStoreError> {
        let conn = self.connection()?;
        let mut stmt = conn.prepare(
            "SELECT player_record_index
             FROM seat_bind_sources
             WHERE bind_source = ?1
             ORDER BY player_record_index",
        )?;
        let seats = stmt
            .query_map([SeatBindSource::Handle.as_str()], |row| {
                row.get::<_, i64>(0)
            })?
            .map(|seat| seat.map(|seat| seat as usize))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(seats)
    }

    pub fn set_seat_bind_source(
        &self,
        player_record_index_1_based: usize,
        source: SeatBindSource,
    ) -> Result<(), CampaignStoreError> {
        let conn = self.connection()?;
        conn.execute(
            "INSERT INTO seat_bind_sources (player_record_index, bind_source)
             VALUES (?1, ?2)
             ON CONFLICT(player_record_index)
             DO UPDATE SET bind_source = excluded.bind_source",
            (player_record_index_1_based as i64, source.as_str()),
        )?;
        Ok(())
    }
}
