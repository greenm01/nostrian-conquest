//! Classic `RESULTS.DAT` chain layout.
//!
//! Original `ECMAINT` writes one shared `RESULTS.DAT` for every empire. Each
//! empire's reports form a linked chain of report headers, addressed by
//! 1-based record indexes into the whole file, and that empire's `PLAYER.DAT`
//! record points at the first and last header of its chain.

use std::collections::{BTreeMap, BTreeSet};

use crate::{Player, ReportBlockRow};

pub const CLASSIC_RESULTS_RECORD_SIZE: usize = 84;
const CHAIN_ID_OFFSET: usize = 74;
const NEXT_CHAIN_ID_OFFSET: usize = 78;

/// One empire's report chain, as 1-based record indexes of its first and last
/// report headers in the shared `RESULTS.DAT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassicResultsChain {
    pub first_header: u16,
    pub last_header: u16,
}

/// This empire's chain in the shared `RESULTS.DAT`: original `ECMAINT`
/// output (two- and three-human oracle runs, see `fixtures/ecmaint-results-*`)
/// stores the 1-based record indexes of its first and last report headers in
/// `PLAYER.DAT`, or 0 when it has none.
pub fn results_chain(player: &Player) -> Option<ClassicResultsChain> {
    let first_header = player.results_first as u16;
    (first_header != 0).then(|| ClassicResultsChain {
        first_header,
        last_header: player.results_last as u16,
    })
}

/// Point this empire at its chain in the shared `RESULTS.DAT`. Lay the file
/// out with [`layout_classic_results`] to get the chain.
pub fn set_results_chain(player: &mut Player, chain: Option<ClassicResultsChain>) {
    let chain = chain.unwrap_or(ClassicResultsChain {
        first_header: 0,
        last_header: 0,
    });
    player.results_first = i32::from(chain.first_header);
    player.results_last = i32::from(chain.last_header);
}

/// Whether either end of this empire's results chain is set.
pub fn has_results_chain(player: &Player) -> bool {
    player.results_first != 0 || player.results_last != 0
}

/// Advertise login-time reviewables to classic ECGAME.
///
/// Current compatibility rule:
/// - classic delivery/review probes use both message records together
///   for unread mail
/// - results-only startup probing does not become reviewable when only
///   the results-side record is set
///
/// Until a stricter family split is proven in classic login flow, keep the
/// mirrored advertisement here and route all startup-review producers
/// through this helper.
pub fn set_login_reviewables_present(player: &mut Player, present: bool) {
    player.messages_first = i32::from(present);
    player.messages_last = i32::from(present);
}

/// A laid-out shared `RESULTS.DAT` and the chain of every empire in it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClassicResultsLayout {
    pub bytes: Vec<u8>,
    pub chains: BTreeMap<u8, ClassicResultsChain>,
}

impl ClassicResultsLayout {
    pub fn chain_for(&self, empire_id: u8) -> Option<ClassicResultsChain> {
        self.chains.get(&empire_id).copied()
    }

    /// Point every `PLAYER.DAT` record at its chain in this file.
    pub fn apply_to_players(&self, players: &mut [Player]) {
        for (idx, player) in players.iter_mut().enumerate() {
            player.set_classic_results_chain(self.chain_for((idx + 1) as u8));
        }
    }
}

/// Lay out one shared `RESULTS.DAT` from report rows.
///
/// Each empire's visible blocks are written together, empires in ascending
/// order, and every record's chain ids are rewritten to absolute header
/// indexes within that empire's chain. A broadcast row (viewer `0`) is copied
/// into the chain of every active human empire, because a classic header
/// belongs to exactly one chain. Deleted rows and rows without classic record
/// bytes are left out.
pub fn layout_classic_results(players: &[Player], rows: &[ReportBlockRow]) -> ClassicResultsLayout {
    let broadcast_viewers = players
        .iter()
        .enumerate()
        .filter(|(_, player)| player.is_active_human_player())
        .map(|(idx, _)| (idx + 1) as u8)
        .collect::<BTreeSet<_>>();
    let laid_out_rows = rows
        .iter()
        .filter(|row| !row.recipient_deleted)
        .filter_map(|row| {
            let bytes = row.raw_bytes.as_deref()?;
            (bytes.len() >= CLASSIC_RESULTS_RECORD_SIZE).then_some((row.viewer_empire_id, bytes))
        })
        .collect::<Vec<_>>();
    let viewers = laid_out_rows
        .iter()
        .flat_map(|(viewer, _)| {
            if *viewer == 0 {
                broadcast_viewers.iter().copied().collect::<Vec<_>>()
            } else {
                vec![*viewer]
            }
        })
        .collect::<BTreeSet<_>>();

    let mut layout = ClassicResultsLayout::default();
    for viewer in viewers {
        let blocks = laid_out_rows
            .iter()
            .filter(|(row_viewer, _)| {
                *row_viewer == viewer || (*row_viewer == 0 && broadcast_viewers.contains(&viewer))
            })
            .map(|(_, bytes)| *bytes)
            .collect::<Vec<_>>();
        let mut headers = Vec::with_capacity(blocks.len());
        let mut next_record = layout.bytes.len() / CLASSIC_RESULTS_RECORD_SIZE;
        for block in &blocks {
            headers.push((next_record + 1) as u16);
            next_record += block.len() / CLASSIC_RESULTS_RECORD_SIZE;
        }
        for (block_idx, block) in blocks.iter().enumerate() {
            let chain_id = if block_idx == 0 {
                0
            } else {
                headers[block_idx - 1]
            };
            let next_chain_id = headers.get(block_idx + 1).copied().unwrap_or(0);
            for (record_idx, record) in block.chunks_exact(CLASSIC_RESULTS_RECORD_SIZE).enumerate()
            {
                let start = layout.bytes.len();
                layout.bytes.extend_from_slice(record);
                let record = &mut layout.bytes[start..];
                let next = if record_idx == 0 { next_chain_id } else { 0 };
                record[CHAIN_ID_OFFSET..CHAIN_ID_OFFSET + 2]
                    .copy_from_slice(&chain_id.to_le_bytes());
                record[NEXT_CHAIN_ID_OFFSET..NEXT_CHAIN_ID_OFFSET + 2]
                    .copy_from_slice(&next.to_le_bytes());
            }
        }
        layout.chains.insert(
            viewer,
            ClassicResultsChain {
                first_header: headers[0],
                last_header: headers[headers.len() - 1],
            },
        );
    }
    layout
}

/// Map each report header in a classic `RESULTS.DAT` to the empire whose
/// chain reaches it, walking from each `PLAYER.DAT` first-header pointer.
///
/// Keys are 1-based header record indexes. Headers no chain reaches are
/// absent.
pub fn classic_results_header_viewers(players: &[Player], bytes: &[u8]) -> BTreeMap<u16, u8> {
    let record_count = bytes.len() / CLASSIC_RESULTS_RECORD_SIZE;
    let mut viewers = BTreeMap::new();
    for (idx, player) in players.iter().enumerate() {
        let Some(chain) = player.classic_results_chain() else {
            continue;
        };
        let mut header = chain.first_header;
        while header != 0 && usize::from(header) <= record_count && !viewers.contains_key(&header) {
            viewers.insert(header, (idx + 1) as u8);
            let start = (usize::from(header) - 1) * CLASSIC_RESULTS_RECORD_SIZE;
            let record = &bytes[start..start + CLASSIC_RESULTS_RECORD_SIZE];
            header = u16::from_le_bytes([
                record[NEXT_CHAIN_ID_OFFSET],
                record[NEXT_CHAIN_ID_OFFSET + 1],
            ]);
        }
    }
    viewers
}
