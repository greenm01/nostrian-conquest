use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

pub mod classic;
mod core_diff;
mod core_files;
mod database_projection;
mod intel;

use classic::{parse_conquest_dat, parse_planets_dat};
use database_projection::build_database_dat;
use nc_classic::decode_report_blocks;
use nc_data::{
    CLASSIC_RESULTS_RECORD_SIZE, CampaignStore, CampaignStoreError, CoreGameData,
    MaintenanceEvents, QueuedPlayerMail, ReportBlockRow, classic_results_header_viewers,
    derive_campaign_seed_from_runtime, layout_classic_results, load_mail_queue,
};

pub use core_diff::{
    CoreFileDiffCount, CoreFileDiffOffsets, core_file_diff_counts, core_file_diff_offsets,
    core_file_exact_match_errors, current_known_baseline_diff_counts,
    current_known_baseline_diff_offsets,
};
pub use core_files::{GameDirectoryError, load_classic_core_files, save_classic_core_files};
pub use intel::{extract_player_intel_from_compat_database, merge_player_intel_from_compat};
pub use nc_classic::{DATABASE_RECORD_SIZE, DatabaseDat, DatabaseRecord};

const CLASSIC_AUXILIARY_FILES: &[&str] = &["MESSAGES.DAT", "RESULTS.DAT"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassicMailRecordPreview {
    pub index: usize,
    pub header_bytes: Vec<u8>,
    pub ascii_preview: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassicMessagesInspection {
    pub byte_len: usize,
    pub subject: Option<String>,
    pub printable_runs: Vec<String>,
    pub record_previews: Vec<ClassicMailRecordPreview>,
    pub raw_preview: Option<String>,
}

pub fn import_directory_snapshot(
    store: &CampaignStore,
    dir: &Path,
) -> Result<i64, CampaignStoreError> {
    import_directory_snapshot_with_seed(store, dir, None)
}

pub fn import_directory_snapshot_with_seed(
    store: &CampaignStore,
    dir: &Path,
    campaign_seed: Option<u64>,
) -> Result<i64, CampaignStoreError> {
    let game_data = load_classic_core_files(dir)?;
    let database = load_database_snapshot_or_default(dir, &game_data)?;
    let results_bytes = read_optional_path(dir.join("RESULTS.DAT"))?;
    let queued_mail = load_mail_queue_file(dir)?;
    let mut report_block_rows = decode_report_block_rows(&results_bytes);
    assign_classic_results_viewers(&mut report_block_rows, &game_data, &results_bytes);
    let derived_seed = campaign_seed.or_else(|| {
        Some(derive_campaign_seed_from_runtime(
            &game_data,
            &report_block_rows,
            &queued_mail,
        ))
    });
    let planet_intel_by_viewer = extract_player_intel_from_compat_database(
        &game_data,
        &database,
        game_data.conquest.game_year,
    );
    store.save_runtime_state_structured_with_intel_and_seed(
        game_data.conquest.game_year,
        &game_data,
        &BTreeSet::new(),
        &report_block_rows,
        &queued_mail,
        &planet_intel_by_viewer,
        derived_seed,
    )
}

pub fn export_latest_snapshot_to_dir(
    store: &CampaignStore,
    dir: &Path,
) -> Result<u16, CampaignStoreError> {
    let Some((snapshot_id, year)) = store.latest_snapshot_metadata()? else {
        return Ok(0);
    };
    export_snapshot_to_dir(store, snapshot_id, dir)?;
    Ok(year)
}

pub fn export_snapshot_to_dir(
    store: &CampaignStore,
    snapshot_id: i64,
    dir: &Path,
) -> Result<(), CampaignStoreError> {
    fs::create_dir_all(dir).map_err(|source| CampaignStoreError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    let mut game_data = store.load_snapshot_game_data(snapshot_id)?;
    let report_rows = store.load_snapshot_report_block_rows(snapshot_id, true)?;
    let results =
        layout_classic_results(&game_data.players, &with_classic_record_bytes(&report_rows));
    results.apply_to_players(&mut game_data.players);
    save_classic_core_files(dir, &game_data)?;
    let planet_intel_by_viewer =
        store.load_snapshot_planet_intel_by_viewer(snapshot_id, game_data.conquest.player_count)?;
    let template_database = load_optional_database_template(dir, &game_data)?;
    let database = build_database_dat(
        &game_data,
        &game_data.planets,
        &planet_intel_by_viewer,
        &MaintenanceEvents::default(),
        template_database.as_ref(),
    );
    write_path(dir.join("DATABASE.DAT"), &database.to_bytes())?;

    write_path(dir.join("RESULTS.DAT"), &results.bytes)?;
    write_path(dir.join("MESSAGES.DAT"), &[])?;
    Ok(())
}

pub fn ensure_classic_auxiliary_files(dir: &Path) -> Result<(), CampaignStoreError> {
    for name in CLASSIC_AUXILIARY_FILES {
        let path = dir.join(name);
        if !path.exists() {
            write_path(path, &[])?;
        }
    }
    Ok(())
}

pub fn inspect_classic_messages_dat(bytes: &[u8]) -> ClassicMessagesInspection {
    let printable_runs = printable_runs(bytes, 6);
    let record_previews = if bytes.len() % 40 == 0 {
        bytes
            .chunks_exact(40)
            .enumerate()
            .map(|(index, record)| ClassicMailRecordPreview {
                index,
                header_bytes: record[..record.len().min(8)].to_vec(),
                ascii_preview: ascii_preview(record),
            })
            .collect()
    } else {
        Vec::new()
    };
    let raw_preview = if bytes.is_empty() || !record_previews.is_empty() {
        None
    } else {
        Some(ascii_preview(&bytes[..bytes.len().min(80)]))
    };

    ClassicMessagesInspection {
        byte_len: bytes.len(),
        subject: decode_pascal_ascii(bytes),
        printable_runs,
        record_previews,
        raw_preview,
    }
}

pub fn decode_report_block_rows(bytes: &[u8]) -> Vec<ReportBlockRow> {
    decode_report_blocks(bytes)
        .into_iter()
        .enumerate()
        .map(|(idx, block)| ReportBlockRow {
            viewer_empire_id: 0,
            block_index: idx,
            decoded_text: block.decoded_text,
            raw_bytes: block.raw_bytes,
            recipient_deleted: false,
        })
        .collect()
}

/// Tag each decoded classic report with the empire whose `PLAYER.DAT` chain
/// reaches its header.  Reports no chain reaches stay broadcast (`0`).
fn assign_classic_results_viewers(
    rows: &mut [ReportBlockRow],
    game_data: &CoreGameData,
    results_bytes: &[u8],
) {
    let header_viewers = classic_results_header_viewers(&game_data.players, results_bytes);
    let mut header = 1usize;
    for row in rows {
        let Some(raw_bytes) = row.raw_bytes.as_ref() else {
            continue;
        };
        if let Some(&viewer) = u16::try_from(header)
            .ok()
            .and_then(|header| header_viewers.get(&header))
        {
            row.viewer_empire_id = viewer;
        }
        header += raw_bytes.len() / CLASSIC_RESULTS_RECORD_SIZE;
    }
}

/// Rows with classic record bytes, encoding any text-only row.
fn with_classic_record_bytes(rows: &[ReportBlockRow]) -> Vec<ReportBlockRow> {
    rows.iter()
        .cloned()
        .map(|mut row| {
            if row.raw_bytes.is_none() {
                row.raw_bytes = Some(encode_report_block_rows(std::slice::from_ref(&row)));
            }
            row
        })
        .collect()
}

pub fn encode_report_block_rows(rows: &[ReportBlockRow]) -> Vec<u8> {
    let blocks = rows
        .iter()
        .map(|row| nc_classic::ClassicReportBlock {
            decoded_text: row.decoded_text.clone(),
            raw_bytes: row.raw_bytes.clone(),
        })
        .collect::<Vec<_>>();
    nc_classic::encode_report_blocks(&blocks)
}

pub fn write_default_database_dat_for_game_data(
    dir: &Path,
    game_data: &CoreGameData,
) -> Result<(), CampaignStoreError> {
    let planet_names = game_data
        .planets
        .iter()
        .map(|planet| planet.name.clone())
        .collect::<Vec<_>>();
    let database = DatabaseDat::generate_from_planets_and_year(
        &planet_names,
        game_data.conquest.game_year,
        game_data.conquest.player_count as usize,
        None,
    );
    write_path(dir.join("DATABASE.DAT"), &database.to_bytes())
}

pub fn regenerate_database_dat_from_directory(
    dir: &Path,
    template_path: Option<&Path>,
) -> Result<(), CampaignStoreError> {
    let planets = parse_planets_dat(&read_required_path(dir.join("PLANETS.DAT"))?)
        .map_err(classic_parse_error)?;
    let conquest = parse_conquest_dat(&read_required_path(dir.join("CONQUEST.DAT"))?)
        .map_err(classic_parse_error)?;
    let template = match template_path {
        Some(path) => Some(
            DatabaseDat::parse(&read_required_path(path.to_path_buf())?)
                .map_err(classic_parse_error)?,
        ),
        None => None,
    };
    let planet_names: Vec<String> = planets.iter().map(|planet| planet.name.clone()).collect();
    let database = DatabaseDat::generate_from_planets_and_year(
        &planet_names,
        conquest.game_year,
        conquest.player_count as usize,
        template.as_ref(),
    );
    write_path(dir.join("DATABASE.DAT"), &database.to_bytes())
}

fn read_optional_path(path: PathBuf) -> Result<Vec<u8>, CampaignStoreError> {
    match fs::read(&path) {
        Ok(bytes) => Ok(bytes),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(source) => Err(CampaignStoreError::Io { path, source }),
    }
}

pub(crate) fn classic_parse_error(err: nc_classic::ParseError) -> CampaignStoreError {
    let parse = match err {
        nc_classic::ParseError::WrongSize {
            file_type,
            expected,
            actual,
        } => nc_data::ParseError::WrongSize {
            file_type,
            expected,
            actual,
        },
        nc_classic::ParseError::WrongRecordMultiple {
            file_type,
            record_size,
            actual,
        } => nc_data::ParseError::WrongRecordMultiple {
            file_type,
            record_size,
            actual,
        },
        nc_classic::ParseError::ReservedBuildSlot {
            planet,
            sector,
            units,
        } => nc_data::ParseError::ReservedBuildSlot {
            planet,
            sector,
            units,
        },
    };
    CampaignStoreError::Parse(parse)
}

fn decode_pascal_ascii(bytes: &[u8]) -> Option<String> {
    let len = *bytes.first()? as usize;
    if len == 0 || len + 1 > bytes.len() {
        return None;
    }
    let candidate = &bytes[1..1 + len];
    if candidate.iter().all(|b| b.is_ascii_graphic() || *b == b' ') {
        Some(String::from_utf8_lossy(candidate).to_string())
    } else {
        None
    }
}

fn printable_runs(bytes: &[u8], min_len: usize) -> Vec<String> {
    let mut runs = Vec::new();
    let mut current = Vec::new();

    for &byte in bytes {
        if byte.is_ascii_graphic() || byte == b' ' {
            current.push(byte);
        } else {
            if current.len() >= min_len {
                runs.push(String::from_utf8_lossy(&current).to_string());
            }
            current.clear();
        }
    }

    if current.len() >= min_len {
        runs.push(String::from_utf8_lossy(&current).to_string());
    }

    runs
}

fn ascii_preview(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| {
            if byte.is_ascii_graphic() || *byte == b' ' {
                char::from(*byte)
            } else {
                '.'
            }
        })
        .collect()
}

fn read_required_path(path: PathBuf) -> Result<Vec<u8>, CampaignStoreError> {
    fs::read(&path).map_err(|source| CampaignStoreError::Io { path, source })
}

fn write_path(path: PathBuf, bytes: &[u8]) -> Result<(), CampaignStoreError> {
    fs::write(&path, bytes).map_err(|source| CampaignStoreError::Io { path, source })
}

fn load_mail_queue_file(dir: &Path) -> Result<Vec<QueuedPlayerMail>, CampaignStoreError> {
    let path = dir.join("RUSTMAIL.QUE");
    if !path.exists() {
        return Ok(Vec::new());
    }
    load_mail_queue(dir).map_err(|err| CampaignStoreError::Io {
        path,
        source: std::io::Error::other(err.to_string()),
    })
}

fn load_database_snapshot_or_default(
    dir: &Path,
    game_data: &CoreGameData,
) -> Result<DatabaseDat, CampaignStoreError> {
    let path = dir.join("DATABASE.DAT");
    match fs::read(&path) {
        Ok(bytes) => DatabaseDat::parse(&bytes).map_err(classic_parse_error),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            Ok(default_unknown_database_template(game_data))
        }
        Err(source) => Err(CampaignStoreError::Io { path, source }),
    }
}

fn load_optional_database_template(
    dir: &Path,
    game_data: &CoreGameData,
) -> Result<Option<DatabaseDat>, CampaignStoreError> {
    let path = dir.join("DATABASE.DAT");
    match fs::read(&path) {
        Ok(bytes) => DatabaseDat::parse(&bytes)
            .map(Some)
            .map_err(classic_parse_error),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            Ok(Some(default_unknown_database_template(game_data)))
        }
        Err(source) => Err(CampaignStoreError::Io { path, source }),
    }
}

fn default_unknown_database_template(game_data: &CoreGameData) -> DatabaseDat {
    let names = vec!["UNKNOWN".to_string(); game_data.planets.len()];
    DatabaseDat::generate_from_planets_and_year(
        &names,
        game_data.conquest.game_year,
        game_data.conquest.player_count as usize,
        None,
    )
}
