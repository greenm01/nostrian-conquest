//! ECMAINT's orders pass for the empires it plays (`24a3:3cc6`): rogue
//! empires and human empires on autopilot. It runs once a year before the
//! planet pass and the ticks, so the new orders move the fleets this year.
//! The rules are in `docs/dev/ec-movement-spec.md`, "Rogue And Autopilot
//! Orders".

use super::super::{FleetRemovalRemapInfo, apply_fleet_removal_remap};
use crate::CoreGameData;
use nc_data::{
    Coords, DiplomaticRelation, GameRng, Order, PlayerLifecycleState, PlayerMode, RNG_TAG_ORDERS,
    Real48, TerminalOutcome, recompute_max_speed, set_diplomatic_relation_toward, set_rogue,
};

/// Who the pass is playing for. An autopilot empire's fleets are re-ordered
/// only from Hold; a rogue empire's all are.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Governor {
    Rogue,
    Autopilot,
}

/// Give every empire ECMAINT plays its orders for the year. A civil-disorder
/// empire with no world becomes a rogue first. Fleets removed on the way (a
/// planetless rogue disbands its unarmed ones) are compacted out of the
/// fleet table; the returned remap, present only when some were, names where
/// the others went.
pub(super) fn process_home_orders(
    game_data: &mut CoreGameData,
    campaign_seed: u64,
    player_lifecycle_states: &[PlayerLifecycleState],
) -> Result<Option<FleetRemovalRemapInfo>, Box<dyn std::error::Error>> {
    convert_planetless_civil_disorder(game_data, player_lifecycle_states);
    let mut disbanded = vec![false; game_data.fleets.len()];
    for player_idx in 0..game_data.players.len() {
        let owner = (player_idx + 1) as u8;
        let player = &game_data.players[player_idx];
        match player.mode {
            // The count is the one the last run saved (PLAYER `0x50`).
            PlayerMode::Rogue if player.planet_count > 0 => {
                fleet_orders(game_data, owner, Governor::Rogue);
                starbase_orders(game_data, owner);
            }
            PlayerMode::Rogue => {
                declare_humans_enemies(game_data, owner);
                patrol_orders(game_data, owner, campaign_seed, &mut disbanded);
            }
            PlayerMode::Human if player.autopilot => {
                fleet_orders(game_data, owner, Governor::Autopilot);
            }
            _ => {}
        }
    }
    Ok(disbanded
        .contains(&true)
        .then(|| apply_fleet_removal_remap(game_data, &disbanded)))
}

/// `24a3:0c2a`. At the start of the pass, before any empire is played, every
/// civil-disorder empire with no world becomes a rogue ([`set_rogue`]); its
/// stances stay. An empire whose player is owed a final review keeps the
/// handle that review finds it by.
fn convert_planetless_civil_disorder(
    game_data: &mut CoreGameData,
    player_lifecycle_states: &[PlayerLifecycleState],
) {
    for (player_idx, player) in game_data.players.iter_mut().enumerate() {
        if player.mode != PlayerMode::CivilDisorder || player.planet_count != 0 {
            continue;
        }
        let review_owed = player_lifecycle_states
            .get(player_idx)
            .is_some_and(|lifecycle| lifecycle.terminal_outcome == TerminalOutcome::Defeated);
        set_rogue(player, review_owed);
    }
}

/// `24a3:25f7`. Each fleet the governor orders gets the empire's best world
/// as its target, picked by travel distance per point of present production
/// ([`CoreGameData::nearest_productive_world`]), with target depth 1. In
/// orbit there with warships it takes Guard/Blockade at rest, ROE 10 for a
/// rogue and 6 on autopilot when it was 0. In orbit without warships it
/// keeps its order. Anywhere else it takes Seek Home at its maximum speed,
/// and a rogue's ROE becomes 0. Then every fleet on Guard/Blockade at rest,
/// whoever ordered it, takes the first unescorted starbase in its orbit.
fn fleet_orders(game_data: &mut CoreGameData, owner: u8, governor: Governor) {
    for fleet_idx in 0..game_data.fleets.len() {
        let fleet = &game_data.fleets[fleet_idx];
        if fleet.owner.0 != owner {
            continue;
        }
        let ordered = governor == Governor::Rogue || fleet.order == Order::HoldPosition;
        if let Some(world) = ordered
            .then(|| game_data.nearest_productive_world(fleet_idx + 1))
            .flatten()
        {
            game_data.release_starbase_escort(fleet_idx);
            send_home(&mut game_data.fleets[fleet_idx], world, governor);
        }
        let fleet = &game_data.fleets[fleet_idx];
        if fleet.order == Order::GuardBlockadeWorld && fleet.speed == 0 {
            guard_unescorted_starbase(game_data, fleet_idx);
        }
    }
}

fn send_home(fleet: &mut nc_data::Fleet, world: [u8; 2], governor: Governor) {
    fleet.end_escape();
    fleet.target = Coords::from_array(world);
    fleet.target_depth = 1;
    if fleet.sector.to_array() != world || fleet.depth.round() != 1 {
        fleet.order = Order::SeekHome;
        fleet.speed = fleet.max_speed;
        if governor == Governor::Rogue {
            fleet.rules_of_engagement = 0;
        }
        return;
    }
    let ships = &fleet.ships;
    if ships.battleships + ships.cruisers + ships.destroyers == 0 {
        return;
    }
    fleet.order = Order::GuardBlockadeWorld;
    fleet.speed = 0;
    recompute_max_speed(fleet);
    if governor == Governor::Rogue {
        fleet.rules_of_engagement = 10;
    } else if fleet.rules_of_engagement == 0 {
        fleet.rules_of_engagement = 6;
    }
}

/// A fleet on Guard/Blockade at rest takes the first starbase of its empire
/// that rests in orbit in its sector with no escort: the fleet's order
/// becomes Guard Starbase and the base's escort word the fleet's number. A
/// base rests when it sits at its target, whose depth is the layer it is in.
fn guard_unescorted_starbase(game_data: &mut CoreGameData, fleet_idx: usize) {
    let fleet = &mut game_data.fleets[fleet_idx];
    let Some(base) = game_data
        .bases
        .iter_mut()
        .filter(|base| {
            base.owner == fleet.owner
                && base.base_id.0 != 0
                && base.escort.0 == 0
                && base.sector == fleet.sector
                && base.depth.round() == 1
                && base.target == base.sector
                && i32::from(base.target_depth) == base.depth.round()
        })
        .min_by_key(|base| base.base_id.0)
    else {
        return;
    };
    base.escort = fleet.fleet_number;
    fleet.end_escape();
    fleet.order = Order::GuardStarbase;
    fleet.mission_target = base.base_id.0;
}

/// `24a3:28fe`. A rogue empire's starbases take the same pick from their own
/// position: the world becomes the target with depth 1, and the base is under
/// way unless it already rests in orbit there.
fn starbase_orders(game_data: &mut CoreGameData, owner: u8) {
    for base_idx in 0..game_data.bases.len() {
        let base = &game_data.bases[base_idx];
        if base.owner.0 != owner || base.base_id.0 == 0 {
            continue;
        }
        let Some(world) = game_data.nearest_productive_world_for_base(base_idx + 1) else {
            continue;
        };
        let base = &mut game_data.bases[base_idx];
        base.target = Coords::from_array(world);
        base.target_depth = 1;
        base.moving = base.sector.to_array() != world || base.depth.round() != 1;
    }
}

/// `24a3:0c2a`. A rogue empire with no world marks every human empire its
/// enemy, each year; nothing takes the mark back.
fn declare_humans_enemies(game_data: &mut CoreGameData, owner: u8) {
    for human in 1..=game_data.players.len() as u8 {
        if human != owner && game_data.players[usize::from(human) - 1].mode == PlayerMode::Human {
            set_diplomatic_relation_toward(
                &mut game_data.players[usize::from(owner) - 1],
                human,
                DiplomaticRelation::Enemy,
            );
        }
    }
}

/// `24a3:0dbc`. A rogue empire with no world has nowhere to go home to. Its
/// fleets with warships patrol deep space at ROE 10 and their maximum speed.
/// A fleet whose target is its own sector draws a new one at random, each
/// axis from 1 to 9 times the whole part of the root of the player count, and
/// draws again while it lands on the sector the fleet is in; a fleet already
/// bound elsewhere keeps its target. Its fleets without warships and its
/// starbases are disbanded. An empire with no fleet is left as it is.
fn patrol_orders(
    game_data: &mut CoreGameData,
    owner: u8,
    campaign_seed: u64,
    disbanded: &mut [bool],
) {
    if !game_data.fleets.iter().any(|fleet| fleet.owner.0 == owner) {
        return;
    }
    let reach = 9 * Real48::from_int(i32::from(game_data.conquest.player_count))
        .sqrt()
        .trunc() as u64;
    let mut rng = GameRng::from_context(
        campaign_seed,
        RNG_TAG_ORDERS,
        &[u64::from(game_data.conquest.game_year), u64::from(owner)],
    );
    for fleet_idx in 0..game_data.fleets.len() {
        if game_data.fleets[fleet_idx].owner.0 != owner {
            continue;
        }
        game_data.release_starbase_escort(fleet_idx);
        let fleet = &mut game_data.fleets[fleet_idx];
        let ships = &fleet.ships;
        if ships.battleships + ships.cruisers + ships.destroyers == 0 {
            disbanded[fleet_idx] = true;
            continue;
        }
        fleet.end_escape();
        fleet.rules_of_engagement = 10;
        fleet.order = Order::PatrolSector;
        fleet.target_depth = 0;
        while fleet.target == fleet.sector {
            let x = rng.below(reach) as u8 + 1;
            let y = rng.below(reach) as u8 + 1;
            fleet.target = Coords { x, y };
        }
        fleet.speed = fleet.max_speed;
    }
    for base_idx in 0..game_data.bases.len() {
        if game_data.bases[base_idx].owner.0 == owner && game_data.bases[base_idx].base_id.0 != 0 {
            game_data.unlink_base(base_idx);
            game_data.bases[base_idx] = nc_data::Starbase::default();
        }
    }
}
