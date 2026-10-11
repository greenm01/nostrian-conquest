use std::fs;
use std::path::Path;

use crate::commands::scenario::KnownScenario;
use nc_data::classic::{
    encode_fleet, encode_planet, encode_player, parse_fleets_dat, parse_planets_dat,
    parse_player_dat,
};

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
    let left = parse_player_dat(&fs::read(left_dir.join("PLAYER.DAT"))?)?;
    let right = parse_player_dat(&fs::read(right_dir.join("PLAYER.DAT"))?)?;
    println!("PLAYER.DAT:");
    for (idx, (a, b)) in left.iter().zip(right.iter()).enumerate() {
        let count = diff_count(&encode_player(a), &encode_player(b));
        if count == 0 {
            continue;
        }
        println!(
            "  record {}: {} differing bytes, tax {} -> {}",
            idx + 1,
            count,
            a.tax_rate,
            b.tax_rate
        );
    }
    Ok(())
}

fn compare_planets(left_dir: &Path, right_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let left = parse_planets_dat(&fs::read(left_dir.join("PLANETS.DAT"))?)?;
    let right = parse_planets_dat(&fs::read(right_dir.join("PLANETS.DAT"))?)?;
    println!("PLANETS.DAT:");
    for (idx, (a, b)) in left.iter().zip(right.iter()).enumerate() {
        let count = diff_count(&encode_planet(a), &encode_planet(b));
        if count == 0 {
            continue;
        }
        println!(
            "  record {:02}: {} differing bytes, text '{}' -> '{}'",
            idx + 1,
            count,
            ascii_trim(a.name.as_bytes()),
            ascii_trim(b.name.as_bytes())
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

    let left = parse_fleets_dat(&left_bytes);
    let right = parse_fleets_dat(&right_bytes);
    if let (Ok(left), Ok(right)) = (left, right) {
        for (idx, (a, b)) in left.iter().zip(right.iter()).enumerate() {
            let count = diff_count(&encode_fleet(a), &encode_fleet(b));
            if count == 0 {
                continue;
            }
            println!(
                "  record {:02}: {} differing bytes, current speed {} -> {}, params {:02x?} -> {:02x?}",
                idx + 1,
                count,
                a.speed,
                b.speed,
                [a.order.to_raw(), a.target.x, a.target.y],
                [b.order.to_raw(), b.target.x, b.target.y]
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
