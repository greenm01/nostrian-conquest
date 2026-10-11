use std::fs;
use std::path::{Path, PathBuf};

use nc_data::CoreGameData;

use crate::commands::bombard::{apply_bombard_scenario, validate_bombard_data};
use crate::commands::econ::{apply_econ_scenario, validate_econ_data};
use crate::commands::fleet_battle::{apply_fleet_battle_scenario, validate_fleet_battle_data};
use crate::commands::fleet_order::{apply_fleet_order_scenario, apply_move_scenario};
use crate::commands::guard_starbase::apply_guard_starbase_scenario;
use crate::commands::invade::{apply_invade_scenario, validate_invade_data};
use crate::commands::ipbm::{apply_ipbm_scenario, validate_ipbm_data};
use crate::commands::planet_build::apply_planet_build_scenario;
use crate::commands::runtime::{
    export_runtime_core_projection_in_place, export_runtime_snapshot_in_place,
    load_runtime_game_data,
};
use crate::support::paths::{display_repo_path, repo_root};
use crate::workspace::{copy_init_files, copy_pre_maint_replay_context_files};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KnownScenario {
    FleetOrder,
    PlanetBuild,
    GuardStarbase,
    Ipbm,
    Move,
    Bombard,
    FleetBattle,
    Invade,
    Econ,
}
impl KnownScenario {
    pub(crate) fn all() -> [Self; 9] {
        [
            Self::FleetOrder,
            Self::PlanetBuild,
            Self::GuardStarbase,
            Self::Ipbm,
            Self::Move,
            Self::Bombard,
            Self::FleetBattle,
            Self::Invade,
            Self::Econ,
        ]
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::FleetOrder => "fleet-order",
            Self::PlanetBuild => "planet-build",
            Self::GuardStarbase => "guard-starbase",
            Self::Ipbm => "ipbm",
            Self::Move => "move",
            Self::Bombard => "bombard",
            Self::FleetBattle => "fleet-battle",
            Self::Invade => "invade",
            Self::Econ => "econ",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "fleet-order" => Some(Self::FleetOrder),
            "planet-build" => Some(Self::PlanetBuild),
            "guard-starbase" => Some(Self::GuardStarbase),
            "ipbm" => Some(Self::Ipbm),
            "move" => Some(Self::Move),
            "bombard" => Some(Self::Bombard),
            "fleet-battle" => Some(Self::FleetBattle),
            "invade" => Some(Self::Invade),
            "econ" => Some(Self::Econ),
            _ => None,
        }
    }

    pub(crate) fn description(self) -> &'static str {
        match self {
            Self::FleetOrder => "accepted fleet movement/order fixture rooted in FLEETS.DAT",
            Self::PlanetBuild => "accepted planet build-queue fixture rooted in PLANETS.DAT",
            Self::GuardStarbase => {
                "accepted one-base guard-starbase fixture spanning PLAYER/FLEETS/BASES"
            }
            Self::Ipbm => "accepted zero-record IPBM fixture",
            Self::Move => "accepted multi-tick fleet move fixture rooted in FLEETS.DAT",
            Self::Bombard => "accepted fleet bombardment scenario targeting planet 14 at (15,13)",
            Self::FleetBattle => {
                "accepted multi-fleet battle scenario with 4 empires converging at (10,10)"
            }
            Self::Invade => {
                "accepted heavy invasion scenario: fleet at (15,13) with armies, InvadeWorld order"
            }
            Self::Econ => {
                "accepted economy scenario: fleet 3 bombards planet 14 at (15,13), CA=50 DD=50"
            }
        }
    }

    pub(crate) fn preserved_fixture_dir(self) -> PathBuf {
        let root = repo_root().join("fixtures");
        match self {
            Self::FleetOrder => root.join("ecmaint-fleet-pre/v1.5"),
            Self::PlanetBuild => root.join("ecmaint-build-pre/v1.5"),
            Self::GuardStarbase => root.join("ecmaint-starbase-pre/v1.5"),
            Self::Ipbm => root.join("ecmaint-post/v1.5"),
            Self::Move => root.join("ecmaint-move-pre/v1.5"),
            Self::Bombard => root.join("ecmaint-bombard-pre/v1.5"),
            Self::FleetBattle => root.join("ecmaint-fleet-battle-pre/v1.5"),
            Self::Invade => root.join("ecmaint-invade-pre/v1.5"),
            Self::Econ => root.join("ecmaint-econ-pre/v1.5"),
        }
    }

    pub(crate) fn exact_match_files(self) -> &'static [&'static str] {
        match self {
            Self::FleetOrder => &["FLEETS.DAT"],
            Self::PlanetBuild => &["PLANETS.DAT"],
            Self::GuardStarbase => &["PLAYER.DAT", "FLEETS.DAT", "BASES.DAT"],
            Self::Ipbm => &["PLAYER.DAT", "IPBM.DAT"],
            Self::Move => &["FLEETS.DAT"],
            Self::Bombard => &["FLEETS.DAT", "PLANETS.DAT"],
            Self::FleetBattle => &["FLEETS.DAT", "PLANETS.DAT"],
            Self::Invade => &["FLEETS.DAT", "PLANETS.DAT"],
            Self::Econ => &["FLEETS.DAT", "PLANETS.DAT"],
        }
    }
}

pub(crate) fn apply_known_scenario(
    dir: &Path,
    scenario: KnownScenario,
) -> Result<(), Box<dyn std::error::Error>> {
    match scenario {
        KnownScenario::FleetOrder => apply_fleet_order_scenario(dir),
        KnownScenario::PlanetBuild => apply_planet_build_scenario(dir),
        KnownScenario::GuardStarbase => apply_guard_starbase_scenario(dir),
        KnownScenario::Ipbm => apply_ipbm_scenario(dir),
        KnownScenario::Move => apply_move_scenario(dir),
        KnownScenario::Bombard => apply_bombard_scenario(dir),
        KnownScenario::FleetBattle => apply_fleet_battle_scenario(dir),
        KnownScenario::Invade => apply_invade_scenario(dir),
        KnownScenario::Econ => apply_econ_scenario(dir),
    }
}

pub(crate) fn apply_known_scenarios(
    dir: &Path,
    scenarios: &[KnownScenario],
) -> Result<(), Box<dyn std::error::Error>> {
    for scenario in scenarios {
        apply_known_scenario(dir, *scenario)?;
    }

    println!(
        "Applied scenarios: {}",
        scenarios
            .iter()
            .map(|scenario| scenario.name())
            .collect::<Vec<_>>()
            .join(", ")
    );
    Ok(())
}

pub(crate) fn validate_known_scenario(
    dir: &Path,
    scenario: KnownScenario,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = load_runtime_game_data(dir)?;
    match scenario {
        KnownScenario::FleetOrder => validate_fleet_order_data(&data),
        KnownScenario::PlanetBuild => validate_planet_build_data(&data),
        KnownScenario::GuardStarbase => validate_guard_starbase_data(&data),
        KnownScenario::Ipbm => validate_ipbm_data(&data),
        KnownScenario::Move => validate_move_data(&data),
        KnownScenario::Bombard => validate_bombard_data(&data),
        KnownScenario::FleetBattle => validate_fleet_battle_data(&data),
        KnownScenario::Invade => validate_invade_data(&data),
        KnownScenario::Econ => validate_econ_data(&data),
    }
}

pub(crate) fn validate_all_known_scenarios(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let data = load_runtime_game_data(dir)?;
    let mut matched = 0usize;
    for scenario in KnownScenario::all() {
        let name = scenario.name();
        let result = match scenario {
            KnownScenario::FleetOrder => validate_fleet_order_data(&data),
            KnownScenario::PlanetBuild => validate_planet_build_data(&data),
            KnownScenario::GuardStarbase => validate_guard_starbase_data(&data),
            KnownScenario::Ipbm => validate_ipbm_data(&data),
            KnownScenario::Move => validate_move_data(&data),
            KnownScenario::Bombard => validate_bombard_data(&data),
            KnownScenario::FleetBattle => validate_fleet_battle_data(&data),
            KnownScenario::Invade => validate_invade_data(&data),
            KnownScenario::Econ => validate_econ_data(&data),
        };
        match result {
            Ok(()) => {
                println!("OK   {name}");
                matched += 1;
            }
            Err(err) => println!("FAIL {name}: {err}"),
        }
    }

    if matched == 0 {
        Err("directory does not match any known accepted scenario".into())
    } else {
        Ok(())
    }
}

pub(crate) fn validate_preserved_scenario(
    dir: &Path,
    scenario: KnownScenario,
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture_dir = scenario.preserved_fixture_dir();
    let mut errors = Vec::new();

    for name in scenario.exact_match_files() {
        let actual = fs::read(dir.join(name))?;
        let expected = fs::read(fixture_dir.join(name))?;
        if actual != expected {
            errors.push(format!("{name} differs from preserved fixture"));
        }
    }

    if errors.is_empty() {
        println!("Exact preserved match: {}", scenario.name());
        println!("  fixture: {}", display_repo_path(&fixture_dir));
        for name in scenario.exact_match_files() {
            println!("  {name}");
        }
        Ok(())
    } else {
        Err(errors.join("\n").into())
    }
}

pub(crate) fn validate_all_preserved_scenarios(
    dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut matched = 0usize;
    for scenario in KnownScenario::all() {
        let name = scenario.name();
        match validate_preserved_scenario(dir, scenario) {
            Ok(()) => {
                println!("OK   {name}");
                matched += 1;
            }
            Err(err) => println!("FAIL {name}: {err}"),
        }
    }

    if matched == 0 {
        Err("directory does not exactly match any preserved accepted scenario".into())
    } else {
        Ok(())
    }
}

pub(crate) fn init_known_scenario(
    source: &Path,
    target: &Path,
    scenario: KnownScenario,
) -> Result<(), Box<dyn std::error::Error>> {
    copy_init_files(source, target)?;
    apply_known_scenario(target, scenario)?;
    export_runtime_snapshot_in_place(target)?;
    println!("Scenario directory initialized at {}", target.display());
    Ok(())
}

pub(crate) fn init_known_replayable_scenario(
    source: &Path,
    target: &Path,
    scenario: KnownScenario,
) -> Result<(), Box<dyn std::error::Error>> {
    copy_init_files(source, target)?;
    copy_pre_maint_replay_context_files(target)?;
    apply_known_scenario(target, scenario)?;
    export_runtime_core_projection_in_place(target)?;
    println!(
        "Replayable scenario directory initialized at {}",
        target.display()
    );
    Ok(())
}

pub(crate) fn init_all_known_scenarios(
    source: &Path,
    target_root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(target_root)?;
    let mut manifest = String::new();
    manifest.push_str("Known scenarios\n");
    manifest.push_str(&format!("source={}\n", display_repo_path(source)));
    manifest.push_str(&format!("target_root={}\n", target_root.display()));
    manifest.push('\n');
    for scenario in KnownScenario::all() {
        let scenario_dir = target_root.join(scenario.name());
        init_known_scenario(source, &scenario_dir, scenario)?;
        manifest.push_str(&format!("{}\n", scenario.name()));
        manifest.push_str(&format!("  description={}\n", scenario.description()));
        manifest.push_str(&format!("  dir={}\n", scenario_dir.display()));
        manifest.push_str(&format!(
            "  validate=nc-cli validate {} {}\n\n",
            scenario_dir.display(),
            scenario.name()
        ));
    }
    fs::write(target_root.join("SCENARIOS.txt"), manifest)?;
    println!(
        "Initialized all known scenarios under {}",
        target_root.display()
    );
    Ok(())
}

pub(crate) fn init_known_scenario_chain(
    source: &Path,
    target: &Path,
    scenarios: &[KnownScenario],
) -> Result<(), Box<dyn std::error::Error>> {
    copy_init_files(source, target)?;
    apply_known_scenarios(target, scenarios)?;
    export_runtime_snapshot_in_place(target)?;
    println!(
        "Scenario chain directory initialized at {}",
        target.display()
    );
    Ok(())
}

pub(crate) fn print_known_scenarios() {
    println!("Known scenarios:");
    for scenario in KnownScenario::all() {
        println!("  {}: {}", scenario.name(), scenario.description());
    }
}

pub(crate) fn print_known_scenario_details(scenario: KnownScenario) {
    println!("Scenario: {}", scenario.name());
    println!("Description: {}", scenario.description());
    println!(
        "Preserved fixture: {}",
        display_repo_path(&scenario.preserved_fixture_dir())
    );
    println!("Exact-match files:");
    for name in scenario.exact_match_files() {
        println!("  {name}");
    }
}

fn validate_fleet_order_data(data: &CoreGameData) -> Result<(), Box<dyn std::error::Error>> {
    let errors = data.fleet_order_errors_current_known(1, 0x03, 0x0C, [0x0F, 0x0D], None, None);
    if errors.is_empty() {
        println!("Valid fleet-order scenario");
        println!("  FLEET[1].speed = 3");
        println!("  FLEET[1].order = 0x0c");
        println!("  FLEET[1].target = (15, 13)");
        println!(
            "  FLEET[1].aux = {:02x?}",
            data.fleets[0].mission_aux_bytes()
        );
        Ok(())
    } else {
        Err(errors.join("\n").into())
    }
}

fn validate_planet_build_data(data: &CoreGameData) -> Result<(), Box<dyn std::error::Error>> {
    let errors = data.planet_build_errors_current_known(15, 0x03, 0x01);
    if errors.is_empty() {
        println!("Valid planet-build scenario");
        println!("  PLANET[15].build_slot = 0x03");
        println!("  PLANET[15].build_kind = 0x01");
        Ok(())
    } else {
        Err(errors.join("\n").into())
    }
}

fn validate_guard_starbase_data(data: &CoreGameData) -> Result<(), Box<dyn std::error::Error>> {
    let errors = data.guard_starbase_onebase_errors_current_known();
    if errors.is_empty() {
        println!("Valid guard-starbase scenario");
        let fleet = &data.fleets[0];
        let player1 = &data.players[0];
        let base = &data.bases[0];
        println!("  PLAYER[1].first_base_record = 1");
        println!(
            "  linkage keys: player[44]={} fleet[00]={} fleet[05]={} base[07]={}",
            player1.first_base_record_raw(),
            fleet.local_slot_word_raw(),
            fleet.fleet_id_word_raw(),
            base.record_number_raw()
        );
        println!(
            "  one-base guard-starbase linkage holds at coords {:?}",
            base.coords_raw()
        );
        Ok(())
    } else {
        Err(errors.join("\n").into())
    }
}

fn validate_move_data(data: &CoreGameData) -> Result<(), Box<dyn std::error::Error>> {
    let errors = data.fleet_order_errors_current_known(1, 0x03, 0x01, [0x1A, 0x0D], None, None);
    if errors.is_empty() {
        println!("Valid move scenario");
        println!("  FLEET[1].speed = 3");
        println!("  FLEET[1].order = 0x01");
        println!("  FLEET[1].target = (26, 13)");
        Ok(())
    } else {
        Err(errors.join("\n").into())
    }
}
