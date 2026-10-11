mod bundle;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use nc_data::{
    CampaignRuntimeState, CampaignStore, CoreGameData, PlanetIntelSnapshot, QueuedPlayerMail,
    TurnSubmission,
};
use nc_harness::{ScenarioSpec, build_scenario, save_built_scenario};

use crate::commands::maint::run_rust_maintenance;
use crate::commands::runtime::load_runtime_state_preferring_live_directory;
use crate::support::paths::resolve_repo_path;

const CAMPAIGN_MANIFEST_FILE_NAME: &str = "ec-bot-campaign.kdl";
const DOCTRINES: &[&str] = &[
    "landgrabber",
    "surveyor",
    "shipwright",
    "fortifier",
    "raider",
    "blockader",
    "invader",
    "bombardier",
    "marshal",
    "schemer",
    "zealot",
    "kingmaker",
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct CampaignManifest {
    game_id: String,
    scenario_path: PathBuf,
    campaign_dir: PathBuf,
    workspace_root: PathBuf,
    bundle_profile: BundleProfile,
    initial_year: u16,
    last_completed_turn: u16,
    players: Vec<PlayerAssignment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlayerAssignment {
    record_index_1_based: usize,
    doctrine: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TurnState {
    Waiting,
    Ready,
    Claimed,
    Submitted,
    Validated,
    Rejected,
    Applied,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlayerTurnStatus {
    player_record_index_1_based: usize,
    turn_index_1_based: u16,
    year: u16,
    doctrine: String,
    state: TurnState,
    bundle_dir_name: String,
    turn_file_name: String,
    notes_file_name: String,
    error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BundleProfile {
    Human,
    Llm,
}

impl BundleProfile {
    fn as_str(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Llm => "llm",
        }
    }

    fn parse(value: &str) -> Result<Self, Box<dyn std::error::Error>> {
        match value {
            "human" => Ok(Self::Human),
            "llm" => Ok(Self::Llm),
            _ => Err(format!("unknown bundle profile: {value}").into()),
        }
    }
}

impl TurnState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Waiting => "waiting",
            Self::Ready => "ready",
            Self::Claimed => "claimed",
            Self::Submitted => "submitted",
            Self::Validated => "validated",
            Self::Rejected => "rejected",
            Self::Applied => "applied",
            Self::Superseded => "superseded",
        }
    }

    fn parse(value: &str) -> Result<Self, Box<dyn std::error::Error>> {
        match value {
            "waiting" => Ok(Self::Waiting),
            "ready" => Ok(Self::Ready),
            "claimed" => Ok(Self::Claimed),
            "submitted" => Ok(Self::Submitted),
            "validated" => Ok(Self::Validated),
            "rejected" => Ok(Self::Rejected),
            "applied" => Ok(Self::Applied),
            "superseded" => Ok(Self::Superseded),
            _ => Err(format!("unknown turn status state: {value}").into()),
        }
    }
}

pub(crate) fn run_init_campaign_args(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let parsed = parse_init_or_play_args(args, false)?;
    let manifest = initialize_campaign(
        &parsed.file,
        &parsed.dir,
        &parsed.game_id,
        parsed
            .requested_bundle_profile
            .unwrap_or(BundleProfile::Human),
        parsed.export_classic,
    )?;
    let year = current_campaign_year(&manifest.campaign_dir)?;
    let turn = turn_index_for_year(manifest.initial_year, year)?;
    println!("Initialized bot campaign.");
    println!("  game_id={}", manifest.game_id);
    println!("  campaign_dir={}", manifest.campaign_dir.display());
    println!("  workspace_root={}", manifest.workspace_root.display());
    println!("  bundle_profile={}", manifest.bundle_profile.as_str());
    println!("  open_turn={turn}");
    println!("  year={year}");
    Ok(())
}

pub(crate) fn run_open_turn_args(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let dir = parse_dir_only_args(args, "harness open-turn requires --dir <campaign_dir>")?;
    let manifest = load_manifest_for_campaign_dir(&dir)?;
    let report = open_turn_internal(&manifest)?;
    print_open_turn_report(&manifest, &report);
    Ok(())
}

pub(crate) fn run_claim_turn_args(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let (dir, player) = parse_dir_player_args(
        args,
        "harness claim-turn requires --dir <campaign_dir> --player <record>",
    )?;
    let manifest = load_manifest_for_campaign_dir(&dir)?;
    let report = claim_turn_internal(&manifest, player)?;
    print_claim_turn_report(&manifest, &report);
    Ok(())
}

pub(crate) fn run_scan_turn_args(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let dir = parse_dir_only_args(args, "harness scan-turn requires --dir <campaign_dir>")?;
    let manifest = load_manifest_for_campaign_dir(&dir)?;
    let report = scan_turn_internal(&manifest)?;
    print_scan_turn_report(&manifest, &report);
    Ok(())
}

pub(crate) fn run_apply_turn_batch_args(
    args: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = parse_dir_only_args(
        args,
        "harness apply-turn-batch requires --dir <campaign_dir>",
    )?;
    let manifest = load_manifest_for_campaign_dir(&dir)?;
    let report = apply_turn_batch_internal(&manifest)?;
    print_apply_turn_report(&manifest, &report);
    Ok(())
}

pub(crate) fn run_play_until_args(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let parsed = parse_init_or_play_args(args, true)?;
    let mut manifest = if campaign_manifest_path_in_dir(&parsed.dir).exists() {
        let manifest = load_manifest_for_campaign_dir(&parsed.dir)?;
        if manifest.game_id != parsed.game_id {
            return Err(format!(
                "campaign game_id mismatch: manifest has {}, CLI requested {}",
                manifest.game_id, parsed.game_id
            )
            .into());
        }
        if let Some(requested_bundle_profile) = parsed.requested_bundle_profile {
            if manifest.bundle_profile != requested_bundle_profile {
                return Err(format!(
                    "campaign bundle profile mismatch: manifest has {}, CLI requested {}",
                    manifest.bundle_profile.as_str(),
                    requested_bundle_profile.as_str()
                )
                .into());
            }
        }
        manifest
    } else {
        initialize_campaign(
            &parsed.file,
            &parsed.dir,
            &parsed.game_id,
            parsed
                .requested_bundle_profile
                .unwrap_or(BundleProfile::Human),
            parsed.export_classic,
        )?
    };

    loop {
        let current_year = current_campaign_year(&manifest.campaign_dir)?;
        let current_turn = turn_index_for_year(manifest.initial_year, current_year)?;
        if current_turn >= parsed.target_turn.unwrap_or(current_turn) {
            let report = open_turn_internal(&manifest)?;
            println!(
                "Campaign is ready for inspection at turn {} (year {}).",
                current_turn, current_year
            );
            print_open_turn_report(&manifest, &report);
            return Ok(());
        }

        open_turn_internal(&manifest)?;
        let scan = scan_turn_internal(&manifest)?;
        if !scan.blocking_players.is_empty() {
            println!(
                "Campaign blocked before turn {}. Waiting on players: {}",
                current_turn,
                join_usize_list(&scan.blocking_players)
            );
            print_scan_turn_report(&manifest, &scan);
            return Ok(());
        }

        let applied = apply_turn_batch_internal(&manifest)?;
        print_apply_turn_report(&manifest, &applied);
        manifest = load_manifest_for_campaign_dir(&parsed.dir)?;
    }
}

#[derive(Debug)]
struct ParsedHarnessCampaignArgs {
    file: PathBuf,
    dir: PathBuf,
    game_id: String,
    export_classic: bool,
    target_turn: Option<u16>,
    requested_bundle_profile: Option<BundleProfile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OpenTurnReport {
    turn_index_1_based: u16,
    year: u16,
    players: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ClaimTurnReport {
    turn_index_1_based: u16,
    year: u16,
    player: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScanTurnReport {
    turn_index_1_based: u16,
    year: u16,
    ready_players: Vec<usize>,
    claimed_players: Vec<usize>,
    submitted_players: Vec<usize>,
    validated_players: Vec<usize>,
    rejected_players: Vec<usize>,
    blocking_players: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ApplyTurnBatchReport {
    applied_turn_index_1_based: u16,
    applied_year: u16,
    next_turn_index_1_based: u16,
    next_year: u16,
    players: Vec<usize>,
}

fn initialize_campaign(
    scenario_path: &Path,
    campaign_dir: &Path,
    game_id: &str,
    bundle_profile: BundleProfile,
    export_classic: bool,
) -> Result<CampaignManifest, Box<dyn std::error::Error>> {
    let spec = ScenarioSpec::load_kdl(scenario_path)?;
    let built = build_scenario(&spec)?;
    save_built_scenario(&built, campaign_dir, export_classic).map_err(|err| {
        format!(
            "unable to save built campaign into {}: {err}",
            campaign_dir.display()
        )
    })?;

    let players = default_player_assignments(&built.game_data, spec.metadata.seed, game_id);
    let manifest = CampaignManifest {
        game_id: game_id.to_string(),
        scenario_path: scenario_path.to_path_buf(),
        campaign_dir: campaign_dir.to_path_buf(),
        workspace_root: default_workspace_root(game_id),
        bundle_profile,
        initial_year: built.game_data.conquest.game_year,
        last_completed_turn: 0,
        players,
    };
    save_manifest(&manifest)?;
    open_turn_internal(&manifest)?;
    Ok(manifest)
}

fn open_turn_internal(
    manifest: &CampaignManifest,
) -> Result<OpenTurnReport, Box<dyn std::error::Error>> {
    let state = load_campaign_runtime_state(&manifest.campaign_dir)?;
    let year = state.game_data.conquest.game_year;
    let turn = turn_index_for_year(manifest.initial_year, year)?;

    fs::create_dir_all(manifest.workspace_root.join("campaign")).map_err(|err| {
        format!(
            "unable to create campaign workspace dir {}: {err}",
            manifest.workspace_root.join("campaign").display()
        )
    })?;
    for assignment in &manifest.players {
        let mut status = load_status_if_present(manifest, assignment.record_index_1_based, turn)?
            .unwrap_or_else(|| default_status(assignment, turn, year));
        status.year = year;
        status.turn_index_1_based = turn;
        status.doctrine = assignment.doctrine.clone();
        if matches!(status.state, TurnState::Waiting | TurnState::Superseded) {
            status.state = TurnState::Ready;
            status.error = None;
        }
        save_status(manifest, &status)?;
        bundle::ensure_player_bundle(manifest, &state, assignment, Some(&status), turn, year)?;
    }

    Ok(OpenTurnReport {
        turn_index_1_based: turn,
        year,
        players: manifest
            .players
            .iter()
            .map(|assignment| assignment.record_index_1_based)
            .collect(),
    })
}

fn claim_turn_internal(
    manifest: &CampaignManifest,
    player_record_index_1_based: usize,
) -> Result<ClaimTurnReport, Box<dyn std::error::Error>> {
    let state = load_campaign_runtime_state(&manifest.campaign_dir)?;
    let year = state.game_data.conquest.game_year;
    let turn = turn_index_for_year(manifest.initial_year, year)?;
    let assignment = manifest
        .players
        .iter()
        .find(|assignment| assignment.record_index_1_based == player_record_index_1_based)
        .ok_or_else(|| {
            format!("player {player_record_index_1_based} is not active in this campaign")
        })?;

    let mut status = load_status_if_present(manifest, player_record_index_1_based, turn)?
        .unwrap_or_else(|| default_status(assignment, turn, year));
    status.year = year;
    status.turn_index_1_based = turn;
    status.doctrine = assignment.doctrine.clone();
    match status.state {
        TurnState::Applied | TurnState::Superseded => {
            return Err(format!(
                "player {} turn {} is already closed ({})",
                player_record_index_1_based,
                turn,
                status.state.as_str()
            )
            .into());
        }
        TurnState::Validated => {
            return Err(format!(
                "player {} turn {} is already validated",
                player_record_index_1_based, turn
            )
            .into());
        }
        TurnState::Ready
        | TurnState::Claimed
        | TurnState::Rejected
        | TurnState::Submitted
        | TurnState::Waiting => {}
    }
    status.state = TurnState::Claimed;
    status.error = None;
    save_status(manifest, &status)?;
    bundle::ensure_player_bundle(manifest, &state, assignment, Some(&status), turn, year)?;

    Ok(ClaimTurnReport {
        turn_index_1_based: turn,
        year,
        player: player_record_index_1_based,
    })
}

fn scan_turn_internal(
    manifest: &CampaignManifest,
) -> Result<ScanTurnReport, Box<dyn std::error::Error>> {
    let state = load_campaign_runtime_state(&manifest.campaign_dir)?;
    let year = state.game_data.conquest.game_year;
    let turn = turn_index_for_year(manifest.initial_year, year)?;
    let mut report = ScanTurnReport {
        turn_index_1_based: turn,
        year,
        ready_players: Vec::new(),
        claimed_players: Vec::new(),
        submitted_players: Vec::new(),
        validated_players: Vec::new(),
        rejected_players: Vec::new(),
        blocking_players: Vec::new(),
    };

    for assignment in &manifest.players {
        let mut status = load_status_if_present(manifest, assignment.record_index_1_based, turn)?
            .unwrap_or_else(|| default_status(assignment, turn, year));
        let turn_path = player_turn_path(manifest, assignment.record_index_1_based, turn);
        if turn_path.exists()
            && !matches!(
                status.state,
                TurnState::Validated | TurnState::Applied | TurnState::Superseded
            )
        {
            status.state = TurnState::Submitted;
        }

        if turn_path.exists() {
            match validate_turn_file(
                &state.game_data,
                &state.queued_mail,
                assignment.record_index_1_based,
                &turn_path,
            ) {
                Ok(()) => {
                    status.state = TurnState::Validated;
                    status.error = None;
                }
                Err(err) => {
                    status.state = TurnState::Rejected;
                    status.error = Some(err.to_string());
                }
            }
        }

        classify_status(&status, &mut report);
        save_status(manifest, &status)?;
        bundle::ensure_player_bundle(manifest, &state, assignment, Some(&status), turn, year)?;
    }

    Ok(report)
}

fn apply_turn_batch_internal(
    manifest: &CampaignManifest,
) -> Result<ApplyTurnBatchReport, Box<dyn std::error::Error>> {
    let state = load_campaign_runtime_state(&manifest.campaign_dir)?;
    let year = state.game_data.conquest.game_year;
    let turn = turn_index_for_year(manifest.initial_year, year)?;

    let mut statuses = Vec::new();
    for assignment in &manifest.players {
        let status = load_status_if_present(manifest, assignment.record_index_1_based, turn)?
            .ok_or_else(|| {
                format!(
                    "missing turn status for player {} turn {}",
                    assignment.record_index_1_based, turn
                )
            })?;
        if status.state != TurnState::Validated {
            return Err(format!(
                "cannot apply turn batch: player {} is {}",
                assignment.record_index_1_based,
                status.state.as_str()
            )
            .into());
        }
        statuses.push(status);
    }

    let store = CampaignStore::open_default_in_dir(&manifest.campaign_dir)?;
    let mut runtime = load_runtime_state_preferring_live_directory(&manifest.campaign_dir, &store)?;

    for assignment in &manifest.players {
        let turn_path = player_turn_path(manifest, assignment.record_index_1_based, turn);
        let submission = TurnSubmission::load_kdl(&turn_path)?;
        if submission.player_record_index_1_based != assignment.record_index_1_based {
            return Err(format!(
                "turn file {} declares player {}, expected {}",
                turn_path.display(),
                submission.player_record_index_1_based,
                assignment.record_index_1_based
            )
            .into());
        }
        submission.apply_to(&mut runtime.game_data, &mut runtime.queued_mail)?;
    }

    store.save_runtime_state_structured(
        runtime.game_year,
        &runtime.game_data,
        &runtime.planet_scorch_orders,
        &runtime.report_block_rows,
        &runtime.queued_mail,
    )?;
    run_rust_maintenance(&manifest.campaign_dir, 1)?;

    for mut status in statuses {
        status.state = TurnState::Applied;
        status.error = None;
        save_status(manifest, &status)?;
    }

    let mut updated = load_manifest_for_campaign_dir(&manifest.campaign_dir)?;
    updated.last_completed_turn = turn;
    save_manifest(&updated)?;

    let next_year = current_campaign_year(&manifest.campaign_dir)?;
    let next_turn = turn_index_for_year(updated.initial_year, next_year)?;
    open_turn_internal(&updated)?;

    Ok(ApplyTurnBatchReport {
        applied_turn_index_1_based: turn,
        applied_year: year,
        next_turn_index_1_based: next_turn,
        next_year,
        players: updated
            .players
            .iter()
            .map(|assignment| assignment.record_index_1_based)
            .collect(),
    })
}

fn classify_status(status: &PlayerTurnStatus, report: &mut ScanTurnReport) {
    match status.state {
        TurnState::Ready | TurnState::Waiting => {
            report
                .ready_players
                .push(status.player_record_index_1_based);
            report
                .blocking_players
                .push(status.player_record_index_1_based);
        }
        TurnState::Claimed => {
            report
                .claimed_players
                .push(status.player_record_index_1_based);
            report
                .blocking_players
                .push(status.player_record_index_1_based);
        }
        TurnState::Submitted => {
            report
                .submitted_players
                .push(status.player_record_index_1_based);
            report
                .blocking_players
                .push(status.player_record_index_1_based);
        }
        TurnState::Validated => {
            report
                .validated_players
                .push(status.player_record_index_1_based);
        }
        TurnState::Rejected => {
            report
                .rejected_players
                .push(status.player_record_index_1_based);
            report
                .blocking_players
                .push(status.player_record_index_1_based);
        }
        TurnState::Applied | TurnState::Superseded => {}
    }
}

fn load_snapshots_for_viewer(
    campaign_dir: &Path,
    viewer: usize,
) -> Result<BTreeMap<usize, PlanetIntelSnapshot>, Box<dyn std::error::Error>> {
    let store = CampaignStore::open_default_in_dir(campaign_dir)?;
    Ok(store
        .latest_planet_intel_for_viewer(viewer as u8)?
        .into_iter()
        .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
        .collect())
}

fn validate_turn_file(
    game_data: &CoreGameData,
    queued_mail: &[QueuedPlayerMail],
    player_record_index_1_based: usize,
    turn_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let submission = TurnSubmission::load_kdl(turn_path)?;
    if submission.player_record_index_1_based != player_record_index_1_based {
        return Err(format!(
            "turn file declares player {}, expected {}",
            submission.player_record_index_1_based, player_record_index_1_based
        )
        .into());
    }
    let mut preview_game_data = game_data.clone();
    let mut preview_queue = queued_mail.to_vec();
    submission.apply_to(&mut preview_game_data, &mut preview_queue)?;
    Ok(())
}

fn load_campaign_runtime_state(
    campaign_dir: &Path,
) -> Result<CampaignRuntimeState, Box<dyn std::error::Error>> {
    let store = CampaignStore::open_default_in_dir(campaign_dir)?;
    load_runtime_state_preferring_live_directory(campaign_dir, &store)
}

fn current_campaign_year(campaign_dir: &Path) -> Result<u16, Box<dyn std::error::Error>> {
    Ok(load_campaign_runtime_state(campaign_dir)?
        .game_data
        .conquest
        .game_year)
}

fn default_player_assignments(
    game_data: &CoreGameData,
    scenario_seed: u64,
    game_id: &str,
) -> Vec<PlayerAssignment> {
    let mut players = game_data
        .players
        .iter()
        .enumerate()
        .filter(|(_, record)| {
            record.mode.code() != 0
                || !record.handle.trim().to_string().is_empty()
                || !record.empire_name.trim().to_string().is_empty()
        })
        .map(|(idx, _)| idx + 1)
        .collect::<Vec<_>>();
    if players.is_empty() {
        players.extend(1..=game_data.conquest.player_count as usize);
    }
    let doctrines = shuffled_doctrines(scenario_seed, game_id);
    players
        .into_iter()
        .enumerate()
        .map(|(idx, player)| PlayerAssignment {
            record_index_1_based: player,
            doctrine: doctrines[idx % doctrines.len()].to_string(),
        })
        .collect()
}

fn shuffled_doctrines(scenario_seed: u64, game_id: &str) -> Vec<&'static str> {
    let mut doctrines = DOCTRINES.to_vec();
    let mut state = mixed_doctrine_seed(scenario_seed, game_id);
    for idx in (1..doctrines.len()).rev() {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let swap_idx = ((state >> 32) as usize) % (idx + 1);
        doctrines.swap(idx, swap_idx);
    }
    doctrines
}

fn mixed_doctrine_seed(scenario_seed: u64, game_id: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in game_id.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    let mixed = scenario_seed.rotate_left(19) ^ hash ^ 0xEC15_0000_0000_0012;
    if mixed == 0 {
        0x9E37_79B9_7F4A_7C15
    } else {
        mixed
    }
}

fn default_workspace_root(game_id: &str) -> PathBuf {
    std::env::temp_dir().join("ec-llm-turns").join(game_id)
}

fn campaign_manifest_path_in_dir(campaign_dir: &Path) -> PathBuf {
    campaign_dir.join(CAMPAIGN_MANIFEST_FILE_NAME)
}

fn workspace_manifest_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join("campaign").join("manifest.kdl")
}

fn player_workspace_dir(manifest: &CampaignManifest, player: usize) -> PathBuf {
    manifest.workspace_root.join(format!("player-{player}"))
}

fn player_bundle_dir(manifest: &CampaignManifest, player: usize, turn: u16) -> PathBuf {
    player_workspace_dir(manifest, player).join(format!("bundle-turn-{turn:04}"))
}

fn player_status_path(manifest: &CampaignManifest, player: usize, turn: u16) -> PathBuf {
    player_workspace_dir(manifest, player).join(format!("status-turn-{turn:04}.kdl"))
}

fn player_turn_path(manifest: &CampaignManifest, player: usize, turn: u16) -> PathBuf {
    player_workspace_dir(manifest, player).join(format!("turn-{turn:04}.kdl"))
}

fn player_notes_path(manifest: &CampaignManifest, player: usize, turn: u16) -> PathBuf {
    player_workspace_dir(manifest, player).join(format!("notes-{turn:04}.md"))
}

fn default_status(
    assignment: &PlayerAssignment,
    turn_index_1_based: u16,
    year: u16,
) -> PlayerTurnStatus {
    PlayerTurnStatus {
        player_record_index_1_based: assignment.record_index_1_based,
        turn_index_1_based,
        year,
        doctrine: assignment.doctrine.clone(),
        state: TurnState::Ready,
        bundle_dir_name: format!("bundle-turn-{turn_index_1_based:04}"),
        turn_file_name: format!("turn-{turn_index_1_based:04}.kdl"),
        notes_file_name: format!("notes-{turn_index_1_based:04}.md"),
        error: None,
    }
}

fn save_manifest(manifest: &CampaignManifest) -> Result<(), Box<dyn std::error::Error>> {
    let text = render_manifest(manifest);
    fs::create_dir_all(manifest.workspace_root.join("campaign")).map_err(|err| {
        format!(
            "unable to create workspace dir {}: {err}",
            manifest.workspace_root.join("campaign").display()
        )
    })?;
    fs::write(campaign_manifest_path_in_dir(&manifest.campaign_dir), &text).map_err(|err| {
        format!(
            "unable to write campaign manifest {}: {err}",
            campaign_manifest_path_in_dir(&manifest.campaign_dir).display()
        )
    })?;
    fs::write(workspace_manifest_path(&manifest.workspace_root), text).map_err(|err| {
        format!(
            "unable to write workspace manifest {}: {err}",
            workspace_manifest_path(&manifest.workspace_root).display()
        )
    })?;
    Ok(())
}

fn load_manifest_for_campaign_dir(
    campaign_dir: &Path,
) -> Result<CampaignManifest, Box<dyn std::error::Error>> {
    parse_manifest_file(&campaign_manifest_path_in_dir(campaign_dir))
}

fn load_status_if_present(
    manifest: &CampaignManifest,
    player: usize,
    turn: u16,
) -> Result<Option<PlayerTurnStatus>, Box<dyn std::error::Error>> {
    let path = player_status_path(manifest, player, turn);
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(parse_status_file(&path)?))
}

fn save_status(
    manifest: &CampaignManifest,
    status: &PlayerTurnStatus,
) -> Result<(), Box<dyn std::error::Error>> {
    let player_dir = player_workspace_dir(manifest, status.player_record_index_1_based);
    fs::create_dir_all(&player_dir).map_err(|err| {
        format!(
            "unable to create player workspace dir {}: {err}",
            player_dir.display()
        )
    })?;
    let status_path = player_status_path(
        manifest,
        status.player_record_index_1_based,
        status.turn_index_1_based,
    );
    fs::write(&status_path, render_status(status)).map_err(|err| {
        format!(
            "unable to write status file {}: {err}",
            status_path.display()
        )
    })?;
    Ok(())
}

fn render_manifest(manifest: &CampaignManifest) -> String {
    let mut out = format!(
        "campaign-play game_id=\"{}\" scenario=\"{}\" campaign_dir=\"{}\" workspace_root=\"{}\" bundle_profile=\"{}\" initial_year={} last_completed_turn={}\n",
        kdl_escape(&manifest.game_id),
        kdl_escape(&manifest.scenario_path.display().to_string()),
        kdl_escape(&manifest.campaign_dir.display().to_string()),
        kdl_escape(&manifest.workspace_root.display().to_string()),
        manifest.bundle_profile.as_str(),
        manifest.initial_year,
        manifest.last_completed_turn
    );
    for player in &manifest.players {
        out.push_str(&format!(
            "player record={} doctrine=\"{}\"\n",
            player.record_index_1_based,
            kdl_escape(&player.doctrine)
        ));
    }
    out
}

fn render_status(status: &PlayerTurnStatus) -> String {
    format!(
        "turn-status player={} turn={} year={} state=\"{}\" doctrine=\"{}\" bundle_dir=\"{}\" turn_file=\"{}\" notes_file=\"{}\" error=\"{}\"\n",
        status.player_record_index_1_based,
        status.turn_index_1_based,
        status.year,
        status.state.as_str(),
        kdl_escape(&status.doctrine),
        kdl_escape(&status.bundle_dir_name),
        kdl_escape(&status.turn_file_name),
        kdl_escape(&status.notes_file_name),
        kdl_escape(status.error.as_deref().unwrap_or(""))
    )
}

fn parse_manifest_file(path: &Path) -> Result<CampaignManifest, Box<dyn std::error::Error>> {
    let document: kdl::KdlDocument = fs::read_to_string(path)?
        .parse()
        .map_err(|err| format!("invalid KDL in {}: {err}", path.display()))?;
    let root = document
        .get("campaign-play")
        .ok_or_else(|| format!("missing campaign-play node in {}", path.display()))?;
    let mut players = Vec::new();
    for node in document.nodes() {
        if node.name().value() == "player" {
            players.push(PlayerAssignment {
                record_index_1_based: prop_usize_1_based(node, "record")?,
                doctrine: prop_string(node, "doctrine")?,
            });
        }
    }
    Ok(CampaignManifest {
        game_id: prop_string(root, "game_id")?,
        scenario_path: PathBuf::from(prop_string(root, "scenario")?),
        campaign_dir: PathBuf::from(prop_string(root, "campaign_dir")?),
        workspace_root: PathBuf::from(prop_string(root, "workspace_root")?),
        bundle_profile: opt_prop_string(root, "bundle_profile")?
            .map(|value| BundleProfile::parse(&value))
            .transpose()?
            .unwrap_or(BundleProfile::Human),
        initial_year: prop_u16(root, "initial_year")?,
        last_completed_turn: prop_u16(root, "last_completed_turn")?,
        players,
    })
}

fn parse_status_file(path: &Path) -> Result<PlayerTurnStatus, Box<dyn std::error::Error>> {
    let document: kdl::KdlDocument = fs::read_to_string(path)?
        .parse()
        .map_err(|err| format!("invalid KDL in {}: {err}", path.display()))?;
    let root = document
        .get("turn-status")
        .ok_or_else(|| format!("missing turn-status node in {}", path.display()))?;
    Ok(PlayerTurnStatus {
        player_record_index_1_based: prop_usize_1_based(root, "player")?,
        turn_index_1_based: prop_u16(root, "turn")?,
        year: prop_u16(root, "year")?,
        doctrine: prop_string(root, "doctrine")?,
        state: TurnState::parse(&prop_string(root, "state")?)?,
        bundle_dir_name: prop_string(root, "bundle_dir")?,
        turn_file_name: prop_string(root, "turn_file")?,
        notes_file_name: prop_string(root, "notes_file")?,
        error: opt_prop_string(root, "error")?.filter(|value| !value.is_empty()),
    })
}

fn parse_init_or_play_args(
    args: Vec<String>,
    require_turn: bool,
) -> Result<ParsedHarnessCampaignArgs, Box<dyn std::error::Error>> {
    let mut file = None;
    let mut dir = None;
    let mut game_id = None;
    let mut export_classic = false;
    let mut target_turn = None;
    let mut requested_bundle_profile = None;
    let mut remaining = args.into_iter();
    while let Some(arg) = remaining.next() {
        match arg.as_str() {
            "--file" => {
                let Some(value) = remaining.next() else {
                    return Err("missing path after --file".into());
                };
                file = Some(resolve_repo_path(&value));
            }
            "--dir" => {
                let Some(value) = remaining.next() else {
                    return Err("missing path after --dir".into());
                };
                dir = Some(resolve_repo_path(&value));
            }
            "--game-id" => {
                let Some(value) = remaining.next() else {
                    return Err("missing value after --game-id".into());
                };
                game_id = Some(value);
            }
            "--turn" => {
                let Some(value) = remaining.next() else {
                    return Err("missing value after --turn".into());
                };
                target_turn = Some(value.parse::<u16>()?);
            }
            "--export-classic" => export_classic = true,
            "--bundle-profile" => {
                let Some(value) = remaining.next() else {
                    return Err("missing value after --bundle-profile".into());
                };
                requested_bundle_profile = Some(BundleProfile::parse(&value)?);
            }
            other => return Err(format!("unknown harness campaign argument: {other}").into()),
        }
    }

    let Some(dir) = dir else {
        return Err("campaign command requires --dir <campaign_dir>".into());
    };
    let Some(game_id) = game_id else {
        return Err("campaign command requires --game-id <id>".into());
    };
    let file = file.unwrap_or_else(|| dir.join("scenario.kdl"));
    if require_turn && target_turn.is_none() {
        return Err("play-until requires --turn <n>".into());
    }
    Ok(ParsedHarnessCampaignArgs {
        file,
        dir,
        game_id,
        export_classic,
        target_turn,
        requested_bundle_profile,
    })
}

fn parse_dir_only_args(
    args: Vec<String>,
    missing_message: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut dir = None;
    let mut remaining = args.into_iter();
    while let Some(arg) = remaining.next() {
        match arg.as_str() {
            "--dir" => {
                let Some(value) = remaining.next() else {
                    return Err("missing path after --dir".into());
                };
                dir = Some(resolve_repo_path(&value));
            }
            other => return Err(format!("unknown harness campaign argument: {other}").into()),
        }
    }
    dir.ok_or_else(|| missing_message.into())
}

fn parse_dir_player_args(
    args: Vec<String>,
    missing_message: &str,
) -> Result<(PathBuf, usize), Box<dyn std::error::Error>> {
    let mut dir = None;
    let mut player = None;
    let mut remaining = args.into_iter();
    while let Some(arg) = remaining.next() {
        match arg.as_str() {
            "--dir" => {
                let Some(value) = remaining.next() else {
                    return Err("missing path after --dir".into());
                };
                dir = Some(resolve_repo_path(&value));
            }
            "--player" => {
                let Some(value) = remaining.next() else {
                    return Err("missing value after --player".into());
                };
                player = Some(value.parse::<usize>()?);
            }
            other => return Err(format!("unknown harness campaign argument: {other}").into()),
        }
    }
    let Some(dir) = dir else {
        return Err(missing_message.into());
    };
    let Some(player) = player else {
        return Err(missing_message.into());
    };
    if player == 0 {
        return Err("player record must be 1-based".into());
    }
    Ok((dir, player))
}

fn turn_index_for_year(
    initial_year: u16,
    current_year: u16,
) -> Result<u16, Box<dyn std::error::Error>> {
    if current_year < initial_year {
        return Err(format!(
            "campaign year went backwards: initial year {initial_year}, current year {current_year}"
        )
        .into());
    }
    Ok(current_year - initial_year + 1)
}

fn print_open_turn_report(manifest: &CampaignManifest, report: &OpenTurnReport) {
    println!("Opened campaign turn.");
    println!("  game_id={}", manifest.game_id);
    println!("  turn={}", report.turn_index_1_based);
    println!("  year={}", report.year);
    println!("  players={}", join_usize_list(&report.players));
    println!("  workspace_root={}", manifest.workspace_root.display());
}

fn print_claim_turn_report(manifest: &CampaignManifest, report: &ClaimTurnReport) {
    println!("Claimed campaign turn.");
    println!("  game_id={}", manifest.game_id);
    println!("  player={}", report.player);
    println!("  turn={}", report.turn_index_1_based);
    println!("  year={}", report.year);
}

fn print_scan_turn_report(manifest: &CampaignManifest, report: &ScanTurnReport) {
    println!("Scanned campaign turn.");
    println!("  game_id={}", manifest.game_id);
    println!("  turn={}", report.turn_index_1_based);
    println!("  year={}", report.year);
    println!("  ready={}", join_usize_list(&report.ready_players));
    println!("  claimed={}", join_usize_list(&report.claimed_players));
    println!("  submitted={}", join_usize_list(&report.submitted_players));
    println!("  validated={}", join_usize_list(&report.validated_players));
    println!("  rejected={}", join_usize_list(&report.rejected_players));
    println!("  blocking={}", join_usize_list(&report.blocking_players));
}

fn print_apply_turn_report(manifest: &CampaignManifest, report: &ApplyTurnBatchReport) {
    println!("Applied campaign turn batch.");
    println!("  game_id={}", manifest.game_id);
    println!("  applied_turn={}", report.applied_turn_index_1_based);
    println!("  applied_year={}", report.applied_year);
    println!("  next_turn={}", report.next_turn_index_1_based);
    println!("  next_year={}", report.next_year);
    println!("  players={}", join_usize_list(&report.players));
}

fn join_usize_list(values: &[usize]) -> String {
    if values.is_empty() {
        "-".to_string()
    } else {
        values
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(",")
    }
}

fn kdl_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn prop_string(node: &kdl::KdlNode, name: &str) -> Result<String, Box<dyn std::error::Error>> {
    node.get(name)
        .and_then(|value| value.as_string())
        .map(str::to_string)
        .ok_or_else(|| format!("missing or invalid string property: {name}").into())
}

fn opt_prop_string(
    node: &kdl::KdlNode,
    name: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    Ok(node
        .get(name)
        .and_then(|value| value.as_string())
        .map(str::to_string))
}

fn prop_u16(node: &kdl::KdlNode, name: &str) -> Result<u16, Box<dyn std::error::Error>> {
    let value = node
        .get(name)
        .and_then(|value| value.as_integer())
        .ok_or_else(|| format!("missing or invalid integer property: {name}"))?;
    Ok(u16::try_from(value).map_err(|_| format!("property {name} out of u16 range: {value}"))?)
}

fn prop_usize_1_based(
    node: &kdl::KdlNode,
    name: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let value = node
        .get(name)
        .and_then(|value| value.as_integer())
        .ok_or_else(|| format!("missing or invalid integer property: {name}"))?;
    let converted = usize::try_from(value)
        .map_err(|_| format!("property {name} out of usize range: {value}"))?;
    if converted == 0 {
        return Err(format!("{name} must be 1-based").into());
    }
    Ok(converted)
}
