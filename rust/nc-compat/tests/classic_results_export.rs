//! Classic RESULTS.DAT export with several human empires, checked against
//! original ECMAINT output preserved in `fixtures/ecmaint-results-*`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use nc_compat::{
    export_latest_snapshot_to_dir, import_directory_snapshot, load_classic_core_files,
};
use nc_data::{CampaignStore, ReportBlockRow, carry_unread_report_blocks, set_results_chain};

const RECORD: usize = 84;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn oracle_post(name: &str) -> PathBuf {
    repo_root().join(format!("fixtures/{name}-post/v1.5"))
}

fn scratch_dir(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir =
        std::env::temp_dir().join(format!("nc-compat-{label}-{}-{unique}", std::process::id()));
    fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn word(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

/// PLAYER.DAT +0x38 / +0x3c for each player record.
fn player_pointers(player_dat: &[u8]) -> Vec<(u16, u16)> {
    player_dat
        .chunks_exact(110)
        .map(|record| (word(record, 0x38), word(record, 0x3c)))
        .collect()
}

/// Header record indexes (1-based) reached from `first`, following NextChainId
/// and checking each header's ChainId names the previous header.
fn walk_chain(results: &[u8], first: u16) -> Vec<u16> {
    let mut headers = Vec::new();
    let mut previous = 0;
    let mut header = first;
    while header != 0 {
        let record = &results[(usize::from(header) - 1) * RECORD..][..RECORD];
        assert_eq!(word(record, 74), previous, "header {header} ChainId");
        headers.push(header);
        previous = header;
        header = word(record, 78);
    }
    headers
}

/// The records of the report whose header is at `header`.
fn report_records(results: &[u8], header: u16) -> Vec<&[u8]> {
    let start = (usize::from(header) - 1) * RECORD;
    let count = usize::from(results[start]);
    results[start..start + count * RECORD]
        .chunks_exact(RECORD)
        .collect()
}

fn report_text(results: &[u8], header: u16) -> Vec<Vec<u8>> {
    report_records(results, header)
        .into_iter()
        .map(|record| record[..74].to_vec())
        .collect()
}

/// Each player's chain as the report text it reaches, checking that every
/// continuation shares its header's ChainId and has NextChainId zero, and that
/// +0x3c names the last header.
fn chains_by_player(player_dat: &[u8], results: &[u8]) -> Vec<Vec<Vec<Vec<u8>>>> {
    player_pointers(player_dat)
        .into_iter()
        .map(|(first, last)| {
            let headers = walk_chain(results, first);
            assert_eq!(
                headers.last().copied().unwrap_or(0),
                last,
                "+0x3c last header"
            );
            for &header in &headers {
                let records = report_records(results, header);
                let chain_id = word(records[0], 74);
                for record in &records[1..] {
                    assert_eq!(word(record, 74), chain_id);
                    assert_eq!(word(record, 78), 0);
                }
            }
            headers
                .into_iter()
                .map(|header| report_text(results, header))
                .collect()
        })
        .collect()
}

/// Rows a Rust engine would store for the oracle file: one per report,
/// tagged with the player whose chain reaches it, with chain ids counted
/// within that player's reports only.
fn per_viewer_rows(player_dat: &[u8], results: &[u8]) -> Vec<ReportBlockRow> {
    let mut rows = Vec::new();
    for (idx, (first, _)) in player_pointers(player_dat).into_iter().enumerate() {
        let mut local_record = 0u16;
        let mut previous_local_header = 0u16;
        for (block_index, header) in walk_chain(results, first).into_iter().enumerate() {
            let records = report_records(results, header);
            let mut raw = Vec::new();
            for (record_idx, record) in records.iter().enumerate() {
                let mut record = record.to_vec();
                record[74..76].copy_from_slice(&previous_local_header.to_le_bytes());
                if record_idx == 0 {
                    record[78..80].fill(0);
                }
                raw.extend_from_slice(&record);
            }
            previous_local_header = local_record + 1;
            local_record += records.len() as u16;
            rows.push(ReportBlockRow {
                viewer_empire_id: (idx + 1) as u8,
                block_index,
                decoded_text: String::new(),
                raw_bytes: Some(raw),
                recipient_deleted: false,
            });
        }
    }
    rows
}

fn export_rows(label: &str, oracle: &Path, rows: &[ReportBlockRow]) -> (Vec<u8>, Vec<u8>) {
    let mut game_data = load_classic_core_files(oracle).expect("oracle game data");
    for player in &mut game_data.players {
        set_results_chain(player, None);
    }
    let dir = scratch_dir(label);
    let store = CampaignStore::open_default_in_dir(&dir).expect("store");
    store
        .save_runtime_state_structured(&game_data, &BTreeSet::new(), rows, &[])
        .expect("save");
    export_latest_snapshot_to_dir(&store, &dir).expect("export");
    let exported = (
        fs::read(dir.join("PLAYER.DAT")).expect("PLAYER.DAT"),
        fs::read(dir.join("RESULTS.DAT")).expect("RESULTS.DAT"),
    );
    let _ = fs::remove_dir_all(&dir);
    exported
}

#[test]
fn per_viewer_rows_export_to_the_oracle_two_human_results_file() {
    let oracle = oracle_post("ecmaint-results-two-human");
    let player_dat = fs::read(oracle.join("PLAYER.DAT")).unwrap();
    let results = fs::read(oracle.join("RESULTS.DAT")).unwrap();
    assert_eq!(
        player_pointers(&player_dat),
        vec![(1, 6), (12, 16), (0, 0), (0, 0)]
    );

    let rows = per_viewer_rows(&player_dat, &results);
    let (exported_player, exported_results) = export_rows("two-human", &oracle, &rows);

    assert_eq!(exported_results, results);
    assert_eq!(
        player_pointers(&exported_player),
        player_pointers(&player_dat)
    );
}

#[test]
fn per_viewer_rows_export_every_interleaved_oracle_chain() {
    let oracle = oracle_post("ecmaint-results-interleaved");
    let player_dat = fs::read(oracle.join("PLAYER.DAT")).unwrap();
    let results = fs::read(oracle.join("RESULTS.DAT")).unwrap();
    assert_eq!(
        player_pointers(&player_dat),
        vec![(22, 26), (12, 36), (1, 47), (0, 0)]
    );

    let rows = per_viewer_rows(&player_dat, &results);
    let (exported_player, exported_results) = export_rows("interleaved", &oracle, &rows);

    assert_eq!(exported_results.len(), results.len());
    assert_eq!(
        chains_by_player(&exported_player, &exported_results),
        chains_by_player(&player_dat, &results)
    );
}

#[test]
fn imported_interleaved_oracle_results_keep_every_player_chain_on_export() {
    let oracle = oracle_post("ecmaint-results-interleaved");
    let player_dat = fs::read(oracle.join("PLAYER.DAT")).unwrap();
    let results = fs::read(oracle.join("RESULTS.DAT")).unwrap();

    let dir = scratch_dir("import");
    for entry in fs::read_dir(&oracle).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            fs::copy(entry.path(), dir.join(entry.file_name())).unwrap();
        }
    }
    let store = CampaignStore::open_default_in_dir(&dir).expect("store");
    import_directory_snapshot(&store, &dir).expect("import");
    let rows = store
        .load_snapshot_report_block_rows(store.latest_snapshot_metadata().unwrap().unwrap().0, true)
        .unwrap();
    let viewers = rows
        .iter()
        .map(|row| row.viewer_empire_id)
        .collect::<BTreeSet<_>>();
    assert_eq!(viewers, BTreeSet::from([1, 2, 3]));

    export_latest_snapshot_to_dir(&store, &dir).expect("export");
    let exported_player = fs::read(dir.join("PLAYER.DAT")).unwrap();
    let exported_results = fs::read(dir.join("RESULTS.DAT")).unwrap();
    let _ = fs::remove_dir_all(&dir);

    assert_eq!(exported_results.len(), results.len());
    assert_eq!(
        chains_by_player(&exported_player, &exported_results),
        chains_by_player(&player_dat, &results)
    );
}

/// A year's export carries each empire's unread reports from earlier years
/// at the head of its chain, before the year's own, and leaves out what
/// the player deleted.
#[test]
fn unread_reports_from_earlier_years_lead_each_chain_on_export() {
    let oracle = oracle_post("ecmaint-results-two-human");
    let player_dat = fs::read(oracle.join("PLAYER.DAT")).unwrap();
    let results = fs::read(oracle.join("RESULTS.DAT")).unwrap();
    let original = chains_by_player(&player_dat, &results);

    // Last year's rows: the oracle's, with empire 1's first report deleted.
    let mut previous = per_viewer_rows(&player_dat, &results);
    assert_eq!(previous[0].viewer_empire_id, 1);
    previous[0].recipient_deleted = true;
    let fresh = [(1, "New for one"), (2, "New for two")]
        .into_iter()
        .map(|(viewer_empire_id, text): (u8, &str)| {
            // One header record: record count, text length, text.
            let mut raw = vec![0u8; RECORD];
            raw[0] = 1;
            raw[1] = text.len() as u8;
            raw[2..2 + text.len()].copy_from_slice(text.as_bytes());
            ReportBlockRow {
                viewer_empire_id,
                block_index: 0,
                decoded_text: text.to_string(),
                raw_bytes: Some(raw),
                recipient_deleted: false,
            }
        })
        .collect();
    let rows = carry_unread_report_blocks(&previous, fresh);

    let (exported_player, exported_results) = export_rows("carried", &oracle, &rows);
    let chains = chains_by_player(&exported_player, &exported_results);
    let holds = |report: &[Vec<u8>], text: &str| {
        report
            .iter()
            .any(|record| record.windows(text.len()).any(|w| w == text.as_bytes()))
    };

    assert_eq!(chains[0].len(), original[0].len());
    assert_eq!(chains[0][..original[0].len() - 1], original[0][1..]);
    assert!(holds(chains[0].last().unwrap(), "New for one"));
    assert_eq!(chains[1].len(), original[1].len() + 1);
    assert_eq!(chains[1][..original[1].len()], original[1][..]);
    assert!(holds(chains[1].last().unwrap(), "New for two"));
}
