use std::collections::{BTreeMap, BTreeSet};

use nc_data::{CoreGameData, MaintenanceEvents, ReportBlockRow};

use super::binary::{
    RESULTS_END_OF_TRANSMISSION, RESULTS_RECORD_SIZE, classic_results_chain_tail_for_year,
    classic_results_lines, classic_results_record_count, push_classic_results_chunked,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NarrativePhase {
    MovementPrelude,
    IntelObservation,
    ContactIdentify,
    BattleResolution,
    DefenderAftermath,
    AttackerAftermath,
    CombatFollowOn,
    Generic,
}

#[derive(Debug, Clone, Copy)]
pub enum ReportTarget {
    /// Goes into RESULTS.DAT and marks `recipient` as the intended reviewer.
    Both { recipient: u8 },
}

pub struct ReportEntry {
    pub text: String,
    pub kind: u8,
    pub tail: [u8; 10],
    pub target: ReportTarget,
    pub repeat_next_pointer: bool,
    pub stardate_week: Option<u8>,
    pub narrative_phase: NarrativePhase,
}

pub fn build_results_dat(game_data: &CoreGameData, events: &MaintenanceEvents) -> Vec<u8> {
    build_results_rows_with_review(game_data, events)
        .into_iter()
        .flat_map(|row| row.raw_bytes.unwrap_or_default())
        .collect()
}

pub fn build_results_report_blocks(
    game_data: &CoreGameData,
    events: &MaintenanceEvents,
) -> Vec<ReportBlockRow> {
    build_results_rows_with_review(game_data, events)
}

pub fn apply_results_reviewable_flags(game_data: &mut CoreGameData, rows: &[ReportBlockRow]) {
    let occupied_viewers = occupied_result_viewers(game_data);
    let mut visible_record_counts = BTreeMap::<u8, u16>::new();
    for row in rows {
        if row.recipient_deleted {
            continue;
        }
        let record_count = row
            .raw_bytes
            .as_ref()
            .map(|bytes| (bytes.len() / RESULTS_RECORD_SIZE) as u16)
            .unwrap_or(0);
        if row.viewer_empire_id == 0 {
            for &viewer_empire_raw in &occupied_viewers {
                *visible_record_counts.entry(viewer_empire_raw).or_default() += record_count;
            }
        } else {
            *visible_record_counts
                .entry(row.viewer_empire_id)
                .or_default() += record_count;
        }
    }

    for (idx, player) in game_data.player.records.iter_mut().enumerate() {
        let viewer_empire_raw = (idx + 1) as u8;
        let visible_record_count = *visible_record_counts.get(&viewer_empire_raw).unwrap_or(&0);
        let has_results = visible_record_count > 0;
        player.set_classic_results_review_state_present(has_results);
        player.set_classic_results_chain_state(
            has_results,
            if has_results { visible_record_count } else { 0 },
        );
    }
}

pub(super) struct ResultsReviewPlan {
    viewer_entries: BTreeMap<u8, Vec<ReportEntry>>,
    viewers_with_results: BTreeSet<u8>,
}

pub(super) fn build_results_rows_with_review(
    game_data: &CoreGameData,
    events: &MaintenanceEvents,
) -> Vec<ReportBlockRow> {
    let result_entries = generate_report_entries(game_data, events);
    let year = game_data.conquest.game_year();
    let ResultsReviewPlan {
        viewer_entries,
        viewers_with_results,
    } = results_review_plan(&result_entries);
    let mut rows = Vec::new();
    for (viewer_empire_id, entries) in viewer_entries {
        rows.extend(build_rows_for_viewer(viewer_empire_id, &entries, year));
    }
    debug_assert!(rows.iter().all(|row| {
        row.viewer_empire_id == 0 || viewers_with_results.contains(&row.viewer_empire_id)
    }));
    rows
}

pub(super) fn results_review_plan(result_entries: &[ReportEntry]) -> ResultsReviewPlan {
    let mut viewers_with_results = BTreeSet::new();
    let mut viewer_entries = BTreeMap::<u8, Vec<ReportEntry>>::new();

    for entry in result_entries {
        match entry.target {
            ReportTarget::Both { recipient } if recipient != 0 => {
                viewers_with_results.insert(recipient);
                viewer_entries
                    .entry(recipient)
                    .or_default()
                    .push(clone_report_entry(entry));
            }
            _ => {}
        }
    }

    ResultsReviewPlan {
        viewer_entries,
        viewers_with_results,
    }
}

pub(super) fn build_rows_for_viewer(
    viewer_empire_id: u8,
    entries: &[ReportEntry],
    year: u16,
) -> Vec<ReportBlockRow> {
    let record_counts = entries
        .iter()
        .map(|entry| classic_results_record_count(&entry.text, entry.kind))
        .collect::<Vec<_>>();
    let mut header_record_indexes = Vec::with_capacity(record_counts.len());
    let mut next_header_record_index = 0usize;
    for record_count in &record_counts {
        header_record_indexes.push(next_header_record_index);
        next_header_record_index += *record_count;
    }

    entries
        .iter()
        .enumerate()
        .map(|(block_index, entry)| {
            let chain_id = if block_index == 0 {
                0
            } else {
                (header_record_indexes[block_index - 1] + 1) as u16
            };
            let next_chain_id = if block_index + 1 < header_record_indexes.len() {
                (header_record_indexes[block_index + 1] + 1) as u16
            } else {
                0
            };
            let header_tail =
                classic_results_chain_tail_for_year(entry.tail, year, chain_id, next_chain_id);
            let continuation_next_chain_id = if entry.repeat_next_pointer {
                next_chain_id
            } else {
                0
            };
            let continuation_tail = classic_results_chain_tail_for_year(
                entry.tail,
                year,
                chain_id,
                continuation_next_chain_id,
            );
            let mut raw_bytes = Vec::new();
            push_classic_results_chunked(
                &mut raw_bytes,
                header_tail,
                continuation_tail,
                &entry.text,
            );
            let mut lines = classic_results_lines(&entry.text);
            lines.push(RESULTS_END_OF_TRANSMISSION.to_string());
            ReportBlockRow {
                viewer_empire_id,
                block_index,
                decoded_text: lines.join("\n"),
                raw_bytes: Some(raw_bytes),
                recipient_deleted: false,
            }
        })
        .collect()
}

/// Broadcast reports go to every empire with an active human player.  Rogue
/// (`0xff`) empires run on autopilot and civil-disorder/unjoined (`0x00`)
/// empires have no commander, so neither has anyone to review results.
pub(super) fn occupied_result_viewers(game_data: &CoreGameData) -> Vec<u8> {
    game_data
        .player
        .records
        .iter()
        .enumerate()
        .filter_map(|(idx, player)| player.is_active_human_player().then_some((idx + 1) as u8))
        .collect()
}

pub(super) fn clone_report_entry(entry: &ReportEntry) -> ReportEntry {
    ReportEntry {
        text: entry.text.clone(),
        kind: entry.kind,
        tail: entry.tail,
        target: entry.target,
        repeat_next_pointer: entry.repeat_next_pointer,
        stardate_week: entry.stardate_week,
        narrative_phase: entry.narrative_phase,
    }
}

pub mod combat;
pub mod intel;
pub mod misc;
pub mod missions;

pub fn generate_report_entries(
    game_data: &CoreGameData,
    events: &MaintenanceEvents,
) -> Vec<ReportEntry> {
    let year = game_data.conquest.game_year();
    let mut entries: Vec<ReportEntry> = Vec::new();

    combat::push_combat_entries(&mut entries, game_data, events, year);
    intel::push_intel_entries(&mut entries, game_data, events, year);
    let consumed_roe_disposition_indices =
        missions::push_mission_entries(&mut entries, game_data, events, year);
    misc::push_misc_entries(&mut entries, game_data, events, year);
    misc::push_roe_entries(
        &mut entries,
        game_data,
        events,
        year,
        &consumed_roe_disposition_indices,
    );

    entries.sort_by_key(|e| (e.stardate_week.unwrap_or(0), e.narrative_phase));
    entries
}

pub fn narrative_phase_for_report_text(text: &str) -> NarrativePhase {
    if text.contains("Sensor contact") && text.contains("detected and identified")
        || text.contains("We identified an alien fleet")
        || text.contains("We have located and identified an alien fleet")
        || text.contains("we are avoiding engagement")
    {
        NarrativePhase::ContactIdentify
    } else if text.contains("ALERT: Enemy fleet contact!")
        || text.contains("ALERT: Fleet contact lost!")
        || text.contains("ALERT: Starbase contact lost!")
        || text.contains("We lost all contact")
        || text.contains("We successfully intercepted")
        || text.contains("We were attacked by")
        || text.contains("We engaged")
        || text.contains("We attempted to disengage")
    {
        NarrativePhase::BattleResolution
    } else if text.contains("ALERT: Orbital bombardment underway!")
        || text.contains("Planet captured by")
        || text.contains("Planet seized in a blitz assault by")
        || text.contains("Our world has been bombarded")
        || text.contains("We have been bombarded")
        || text.contains("We have been invaded and captured")
    {
        NarrativePhase::DefenderAftermath
    } else if text.contains("ALERT: Planetary invasion successful!")
        || text.contains("ALERT: Planetary invasion repulsed!")
        || text.contains("ALERT: Planetary invasion blocked!")
        || text.contains("ALERT: Blitz assault successful!")
        || text.contains("ALERT: Blitz assault failed!")
        || text.contains("Bombardment mission report")
        || text.contains("Invasion mission report")
        || text.contains("Blitz mission report")
    {
        if text.contains("preparing for bombardment")
            || text.contains("preparing to begin the invasion")
            || text.contains("preparing to launch the assault")
        {
            NarrativePhase::MovementPrelude
        } else if text.contains("Hostile action stripped us")
            || text.contains("Enemy ground batteries prevented a landing")
        {
            NarrativePhase::CombatFollowOn
        } else {
            NarrativePhase::AttackerAftermath
        }
    } else if text.contains("Viewing mission report") || text.contains("Scouting mission report") {
        if text.contains("completed a long range viewing analysis")
            || text.contains("compiled the following data")
        {
            NarrativePhase::IntelObservation
        } else if text.contains("Hostile action forced us to abort")
            || text.contains("forced us to break off")
        {
            NarrativePhase::CombatFollowOn
        } else if text.contains("Sensor contact") && text.contains("detected and identified") {
            NarrativePhase::ContactIdentify
        } else if text.contains("We identified an alien fleet") {
            NarrativePhase::ContactIdentify
        } else {
            NarrativePhase::MovementPrelude
        }
    } else if text.contains("Move mission report")
        || text.contains("Guard Starbase mission report")
        || text.contains("Guard/Blockade World mission report")
        || text.contains("Patrol mission report")
        || text.contains("Seek-Home mission report")
        || text.contains("Rendezvous mission report")
        || text.contains("Colonization mission report")
        || text.contains("Salvage mission report")
        || text.contains("Join mission report")
    {
        if text.contains("Hostile action forced") || text.contains("was destroyed") {
            NarrativePhase::CombatFollowOn
        } else {
            NarrativePhase::MovementPrelude
        }
    } else {
        NarrativePhase::Generic
    }
}
