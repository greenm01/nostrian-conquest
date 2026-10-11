use std::collections::{BTreeMap, BTreeSet};

use nc_data::{CoreGameData, MaintenanceEvents, Mission};

use crate::maint::results::binary::classic_results_lines;
use crate::maint::results::entries::{ReportEntry, ReportTarget, narrative_phase_for_report_text};
use crate::maint::results::format::{
    compact_fleet_number_list, fleet_number_from_idx, report_header,
};
use crate::maint::results::mod_constants::{JOIN_SUMMARY_PREVIEW_LINE_BUDGET, RESULTS_TAIL_FLEET};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JoinSummaryCompression {
    Full,
    RetargetCounts,
    RetargetAndCompletionCounts,
    SectionCounts,
}

#[derive(Debug, Default)]
struct JoinSummaryData {
    completed_by_host: BTreeMap<u8, Vec<u8>>,
    retargeted_joiners: BTreeSet<u8>,
    lost_hosts: BTreeMap<Option<u8>, Vec<u8>>,
    summary_week: Option<u8>,
}

fn fleet_numbers_subject(numbers: &[u8], compression: JoinSummaryCompression) -> String {
    match numbers {
        [] => "0 fleets".to_string(),
        [only] => format!("Fleet {only}"),
        many => match compression {
            JoinSummaryCompression::Full => {
                let compact = compact_fleet_number_list(many);
                let mut chars = compact.chars();
                match chars.next() {
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    None => compact,
                }
            }
            _ => format!("{} fleets", many.len()),
        },
    }
}

fn fleet_count_subject(total_fleets: usize) -> String {
    if total_fleets == 1 {
        "1 fleet".to_string()
    } else {
        format!("{total_fleets} fleets")
    }
}

fn join_summary_section_lines(
    label: &str,
    mut lines: Vec<String>,
    compression: JoinSummaryCompression,
    total_fleets: usize,
) -> Vec<String> {
    if lines.is_empty() {
        return Vec::new();
    }
    if compression == JoinSummaryCompression::SectionCounts {
        return vec![format!("{label}: {}.", fleet_count_subject(total_fleets))];
    }
    for line in &mut lines {
        *line = format!("{label}: {line}");
    }
    lines
}

pub fn build_join_summary_entries(
    game_data: &CoreGameData,
    events: &MaintenanceEvents,
    year: u16,
) -> Vec<ReportEntry> {
    let mut by_empire: BTreeMap<u8, JoinSummaryData> = BTreeMap::new();

    for event in events
        .fleet_merge_events
        .iter()
        .filter(|event| event.kind == Mission::JoinAnotherFleet)
    {
        let summary = by_empire.entry(event.owner_empire).or_default();
        summary
            .completed_by_host
            .entry(event.host_fleet_number)
            .or_default()
            .push(event.absorbed_fleet_number);
        summary.summary_week = match (summary.summary_week, event.stardate_week) {
            (Some(existing), Some(candidate)) => Some(existing.min(candidate)),
            (None, Some(candidate)) => Some(candidate),
            (existing, None) => existing,
        };
    }

    for event in &events.join_host_events {
        match *event {
            nc_data::JoinMissionHostEvent::Retargeted {
                fleet_idx,
                owner_empire,
                ..
            } => {
                if let Some(fleet_number) = fleet_number_from_idx(game_data, fleet_idx) {
                    by_empire
                        .entry(owner_empire)
                        .or_default()
                        .retargeted_joiners
                        .insert(fleet_number);
                }
            }
            nc_data::JoinMissionHostEvent::HostDestroyed {
                fleet_idx,
                owner_empire,
                destroyed_host_fleet_number,
                ..
            } => {
                if let Some(fleet_number) = fleet_number_from_idx(game_data, fleet_idx) {
                    by_empire
                        .entry(owner_empire)
                        .or_default()
                        .lost_hosts
                        .entry(
                            destroyed_host_fleet_number.filter(|fleet_number| *fleet_number != 0),
                        )
                        .or_default()
                        .push(fleet_number);
                }
            }
        }
    }

    for event in &events.mission_retarget_events {
        if let nc_data::MissionRetargetEvent::Retargeted {
            fleet_idx,
            reporting_fleet_number,
            owner_empire,
            mission: Mission::JoinAnotherFleet,
            ..
        } = *event
        {
            if let Some(fleet_number) = reporting_fleet_number
                .filter(|fleet_number| *fleet_number != 0)
                .or_else(|| fleet_number_from_idx(game_data, fleet_idx))
            {
                by_empire
                    .entry(owner_empire)
                    .or_default()
                    .retargeted_joiners
                    .insert(fleet_number);
            }
        }
    }

    let mut entries = Vec::new();
    for (owner_empire, mut summary) in by_empire {
        if summary.completed_by_host.is_empty()
            && summary.retargeted_joiners.is_empty()
            && summary.lost_hosts.is_empty()
        {
            continue;
        }

        for absorbed_numbers in summary.completed_by_host.values_mut() {
            absorbed_numbers.sort_unstable();
        }
        for lost_numbers in summary.lost_hosts.values_mut() {
            lost_numbers.sort_unstable();
        }

        let build_body_lines = |compression: JoinSummaryCompression| {
            let mut lines = vec!["Join mission summary".to_string()];

            let completed_subject_compression = match compression {
                JoinSummaryCompression::Full | JoinSummaryCompression::RetargetCounts => {
                    JoinSummaryCompression::Full
                }
                JoinSummaryCompression::RetargetAndCompletionCounts
                | JoinSummaryCompression::SectionCounts => {
                    JoinSummaryCompression::RetargetAndCompletionCounts
                }
            };
            let mut completed_lines = Vec::new();
            for (host_fleet_number, absorbed_numbers) in &summary.completed_by_host {
                completed_lines.push(format!(
                    "{} merged into Fleet {}.",
                    fleet_numbers_subject(absorbed_numbers, completed_subject_compression),
                    host_fleet_number
                ));
            }
            lines.extend(join_summary_section_lines(
                "Completed joins",
                completed_lines,
                if compression == JoinSummaryCompression::SectionCounts {
                    JoinSummaryCompression::SectionCounts
                } else {
                    JoinSummaryCompression::Full
                },
                summary.completed_by_host.values().map(Vec::len).sum(),
            ));

            if !summary.retargeted_joiners.is_empty() {
                let retargeted_numbers = summary
                    .retargeted_joiners
                    .iter()
                    .copied()
                    .collect::<Vec<_>>();
                let retargeted_lines = vec![format!(
                    "{}.",
                    fleet_numbers_subject(
                        &retargeted_numbers,
                        match compression {
                            JoinSummaryCompression::Full => JoinSummaryCompression::Full,
                            _ => JoinSummaryCompression::RetargetCounts,
                        }
                    )
                )];
                lines.extend(join_summary_section_lines(
                    "Retargeted to follow host",
                    retargeted_lines,
                    if compression == JoinSummaryCompression::SectionCounts {
                        JoinSummaryCompression::SectionCounts
                    } else {
                        JoinSummaryCompression::Full
                    },
                    retargeted_numbers.len(),
                ));
            }

            let mut lost_lines = Vec::new();
            for (destroyed_host_fleet_number, fleet_numbers) in &summary.lost_hosts {
                let subject = fleet_numbers_subject(fleet_numbers, JoinSummaryCompression::Full);
                let action = if fleet_numbers.len() == 1 {
                    "is"
                } else {
                    "are"
                };
                let line = match destroyed_host_fleet_number {
                    Some(host_fleet_number) => format!(
                        "{subject} lost host Fleet {host_fleet_number} and {action} holding position."
                    ),
                    None => {
                        format!("{subject} lost their host and {action} holding position.")
                    }
                };
                lost_lines.push(line);
            }
            lines.extend(join_summary_section_lines(
                "Lost hosts",
                lost_lines,
                if compression == JoinSummaryCompression::SectionCounts {
                    JoinSummaryCompression::SectionCounts
                } else {
                    JoinSummaryCompression::Full
                },
                summary.lost_hosts.values().map(Vec::len).sum(),
            ));
            lines
        };

        let source = "From your Fleet Command Center:";
        let header = report_header(source, summary.summary_week, year);
        let mut chosen_lines = build_body_lines(JoinSummaryCompression::Full);
        for compression in [
            JoinSummaryCompression::Full,
            JoinSummaryCompression::RetargetCounts,
            JoinSummaryCompression::RetargetAndCompletionCounts,
            JoinSummaryCompression::SectionCounts,
        ] {
            let candidate_lines = build_body_lines(compression);
            let candidate_text = format!("{header}\n{}", candidate_lines.join("\n"));
            chosen_lines = candidate_lines;
            if classic_results_lines(&candidate_text).len() <= JOIN_SUMMARY_PREVIEW_LINE_BUDGET {
                break;
            }
        }
        let body = chosen_lines.join("\n");
        entries.push(ReportEntry {
            text: format!("{header}\n{body}"),
            kind: 0x05,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both {
                recipient: owner_empire,
            },
            repeat_next_pointer: false,
            stardate_week: summary.summary_week,
            narrative_phase: narrative_phase_for_report_text(&body),
        });
    }
    entries
}
