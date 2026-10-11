use std::collections::BTreeMap;

use crate::{Player, PlayerMode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportBlockRow {
    /// `0` means a broadcast/legacy row visible to every viewer.
    pub viewer_empire_id: u8,
    pub block_index: usize,
    pub decoded_text: String,
    pub raw_bytes: Option<Vec<u8>>,
    pub recipient_deleted: bool,
}

impl ReportBlockRow {
    pub fn is_visible_to_viewer(&self, viewer_empire_id: u8) -> bool {
        self.viewer_empire_id == 0 || self.viewer_empire_id == viewer_empire_id
    }
}

/// Delete the report at `row_index` from one empire's view.
///
/// A broadcast row (viewer `0`) reaches every active human empire, as in the
/// classic export, so deleting it for one empire replaces it with a copy for
/// each other active human empire. The copies take the next free block
/// index, so they follow those empires' own reports. A row already deleted is
/// left alone.
pub fn delete_report_block_for_viewer(
    rows: &mut Vec<ReportBlockRow>,
    players: &[Player],
    row_index: usize,
    viewer_empire_id: u8,
) {
    let Some(row) = rows.get_mut(row_index).filter(|row| !row.recipient_deleted) else {
        return;
    };
    row.recipient_deleted = true;
    if row.viewer_empire_id != 0 {
        return;
    }
    let broadcast = row.clone();
    let block_index = rows.iter().map(|row| row.block_index).max().unwrap_or(0) + 1;
    let copies = players
        .iter()
        .enumerate()
        .filter(|(_, player)| player.mode == PlayerMode::Human)
        .map(|(idx, _)| (idx + 1) as u8)
        .filter(|empire_id| *empire_id != viewer_empire_id)
        .map(|empire_id| ReportBlockRow {
            viewer_empire_id: empire_id,
            block_index,
            recipient_deleted: false,
            ..broadcast.clone()
        })
        .collect::<Vec<_>>();
    rows.extend(copies);
}

/// Delete every report one empire can see from that empire's view.
pub fn delete_report_blocks_for_viewer(
    rows: &mut Vec<ReportBlockRow>,
    players: &[Player],
    viewer_empire_id: u8,
) {
    for row_index in 0..rows.len() {
        if rows[row_index].is_visible_to_viewer(viewer_empire_id) {
            delete_report_block_for_viewer(rows, players, row_index, viewer_empire_id);
        }
    }
}

/// The rows a maintenance year saves: every row of `previous` that no
/// player has deleted, in its order, then `fresh` (the year's own reports).
/// Reports wait until the player deletes them, as classic's RESULTS.DAT
/// chains do; nothing purges them by age. Block indexes are renumbered per
/// viewer so the chain order holds and the store's key stays unique.
pub fn carry_unread_report_blocks(
    previous: &[ReportBlockRow],
    fresh: Vec<ReportBlockRow>,
) -> Vec<ReportBlockRow> {
    let mut rows = previous
        .iter()
        .filter(|row| !row.recipient_deleted)
        .cloned()
        .chain(fresh)
        .collect::<Vec<_>>();
    let mut next_index = BTreeMap::<u8, usize>::new();
    for row in &mut rows {
        let index = next_index.entry(row.viewer_empire_id).or_default();
        row.block_index = *index;
        *index += 1;
    }
    rows
}
