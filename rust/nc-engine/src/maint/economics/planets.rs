use crate::CoreGameData;
use nc_data::{
    GameRng, PlanetScorchEvent, RNG_TAG_ECONOMY, Real48, next_production,
    present_production_points, stored_production_points, tax_income,
};

/// The yearly pass over every owned world. Production and goods keep their
/// fractions; income is added to the treasury as it stands, debt included.
pub(super) fn process_planet_economics(
    game_data: &mut CoreGameData,
    campaign_seed: u64,
    _planets_with_builds: &[usize],
    starbase_planets: &[usize],
) -> Result<Vec<PlanetScorchEvent>, Box<dyn std::error::Error>> {
    let game_year = u64::from(game_data.conquest.game_year);
    let mut scorch_events = Vec::new();
    for planet_idx in 0..game_data.planets.len() {
        let owner_empire = game_data.planets[planet_idx].owner.0;
        if owner_empire == 0 {
            continue;
        }
        // A captured world's status steps 0 -> 1 -> 2 over two yearly passes
        // (ECMAINT 0x10436) and produces nothing for its new owner until then.
        let status = game_data.planets[planet_idx].years_under_owner;
        let converting = status < 2;
        if converting {
            game_data.planets[planet_idx].years_under_owner = status + 1;
        }
        let Some(player) = game_data
            .players
            .get(owner_empire.saturating_sub(1) as usize)
        else {
            continue;
        };
        // Civil disorder (0x00) freezes the economy; rogue empires (0xff) keep it.
        if player.mode.code() == 0x00 {
            continue;
        }

        let tax_rate = player.tax_rate;
        let planet = &mut game_data.planets[planet_idx];
        let production = planet.production.to_f64();
        let potential = f64::from(planet.potential_production);
        let goods = planet.stored_goods.to_f64();

        // Scorch-Earth decays the world in place of growth and income, and
        // applies to a converting world too (ECMAINT `1852:0449`).
        if planet.scorch_earth {
            let mut rng = GameRng::from_context(
                campaign_seed,
                RNG_TAG_ECONOMY,
                &[game_year, planet_idx as u64],
            );
            let roll = rng.below(20);
            let (production_before, goods_before) = (
                present_production_points(planet),
                stored_production_points(planet),
            );
            planet.production = Real48::from_f64(scorched_production(production, potential, roll));
            planet.stored_goods = Real48::from_f64(scorched_goods(goods));
            scorch_events.push(PlanetScorchEvent {
                planet_idx,
                owner_empire,
                production_before,
                production_after: present_production_points(planet),
                goods_before,
                goods_after: stored_production_points(planet),
            });
            continue;
        }
        if converting {
            continue;
        }

        // Growth comes first so that revenue is credited at the post-growth
        // production, which is what the Revenue fields forecast.
        let has_starbase = starbase_planets.contains(&planet_idx);
        let new_production = next_production(production, potential, tax_rate, has_starbase);
        planet.production = Real48::from_f64(new_production);
        planet.stored_goods = Real48::from_f64(goods + tax_income(new_production, tax_rate));
    }

    Ok(scorch_events)
}

/// Scorch-Earth keeps `0.30 + roll / 100` of production (`roll` is
/// `Random(20)`), at least 1, and never more than the potential, which the
/// order leaves alone: a world with no potential stays at 0.
fn scorched_production(production: f64, potential: f64, roll: u64) -> f64 {
    if potential <= 0.0 {
        return 0.0;
    }
    (production * (0.30 + roll as f64 / 100.0))
        .max(1.0)
        .min(potential)
}

/// Positive goods shrink to `min(G / 10, sqrt(G))`; a debt is left as it is.
fn scorched_goods(goods: f64) -> f64 {
    if goods > 0.0 {
        (goods / 10.0).min(goods.sqrt())
    } else {
        goods
    }
}

pub(super) fn recompute_player_planet_stats(game_data: &mut CoreGameData) {
    let n_players = game_data.players.len();
    let mut planet_counts = vec![0u8; n_players + 1];
    let mut pot_prod_sums = vec![0u16; n_players + 1];

    for planet in &game_data.planets {
        let owner = planet.owner.0 as usize;
        if owner > 0 && owner <= n_players {
            planet_counts[owner] = planet_counts[owner].saturating_add(1);
            let current_prod = present_production_points(planet);
            pot_prod_sums[owner] = pot_prod_sums[owner].saturating_add(current_prod);
        }
    }

    for player_idx in 0..n_players {
        let owner_slot = player_idx + 1;
        game_data.players[player_idx].planet_count = planet_counts[owner_slot];
        game_data.players[player_idx].production = pot_prod_sums[owner_slot];
    }
}
