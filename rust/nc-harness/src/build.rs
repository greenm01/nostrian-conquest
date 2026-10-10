use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use nc_compat::{encode_report_block_rows, export_latest_snapshot_to_dir};
use nc_data::{
    CampaignStore, CoreGameData, QueuedPlayerMail, ReportBlockRow, layout_classic_results,
};
use nc_engine::{build_seeded_initialized_game, build_seeded_new_game};

use crate::error::HarnessError;
use crate::spec::{ReviewBlockSpec, ScenarioBaseline, ScenarioSpec};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioBuildReport {
    pub label: Option<String>,
    pub player_count: u8,
    pub year: u16,
    pub planet_records: usize,
    pub fleet_records: usize,
    pub queue_mail_count: usize,
    pub results_blocks: usize,
    pub message_blocks: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltScenario {
    pub game_data: CoreGameData,
    pub report_block_rows: Vec<ReportBlockRow>,
    pub queued_mail: Vec<QueuedPlayerMail>,
    pub report: ScenarioBuildReport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedScenarioReport {
    pub report: ScenarioBuildReport,
    pub export_classic: bool,
}

pub fn build_scenario(spec: &ScenarioSpec) -> Result<BuiltScenario, HarnessError> {
    let mut game_data = match spec.metadata.baseline {
        ScenarioBaseline::BuilderCompatible => build_seeded_initialized_game(
            spec.metadata.player_count,
            spec.metadata.year,
            spec.metadata.seed,
        )?,
        ScenarioBaseline::JoinableNewGame => build_seeded_new_game(
            spec.metadata.player_count,
            spec.metadata.year,
            spec.metadata.seed,
        )?,
    };

    apply_house_specs(spec, &mut game_data)?;
    apply_diplomacy(spec, &mut game_data)?;
    apply_planet_specs(spec, &mut game_data)?;
    apply_commissions(spec, &mut game_data)?;
    apply_fleet_specs(spec, &mut game_data)?;

    let mut queued_mail = Vec::new();
    apply_turn_files(spec, 1, &mut game_data, &mut queued_mail)?;
    apply_queued_mail(spec, &mut queued_mail);
    apply_message_blocks(spec, &mut queued_mail, game_data.conquest.player_count());

    let report_block_rows = build_report_block_rows(&spec.results_blocks);
    sync_review_flags(&mut game_data, &report_block_rows, &spec.message_blocks);

    Ok(BuiltScenario {
        report: ScenarioBuildReport {
            label: spec.metadata.label.clone(),
            player_count: spec.metadata.player_count,
            year: spec.metadata.year,
            planet_records: game_data.planets.records.len(),
            fleet_records: game_data.fleets.records.len(),
            queue_mail_count: queued_mail.len(),
            results_blocks: spec.results_blocks.len(),
            message_blocks: spec.message_blocks.len(),
        },
        game_data,
        report_block_rows,
        queued_mail,
    })
}

pub fn save_built_scenario(
    built: &BuiltScenario,
    target_dir: &Path,
    export_classic: bool,
) -> Result<SavedScenarioReport, HarnessError> {
    fs::create_dir_all(target_dir)?;
    let store = CampaignStore::open_default_in_dir(target_dir)?;
    store.save_runtime_state_structured(
        &built.game_data,
        &BTreeSet::new(),
        &built.report_block_rows,
        &built.queued_mail,
    )?;
    if export_classic {
        export_latest_snapshot_to_dir(&store, target_dir)?;
    }
    Ok(SavedScenarioReport {
        report: built.report.clone(),
        export_classic,
    })
}

fn apply_house_specs(
    spec: &ScenarioSpec,
    game_data: &mut CoreGameData,
) -> Result<(), HarnessError> {
    for house in &spec.houses {
        if house.record_index_1_based == 0
            || house.record_index_1_based > game_data.player.records.len()
        {
            return Err(HarnessError::Validation(format!(
                "house record out of range: {}",
                house.record_index_1_based
            )));
        }

        let should_join = matches!(spec.metadata.baseline, ScenarioBaseline::BuilderCompatible)
            || house.empire_name.is_some()
            || house.handle.is_some()
            || house.homeworld_name.is_some()
            || house.tax_rate.is_some();

        if should_join && matches!(spec.metadata.baseline, ScenarioBaseline::JoinableNewGame) {
            let empire_name = house
                .empire_name
                .clone()
                .unwrap_or_else(|| format!("Empire {}", house.record_index_1_based));
            game_data.join_player(house.record_index_1_based, &empire_name)?;
        }

        let player = game_data
            .player
            .records
            .get_mut(house.record_index_1_based - 1)
            .ok_or_else(|| {
                HarnessError::Validation(format!(
                    "missing player record {}",
                    house.record_index_1_based
                ))
            })?;
        player.set_autopilot_flag(u8::from(house.autopilot));
        if let Some(handle) = &house.handle {
            player.set_assigned_player_handle_raw(handle);
        }
        if let Some(empire_name) = &house.empire_name {
            player.set_controlled_empire_name_raw(empire_name);
        }
        if let Some(tax_rate) = house.tax_rate {
            game_data.set_player_tax_rate(house.record_index_1_based, tax_rate)?;
        }
        if let Some(homeworld_name) = &house.homeworld_name {
            game_data.rename_player_homeworld(house.record_index_1_based, homeworld_name)?;
        }
    }
    Ok(())
}

fn apply_diplomacy(spec: &ScenarioSpec, game_data: &mut CoreGameData) -> Result<(), HarnessError> {
    for relation in &spec.diplomacy {
        game_data.set_stored_diplomatic_relation(
            relation.from_empire_raw,
            relation.to_empire_raw,
            relation.relation,
        )?;
    }
    Ok(())
}

fn apply_planet_specs(
    spec: &ScenarioSpec,
    game_data: &mut CoreGameData,
) -> Result<(), HarnessError> {
    for planet_spec in &spec.planets {
        let planet = game_data
            .planets
            .records
            .get_mut(planet_spec.record_index_1_based - 1)
            .ok_or_else(|| {
                HarnessError::Validation(format!(
                    "planet record out of range: {}",
                    planet_spec.record_index_1_based
                ))
            })?;
        if let Some(coords) = planet_spec.coords {
            planet.set_coords_raw(coords);
        }
        if let Some(owner_empire_raw) = planet_spec.owner_empire_raw {
            planet.set_owner_empire_slot_raw(owner_empire_raw);
            planet.set_ownership_status_raw(if owner_empire_raw == 0 { 0 } else { 2 });
        }
        if let Some(name) = &planet_spec.name {
            planet.set_planet_name(name);
        }
        if let Some(potential) = planet_spec.potential_production {
            planet.set_potential_production_points(potential.min(u16::from(u8::MAX)) as u8);
        }
        if let Some(present) = planet_spec.present_production {
            if !planet.set_present_production_points(present) {
                return Err(HarnessError::Validation(format!(
                    "planet {} present production could not be encoded",
                    planet_spec.record_index_1_based
                )));
            }
        }
        if let Some(stored) = planet_spec.stored_production {
            planet.set_stored_production_points(stored);
        }
        if let Some(armies) = planet_spec.armies {
            planet.set_army_count_raw(armies);
        }
        if let Some(ground_batteries) = planet_spec.ground_batteries {
            planet.set_ground_batteries_raw(ground_batteries);
        }
        if !planet_spec.stardock.is_empty() {
            for slot in 0..nc_data::STARDOCK_SLOT_COUNT {
                planet.set_stardock_count_raw(slot, 0);
            }
            for slot in &planet_spec.stardock {
                // Each item type has its own stardock slot.
                if nc_data::stardock_slot_for_kind(slot.kind_raw) != Some(slot.slot_0_based) {
                    return Err(HarnessError::Validation(format!(
                        "planet {} stardock slot {} does not hold kind {}",
                        planet_spec.record_index_1_based,
                        slot.slot_0_based + 1,
                        slot.kind_raw
                    )));
                }
                planet.set_stardock_count_raw(slot.slot_0_based, slot.count);
            }
        }
    }
    Ok(())
}

fn apply_commissions(
    spec: &ScenarioSpec,
    game_data: &mut CoreGameData,
) -> Result<(), HarnessError> {
    for planet_spec in &spec.planets {
        if planet_spec.commissions.is_empty() {
            continue;
        }
        let owner_empire = game_data
            .planets
            .records
            .get(planet_spec.record_index_1_based - 1)
            .ok_or_else(|| {
                HarnessError::Validation(format!(
                    "planet record out of range: {}",
                    planet_spec.record_index_1_based
                ))
            })?
            .owner_empire_slot_raw();
        if owner_empire == 0 {
            return Err(HarnessError::Validation(format!(
                "planet {} must be owned before commissioning stardock units",
                planet_spec.record_index_1_based
            )));
        }
        for commission in &planet_spec.commissions {
            game_data.commission_planet_stardock_slot(
                owner_empire as usize,
                planet_spec.record_index_1_based,
                commission.slot_0_based,
            )?;
        }
    }
    Ok(())
}

fn apply_fleet_specs(
    spec: &ScenarioSpec,
    game_data: &mut CoreGameData,
) -> Result<(), HarnessError> {
    for fleet_spec in &spec.fleets {
        let fleet = game_data
            .fleets
            .records
            .get_mut(fleet_spec.record_index_1_based - 1)
            .ok_or_else(|| {
                HarnessError::Validation(format!(
                    "fleet record out of range: {}",
                    fleet_spec.record_index_1_based
                ))
            })?;
        if let Some(owner_empire_raw) = fleet_spec.owner_empire_raw {
            if owner_empire_raw != fleet.owner_empire_raw() {
                return Err(HarnessError::Validation(format!(
                    "fleet {} owner reassignment is not supported; existing owner is {}",
                    fleet_spec.record_index_1_based,
                    fleet.owner_empire_raw()
                )));
            }
        }
        if let Some(coords) = fleet_spec.coords {
            fleet.set_current_location_coords_raw(coords);
            if fleet_spec.order.is_none() {
                fleet.set_standing_order_target_coords_raw(coords);
            }
        }
        if let Some(ships) = &fleet_spec.ships {
            fleet.set_battleship_count(ships.battleships);
            fleet.set_cruiser_count(ships.cruisers);
            fleet.set_destroyer_count(ships.destroyers);
            fleet.set_scout_count(ships.scouts);
            fleet.set_troop_transport_count(ships.transports);
            fleet.set_army_count(ships.loaded_armies);
            fleet.set_etac_count(ships.etacs);
            fleet.recompute_max_speed_from_composition();
        }
        if let Some(invasion_armies) = fleet_spec.invasion_armies {
            // An early tool wrote this byte (0x08) as invasion armies; classic
            // reads it as the high byte of the previous-fleet word.
            fleet.set_previous_fleet_id(u16::from_le_bytes([fleet.raw[0x07], invasion_armies]));
        }
        if let Some(rules_of_engagement) = fleet_spec.rules_of_engagement {
            fleet.set_rules_of_engagement(rules_of_engagement);
        }
        if let Some(current_speed) = fleet_spec.current_speed {
            if current_speed > fleet.max_speed() {
                return Err(HarnessError::Validation(format!(
                    "fleet {} speed {} exceeds max speed {}",
                    fleet_spec.record_index_1_based,
                    current_speed,
                    fleet.max_speed()
                )));
            }
            fleet.set_current_speed(current_speed);
        }
        if let Some(order) = &fleet_spec.order {
            game_data.set_fleet_order(
                fleet_spec.record_index_1_based,
                order.speed,
                order.kind.to_raw(),
                order.target,
                order.aux0,
                order.aux1,
            )?;
        }
    }
    Ok(())
}

/// Submit the scenario's turn files for one 1-based maintenance turn.
pub(crate) fn apply_turn_files(
    spec: &ScenarioSpec,
    turn: u16,
    game_data: &mut CoreGameData,
    queued_mail: &mut Vec<QueuedPlayerMail>,
) -> Result<(), HarnessError> {
    for turn_file in spec.turn_files.iter().filter(|file| file.turn == turn) {
        let submission = nc_data::TurnSubmission::load_kdl(&turn_file.path)
            .map_err(|err| HarnessError::Validation(err.to_string()))?;
        submission
            .apply_to(game_data, queued_mail)
            .map_err(|err| HarnessError::Validation(err.to_string()))?;
    }
    Ok(())
}

fn apply_queued_mail(spec: &ScenarioSpec, queued_mail: &mut Vec<QueuedPlayerMail>) {
    queued_mail.extend(spec.queued_mail.iter().map(|mail| QueuedPlayerMail {
        sender_empire_id: mail.sender_empire_raw,
        recipient_empire_id: mail.recipient_empire_raw,
        year: mail.year.unwrap_or(spec.metadata.year),
        subject: mail.subject.clone(),
        body: mail.body.clone(),
        recipient_deleted: false,
    }));
}

fn apply_message_blocks(
    spec: &ScenarioSpec,
    queued_mail: &mut Vec<QueuedPlayerMail>,
    player_count: u8,
) {
    queued_mail.extend(spec.message_blocks.iter().filter_map(|block| {
        let recipient_empire_id = block.player_record_index_1_based? as u8;
        Some(QueuedPlayerMail {
            sender_empire_id: synthetic_message_sender(recipient_empire_id, player_count),
            recipient_empire_id,
            year: spec.metadata.year,
            subject: String::new(),
            body: block.text.clone(),
            recipient_deleted: false,
        })
    }));
}

fn synthetic_message_sender(recipient_empire_id: u8, player_count: u8) -> u8 {
    (1..=player_count)
        .find(|empire_id| *empire_id != recipient_empire_id)
        .unwrap_or(recipient_empire_id)
}

fn sync_review_flags(
    game_data: &mut CoreGameData,
    report_block_rows: &[ReportBlockRow],
    message_blocks: &[ReviewBlockSpec],
) {
    for player in &mut game_data.player.records {
        player.set_classic_login_reviewables_present(false);
        player.set_classic_reports_pending_flag_raw(0);
        player.set_classic_messages_pending_flag_raw(0);
    }
    layout_classic_results(&game_data.player.records, report_block_rows)
        .apply_to_players(&mut game_data.player.records);

    let mut message_counts = vec![0usize; game_data.player.records.len()];
    for block in message_blocks {
        if let Some(player) = block.player_record_index_1_based {
            if let Some(count) = message_counts.get_mut(player - 1) {
                *count += 1;
            }
        }
    }

    for (idx, player) in game_data.player.records.iter_mut().enumerate() {
        if player.classic_results_chain().is_some() {
            player.set_classic_results_review_state_present(true);
            player.set_classic_reports_pending_flag_raw(1);
        }
        if message_counts[idx] > 0 {
            player.set_classic_messages_review_state_present(true);
            player.set_classic_messages_pending_flag_raw(1);
        }
    }
}

fn build_report_block_rows(blocks: &[ReviewBlockSpec]) -> Vec<ReportBlockRow> {
    blocks
        .iter()
        .enumerate()
        .map(|(idx, block)| {
            let mut row = ReportBlockRow {
                viewer_empire_id: block.player_record_index_1_based.unwrap_or(0) as u8,
                block_index: idx,
                decoded_text: normalize_report_block_text(&block.text),
                raw_bytes: None,
                recipient_deleted: false,
            };
            row.raw_bytes = Some(encode_report_block_rows(std::slice::from_ref(&row)));
            row
        })
        .collect()
}

fn normalize_report_block_text(text: &str) -> String {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines = normalized
        .split('\n')
        .map(str::trim_end)
        .collect::<Vec<_>>();
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    if lines.last().copied() != Some("<end of transmission>") {
        lines.push("<end of transmission>");
    }
    lines.join("\n")
}
