use rusqlite::{Connection, Row, Transaction, named_params, params};

use super::{CampaignStoreError, insert_row, real48_at, real48_column};
use crate::model::{
    Coords, CrippledShips, EmpireId, EscapeEpisode, Fleet, FleetId, Order, RecordNumber, ShipCounts,
};

/// One row per FLEETS.DAT record, plus the escape episode the fleet is in
/// (NULL when it is in none): the sector and layer it is escaping from and
/// the bit set of empires that have had their pursuit exchange.
pub(super) const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS snapshot_fleets (
        snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
        record_index INTEGER NOT NULL,
        fleet_number INTEGER NOT NULL,
        owner INTEGER NOT NULL,
        next_record INTEGER NOT NULL,
        record_number INTEGER NOT NULL,
        prev_record INTEGER NOT NULL,
        max_speed INTEGER NOT NULL,
        speed INTEGER NOT NULL,
        sector_x INTEGER NOT NULL,
        sector_y INTEGER NOT NULL,
        x_fraction_real48 INTEGER NOT NULL,
        y_fraction_real48 INTEGER NOT NULL,
        depth_real48 INTEGER NOT NULL,
        standing_order INTEGER NOT NULL,
        target_x INTEGER NOT NULL,
        target_y INTEGER NOT NULL,
        target_depth INTEGER NOT NULL,
        mission_target INTEGER NOT NULL,
        rules_of_engagement INTEGER NOT NULL,
        battleships INTEGER NOT NULL,
        cruisers INTEGER NOT NULL,
        destroyers INTEGER NOT NULL,
        empty_transports INTEGER NOT NULL,
        loaded_transports INTEGER NOT NULL,
        etacs INTEGER NOT NULL,
        scouts INTEGER NOT NULL,
        escape_sector_x INTEGER,
        escape_sector_y INTEGER,
        escape_layer INTEGER,
        escape_pursuers INTEGER,
        PRIMARY KEY(snapshot_id, record_index)
    );";

pub(super) fn write<'a>(
    tx: &Transaction<'_>,
    snapshot_id: i64,
    records: impl IntoIterator<Item = (usize, &'a Fleet)>,
) -> Result<(), CampaignStoreError> {
    for (index, fleet) in records {
        let ships = &fleet.ships;
        insert_row(
            tx,
            "snapshot_fleets",
            named_params! {
                ":snapshot_id": snapshot_id,
                ":record_index": index + 1,
                ":fleet_number": fleet.fleet_number.0,
                ":owner": fleet.owner.0,
                ":next_record": fleet.next.0,
                ":record_number": fleet.record.0,
                ":prev_record": fleet.prev.0,
                ":max_speed": fleet.max_speed,
                ":speed": fleet.speed,
                ":sector_x": fleet.sector.x,
                ":sector_y": fleet.sector.y,
                ":x_fraction_real48": real48_column(fleet.x_fraction),
                ":y_fraction_real48": real48_column(fleet.y_fraction),
                ":depth_real48": real48_column(fleet.depth),
                ":standing_order": fleet.order.code(),
                ":target_x": fleet.target.x,
                ":target_y": fleet.target.y,
                ":target_depth": fleet.target_depth,
                ":mission_target": fleet.mission_target,
                ":rules_of_engagement": fleet.rules_of_engagement,
                ":battleships": ships.battleships,
                ":cruisers": ships.cruisers,
                ":destroyers": ships.destroyers,
                ":empty_transports": ships.empty_transports,
                ":loaded_transports": ships.loaded_transports,
                ":etacs": ships.etacs,
                ":scouts": ships.scouts,
                ":escape_sector_x": fleet.escape.map(|escape| escape.sector.x),
                ":escape_sector_y": fleet.escape.map(|escape| escape.sector.y),
                ":escape_layer": fleet.escape.map(|escape| escape.layer),
                ":escape_pursuers": fleet.escape.map(|escape| escape.pursuers),
            },
        )?;
    }
    Ok(())
}

pub(super) fn load(conn: &Connection, snapshot_id: i64) -> Result<Vec<Fleet>, CampaignStoreError> {
    let mut stmt =
        conn.prepare("SELECT * FROM snapshot_fleets WHERE snapshot_id = ?1 ORDER BY record_index")?;
    let records = stmt
        .query_map(params![snapshot_id], fleet_from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(records)
}

fn fleet_from_row(row: &Row<'_>) -> rusqlite::Result<Fleet> {
    let escape = match row.get::<_, Option<u8>>("escape_sector_x")? {
        Some(x) => Some(EscapeEpisode {
            sector: Coords::new(x, row.get("escape_sector_y")?),
            layer: row.get("escape_layer")?,
            pursuers: row.get("escape_pursuers")?,
        }),
        None => None,
    };
    Ok(Fleet {
        fleet_number: FleetId(row.get("fleet_number")?),
        owner: EmpireId(row.get("owner")?),
        next: RecordNumber(row.get("next_record")?),
        record: RecordNumber(row.get("record_number")?),
        prev: RecordNumber(row.get("prev_record")?),
        max_speed: row.get("max_speed")?,
        speed: row.get("speed")?,
        sector: Coords::new(row.get("sector_x")?, row.get("sector_y")?),
        x_fraction: real48_at(row, "x_fraction_real48")?,
        y_fraction: real48_at(row, "y_fraction_real48")?,
        depth: real48_at(row, "depth_real48")?,
        order: Order::from_code(row.get("standing_order")?),
        target: Coords::new(row.get("target_x")?, row.get("target_y")?),
        target_depth: row.get("target_depth")?,
        mission_target: row.get("mission_target")?,
        rules_of_engagement: row.get("rules_of_engagement")?,
        ships: ShipCounts {
            battleships: row.get("battleships")?,
            cruisers: row.get("cruisers")?,
            destroyers: row.get("destroyers")?,
            empty_transports: row.get("empty_transports")?,
            loaded_transports: row.get("loaded_transports")?,
            etacs: row.get("etacs")?,
            scouts: row.get("scouts")?,
        },
        crippled: CrippledShips::default(),
        escape,
    })
}
