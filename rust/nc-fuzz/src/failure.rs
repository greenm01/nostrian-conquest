//! A failing case written as an nc-harness combat scenario plus one turn
//! file per submission, replayable with `nc-cli harness run-combat`.

use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use nc_data::DiplomaticRelation;
use nc_harness::ScenarioSpec;

use crate::game::CaseRun;

pub const SCENARIO_FILE_NAME: &str = "scenario.kdl";

/// Where a driver writes its failure: `$NC_FUZZ_OUT/<driver>`, or the
/// system temp directory. Never inside the repository by default.
pub fn failure_dir(driver: &str) -> PathBuf {
    std::env::var_os("NC_FUZZ_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("nc-fuzz"))
        .join(driver)
}

/// Write `run` into `dir`, replacing any earlier scenario there, and return
/// the scenario path. Shrinking replays the case many times; the last write
/// is the smallest failing case.
pub fn write_scenario(run: &CaseRun, dir: &Path) -> io::Result<PathBuf> {
    if dir.exists() {
        fs::remove_dir_all(dir)?;
    }
    fs::create_dir_all(dir)?;
    let mut kdl = String::new();
    for violation in &run.violations {
        let _ = writeln!(kdl, "// violation: {violation}");
    }
    let _ = writeln!(kdl, "// {}", run.setup.summary());
    render_setup(&mut kdl, &run.setup.scenario_spec(), run.turns.len().max(1));
    for (turn_idx, submissions) in run.turns.iter().enumerate() {
        let turn = turn_idx + 1;
        for submission in submissions {
            let file_name = format!(
                "turn-{turn:02}-empire-{}.kdl",
                submission.player_record_index_1_based
            );
            fs::write(dir.join(&file_name), submission.to_kdl_string())?;
            let _ = writeln!(kdl, "turn-file path=\"{file_name}\" turn={turn}");
        }
    }
    let path = dir.join(SCENARIO_FILE_NAME);
    fs::write(&path, kdl)?;
    Ok(path)
}

/// The scenario nodes a fuzz setup uses.
fn render_setup(kdl: &mut String, spec: &ScenarioSpec, turns: usize) {
    let meta = &spec.metadata;
    let _ = writeln!(
        kdl,
        "combat-scenario player_count={} year={} seed={} baseline=\"builder-compatible\" turns={turns} label=\"nc-fuzz\"\n",
        meta.player_count, meta.year, meta.seed
    );
    for house in &spec.houses {
        let autopilot = if house.autopilot {
            " autopilot=#true"
        } else {
            ""
        };
        let _ = writeln!(
            kdl,
            "house record={}{autopilot}",
            house.record_index_1_based
        );
    }
    for relation in &spec.diplomacy {
        let status = match relation.relation {
            DiplomaticRelation::Enemy => "enemy",
            DiplomaticRelation::Neutral => "neutral",
        };
        let _ = writeln!(
            kdl,
            "relation from={} to={} status=\"{status}\"",
            relation.from_empire_raw, relation.to_empire_raw
        );
    }
    for planet in &spec.planets {
        let owner = planet.owner_empire_raw.unwrap_or_default();
        let _ = writeln!(
            kdl,
            "planet record={} {{\n  owner record={owner}\n}}",
            planet.record_index_1_based
        );
    }
    for fleet in &spec.fleets {
        let _ = writeln!(kdl, "fleet record={} {{", fleet.record_index_1_based);
        if let Some([x, y]) = fleet.coords {
            let _ = writeln!(kdl, "  coords x={x} y={y}");
        }
        if let Some(ships) = &fleet.ships {
            let _ = writeln!(
                kdl,
                "  ships bb={} ca={} dd={} sc={} tt={} armies={} etac={}",
                ships.battleships,
                ships.cruisers,
                ships.destroyers,
                ships.scouts,
                ships.transports,
                ships.loaded_armies,
                ships.etacs
            );
        }
        kdl.push_str("}\n");
    }
    kdl.push('\n');
}
