//! Results reports: every line fits a classic 72-byte printable record, rows
//! go only to entitled empires, and broadcasts reach every active empire.
//! A campaign-outlook announcement is a broadcast: while any empire has an
//! active player it must produce a broadcast row, and every broadcast row
//! must be flagged for review by every active empire.
//!
//! An empire is entitled to results when it has an active player at either
//! end of the turn, or when it has a terminal outcome (defeat, loss, or win)
//! that its former player reviews.

use nc_data::{CoreGameData, TerminalOutcome};

use super::{TurnObservation, Violation};

const RECORD_SIZE: usize = 84;
const TEXT_SIZE: usize = 72;

pub fn check(observation: &TurnObservation) -> Vec<Violation> {
    let mut violations = Vec::new();
    let active = |state: &CoreGameData, viewer: u8| {
        state
            .player
            .records
            .get(usize::from(viewer).wrapping_sub(1))
            .is_some_and(|player| player.is_active_human_player())
    };
    let entitled = |viewer: u8| {
        active(observation.before, viewer)
            || active(observation.after, viewer)
            || observation
                .lifecycle
                .get(usize::from(viewer).wrapping_sub(1))
                .is_some_and(|state| state.terminal_outcome != TerminalOutcome::None)
    };

    let mut has_broadcast = false;
    for row in observation.report_rows {
        let viewer = row.viewer_empire_id;
        if viewer == 0 {
            has_broadcast |= !row.recipient_deleted;
        } else if !entitled(viewer) {
            violations.push(Violation::ReportRecipient { viewer });
        }
        for line in row.decoded_text.lines() {
            if !printable_record_text(line.as_bytes()) {
                violations.push(Violation::ReportLine {
                    viewer,
                    line: line.to_string(),
                });
            }
        }
        let raw = row.raw_bytes.as_deref().unwrap_or_default();
        let bad_record = raw.len() % RECORD_SIZE != 0
            || raw.chunks(RECORD_SIZE).any(|record| {
                let len = usize::from(record[1]);
                len > TEXT_SIZE || !printable_record_text(&record[2..2 + len])
            });
        if bad_record {
            violations.push(Violation::ReportLine {
                viewer,
                line: format!("raw records {raw:?}"),
            });
        }
    }

    let announced = !observation.events.campaign_outlook_events.is_empty();
    if has_broadcast || announced {
        for (idx, player) in observation.flagged.player.records.iter().enumerate() {
            if player.is_active_human_player()
                && (!has_broadcast || !player.has_classic_results_review_state())
            {
                violations.push(Violation::BroadcastMissed {
                    empire: (idx + 1) as u8,
                });
            }
        }
    }
    violations
}

fn printable_record_text(text: &[u8]) -> bool {
    text.len() <= TEXT_SIZE && text.iter().all(|byte| (0x20..=0x7e).contains(byte))
}
