use std::path::Path;

use crate::commands::runtime::{load_runtime_game_data, with_runtime_game_mut};

const PROBE_COLONY_SPECS: [(&str, u16, u16, u8, u8); 2] =
    [("Mid Colony", 50, 100, 3, 1), ("New Colony", 25, 100, 1, 0)];
const STARBASE_PROBE_COLONY_SPECS: [(&str, bool); 2] =
    [("Plain Colony", false), ("Base Colony", true)];
const STARBASE_PROBE_PRESENT_PRODUCTION: u16 = 50;
const STARBASE_PROBE_POTENTIAL_PRODUCTION: u16 = 100;
const STARBASE_PROBE_ARMIES: u8 = 3;
const STARBASE_PROBE_BATTERIES: u8 = 1;

pub(crate) fn init_tax_growth_probe(
    dir: &Path,
    player_record_index_1_based: usize,
    tax_rate: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    with_runtime_game_mut(dir, |data| {
        data.set_player_tax_rate(player_record_index_1_based, tax_rate)?;

        let empire_raw = player_record_index_1_based as u8;
        let Some(homeworld_idx) = data.planets.iter().position(|planet| {
            planet.owner_empire_slot_raw() == empire_raw && planet.is_homeworld_seed_ignoring_name()
        }) else {
            return Err(format!(
                "player {} homeworld seed not found",
                player_record_index_1_based
            )
            .into());
        };

        {
            let homeworld = &mut data.planets[homeworld_idx];
            homeworld.set_stored_production_points(0);
            homeworld.set_army_count_raw(10);
            homeworld.set_ground_batteries_raw(4);
            homeworld.set_ownership_status_raw(2);
        }

        let unowned_indices = data
            .planets
            .iter()
            .enumerate()
            .filter(|(_, planet)| planet.owner_empire_slot_raw() == 0)
            .map(|(idx, _)| idx)
            .rev()
            .take(PROBE_COLONY_SPECS.len())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>();

        if unowned_indices.len() != PROBE_COLONY_SPECS.len() {
            return Err("not enough unowned planets available for economy probe".into());
        }

        for (planet_idx, (name, present, potential, armies, batteries)) in
            unowned_indices.into_iter().zip(PROBE_COLONY_SPECS)
        {
            let coords = data.planets[planet_idx].coords_raw();
            let planet = &mut data.planets[planet_idx];
            planet.set_as_owned_target_world(
                coords,
                [0; 2],
                [0; 6],
                0,
                probe_name_len(name),
                probe_name_buffer(name),
                [0; 7],
                armies,
                batteries,
                2,
                empire_raw,
            );
            planet.set_potential_production_points(potential.min(u16::from(u8::MAX)) as u8);
            planet.set_present_production_points(present);
            for slot in 0..10 {
                planet.set_build_count_raw(slot, 0);
                planet.set_build_kind_raw(slot, 0);
            }
            for slot in 0..nc_data::STARDOCK_SLOT_COUNT {
                planet.set_stardock_count_raw(slot, 0);
            }
            planet.set_stored_production_points(0);
        }

        Ok(())
    })?;

    println!(
        "Initialized economy tax-growth probe at {} for player {} tax={}%",
        dir.display(),
        player_record_index_1_based,
        tax_rate
    );
    print_economy_report(dir, player_record_index_1_based)?;
    Ok(())
}

pub(crate) fn init_starbase_tax_probe(
    dir: &Path,
    player_record_index_1_based: usize,
    tax_rate: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    with_runtime_game_mut(dir, |data| {
        let empire_raw = player_record_index_1_based as u8;
        let empire_name = data
            .players
            .get(player_record_index_1_based - 1)
            .map(|player| player.controlled_empire_name_summary())
            .filter(|name| !name.trim().is_empty() && name != "In Civil Disorder")
            .unwrap_or_else(|| format!("Probe Empire {player_record_index_1_based}"));

        data.join_player(player_record_index_1_based, &empire_name)?;
        data.set_player_tax_rate(player_record_index_1_based, tax_rate)?;

        if let Some(player) = data.players.get_mut(player_record_index_1_based - 1) {
            player.set_first_base_record_raw(0);
        }
        data.bases
            .retain(|base| base.owner_empire_raw() != empire_raw);

        let Some(homeworld_idx) = data.planets.iter().position(|planet| {
            planet.owner_empire_slot_raw() == empire_raw && planet.is_homeworld_seed_ignoring_name()
        }) else {
            return Err(format!(
                "player {} homeworld seed not found",
                player_record_index_1_based
            )
            .into());
        };

        {
            let homeworld = &mut data.planets[homeworld_idx];
            homeworld.set_stored_production_points(0);
            homeworld.set_army_count_raw(10);
            homeworld.set_ground_batteries_raw(4);
            homeworld.set_ownership_status_raw(2);
        }

        let unowned_indices = data
            .planets
            .iter()
            .enumerate()
            .filter(|(_, planet)| planet.owner_empire_slot_raw() == 0)
            .map(|(idx, _)| idx)
            .rev()
            .take(STARBASE_PROBE_COLONY_SPECS.len())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>();

        if unowned_indices.len() != STARBASE_PROBE_COLONY_SPECS.len() {
            return Err("not enough unowned planets available for starbase economy probe".into());
        }

        let mut starbase_planet_index_1_based = None;
        for (planet_idx, (name, has_starbase)) in
            unowned_indices.into_iter().zip(STARBASE_PROBE_COLONY_SPECS)
        {
            let planet = &mut data.planets[planet_idx];
            planet.set_owner_empire_slot_raw(empire_raw);
            planet.set_ownership_status_raw(2);
            planet.set_planet_name(name);
            planet.set_potential_production_points(
                STARBASE_PROBE_POTENTIAL_PRODUCTION.min(u16::from(u8::MAX)) as u8,
            );
            if !planet.set_present_production_points(STARBASE_PROBE_PRESENT_PRODUCTION) {
                return Err(format!(
                    "unsupported probe production value {} for planet {}",
                    STARBASE_PROBE_PRESENT_PRODUCTION,
                    planet_idx + 1
                )
                .into());
            }
            planet.set_army_count_raw(STARBASE_PROBE_ARMIES);
            planet.set_ground_batteries_raw(STARBASE_PROBE_BATTERIES);
            for slot in 0..10 {
                planet.set_build_count_raw(slot, 0);
                planet.set_build_kind_raw(slot, 0);
            }
            for slot in 0..nc_data::STARDOCK_SLOT_COUNT {
                planet.set_stardock_count_raw(slot, 0);
            }
            planet.set_stored_production_points(0);

            if has_starbase {
                let slot = nc_data::stardock_slot_for_kind(9).expect("starbase stardock slot");
                planet.set_stardock_count_raw(slot, 1);
                starbase_planet_index_1_based = Some(planet_idx + 1);
            }
        }

        if let Some(planet_index_1_based) = starbase_planet_index_1_based {
            data.commission_planet_stardock_slot(
                player_record_index_1_based,
                planet_index_1_based,
                nc_data::stardock_slot_for_kind(9).expect("starbase stardock slot"),
            )?;
        }

        Ok(())
    })?;

    println!(
        "Initialized economy starbase tax-burden probe at {} for player {} tax={}%",
        dir.display(),
        player_record_index_1_based,
        tax_rate
    );
    print_economy_report(dir, player_record_index_1_based)?;
    Ok(())
}

pub(crate) fn print_economy_report(
    dir: &Path,
    player_record_index_1_based: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = load_runtime_game_data(dir)?;
    let economy = data.empire_economy_summary(player_record_index_1_based);
    let player = data
        .players
        .get(player_record_index_1_based - 1)
        .ok_or("missing player record")?;

    println!(
        "Economy report: dir={} player={} empire=\"{}\"",
        dir.display(),
        player_record_index_1_based,
        player.controlled_empire_name_summary()
    );
    println!(
        "  tax_rate={} present={} potential={} available={} last_run_year={}",
        economy.tax_rate,
        economy.present_production,
        economy.potential_production,
        economy.total_available_points,
        player.last_run_year_raw()
    );
    println!(
        "  planets={} rank_planets={} rank_production={} efficiency={:.3}%",
        economy.owned_planets,
        economy.rank_by_planets,
        economy.rank_by_present_production,
        economy.efficiency_percent
    );
    println!("  planets:");
    println!(
        "    {:>3} {:>7} {:<13} {:>7} {:>9} {:>6} {:>6} {:>6} {:>5} {:>4} {:>4}",
        "rec",
        "coords",
        "name",
        "present",
        "potential",
        "stored",
        "rev",
        "grow",
        "cap",
        "army",
        "bat"
    );
    for row in data.empire_planet_economy_rows(player_record_index_1_based) {
        let [x, y] = row.coords;
        println!(
            "    {:>3} ({:02},{:02}) {:<13} {:>7} {:>9} {:>6} {:>6} {:>6} {:>5} {:>4} {:>4}{}{}",
            row.planet_record_index_1_based,
            x,
            y,
            row.planet_name,
            row.present_production,
            row.potential_production,
            row.stored_production_points,
            row.yearly_tax_revenue,
            row.yearly_growth_delta,
            row.build_capacity,
            row.armies,
            row.ground_batteries,
            if row.has_friendly_starbase {
                " starbase"
            } else {
                ""
            },
            if row.is_homeworld_seed {
                " homeworld"
            } else {
                ""
            }
        );
    }

    Ok(())
}

fn probe_name_len(name: &str) -> u8 {
    name.len().min(13) as u8
}

fn probe_name_buffer(name: &str) -> [u8; 13] {
    let mut buffer = [0u8; 13];
    let bytes = name.as_bytes();
    let len = bytes.len().min(buffer.len());
    buffer[..len].copy_from_slice(&bytes[..len]);
    buffer
}
