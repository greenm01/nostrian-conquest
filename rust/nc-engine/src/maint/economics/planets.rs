use crate::{CoreGameData, yearly_growth_delta, yearly_high_tax_penalty, yearly_tax_revenue};

pub(super) fn process_planet_economics(
    game_data: &mut CoreGameData,
    _planets_with_builds: &[usize],
    starbase_planets: &[usize],
) -> Result<(), Box<dyn std::error::Error>> {
    for planet_idx in 0..game_data.planets.records.len() {
        let owner_empire = game_data.planets.records[planet_idx].owner_empire_slot_raw();
        if owner_empire == 0 {
            continue;
        }
        // A captured world's status steps 0 -> 1 -> 2 over two yearly passes
        // (ECMAINT 0x10436) and produces nothing for its new owner until then.
        let status = game_data.planets.records[planet_idx].ownership_status_raw();
        if status < 2 {
            game_data.planets.records[planet_idx].set_ownership_status_raw(status + 1);
            continue;
        }
        let Some(player) = game_data
            .player
            .records
            .get(owner_empire.saturating_sub(1) as usize)
        else {
            continue;
        };
        // Civil disorder (0x00) freezes the economy; rogue empires (0xff) keep it.
        if player.owner_mode_raw() == 0x00 {
            continue;
        }

        let tax_rate = player.tax_rate();
        let current_production = game_data.planets.records[planet_idx]
            .present_production_points()
            .unwrap_or(0);
        let potential_production =
            game_data.planets.records[planet_idx].potential_production_points();
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

        let planet = &mut game_data.planets.records[planet_idx];
        let _ = planet.set_present_production_points(new_current_production);
        planet.set_stored_production_points(
            planet.stored_production_points().saturating_add(revenue),
        );
    }

    Ok(())
}

pub(super) fn recompute_player_planet_stats(game_data: &mut CoreGameData) {
    let n_players = game_data.player.records.len();
    let mut planet_counts = vec![0u8; n_players + 1];
    let mut pot_prod_sums = vec![0u16; n_players + 1];

    for planet in &game_data.planets.records {
        let owner = planet.owner_empire_slot_raw() as usize;
        if owner > 0 && owner <= n_players {
            planet_counts[owner] = planet_counts[owner].saturating_add(1);
            let current_prod = planet.present_production_points().unwrap_or(0);
            pot_prod_sums[owner] = pot_prod_sums[owner].saturating_add(current_prod);
        }
    }

    for player_idx in 0..n_players {
        let owner_slot = player_idx + 1;
        game_data.player.records[player_idx].set_planet_count_raw(planet_counts[owner_slot]);
        game_data.player.records[player_idx].set_production_score_raw(pot_prod_sums[owner_slot]);
    }
}
