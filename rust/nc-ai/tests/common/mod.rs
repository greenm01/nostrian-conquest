//! Helpers shared by the nc-ai tests.

pub mod arrays;

use nc_ai::SeatedGame;
use nc_data::{CoreGameData, DiplomaticRelation};
use nc_fuzz::{CaseSetup, Policy};
use nc_harness::{ScenarioPlay, build_scenario};

/// A fresh game on map `map_seed`, with the same campaign seed.
pub fn seated_game(player_count: u8, map_seed: u64) -> SeatedGame {
    let setup = CaseSetup {
        player_count,
        map_seed,
        policies: vec![Policy::Aggressor; usize::from(player_count)],
        collapsed_empire: None,
        rally: false,
        war: false,
        armadas: vec![None; usize::from(player_count)],
        turns: 100,
    };
    let built = build_scenario(&setup.scenario_spec()).expect("setup should build");
    SeatedGame::new(ScenarioPlay::new(built).with_campaign_seed(map_seed))
}

/// Change every record `empire` does not own: other empires' worlds,
/// fleets, starbases and player records. Unowned worlds change too, and
/// worlds `empire` does not own swap between unowned and another empire.
pub fn scramble_others(game_data: &CoreGameData, empire: u8) -> CoreGameData {
    let mut scrambled = game_data.clone();
    let other = if empire == 1 { 2 } else { 1 };
    for planet in scrambled
        .planets
        .iter_mut()
        .filter(|planet| planet.owner.0 != empire)
    {
        planet.owner.0 = if planet.owner.0 == 0 { other } else { 0 };
        planet.armies += 7;
        planet.ground_batteries += 3;
        planet.potential_production = planet.potential_production.wrapping_add(11);
        planet.stardock.destroyers += 5;
        planet.stardock.battleships += 2;
    }
    for fleet in scrambled
        .fleets
        .iter_mut()
        .filter(|fleet| fleet.owner.0 != empire)
    {
        fleet.ships.battleships += 9;
        fleet.ships.scouts += 4;
        fleet.speed = fleet.speed.wrapping_add(1);
        fleet.sector.x = fleet.sector.x.wrapping_add(1);
        fleet.rules_of_engagement = fleet.rules_of_engagement.wrapping_add(1);
    }
    for base in scrambled
        .bases
        .iter_mut()
        .filter(|base| base.owner.0 != empire)
    {
        base.sector.y = base.sector.y.wrapping_add(1);
        base.moving = !base.moving;
    }
    for (idx, player) in scrambled.players.iter_mut().enumerate() {
        if idx + 1 != usize::from(empire) {
            player.tax_rate = 99 - player.tax_rate.min(99);
            player.diplomacy[0] = match player.diplomacy[0] {
                DiplomaticRelation::Enemy => DiplomaticRelation::Neutral,
                _ => DiplomaticRelation::Enemy,
            };
        }
    }
    scrambled
}
