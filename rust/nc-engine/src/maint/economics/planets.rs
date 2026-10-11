use crate::{CoreGameData, yearly_growth_delta, yearly_high_tax_penalty, yearly_tax_revenue};
use nc_data::{Real48, present_production_points, stored_production_points};

pub(super) fn process_planet_economics(
    game_data: &mut CoreGameData,
    _planets_with_builds: &[usize],
    starbase_planets: &[usize],
) -> Result<(), Box<dyn std::error::Error>> {
    for planet_idx in 0..game_data.planets.len() {
        let owner_empire = game_data.planets[planet_idx].owner.0;
        if owner_empire == 0 {
            continue;
        }
        // A captured world's status steps 0 -> 1 -> 2 over two yearly passes
        // (ECMAINT 0x10436) and produces nothing for its new owner until then.
        let status = game_data.planets[planet_idx].years_under_owner;
        if status < 2 {
            game_data.planets[planet_idx].years_under_owner = status + 1;
            continue;
        }
        let Some(player) = game_data
            .players
            .get(owner_empire.saturating_sub(1) as usize)
        else {
            continue;
        };
        // Civil disorder (0x00) freezes the economy; rogue empires (0xff) keep it.
        if player.mode.to_raw() == 0x00 {
            continue;
        }

        let tax_rate = player.tax_rate;
        let current_production = present_production_points(&game_data.planets[planet_idx]);
        let potential_production = u16::from(game_data.planets[planet_idx].potential_production);
        let has_starbase = starbase_planets.contains(&planet_idx);

        let growth = yearly_growth_delta(
            current_production,
            potential_production,
            tax_rate,
            has_starbase,
        );
        let penalty = yearly_high_tax_penalty(current_production, tax_rate);

        // Apply growth first so that revenue is credited at the post-growth
        // production level.  This keeps the treasury consistent with the Rev
        // column, which is always displayed at the current (post-growth) value.
        let new_current_production = current_production
            .saturating_add(growth)
            .saturating_sub(penalty)
            .min(potential_production);
        let revenue = yearly_tax_revenue(new_current_production, tax_rate);

        let planet = &mut game_data.planets[planet_idx];
        planet.production = Real48::from_f64(f64::from(new_current_production));
        planet.stored_goods = Real48::from_f64(f64::from(
            stored_production_points(planet).saturating_add(revenue),
        ));
    }

    Ok(())
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
