//! Results reports: every line fits a classic 72-byte printable record, rows
//! go only to entitled empires, and a campaign-outlook announcement reaches
//! every active empire. The engine sends the outlook as one targeted row per
//! active empire, each flagged for review; it sends no broadcast (viewer `0`)
//! rows, so one counts as an unentitled recipient.
//!
//! An empire is entitled to results when it has an active player at either
//! end of the turn, or when it has a terminal outcome (defeat, loss, or win)
//! that its former player reviews.

use nc_data::{CoreGameData, PlayerMode, TerminalOutcome};

use super::{TurnObservation, Violation};

const RECORD_SIZE: usize = 84;
const TEXT_SIZE: usize = 72;
/// Shared by the leader's and the other empires' outlook text.
const OUTLOOK_TEXT: &str = "sole remaining serious contender";

pub fn check(observation: &TurnObservation) -> Vec<Violation> {
    let mut violations = Vec::new();
    let active = |state: &CoreGameData, viewer: u8| {
        state
            .players
            .get(usize::from(viewer).wrapping_sub(1))
            .is_some_and(|player| player.mode == PlayerMode::Human)
    };
    let entitled = |viewer: u8| {
        active(observation.before, viewer)
            || active(observation.after, viewer)
            || observation
                .lifecycle
                .get(usize::from(viewer).wrapping_sub(1))
                .is_some_and(|state| state.terminal_outcome != TerminalOutcome::None)
    };

    for row in observation.report_rows {
        let viewer = row.viewer_empire_id;
        if !entitled(viewer) {
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

    if !observation.events.campaign_outlook_events.is_empty() {
        for (idx, player) in observation.flagged.players.iter().enumerate() {
            let empire = (idx + 1) as u8;
            let heard = observation.report_rows.iter().any(|row| {
                row.viewer_empire_id == empire
                    && !row.recipient_deleted
                    && row.decoded_text.contains(OUTLOOK_TEXT)
            });
            if player.mode == PlayerMode::Human && (!heard || player.messages_last == 0) {
                violations.push(Violation::OutlookMissed { empire });
            }
        }
    }
    violations
}

fn printable_record_text(text: &[u8]) -> bool {
    text.len() <= TEXT_SIZE && text.iter().all(|byte| (0x20..=0x7e).contains(byte))
}
