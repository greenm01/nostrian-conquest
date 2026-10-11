use rusqlite::{Connection, Row, Transaction, named_params, params};

use super::{CampaignStoreError, index_below, insert_row, position_of, real48_at, real48_column};
use crate::model::{BuildSlot, Coords, EmpireId, Planet, ProductionItemKind, Stardock};

/// One row per PLANETS.DAT record. `snapshot_planet_build_slots` holds the
/// occupied slots of each planet's build queue (`slot` 0 to 9); a slot with
/// no row is empty.
pub(super) const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS snapshot_planets (
        snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
        record_index INTEGER NOT NULL,
        sector_x INTEGER NOT NULL,
        sector_y INTEGER NOT NULL,
        potential_production INTEGER NOT NULL,
        production_real48 INTEGER NOT NULL,
        stored_goods_real48 INTEGER NOT NULL,
        name TEXT NOT NULL,
        stardock_destroyers INTEGER NOT NULL,
        stardock_cruisers INTEGER NOT NULL,
        stardock_battleships INTEGER NOT NULL,
        stardock_scouts INTEGER NOT NULL,
        stardock_transports INTEGER NOT NULL,
        stardock_etacs INTEGER NOT NULL,
        stardock_starbases INTEGER NOT NULL,
        stardock_ipbms INTEGER NOT NULL,
        armies INTEGER NOT NULL,
        ground_batteries INTEGER NOT NULL,
        years_under_owner INTEGER NOT NULL,
        owner INTEGER NOT NULL,
        scorch_earth INTEGER NOT NULL,
        PRIMARY KEY(snapshot_id, record_index)
    );
    CREATE TABLE IF NOT EXISTS snapshot_planet_build_slots (
        snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
        record_index INTEGER NOT NULL,
        slot INTEGER NOT NULL,
        points_owed INTEGER NOT NULL,
        kind INTEGER NOT NULL,
        PRIMARY KEY(snapshot_id, record_index, slot)
    );";

pub(super) fn write(
    tx: &Transaction<'_>,
    snapshot_id: i64,
    planets: &[Planet],
) -> Result<(), CampaignStoreError> {
    for (index, planet) in planets.iter().enumerate() {
        let record_index = index + 1;
        let dock = &planet.stardock;
        insert_row(
            tx,
            "snapshot_planets",
            named_params! {
                ":snapshot_id": snapshot_id,
                ":record_index": record_index,
                ":sector_x": planet.sector.x,
                ":sector_y": planet.sector.y,
                ":potential_production": planet.potential_production,
                ":production_real48": real48_column(planet.production),
                ":stored_goods_real48": real48_column(planet.stored_goods),
                ":name": planet.name,
                ":stardock_destroyers": dock.destroyers,
                ":stardock_cruisers": dock.cruisers,
                ":stardock_battleships": dock.battleships,
                ":stardock_scouts": dock.scouts,
                ":stardock_transports": dock.transports,
                ":stardock_etacs": dock.etacs,
                ":stardock_starbases": dock.starbases,
                ":stardock_ipbms": dock.ipbms,
                ":armies": planet.armies,
                ":ground_batteries": planet.ground_batteries,
                ":years_under_owner": planet.years_under_owner,
                ":owner": planet.owner.0,
                ":scorch_earth": planet.scorch_earth,
            },
        )?;
        for (slot, entry) in planet.build_queue.iter().enumerate() {
            if *entry == BuildSlot::EMPTY {
                continue;
            }
            insert_row(
                tx,
                "snapshot_planet_build_slots",
                named_params! {
                    ":snapshot_id": snapshot_id,
                    ":record_index": record_index,
                    ":slot": slot,
                    ":points_owed": entry.points_owed,
                    ":kind": entry.kind.to_raw(),
                },
            )?;
        }
    }
    Ok(())
}

pub(super) fn load(
    conn: &mut Connection,
    snapshot_id: i64,
) -> Result<Vec<Planet>, CampaignStoreError> {
    let mut stmt = conn
        .prepare("SELECT * FROM snapshot_planets WHERE snapshot_id = ?1 ORDER BY record_index")?;
    let mut records = stmt
        .query_map(params![snapshot_id], planet_from_row)?
        .collect::<Result<Vec<_>, _>>()?;

    let mut stmt = conn.prepare(
        "SELECT record_index, slot, points_owed, kind
         FROM snapshot_planet_build_slots
         WHERE snapshot_id = ?1",
    )?;
    let rows = stmt.query_map(params![snapshot_id], |row| {
        Ok((
            row.get::<_, usize>("record_index")?,
            row.get::<_, usize>("slot")?,
            BuildSlot {
                points_owed: row.get("points_owed")?,
                kind: ProductionItemKind::from_raw(row.get("kind")?),
            },
        ))
    })?;
    for row in rows {
        let (record_index, slot, entry) = row?;
        let record = position_of(record_index, records.len(), "planet record")?;
        let queue = &mut records[record].build_queue;
        queue[index_below(slot, queue.len(), "build slot")?] = entry;
    }
    Ok(records)
}

fn planet_from_row(row: &Row<'_>) -> rusqlite::Result<Planet> {
    Ok(Planet {
        sector: Coords::new(row.get("sector_x")?, row.get("sector_y")?),
        potential_production: row.get("potential_production")?,
        production: real48_at(row, "production_real48")?,
        stored_goods: real48_at(row, "stored_goods_real48")?,
        name: row.get("name")?,
        build_queue: [BuildSlot::EMPTY; 10],
        stardock: Stardock {
            destroyers: row.get("stardock_destroyers")?,
            cruisers: row.get("stardock_cruisers")?,
            battleships: row.get("stardock_battleships")?,
            scouts: row.get("stardock_scouts")?,
            transports: row.get("stardock_transports")?,
            etacs: row.get("stardock_etacs")?,
            starbases: row.get("stardock_starbases")?,
            ipbms: row.get("stardock_ipbms")?,
        },
        armies: row.get("armies")?,
        ground_batteries: row.get("ground_batteries")?,
        years_under_owner: row.get("years_under_owner")?,
        owner: EmpireId(row.get("owner")?),
        scorch_earth: row.get("scorch_earth")?,
    })
}
