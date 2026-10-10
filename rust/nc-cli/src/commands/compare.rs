use std::fs;
use std::path::Path;

use nc_data::{FleetDat, PlanetDat, PlayerDat};

use crate::commands::scenario::KnownScenario;

pub(crate) fn compare_preserved_scenario(
    dir: &Path,
    scenario: KnownScenario,
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture_dir = scenario.preserved_fixture_dir();
    println!("Scenario: {}", scenario.name());
    println!("Actual:   {}", dir.display());
    println!("Fixture:  {}", fixture_dir.display());
    println!();

    for name in scenario.exact_match_files() {
        compare_raw_file(dir, &fixture_dir, name)?;
    }

    Ok(())
}

pub(crate) fn compare_all_preserved_scenarios(
    dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    for scenario in KnownScenario::all() {
        compare_preserved_scenario(dir, scenario)?;
        println!();
    }
    Ok(())
}

pub(crate) fn compare_dirs(left: &Path, right: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("Left:  {}", left.display());
    println!("Right: {}", right.display());
    println!();

    compare_raw_file(left, right, "SETUP.DAT")?;
    compare_raw_file(left, right, "CONQUEST.DAT")?;
    compare_raw_file(left, right, "DATABASE.DAT")?;
    compare_player(left, right)?;
    compare_planets(left, right)?;
    compare_fleets(left, right)?;

    Ok(())
}

pub(crate) fn compare_raw_file(
    left_dir: &Path,
    right_dir: &Path,
    name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let left = fs::read(left_dir.join(name))?;
    let right = fs::read(right_dir.join(name))?;
    println!(
        "{name}: size {} vs {}, differing bytes {}",
        left.len(),
        right.len(),
        diff_count(&left, &right)
    );
    Ok(())
}

fn compare_player(left_dir: &Path, right_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let left = PlayerDat::parse(&fs::read(left_dir.join("PLAYER.DAT"))?)?;
    let right = PlayerDat::parse(&fs::read(right_dir.join("PLAYER.DAT"))?)?;
    println!("PLAYER.DAT:");
    for (idx, (a, b)) in left.records.iter().zip(right.records.iter()).enumerate() {
        let count = diff_count(&a.to_bytes(), &b.to_bytes());
        if count == 0 {
            continue;
        }
        println!(
            "  record {}: {} differing bytes, tax {} -> {}",
            idx + 1,
            count,
            a.tax_rate(),
            b.tax_rate()
        );
    }
    Ok(())
}

fn compare_planets(left_dir: &Path, right_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let left = PlanetDat::parse(&fs::read(left_dir.join("PLANETS.DAT"))?)?;
    let right = PlanetDat::parse(&fs::read(right_dir.join("PLANETS.DAT"))?)?;
    println!("PLANETS.DAT:");
    for (idx, (a, b)) in left.records.iter().zip(right.records.iter()).enumerate() {
        let count = diff_count(&a.to_bytes(), &b.to_bytes());
        if count == 0 {
            continue;
        }
        println!(
            "  record {:02}: {} differing bytes, text '{}' -> '{}'",
            idx + 1,
            count,
            ascii_trim(a.status_or_name_bytes()),
            ascii_trim(b.status_or_name_bytes())
        );
    }
    Ok(())
}

fn compare_fleets(left_dir: &Path, right_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let left_bytes = fs::read(left_dir.join("FLEETS.DAT"))?;
    let right_bytes = fs::read(right_dir.join("FLEETS.DAT"))?;
    println!(
        "FLEETS.DAT: size {} vs {}, differing bytes {}",
        left_bytes.len(),
        right_bytes.len(),
        diff_count(&left_bytes, &right_bytes)
    );

    let left = FleetDat::parse(&left_bytes);
    let right = FleetDat::parse(&right_bytes);
    if let (Ok(left), Ok(right)) = (left, right) {
        for (idx, (a, b)) in left.records.iter().zip(right.records.iter()).enumerate() {
            let count = diff_count(&a.to_bytes(), &b.to_bytes());
            if count == 0 {
                continue;
            }
            println!(
                "  record {:02}: {} differing bytes, current speed {} -> {}, params {:02x?} -> {:02x?}",
                idx + 1,
                count,
                a.current_speed(),
                b.current_speed(),
                a.mission_param_bytes(),
                b.mission_param_bytes()
            );
        }
    }

    Ok(())
}

fn ascii_trim(bytes: &[u8]) -> String {
    let text = bytes
        .iter()
        .map(|b| {
            if (32..127).contains(b) {
                *b as char
            } else {
                ' '
            }
        })
        .collect::<String>();
    text.trim().to_string()
}

fn diff_count(left: &[u8], right: &[u8]) -> usize {
    let shared = left
        .iter()
        .zip(right.iter())
        .filter(|(a, b)| a != b)
        .count();
    shared + left.len().abs_diff(right.len())
}
