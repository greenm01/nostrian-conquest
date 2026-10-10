//! Byte comparisons of the core files in their classic encoding, for the
//! oracle commands that check a directory against a preserved baseline.
//! Bytes no classic program reads are masked: a canonical fixture carries
//! ECMAINT's garbage there and the model writes zeros.

use nc_data::CoreGameData;
use nc_data::classic::{ClassicFile, without_ignored_bytes};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreFileDiffCount {
    pub name: &'static str,
    pub differing_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreFileDiffOffsets {
    pub name: &'static str,
    pub differing_offsets: Vec<usize>,
}

/// Per core file, how many bytes of `left` differ from `right`.
pub fn core_file_diff_counts(left: &CoreGameData, right: &CoreGameData) -> Vec<CoreFileDiffCount> {
    core_file_bytes(left, right)
        .into_iter()
        .map(|(name, left, right)| CoreFileDiffCount {
            name,
            differing_bytes: byte_diff_count(&left, &right),
        })
        .collect()
}

/// Per core file, the offsets where `left` differs from `right`.
pub fn core_file_diff_offsets(
    left: &CoreGameData,
    right: &CoreGameData,
) -> Vec<CoreFileDiffOffsets> {
    core_file_bytes(left, right)
        .into_iter()
        .map(|(name, left, right)| CoreFileDiffOffsets {
            name,
            differing_offsets: byte_diff_offsets(&left, &right),
        })
        .collect()
}

/// One message per core file of `left` that differs from `right`, named by
/// `label`.
pub fn core_file_exact_match_errors(
    left: &CoreGameData,
    right: &CoreGameData,
    label: &str,
) -> Vec<String> {
    core_file_diff_counts(left, right)
        .into_iter()
        .filter(|diff| diff.differing_bytes != 0)
        .map(|diff| {
            format!(
                "{} differs by {} bytes from {}",
                diff.name, diff.differing_bytes, label
            )
        })
        .collect()
}

/// `core_file_diff_counts` against `game_data` synced to the current-known
/// initialized post-maintenance baseline.
pub fn current_known_baseline_diff_counts(game_data: &CoreGameData) -> Vec<CoreFileDiffCount> {
    core_file_diff_counts(game_data, &current_known_baseline(game_data))
}

/// `core_file_diff_offsets` against `game_data` synced to the current-known
/// initialized post-maintenance baseline.
pub fn current_known_baseline_diff_offsets(game_data: &CoreGameData) -> Vec<CoreFileDiffOffsets> {
    core_file_diff_offsets(game_data, &current_known_baseline(game_data))
}

fn current_known_baseline(game_data: &CoreGameData) -> CoreGameData {
    let mut normalized = game_data.clone();
    normalized.sync_current_known_initialized_post_maint_baseline();
    normalized
}

/// The seven core files of `left` and `right` as masked classic bytes.
fn core_file_bytes(
    left: &CoreGameData,
    right: &CoreGameData,
) -> [(&'static str, Vec<u8>, Vec<u8>); 7] {
    let masked = |file: ClassicFile, left: Vec<u8>, right: Vec<u8>| {
        (
            file.file_name(),
            without_ignored_bytes(file, &left),
            without_ignored_bytes(file, &right),
        )
    };
    [
        masked(
            ClassicFile::Player,
            left.player.to_bytes(),
            right.player.to_bytes(),
        ),
        masked(
            ClassicFile::Planets,
            left.planets.to_bytes(),
            right.planets.to_bytes(),
        ),
        masked(
            ClassicFile::Fleets,
            left.fleets.to_bytes(),
            right.fleets.to_bytes(),
        ),
        masked(
            ClassicFile::Bases,
            left.bases.to_bytes(),
            right.bases.to_bytes(),
        ),
        masked(
            ClassicFile::Ipbm,
            left.ipbm.to_bytes(),
            right.ipbm.to_bytes(),
        ),
        masked(
            ClassicFile::Setup,
            left.setup.to_bytes(),
            right.setup.to_bytes(),
        ),
        masked(
            ClassicFile::Conquest,
            left.conquest.to_bytes(),
            right.conquest.to_bytes(),
        ),
    ]
}

fn byte_diff_count(left: &[u8], right: &[u8]) -> usize {
    left.iter()
        .zip(right.iter())
        .filter(|(a, b)| a != b)
        .count()
        + left.len().abs_diff(right.len())
}

fn byte_diff_offsets(left: &[u8], right: &[u8]) -> Vec<usize> {
    let shared_len = left.len().min(right.len());
    let mut offsets: Vec<usize> = left[..shared_len]
        .iter()
        .zip(right[..shared_len].iter())
        .enumerate()
        .filter_map(|(idx, (a, b))| (a != b).then_some(idx))
        .collect();

    let extra_len = left.len().max(right.len());
    offsets.extend(shared_len..extra_len);
    offsets
}
