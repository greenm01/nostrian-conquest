use rusqlite::{Connection, params, params_from_iter, types::Value};

use super::CampaignStoreError;
use super::hex::{decode_hex, encode_hex};
use crate::{
    BaseDat, BaseRecord, CONQUEST_DAT_SIZE, ConquestDat, CoreGameData, FleetDat, FleetRecord,
    IpbmDat, IpbmRecord, PLANET_RECORD_SIZE, PlanetDat, PlanetRecord, PlayerDat, PlayerRecord,
    Real48, SETUP_DAT_SIZE, SetupDat,
};

pub(super) fn write_snapshot_core_rows(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    game_data: &CoreGameData,
) -> Result<(), CampaignStoreError> {
    write_player_rows(tx, snapshot_id, &game_data.player)?;
    write_planet_rows(tx, snapshot_id, &game_data.planets)?;
    write_fleet_rows(tx, snapshot_id, &game_data.fleets)?;
    write_base_rows(tx, snapshot_id, &game_data.bases)?;
    write_ipbm_rows(tx, snapshot_id, &game_data.ipbm)?;
    write_setup_row(tx, snapshot_id, &game_data.setup)?;
    write_conquest_row(tx, snapshot_id, &game_data.conquest)?;
    Ok(())
}

pub(super) fn load_snapshot_game_data(
    conn: &mut Connection,
    snapshot_id: i64,
) -> Result<CoreGameData, CampaignStoreError> {
    Ok(CoreGameData {
        player: load_player_rows(conn, snapshot_id)?,
        planets: load_planet_rows(conn, snapshot_id)?,
        fleets: load_fleet_rows(conn, snapshot_id)?,
        bases: load_base_rows(conn, snapshot_id)?,
        ipbm: load_ipbm_rows(conn, snapshot_id)?,
        setup: load_setup_row(conn, snapshot_id)?,
        conquest: load_conquest_row(conn, snapshot_id)?,
    })
}

const CONQUEST_CONTROL_WORD_OFFSETS: [usize; 25] = [
    0x0A, 0x0C, 0x0E, 0x10, 0x12, 0x14, 0x16, 0x18, 0x1A, 0x1C, 0x1E, 0x20, 0x22, 0x24, 0x26, 0x28,
    0x2A, 0x2C, 0x2E, 0x30, 0x32, 0x34, 0x36, 0x38, 0x3A,
];

const CONQUEST_CONTROL_BYTE_OFFSETS: [usize; 25] = [
    0x3C, 0x3D, 0x3E, 0x3F, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x4B,
    0x4C, 0x4D, 0x4E, 0x4F, 0x50, 0x51, 0x52, 0x53, 0x54,
];

fn write_player_rows(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    data: &PlayerDat,
) -> Result<(), CampaignStoreError> {
    let mut stmt = tx.prepare(
        "INSERT INTO snapshot_players(
             snapshot_id, record_index, occupied_flag, owner_mode_raw,
             handle_raw_hex, legacy_status_name_max_len_raw, legacy_status_name_len_raw,
             name_block_raw_hex, fleet_chain_head_raw, fleet_chain_tail_raw,
             starbase_count_raw, starbase_presence_flag_raw, ipbm_count_raw,
             homeworld_planet_index_raw, last_run_year_raw, planet_count_raw,
             tax_rate, production_score_raw, review_state_raw_hex,
             diplomacy_raw_hex, autopilot_flag
         ) VALUES (
             ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
             ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21
         )",
    )?;
    for (idx, record) in data.records.iter().enumerate() {
        stmt.execute(params![
            snapshot_id,
            (idx + 1) as i64,
            i64::from(record.occupied_flag()),
            i64::from(record.owner_mode_raw()),
            encode_hex(record.handle_bytes()),
            i64::from(record.legacy_status_name_max_len_raw()),
            i64::from(record.legacy_status_name_len_raw()),
            encode_hex(&record.raw[0x1C..0x30]),
            i64::from(record.fleet_chain_head_raw()),
            i64::from(record.fleet_chain_tail_raw()),
            i64::from(record.first_base_record_raw()),
            i64::from(record.last_base_record_raw()),
            i64::from(record.ipbm_count_raw()),
            i64::from(record.homeworld_planet_index_1_based_raw()),
            i64::from(record.last_run_year_raw()),
            i64::from(record.planet_count_raw()),
            i64::from(record.tax_rate()),
            i64::from(record.production_score_raw()),
            encode_hex(&record.raw[0x30..0x40]),
            encode_hex(&record.raw[0x54..0x6D]),
            i64::from(record.autopilot_flag()),
        ])?;
    }
    Ok(())
}

fn write_planet_rows(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    data: &PlanetDat,
) -> Result<(), CampaignStoreError> {
    let mut stmt = tx.prepare(
        "INSERT INTO snapshot_planets(
             snapshot_id, record_index, coords_x, coords_y, potential_production_raw_hex,
             factories_raw_hex, stored_production_points, economy_marker_raw,
             name_len_raw, name_buffer_raw_hex, name_suffix_raw_hex,
             build_queue_raw_hex, infrastructure_raw_hex, population_raw_hex,
             armies_raw, ground_batteries_raw, ownership_status_raw,
             owner_empire_slot_raw, tail_raw_hex
         ) VALUES (
             ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
             ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19
         )",
    )?;
    for (idx, record) in data.records.iter().enumerate() {
        let [x, y] = record.coords_raw();
        stmt.execute(params![
            snapshot_id,
            (idx + 1) as i64,
            i64::from(x),
            i64::from(y),
            encode_hex(&record.potential_production_raw()),
            encode_hex(&record.factories_raw()),
            i64::from(record.stored_production_points()),
            i64::from(record.economy_marker_raw()),
            i64::from(record.string_len()),
            encode_hex(&record.raw[0x10..0x1D]),
            encode_hex(&record.raw[0x1D..0x24]),
            encode_hex(&record.raw[0x24..0x38]),
            encode_hex(&record.raw[0x38..0x52]),
            encode_hex(&record.raw[0x52..0x58]),
            i64::from(record.army_count_raw()),
            i64::from(record.ground_batteries_raw()),
            i64::from(record.ownership_status_raw()),
            i64::from(record.owner_empire_slot_raw()),
            encode_hex(&record.raw[0x5E..PLANET_RECORD_SIZE]),
        ])?;
    }
    Ok(())
}

fn write_fleet_rows(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    data: &FleetDat,
) -> Result<(), CampaignStoreError> {
    let mut stmt = tx.prepare(
        "INSERT INTO snapshot_fleets(
             snapshot_id, record_index, local_slot_word_raw, owner_empire_raw,
             next_fleet_link_word_raw, fleet_id_word_raw, previous_fleet_id_raw,
             max_speed, current_speed, current_x, current_y,
             tuple_a_raw_hex, tuple_b_raw_hex, tuple_c_raw_hex,
             standing_order_code_raw, target_x, target_y, world_target_flag_raw,
             mission_id_raw, rules_of_engagement, battleship_count, cruiser_count,
             destroyer_count, empty_transport_count, loaded_transport_count, etac_count,
             scout_count, strength_cache_raw
         ) VALUES (
             ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
             ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20,
             ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28
         )",
    )?;
    for (idx, record) in data.records.iter().enumerate() {
        let [current_x, current_y] = record.current_location_coords_raw();
        let [target_x, target_y] = record.standing_order_target_coords_raw();
        stmt.execute(params![
            snapshot_id,
            (idx + 1) as i64,
            i64::from(record.local_slot_word_raw()),
            i64::from(record.owner_empire_raw()),
            i64::from(record.next_fleet_link_word_raw()),
            i64::from(record.fleet_id_word_raw()),
            i64::from(record.previous_fleet_id()),
            i64::from(record.max_speed()),
            i64::from(record.current_speed()),
            i64::from(current_x),
            i64::from(current_y),
            encode_hex(&record.x_fraction().to_bytes()),
            encode_hex(&record.y_fraction().to_bytes()),
            encode_hex(&record.depth().to_bytes()),
            i64::from(record.standing_order_code_raw()),
            i64::from(target_x),
            i64::from(target_y),
            i64::from(record.world_target_flag_raw()),
            i64::from(record.mission_id_raw()),
            i64::from(record.rules_of_engagement()),
            i64::from(record.battleship_count()),
            i64::from(record.cruiser_count()),
            i64::from(record.destroyer_count()),
            i64::from(record.empty_transport_count()),
            i64::from(record.loaded_transport_count()),
            i64::from(record.etac_count()),
            i64::from(record.scout_count()),
            i64::from(record.strength_cache_raw()),
        ])?;
    }
    Ok(())
}

fn write_base_rows(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    data: &BaseDat,
) -> Result<(), CampaignStoreError> {
    let mut stmt = tx.prepare(
        "INSERT INTO snapshot_bases(
             snapshot_id, record_index, escort_fleet_slot_raw, base_id_raw,
             owner_empire_raw, next_base_link_raw, record_number_raw,
             previous_base_link_raw, current_x, current_y, x_fraction_raw_hex,
             y_fraction_raw_hex, depth_raw_hex, moving_flag_raw, target_x, target_y,
             target_depth_raw
         ) VALUES (
             ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17
         )",
    )?;
    for (idx, record) in data.records.iter().enumerate() {
        let [current_x, current_y] = record.coords_raw();
        let [target_x, target_y] = record.target_coords_raw();
        stmt.execute(params![
            snapshot_id,
            (idx + 1) as i64,
            i64::from(record.escort_fleet_slot_raw()),
            i64::from(record.base_id_raw()),
            i64::from(record.owner_empire_raw()),
            i64::from(record.next_base_link_raw()),
            i64::from(record.record_number_raw()),
            i64::from(record.previous_base_link_raw()),
            i64::from(current_x),
            i64::from(current_y),
            encode_hex(&record.x_fraction().to_bytes()),
            encode_hex(&record.y_fraction().to_bytes()),
            encode_hex(&record.depth().to_bytes()),
            i64::from(record.moving_flag_raw()),
            i64::from(target_x),
            i64::from(target_y),
            i64::from(record.target_depth_raw()),
        ])?;
    }
    Ok(())
}

fn write_ipbm_rows(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    data: &IpbmDat,
) -> Result<(), CampaignStoreError> {
    let mut stmt = tx.prepare(
        "INSERT INTO snapshot_ipbms(
             snapshot_id, record_index, prefix_raw_hex, tuple_a_raw_hex,
             tuple_b_raw_hex, tuple_c_raw_hex, trailing_control_raw_hex
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;
    for (idx, record) in data.records.iter().enumerate() {
        stmt.execute(params![
            snapshot_id,
            (idx + 1) as i64,
            encode_hex(&record.raw[0x00..0x0B]),
            encode_hex(&record.tuple_a_payload_raw()),
            encode_hex(&record.tuple_b_payload_raw()),
            encode_hex(&record.tuple_c_payload_raw()),
            encode_hex(&record.trailing_control_raw()),
        ])?;
    }
    Ok(())
}

fn write_setup_row(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    data: &SetupDat,
) -> Result<(), CampaignStoreError> {
    tx.execute(
        "INSERT INTO snapshot_setup(
             snapshot_id, version_tag_raw_hex, option_prefix_raw_hex,
             snoop_enabled, max_time_between_keys_minutes_raw, byte_514_raw,
             remote_timeout_enabled, local_timeout_enabled, minimum_time_granted_minutes_raw,
             purge_after_turns_raw, byte_519_raw, autopilot_inactive_turns_raw, byte_521_raw
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            snapshot_id,
            encode_hex(data.version_tag()),
            encode_hex(data.option_prefix()),
            i64::from(u8::from(data.snoop_enabled())),
            i64::from(data.max_time_between_keys_minutes_raw()),
            i64::from(data.raw[514]),
            i64::from(u8::from(data.remote_timeout_enabled())),
            i64::from(u8::from(data.local_timeout_enabled())),
            i64::from(data.minimum_time_granted_minutes_raw()),
            i64::from(data.purge_after_turns_raw()),
            i64::from(data.raw[519]),
            i64::from(data.autopilot_inactive_turns_raw()),
            i64::from(data.raw[521]),
        ],
    )?;
    Ok(())
}

fn write_conquest_row(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    data: &ConquestDat,
) -> Result<(), CampaignStoreError> {
    let sql = conquest_insert_sql();
    let mut values = Vec::with_capacity(
        4 + CONQUEST_CONTROL_WORD_OFFSETS.len() + CONQUEST_CONTROL_BYTE_OFFSETS.len(),
    );
    values.push(Value::from(snapshot_id));
    values.push(Value::from(i64::from(data.game_year())));
    values.push(Value::from(i64::from(data.player_count())));
    values.push(Value::from(encode_hex(&data.maintenance_schedule_bytes())));
    for offset in CONQUEST_CONTROL_WORD_OFFSETS {
        values.push(Value::from(i64::from(data.raw_word(offset))));
    }
    for offset in CONQUEST_CONTROL_BYTE_OFFSETS {
        values.push(Value::from(i64::from(data.raw_byte(offset))));
    }
    tx.execute(&sql, params_from_iter(values))?;
    Ok(())
}

fn load_player_rows(
    conn: &mut Connection,
    snapshot_id: i64,
) -> Result<PlayerDat, CampaignStoreError> {
    let mut stmt = conn.prepare(
        "SELECT owner_mode_raw, handle_raw_hex, legacy_status_name_max_len_raw,
                legacy_status_name_len_raw, name_block_raw_hex, fleet_chain_head_raw,
                fleet_chain_tail_raw, starbase_count_raw, starbase_presence_flag_raw,
                ipbm_count_raw, homeworld_planet_index_raw, last_run_year_raw,
                planet_count_raw, tax_rate, production_score_raw, review_state_raw_hex,
                diplomacy_raw_hex, autopilot_flag
         FROM snapshot_players
         WHERE snapshot_id = ?1
         ORDER BY record_index",
    )?;
    let rows = stmt.query_map(params![snapshot_id], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, i64>(6)?,
            row.get::<_, i64>(7)?,
            row.get::<_, i64>(8)?,
            row.get::<_, i64>(9)?,
            row.get::<_, i64>(10)?,
            row.get::<_, i64>(11)?,
            row.get::<_, i64>(12)?,
            row.get::<_, i64>(13)?,
            row.get::<_, i64>(14)?,
            row.get::<_, String>(15)?,
            row.get::<_, String>(16)?,
            row.get::<_, i64>(17)?,
        ))
    })?;

    let mut records = Vec::new();
    for row in rows {
        let (
            owner_mode_raw,
            handle_raw_hex,
            legacy_status_name_max_len_raw,
            legacy_status_name_len_raw,
            name_block_raw_hex,
            fleet_chain_head_raw,
            fleet_chain_tail_raw,
            starbase_count_raw,
            starbase_presence_flag_raw,
            ipbm_count_raw,
            homeworld_planet_index_raw,
            last_run_year_raw,
            planet_count_raw,
            tax_rate,
            production_score_raw,
            review_state_raw_hex,
            diplomacy_raw_hex,
            autopilot_flag,
        ) = row?;

        let mut record = PlayerRecord::new_zeroed();
        record.set_owner_empire_raw(owner_mode_raw as u8);
        record.raw[1..0x1A].copy_from_slice(&decode_hex_exact::<25>(&handle_raw_hex)?);
        record.raw[0x1A] = legacy_status_name_max_len_raw as u8;
        record.raw[0x1B] = legacy_status_name_len_raw as u8;
        record.raw[0x1C..0x30].copy_from_slice(&decode_hex_exact::<20>(&name_block_raw_hex)?);
        record.set_fleet_chain_head_raw(fleet_chain_head_raw as u16);
        record.set_fleet_chain_tail_raw(fleet_chain_tail_raw as u16);
        record.set_first_base_record_raw(starbase_count_raw as u16);
        record.set_last_base_record_raw(starbase_presence_flag_raw as u16);
        record.set_ipbm_count_raw(ipbm_count_raw as u16);
        record.set_homeworld_planet_index_1_based_raw(homeworld_planet_index_raw as u8);
        record.set_last_run_year_raw(last_run_year_raw as u16);
        record.set_planet_count_raw(planet_count_raw as u8);
        record.set_tax_rate_raw(tax_rate as u8);
        record.set_production_score_raw(production_score_raw as u16);
        record.raw[0x30..0x40].copy_from_slice(&decode_hex_exact::<16>(&review_state_raw_hex)?);
        record.raw[0x54..0x6D].copy_from_slice(&decode_hex_exact::<25>(&diplomacy_raw_hex)?);
        record.set_autopilot_flag(autopilot_flag as u8);
        records.push(record);
    }
    Ok(PlayerDat { records })
}

fn load_planet_rows(
    conn: &mut Connection,
    snapshot_id: i64,
) -> Result<PlanetDat, CampaignStoreError> {
    let mut stmt = conn.prepare(
        "SELECT coords_x, coords_y, potential_production_raw_hex, factories_raw_hex,
                stored_production_points, economy_marker_raw, name_len_raw,
                name_buffer_raw_hex, name_suffix_raw_hex, build_queue_raw_hex,
                infrastructure_raw_hex, population_raw_hex, armies_raw,
                ground_batteries_raw, ownership_status_raw, owner_empire_slot_raw, tail_raw_hex
         FROM snapshot_planets
         WHERE snapshot_id = ?1
         ORDER BY record_index",
    )?;
    let rows = stmt.query_map(params![snapshot_id], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, i64>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, String>(9)?,
            row.get::<_, String>(10)?,
            row.get::<_, String>(11)?,
            row.get::<_, i64>(12)?,
            row.get::<_, i64>(13)?,
            row.get::<_, i64>(14)?,
            row.get::<_, i64>(15)?,
            row.get::<_, String>(16)?,
        ))
    })?;

    let mut records = Vec::new();
    for row in rows {
        let (
            coords_x,
            coords_y,
            potential_production_raw_hex,
            factories_raw_hex,
            stored_production_points,
            economy_marker_raw,
            name_len_raw,
            name_buffer_raw_hex,
            name_suffix_raw_hex,
            build_queue_raw_hex,
            infrastructure_raw_hex,
            population_raw_hex,
            armies_raw,
            ground_batteries_raw,
            ownership_status_raw,
            owner_empire_slot_raw,
            tail_raw_hex,
        ) = row?;

        let mut record = PlanetRecord::new_zeroed();
        record.set_coords_raw([coords_x as u8, coords_y as u8]);
        record.set_potential_production_raw(decode_hex_exact::<2>(&potential_production_raw_hex)?);
        record.set_factories_raw(decode_hex_exact::<6>(&factories_raw_hex)?);
        record.set_stored_production_points(stored_production_points as u32);
        record.set_economy_marker_raw(economy_marker_raw as u8);
        record.raw[0x0F] = name_len_raw as u8;
        record.raw[0x10..0x1D].copy_from_slice(&decode_hex_exact::<13>(&name_buffer_raw_hex)?);
        record.raw[0x1D..0x24].copy_from_slice(&decode_hex_exact::<7>(&name_suffix_raw_hex)?);
        record.raw[0x24..0x38].copy_from_slice(&decode_hex_exact::<20>(&build_queue_raw_hex)?);
        record.raw[0x38..0x52].copy_from_slice(&decode_hex_exact::<26>(&infrastructure_raw_hex)?);
        record.set_population_raw(decode_hex_exact::<6>(&population_raw_hex)?);
        record.set_army_count_raw(armies_raw as u8);
        record.set_ground_batteries_raw(ground_batteries_raw as u8);
        record.set_ownership_status_raw(ownership_status_raw as u8);
        record.set_owner_empire_slot_raw(owner_empire_slot_raw as u8);
        record.raw[0x5E..PLANET_RECORD_SIZE]
            .copy_from_slice(&decode_hex_exact::<3>(&tail_raw_hex)?);
        records.push(record);
    }
    Ok(PlanetDat { records })
}

fn load_fleet_rows(
    conn: &mut Connection,
    snapshot_id: i64,
) -> Result<FleetDat, CampaignStoreError> {
    let mut stmt = conn.prepare(
        "SELECT local_slot_word_raw, owner_empire_raw, next_fleet_link_word_raw,
                fleet_id_word_raw, previous_fleet_id_raw, max_speed, current_speed,
                current_x, current_y, tuple_a_raw_hex, tuple_b_raw_hex, tuple_c_raw_hex,
                standing_order_code_raw, target_x, target_y, world_target_flag_raw, mission_id_raw, rules_of_engagement,
                battleship_count, cruiser_count, destroyer_count, empty_transport_count,
                loaded_transport_count, etac_count, scout_count, strength_cache_raw
         FROM snapshot_fleets
         WHERE snapshot_id = ?1
         ORDER BY record_index",
    )?;
    let rows = stmt.query_map(params![snapshot_id], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, i64>(6)?,
            row.get::<_, i64>(7)?,
            row.get::<_, i64>(8)?,
            row.get::<_, String>(9)?,
            row.get::<_, String>(10)?,
            row.get::<_, String>(11)?,
            row.get::<_, i64>(12)?,
            row.get::<_, i64>(13)?,
            row.get::<_, i64>(14)?,
            row.get::<_, i64>(15)?,
            row.get::<_, i64>(16)?,
            row.get::<_, i64>(17)?,
            row.get::<_, i64>(18)?,
            row.get::<_, i64>(19)?,
            row.get::<_, i64>(20)?,
            row.get::<_, i64>(21)?,
            row.get::<_, i64>(22)?,
            row.get::<_, i64>(23)?,
            row.get::<_, i64>(24)?,
            row.get::<_, i64>(25)?,
        ))
    })?;

    let mut records = Vec::new();
    for row in rows {
        let (
            local_slot_word_raw,
            owner_empire_raw,
            next_fleet_link_word_raw,
            fleet_id_word_raw,
            previous_fleet_id_raw,
            max_speed,
            current_speed,
            current_x,
            current_y,
            tuple_a_raw_hex,
            tuple_b_raw_hex,
            tuple_c_raw_hex,
            standing_order_code_raw,
            target_x,
            target_y,
            world_target_flag_raw,
            mission_id_raw,
            rules_of_engagement,
            battleship_count,
            cruiser_count,
            destroyer_count,
            empty_transport_count,
            loaded_transport_count,
            etac_count,
            scout_count,
            strength_cache_raw,
        ) = row?;

        let mut record = FleetRecord::new_zeroed();
        record.set_local_slot_word_raw(local_slot_word_raw as u16);
        record.set_owner_empire_raw(owner_empire_raw as u8);
        record.set_next_fleet_link_word_raw(next_fleet_link_word_raw as u16);
        record.set_fleet_id_word_raw(fleet_id_word_raw as u16);
        record.set_previous_fleet_id(previous_fleet_id_raw as u16);
        record.set_max_speed(max_speed as u8);
        record.set_current_speed(current_speed as u8);
        record.set_current_location_coords_raw([current_x as u8, current_y as u8]);
        record.set_x_fraction(Real48::from_bytes(decode_hex_exact::<6>(&tuple_a_raw_hex)?));
        record.set_y_fraction(Real48::from_bytes(decode_hex_exact::<6>(&tuple_b_raw_hex)?));
        record.set_depth(Real48::from_bytes(decode_hex_exact::<6>(&tuple_c_raw_hex)?));
        record.set_standing_order_code_raw(standing_order_code_raw as u8);
        record.set_standing_order_target_coords_raw([target_x as u8, target_y as u8]);
        record.set_mission_target(world_target_flag_raw as u8, mission_id_raw as u16);
        record.set_rules_of_engagement(rules_of_engagement as u8);
        record.set_battleship_count(battleship_count as u16);
        record.set_cruiser_count(cruiser_count as u16);
        record.set_destroyer_count(destroyer_count as u16);
        record.set_empty_transport_count(empty_transport_count as u16);
        record.set_loaded_transport_count(loaded_transport_count as u16);
        record.set_etac_count(etac_count as u16);
        record.set_scout_count(scout_count as u16);
        record.set_strength_cache_raw(strength_cache_raw as u16);
        records.push(record);
    }
    Ok(FleetDat { records })
}

fn load_base_rows(conn: &mut Connection, snapshot_id: i64) -> Result<BaseDat, CampaignStoreError> {
    let mut stmt = conn.prepare(
        "SELECT escort_fleet_slot_raw, base_id_raw, owner_empire_raw, next_base_link_raw,
                record_number_raw, previous_base_link_raw, current_x, current_y,
                x_fraction_raw_hex, y_fraction_raw_hex, depth_raw_hex, moving_flag_raw,
                target_x, target_y, target_depth_raw
         FROM snapshot_bases
         WHERE snapshot_id = ?1
         ORDER BY record_index",
    )?;
    let mut rows = stmt.query(params![snapshot_id])?;

    let mut records = Vec::new();
    while let Some(row) = rows.next()? {
        let mut record = BaseRecord::new_zeroed();
        record.set_escort_fleet_slot_raw(row.get::<_, i64>(0)? as u16);
        record.set_base_id_raw(row.get::<_, i64>(1)? as u16);
        record.set_owner_empire_raw(row.get::<_, i64>(2)? as u8);
        record.set_next_base_link_raw(row.get::<_, i64>(3)? as u16);
        record.set_record_number_raw(row.get::<_, i64>(4)? as u16);
        record.set_previous_base_link_raw(row.get::<_, i64>(5)? as u16);
        record.set_coords_raw([row.get::<_, i64>(6)? as u8, row.get::<_, i64>(7)? as u8]);
        record.set_x_fraction(Real48::from_bytes(decode_hex_exact::<6>(
            &row.get::<_, String>(8)?,
        )?));
        record.set_y_fraction(Real48::from_bytes(decode_hex_exact::<6>(
            &row.get::<_, String>(9)?,
        )?));
        record.set_depth(Real48::from_bytes(decode_hex_exact::<6>(
            &row.get::<_, String>(10)?,
        )?));
        record.set_moving_flag_raw(row.get::<_, i64>(11)? as u8);
        record.set_target_coords_raw([row.get::<_, i64>(12)? as u8, row.get::<_, i64>(13)? as u8]);
        record.set_target_depth_raw(row.get::<_, i64>(14)? as u8);
        records.push(record);
    }
    Ok(BaseDat { records })
}

fn load_ipbm_rows(conn: &mut Connection, snapshot_id: i64) -> Result<IpbmDat, CampaignStoreError> {
    let mut stmt = conn.prepare(
        "SELECT prefix_raw_hex, tuple_a_raw_hex, tuple_b_raw_hex, tuple_c_raw_hex,
                trailing_control_raw_hex
         FROM snapshot_ipbms
         WHERE snapshot_id = ?1
         ORDER BY record_index",
    )?;
    let rows = stmt.query_map(params![snapshot_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
        ))
    })?;

    let mut records = Vec::new();
    for row in rows {
        let (
            prefix_raw_hex,
            tuple_a_raw_hex,
            tuple_b_raw_hex,
            tuple_c_raw_hex,
            trailing_control_raw_hex,
        ) = row?;

        let mut record = IpbmRecord::new_zeroed();
        record.raw[0x00..0x0B].copy_from_slice(&decode_hex_exact::<11>(&prefix_raw_hex)?);
        record.set_tuple_a_payload_raw(decode_hex_exact::<5>(&tuple_a_raw_hex)?);
        record.set_tuple_b_payload_raw(decode_hex_exact::<5>(&tuple_b_raw_hex)?);
        record.set_tuple_c_payload_raw(decode_hex_exact::<5>(&tuple_c_raw_hex)?);
        record.set_trailing_control_raw(decode_hex_exact::<3>(&trailing_control_raw_hex)?);
        records.push(record);
    }
    Ok(IpbmDat { records })
}

fn load_setup_row(conn: &mut Connection, snapshot_id: i64) -> Result<SetupDat, CampaignStoreError> {
    let (
        version_tag_raw_hex,
        option_prefix_raw_hex,
        snoop_enabled,
        max_time_between_keys_minutes_raw,
        byte_514_raw,
        remote_timeout_enabled,
        local_timeout_enabled,
        minimum_time_granted_minutes_raw,
        purge_after_turns_raw,
        byte_519_raw,
        autopilot_inactive_turns_raw,
        byte_521_raw,
    ) = conn.query_row(
        "SELECT version_tag_raw_hex, option_prefix_raw_hex,
                snoop_enabled, max_time_between_keys_minutes_raw, byte_514_raw,
                remote_timeout_enabled, local_timeout_enabled, minimum_time_granted_minutes_raw,
                purge_after_turns_raw, byte_519_raw, autopilot_inactive_turns_raw, byte_521_raw
         FROM snapshot_setup
         WHERE snapshot_id = ?1",
        params![snapshot_id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, i64>(8)?,
                row.get::<_, i64>(9)?,
                row.get::<_, i64>(10)?,
                row.get::<_, i64>(11)?,
            ))
        },
    )?;

    let mut setup = SetupDat {
        raw: [0; SETUP_DAT_SIZE],
    };
    setup.raw[..5].copy_from_slice(&decode_hex_exact::<5>(&version_tag_raw_hex)?);
    setup.raw[5..13].copy_from_slice(&decode_hex_exact::<8>(&option_prefix_raw_hex)?);
    setup.set_snoop_enabled(snoop_enabled != 0);
    setup.set_max_time_between_keys_minutes_raw(max_time_between_keys_minutes_raw as u8);
    setup.raw[514] = byte_514_raw as u8;
    setup.set_remote_timeout_enabled(remote_timeout_enabled != 0);
    setup.set_local_timeout_enabled(local_timeout_enabled != 0);
    setup.set_minimum_time_granted_minutes_raw(minimum_time_granted_minutes_raw as u8);
    setup.set_purge_after_turns_raw(purge_after_turns_raw as u8);
    setup.raw[519] = byte_519_raw as u8;
    setup.set_autopilot_inactive_turns_raw(autopilot_inactive_turns_raw as u8);
    setup.raw[521] = byte_521_raw as u8;
    Ok(setup)
}

fn load_conquest_row(
    conn: &mut Connection,
    snapshot_id: i64,
) -> Result<ConquestDat, CampaignStoreError> {
    let sql = conquest_select_sql();
    let (game_year, player_count, maintenance_schedule_raw_hex, control_words, control_bytes) =
        conn.query_row(&sql, params![snapshot_id], |row| {
            let mut control_words = [0u16; CONQUEST_CONTROL_WORD_OFFSETS.len()];
            let mut control_bytes = [0u8; CONQUEST_CONTROL_BYTE_OFFSETS.len()];
            let mut column = 3;
            for word in &mut control_words {
                *word = row.get::<_, i64>(column)? as u16;
                column += 1;
            }
            for byte in &mut control_bytes {
                *byte = row.get::<_, i64>(column)? as u8;
                column += 1;
            }
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                control_words,
                control_bytes,
            ))
        })?;

    let mut conquest = ConquestDat {
        raw: [0; CONQUEST_DAT_SIZE],
    };
    conquest.set_game_year(game_year as u16);
    conquest.set_player_count(player_count as u8);
    conquest.raw[3..10].copy_from_slice(&decode_hex_exact::<7>(&maintenance_schedule_raw_hex)?);
    for (index, offset) in CONQUEST_CONTROL_WORD_OFFSETS.into_iter().enumerate() {
        conquest.set_raw_word(offset, control_words[index]);
    }
    for (index, offset) in CONQUEST_CONTROL_BYTE_OFFSETS.into_iter().enumerate() {
        conquest.set_raw_byte(offset, control_bytes[index]);
    }
    Ok(conquest)
}

fn conquest_insert_sql() -> String {
    let mut columns = vec![
        "snapshot_id".to_string(),
        "game_year".to_string(),
        "player_count".to_string(),
        "maintenance_schedule_raw_hex".to_string(),
    ];
    columns.extend(
        CONQUEST_CONTROL_WORD_OFFSETS
            .into_iter()
            .map(|offset| format!("control_word_{offset:02x}_raw")),
    );
    columns.extend(
        CONQUEST_CONTROL_BYTE_OFFSETS
            .into_iter()
            .map(|offset| format!("control_byte_{offset:02x}_raw")),
    );
    let placeholders = (1..=columns.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "INSERT INTO snapshot_conquest({}) VALUES ({})",
        columns.join(", "),
        placeholders
    )
}

fn conquest_select_sql() -> String {
    let mut columns = vec![
        "game_year".to_string(),
        "player_count".to_string(),
        "maintenance_schedule_raw_hex".to_string(),
    ];
    columns.extend(
        CONQUEST_CONTROL_WORD_OFFSETS
            .into_iter()
            .map(|offset| format!("control_word_{offset:02x}_raw")),
    );
    columns.extend(
        CONQUEST_CONTROL_BYTE_OFFSETS
            .into_iter()
            .map(|offset| format!("control_byte_{offset:02x}_raw")),
    );
    format!(
        "SELECT {} FROM snapshot_conquest WHERE snapshot_id = ?1",
        columns.join(", ")
    )
}

fn decode_hex_exact<const N: usize>(value: &str) -> Result<[u8; N], CampaignStoreError> {
    let decoded = decode_hex(value).map_err(invalid_hex_sql_error)?;
    decoded.try_into().map_err(|actual: Vec<u8>| {
        invalid_hex_sql_error(format!("expected {N} bytes, got {}", actual.len()))
    })
}

fn invalid_hex_sql_error(err: String) -> CampaignStoreError {
    CampaignStoreError::Sql(rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, err)),
    ))
}
