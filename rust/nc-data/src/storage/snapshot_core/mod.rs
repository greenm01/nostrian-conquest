//! The core game state's snapshot tables: one typed column per model field,
//! and a child table for each array the model holds. Rows load straight into
//! the model structs; no classic record bytes pass through the store.
//! The crippled hulls of fleets and starbases have their own child tables,
//! as does each empire's local sighting memory.

use rusqlite::{Connection, Row, ToSql, Transaction};

use super::CampaignStoreError;
use crate::support::copy_array;
use crate::{CoreGameData, Real48};

mod bases;
mod conquest;
mod crippled;
mod fleets;
mod ipbms;
mod planets;
mod players;
mod setup;
mod sightings;

pub(super) fn create_tables(conn: &Connection) -> Result<(), CampaignStoreError> {
    for schema in [
        players::SCHEMA,
        planets::SCHEMA,
        fleets::SCHEMA,
        bases::SCHEMA,
        crippled::SCHEMA,
        ipbms::SCHEMA,
        setup::SCHEMA,
        conquest::SCHEMA,
        sightings::SCHEMA,
    ] {
        conn.execute_batch(schema)?;
    }
    Ok(())
}

pub(super) fn write_snapshot_core_rows(
    tx: &Transaction<'_>,
    snapshot_id: i64,
    game_data: &CoreGameData,
) -> Result<(), CampaignStoreError> {
    players::write(tx, snapshot_id, game_data.players.iter().enumerate())?;
    planets::write(tx, snapshot_id, game_data.planets.iter().enumerate())?;
    fleets::write(tx, snapshot_id, game_data.fleets.iter().enumerate())?;
    bases::write(tx, snapshot_id, game_data.bases.iter().enumerate())?;
    crippled::write_fleets(tx, snapshot_id, game_data.fleets.iter().enumerate())?;
    crippled::write_bases(tx, snapshot_id, game_data.bases.iter().enumerate())?;
    ipbms::write(tx, snapshot_id, game_data.ipbms.iter().enumerate())?;
    setup::write(tx, snapshot_id, &game_data.setup)?;
    conquest::write(tx, snapshot_id, &game_data.conquest)?;
    sightings::write(tx, snapshot_id, &game_data.sightings)?;
    Ok(())
}

/// Rewrite, in snapshot `snapshot_id`, only the records that differ between
/// `before` (what the snapshot holds) and `after` (t204): a record's rows in
/// its table and its child tables are replaced, appended records added and
/// dropped ones deleted. The setup, conquest and sighting tables are each
/// rewritten whole when they changed.
pub(super) fn write_changed_core_rows(
    tx: &Transaction<'_>,
    snapshot_id: i64,
    before: &CoreGameData,
    after: &CoreGameData,
) -> Result<(), CampaignStoreError> {
    // A player's 25 diplomacy rows change only with its diplomacy.
    let without_diplomacy = |players: &[crate::Player]| -> Vec<crate::Player> {
        players
            .iter()
            .map(|player| crate::Player {
                diplomacy: Default::default(),
                ..player.clone()
            })
            .collect()
    };
    let changed = replace_records(
        tx,
        snapshot_id,
        &["snapshot_players"],
        &without_diplomacy(&before.players),
        &without_diplomacy(&after.players),
    )?;
    players::write_rows(
        tx,
        snapshot_id,
        changed.iter().map(|&i| (i, &after.players[i])),
    )?;
    let diplomacy = |players: &[crate::Player]| -> Vec<_> {
        players.iter().map(|player| player.diplomacy).collect()
    };
    let changed = replace_records(
        tx,
        snapshot_id,
        &["snapshot_player_diplomacy"],
        &diplomacy(&before.players),
        &diplomacy(&after.players),
    )?;
    players::write_diplomacy(
        tx,
        snapshot_id,
        changed.iter().map(|&i| (i, &after.players[i])),
    )?;
    let changed = replace_records(
        tx,
        snapshot_id,
        &["snapshot_planets", "snapshot_planet_build_slots"],
        &before.planets,
        &after.planets,
    )?;
    planets::write(
        tx,
        snapshot_id,
        changed.iter().map(|&i| (i, &after.planets[i])),
    )?;
    let changed = replace_records(
        tx,
        snapshot_id,
        &["snapshot_fleets", "snapshot_fleet_crippled"],
        &before.fleets,
        &after.fleets,
    )?;
    fleets::write(
        tx,
        snapshot_id,
        changed.iter().map(|&i| (i, &after.fleets[i])),
    )?;
    crippled::write_fleets(
        tx,
        snapshot_id,
        changed.iter().map(|&i| (i, &after.fleets[i])),
    )?;
    let changed = replace_records(
        tx,
        snapshot_id,
        &["snapshot_bases", "snapshot_base_crippled"],
        &before.bases,
        &after.bases,
    )?;
    bases::write(
        tx,
        snapshot_id,
        changed.iter().map(|&i| (i, &after.bases[i])),
    )?;
    crippled::write_bases(
        tx,
        snapshot_id,
        changed.iter().map(|&i| (i, &after.bases[i])),
    )?;
    let changed = replace_records(
        tx,
        snapshot_id,
        &["snapshot_ipbms"],
        &before.ipbms,
        &after.ipbms,
    )?;
    ipbms::write(
        tx,
        snapshot_id,
        changed.iter().map(|&i| (i, &after.ipbms[i])),
    )?;
    if before.setup != after.setup {
        clear_tables(
            tx,
            snapshot_id,
            &["snapshot_setup", "snapshot_setup_com_ports"],
        )?;
        setup::write(tx, snapshot_id, &after.setup)?;
    }
    if before.conquest != after.conquest {
        clear_tables(
            tx,
            snapshot_id,
            &[
                "snapshot_conquest",
                "snapshot_conquest_maintenance_days",
                "snapshot_conquest_previous_rankings",
            ],
        )?;
        conquest::write(tx, snapshot_id, &after.conquest)?;
    }
    if before.sightings != after.sightings {
        clear_tables(tx, snapshot_id, &sightings::TABLES)?;
        sightings::write(tx, snapshot_id, &after.sightings)?;
    }
    Ok(())
}

/// Delete the rows, in `tables`, of every record that differs between
/// `before` and `after` or that `after` no longer holds, and return the
/// 0-based indexes in `after` to write again.
fn replace_records<T: PartialEq>(
    tx: &Transaction<'_>,
    snapshot_id: i64,
    tables: &[&str],
    before: &[T],
    after: &[T],
) -> Result<Vec<usize>, CampaignStoreError> {
    let changed: Vec<usize> = (0..after.len())
        .filter(|&i| before.get(i) != Some(&after[i]))
        .collect();
    let stale = changed
        .iter()
        .copied()
        .filter(|&i| i < before.len())
        .chain(after.len()..before.len());
    for index in stale {
        for table in tables {
            tx.execute(
                &format!("DELETE FROM {table} WHERE snapshot_id = ?1 AND record_index = ?2"),
                rusqlite::params![snapshot_id, index + 1],
            )?;
        }
    }
    Ok(changed)
}

fn clear_tables(
    tx: &Transaction<'_>,
    snapshot_id: i64,
    tables: &[&str],
) -> Result<(), CampaignStoreError> {
    for table in tables {
        tx.execute(
            &format!("DELETE FROM {table} WHERE snapshot_id = ?1"),
            [snapshot_id],
        )?;
    }
    Ok(())
}

pub(super) fn load_snapshot_game_data(
    conn: &Connection,
    snapshot_id: i64,
) -> Result<CoreGameData, CampaignStoreError> {
    let mut fleets = fleets::load(conn, snapshot_id)?;
    crippled::load_fleets(conn, snapshot_id, &mut fleets)?;
    let mut bases = bases::load(conn, snapshot_id)?;
    crippled::load_bases(conn, snapshot_id, &mut bases)?;
    Ok(CoreGameData {
        players: players::load(conn, snapshot_id)?,
        planets: planets::load(conn, snapshot_id)?,
        fleets,
        bases,
        ipbms: ipbms::load(conn, snapshot_id)?,
        setup: setup::load(conn, snapshot_id)?,
        conquest: conquest::load(conn, snapshot_id)?,
        sightings: sightings::load(conn, snapshot_id)?,
    })
}

/// Inserts one row into `table`. Each value's name is its column with a
/// leading `:`, so the column list cannot drift from the values.
fn insert_row(
    tx: &Transaction<'_>,
    table: &str,
    values: &[(&str, &dyn ToSql)],
) -> Result<(), CampaignStoreError> {
    let columns = values
        .iter()
        .map(|(name, _)| name.trim_start_matches(':'))
        .collect::<Vec<_>>()
        .join(", ");
    let placeholders = values
        .iter()
        .map(|(name, _)| *name)
        .collect::<Vec<_>>()
        .join(", ");
    tx.prepare_cached(&format!(
        "INSERT INTO {table}({columns}) VALUES ({placeholders})"
    ))?
    .execute(values)?;
    Ok(())
}

/// `index` when a list of `len` entries has it.
fn index_below(index: usize, len: usize, what: &str) -> Result<usize, CampaignStoreError> {
    if index < len {
        Ok(index)
    } else {
        Err(CampaignStoreError::InvalidState(format!(
            "{what} {index} is outside 0..{len}"
        )))
    }
}

/// The list position of 1-based `number` (a record or empire number) among
/// `len` entries.
fn position_of(number: usize, len: usize, what: &str) -> Result<usize, CampaignStoreError> {
    match number.checked_sub(1) {
        Some(index) if index < len => Ok(index),
        _ => Err(CampaignStoreError::InvalidState(format!(
            "{what} {number} is outside 1..={len}"
        ))),
    }
}

/// A `Real48` column is an INTEGER holding the real's six bytes
/// little-endian, exponent byte lowest, so the value comes back bit for bit.
/// Fleet and starbase motion turns on exact bits (a climb ends one ulp short
/// of 1.0 and arrives a year later). A REAL column would not do: an `f64`
/// holds every Real48 value but not every bit pattern, such as a zero
/// exponent with mantissa bits set. An INTEGER keeps the schema free of
/// opaque byte columns, and 48 bits fit with room to spare.
fn real48_column(value: Real48) -> i64 {
    let mut bytes = [0u8; 8];
    bytes[..6].copy_from_slice(&value.to_bytes());
    i64::from_le_bytes(bytes)
}

fn real48_at(row: &Row<'_>, column: &str) -> rusqlite::Result<Real48> {
    let bits: i64 = row.get(column)?;
    if !(0..1 << 48).contains(&bits) {
        let index = row.as_ref().column_index(column)?;
        return Err(rusqlite::Error::IntegralValueOutOfRange(index, bits));
    }
    Ok(Real48::from_bytes(copy_array(&bits.to_le_bytes()[..6])))
}
