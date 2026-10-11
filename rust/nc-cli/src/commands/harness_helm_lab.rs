use std::fs;
use std::path::{Path, PathBuf};

use nc_data::{CampaignSettings, CampaignStore, set_player_handle};
use nc_engine::{build_seeded_new_game, map_size_for_player_count};

use crate::commands::harness_player1_tui_stress::seed_player1_tui_stress;
use crate::commands::runtime::with_runtime_game_mut;
use crate::support::paths::resolve_repo_path;

const DEFAULT_ROOT: &str = "/tmp/nc-helm-lab";
const DEFAULT_SEED_BASE: u64 = 1515;
const MAP_SIZE_PLAYER_COUNTS: [u8; 4] = [4, 9, 16, 25];
const EMPIRE_NAMES: [&str; 25] = [
    "Aurora League",
    "Red Horizon Pact",
    "Vela Syndicate",
    "Helios Crown",
    "Iron Meridian",
    "Cinder Accord",
    "Northwatch Combine",
    "Lantern Union",
    "Anvil Directorate",
    "Quiet Current",
    "Signal Compact",
    "Pillar Republic",
    "Thornhold Marches",
    "Garnet Hegemony",
    "Silver Wake",
    "Ashen Reach",
    "Cobalt Banner",
    "Harbor Concord",
    "Winter Coil",
    "Saffron Gate",
    "Obsidian Relay",
    "Brass Regency",
    "Solar Frontier",
    "Grey Dominion",
    "Verdant Assembly",
];

#[derive(Debug, Clone, Copy)]
struct HelmLabProfile {
    slug: &'static str,
    player_count: u8,
}

impl HelmLabProfile {
    fn map_size(self) -> u8 {
        map_size_for_player_count(self.player_count)
    }

    fn game_name(self) -> String {
        format!(
            "NC Helm Lab {}x{} ({} players)",
            self.map_size(),
            self.map_size(),
            self.player_count
        )
    }
}

#[derive(Debug, Clone)]
struct HelmLabCampaignReport {
    dir: PathBuf,
    player_count: u8,
    map_size: u8,
    seed: u64,
    planets: usize,
    fleets: usize,
    report_blocks: usize,
    player1_mail: usize,
    player1_full_intel: usize,
    player1_partial_intel: usize,
    commissioned_starbases: usize,
}

pub(crate) fn run_seed_nc_helm_lab_args(
    args: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let parsed = parse_args(args)?;
    let reports = seed_nc_helm_lab(&parsed.root, parsed.seed_base)?;
    write_manifest(&parsed.root, &reports)?;

    println!("Seeded nc-helm lab at {}.", parsed.root.display());
    for report in reports {
        println!(
            "  {}: players={} map={}x{} seed={} planets={} fleets={} reports={} mail={} full_intel={} partial_intel={} starbases={}",
            report
                .dir
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("campaign"),
            report.player_count,
            report.map_size,
            report.map_size,
            report.seed,
            report.planets,
            report.fleets,
            report.report_blocks,
            report.player1_mail,
            report.player1_full_intel,
            report.player1_partial_intel,
            report.commissioned_starbases
        );
        println!(
            "    cargo run -q -p nc-helm -- --dir {}",
            report.dir.display()
        );
    }
    println!("  manifest={}", parsed.root.join("README.txt").display());
    Ok(())
}

#[derive(Debug, Clone)]
struct ParsedArgs {
    root: PathBuf,
    seed_base: u64,
}

fn parse_args(args: Vec<String>) -> Result<ParsedArgs, Box<dyn std::error::Error>> {
    let mut root = PathBuf::from(DEFAULT_ROOT);
    let mut seed_base = DEFAULT_SEED_BASE;
    let mut remaining = args.into_iter();
    while let Some(arg) = remaining.next() {
        match arg.as_str() {
            "--root" => {
                let Some(value) = remaining.next() else {
                    return Err("missing path after --root".into());
                };
                root = resolve_repo_path(&value);
            }
            "--seed-base" => {
                let Some(value) = remaining.next() else {
                    return Err("missing value after --seed-base".into());
                };
                seed_base = value.parse::<u64>()?;
            }
            other => return Err(format!("unknown harness argument: {other}").into()),
        }
    }
    Ok(ParsedArgs { root, seed_base })
}

fn seed_nc_helm_lab(
    root: &Path,
    seed_base: u64,
) -> Result<Vec<HelmLabCampaignReport>, Box<dyn std::error::Error>> {
    ensure_empty_root(root)?;

    let profiles = [
        HelmLabProfile {
            slug: "map18-p4",
            player_count: MAP_SIZE_PLAYER_COUNTS[0],
        },
        HelmLabProfile {
            slug: "map27-p9",
            player_count: MAP_SIZE_PLAYER_COUNTS[1],
        },
        HelmLabProfile {
            slug: "map36-p16",
            player_count: MAP_SIZE_PLAYER_COUNTS[2],
        },
        HelmLabProfile {
            slug: "map45-p25",
            player_count: MAP_SIZE_PLAYER_COUNTS[3],
        },
    ];

    let mut reports = Vec::with_capacity(profiles.len());
    for (idx, profile) in profiles.into_iter().enumerate() {
        let dir = root.join(profile.slug);
        let seed = seed_base + (idx as u64 * 1000) + u64::from(profile.player_count);
        reports.push(seed_one_campaign(&dir, profile, seed)?);
    }
    Ok(reports)
}

fn ensure_empty_root(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if root.exists() {
        let mut entries = fs::read_dir(root)?;
        if entries.next().is_some() {
            return Err(format!(
                "{} must not already contain files; use a fresh --root",
                root.display()
            )
            .into());
        }
    } else {
        fs::create_dir_all(root)?;
    }
    Ok(())
}

fn seed_one_campaign(
    dir: &Path,
    profile: HelmLabProfile,
    seed: u64,
) -> Result<HelmLabCampaignReport, Box<dyn std::error::Error>> {
    fs::create_dir_all(dir)?;

    let store = CampaignStore::open_default_in_dir(dir)?;
    let game_data = build_seeded_new_game(profile.player_count, 3000, seed)?;
    store.save_runtime_state_structured(game_data.conquest.game_year, &game_data, &[], &[])?;
    store.save_campaign_settings(&CampaignSettings::new(profile.slug, &profile.game_name()))?;

    join_helm_lab_empires(dir, profile.player_count)?;
    let stress = seed_player1_tui_stress(dir)?;

    Ok(HelmLabCampaignReport {
        dir: dir.to_path_buf(),
        player_count: profile.player_count,
        map_size: profile.map_size(),
        seed,
        planets: stress.planets,
        fleets: stress.fleets,
        report_blocks: stress.report_blocks,
        player1_mail: stress.player1_mail,
        player1_full_intel: stress.player1_full_intel,
        player1_partial_intel: stress.player1_partial_intel,
        commissioned_starbases: stress.commissioned_starbases,
    })
}

fn join_helm_lab_empires(dir: &Path, player_count: u8) -> Result<(), Box<dyn std::error::Error>> {
    with_runtime_game_mut(dir, |game_data| {
        for player_record_index_1_based in 1..=player_count as usize {
            let empire_name = EMPIRE_NAMES
                .get(player_record_index_1_based - 1)
                .copied()
                .unwrap_or("Frontier League");
            game_data.join_player(player_record_index_1_based, empire_name)?;
            game_data.rename_player_homeworld(
                player_record_index_1_based,
                &homeworld_name(player_record_index_1_based, empire_name),
            )?;
            let player = game_data
                .players
                .get_mut(player_record_index_1_based - 1)
                .ok_or_else(|| {
                    format!("player record index out of range: {player_record_index_1_based}")
                })?;
            set_player_handle(player, &format!("p{player_record_index_1_based:02}"));
            player.autopilot = false;
        }
        Ok(())
    })?;
    Ok(())
}

fn homeworld_name(player_record_index_1_based: usize, empire_name: &str) -> String {
    let short = empire_name
        .split_whitespace()
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or("Prime");
    format!("{short} {:02}", player_record_index_1_based)
}

fn write_manifest(
    root: &Path,
    reports: &[HelmLabCampaignReport],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut text = String::new();
    text.push_str("nc-helm lab\n");
    text.push_str(&format!("root={}\n", root.display()));
    text.push('\n');
    for report in reports {
        text.push_str(&format!(
            "{}\n  dir={}\n  players={}\n  map={}x{}\n  seed={}\n  planets={}\n  fleets={}\n  report_blocks={}\n  player1_mail={}\n  player1_full_intel={}\n  player1_partial_intel={}\n  commissioned_starbases={}\n  launch=cargo run -q -p nc-helm -- --dir {}\n\n",
            report.dir.file_name().and_then(|name| name.to_str()).unwrap_or("campaign"),
            report.dir.display(),
            report.player_count,
            report.map_size,
            report.map_size,
            report.seed,
            report.planets,
            report.fleets,
            report.report_blocks,
            report.player1_mail,
            report.player1_full_intel,
            report.player1_partial_intel,
            report.commissioned_starbases,
            report.dir.display()
        ));
    }
    fs::write(root.join("README.txt"), text)?;
    Ok(())
}
