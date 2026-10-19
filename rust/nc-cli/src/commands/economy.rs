use std::path::Path;

use crate::commands::runtime::{load_runtime_game_data, with_runtime_game_mut};
use nc_data::{
    Coords, EmpireId, Planet, ProductionItemKind, Real48, is_homeworld_seed_ignoring_name,
    set_planet_name,
};

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

        let empire = player_record_index_1_based as u8;
        let Some(homeworld_idx) = data
            .planets
            .iter()
            .position(|planet| planet.owner.0 == empire && is_homeworld_seed_ignoring_name(planet))
        else {
            return Err(format!(
                "player {} homeworld seed not found",
                player_record_index_1_based
            )
            .into());
        };

        {
            let homeworld = &mut data.planets[homeworld_idx];
            homeworld.stored_goods = Real48::ZERO;
            homeworld.armies = 10;
            homeworld.ground_batteries = 4;
            homeworld.years_under_owner = 2;
        }

        let unowned_indices = data
            .planets
            .iter()
            .enumerate()
            .filter(|(_, planet)| planet.owner.0 == 0)
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
            let coords = data.planets[planet_idx].sector.to_array();
            let planet = &mut data.planets[planet_idx];
            *planet = Planet {
                sector: Coords::from_array(coords),
                name: name.to_string(),
                armies: u16::from(armies),
                ground_batteries: u16::from(batteries),
                years_under_owner: 2,
                owner: EmpireId(empire),
                ..Planet::default()
            };
            planet.potential_production = potential.min(u16::from(u8::MAX)) as u8;
            planet.production = Real48::from_f64(f64::from(present));
            for slot in 0..10 {
                planet.build_queue[slot].points_owed = 0;
                planet.build_queue[slot].kind = ProductionItemKind::Unknown(0);
            }
            for slot in 0..nc_data::STARDOCK_SLOT_COUNT {
                planet.stardock[slot] = 0;
            }
            planet.stored_goods = Real48::ZERO;
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
        let empire = player_record_index_1_based as u8;
        let empire_name = data
            .players
            .get(player_record_index_1_based - 1)
            .map(|player| player.empire_name.trim().to_string())
            .filter(|name| !name.trim().is_empty() && name != "In Civil Disorder")
            .unwrap_or_else(|| format!("Probe Empire {player_record_index_1_based}"));

        data.join_player(player_record_index_1_based, &empire_name)?;
        data.set_player_tax_rate(player_record_index_1_based, tax_rate)?;

        if let Some(player) = data.players.get_mut(player_record_index_1_based - 1) {
            player.bases_first.0 = 0;
        }
        data.bases.retain(|base| base.owner.0 != empire);

        let Some(homeworld_idx) = data
            .planets
            .iter()
            .position(|planet| planet.owner.0 == empire && is_homeworld_seed_ignoring_name(planet))
        else {
            return Err(format!(
                "player {} homeworld seed not found",
                player_record_index_1_based
            )
            .into());
        };

        {
            let homeworld = &mut data.planets[homeworld_idx];
            homeworld.stored_goods = Real48::ZERO;
            homeworld.armies = 10;
            homeworld.ground_batteries = 4;
            homeworld.years_under_owner = 2;
        }

        let unowned_indices = data
            .planets
            .iter()
            .enumerate()
            .filter(|(_, planet)| planet.owner.0 == 0)
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
            planet.owner.0 = empire;
            planet.years_under_owner = 2;
            set_planet_name(planet, name);
            planet.potential_production =
                STARBASE_PROBE_POTENTIAL_PRODUCTION.min(u16::from(u8::MAX)) as u8;
            planet.production = Real48::from_f64(f64::from(STARBASE_PROBE_PRESENT_PRODUCTION));
            planet.armies = u16::from(STARBASE_PROBE_ARMIES);
            planet.ground_batteries = u16::from(STARBASE_PROBE_BATTERIES);
            for slot in 0..10 {
                planet.build_queue[slot].points_owed = 0;
                planet.build_queue[slot].kind = ProductionItemKind::Unknown(0);
            }
            for slot in 0..nc_data::STARDOCK_SLOT_COUNT {
                planet.stardock[slot] = 0;
            }
            planet.stored_goods = Real48::ZERO;

            if has_starbase {
                let slot = nc_data::stardock_slot_for_kind(9).expect("starbase stardock slot");
                planet.stardock[slot] = 1;
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
    let report = data
        .economy_report_text(player_record_index_1_based)
        .ok_or("missing player record")?;
    println!("dir={}", dir.display());
    print!("{report}");
    Ok(())
}
