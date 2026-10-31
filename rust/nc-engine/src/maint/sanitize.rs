use super::InvalidPlayerStateEvent;
use crate::{
    CoreGameData, FleetOrderValidationError, FleetPlayerInputValidationError, Order,
    PlayerDiplomacyValidationError, ProductionItemKind,
};
use nc_data::{
    DiplomaticRelation, set_diplomatic_relation_toward, stardock_slot_kind,
    stored_diplomatic_relation,
};

/// ECMAINT checks no order of the empires it plays. A rogue or autopilot
/// fleet on Guard/Blockade or Guard Starbase without warships stays on it,
/// and the orders pass lets it take a starbase (`24a3:25f7`).
fn ecmaint_leaves_unarmed_guard_alone(
    game_data: &CoreGameData,
    fleet_idx: usize,
    reason: FleetOrderValidationError,
) -> bool {
    let fleet = &game_data.fleets[fleet_idx];
    reason == FleetOrderValidationError::MissingCombatShips
        && matches!(
            fleet.order,
            Order::GuardBlockadeWorld | Order::GuardStarbase
        )
        && game_data
            .players
            .get(usize::from(fleet.owner.0).saturating_sub(1))
            .is_some_and(|player| player.played_by_ecmaint())
}

fn capability_loss_requires_abort_and_seek_home(reason: FleetOrderValidationError) -> bool {
    matches!(
        reason,
        FleetOrderValidationError::MissingCombatShips
            | FleetOrderValidationError::MissingScoutShip
            | FleetOrderValidationError::MissingEtac
            | FleetOrderValidationError::MissingLoadedTroopTransports
    )
}

pub(super) fn sanitize_invalid_player_inputs(
    game_data: &mut CoreGameData,
) -> Vec<InvalidPlayerStateEvent> {
    let mut events = Vec::new();

    for fleet_idx in 0..game_data.fleets.len() {
        let mut rescan = true;
        while rescan {
            rescan = false;
            let fleet = &game_data.fleets[fleet_idx];
            let order_code = fleet.order.code();
            let target = fleet.target.to_array();
            let owner_empire = fleet.owner.0;
            let coords = fleet.sector.to_array();
            if let Err(reason) = game_data.validate_fleet_player_inputs(
                fleet_idx + 1,
                order_code,
                target,
                None,
                None,
            ) {
                match reason {
                    FleetPlayerInputValidationError::InvalidOrder(reason) => {
                        // A JoinAnotherFleet fleet with join_host_fleet_id == 0 was
                        // remapped by cull_empty_fleets after its host was culled. That is
                        // an engine-set sentinel, not an invalid player input.  Leave the
                        // fleet alone here; process_join_host_updates will handle it via
                        // the HostDestroyed path.
                        let is_post_cull_join_sentinel =
                            matches!(reason, FleetOrderValidationError::InvalidJoinHost)
                                && game_data.fleets[fleet_idx].mission_target == 0;
                        let should_sanitize = !is_post_cull_join_sentinel
                            && matches!(
                                reason,
                                FleetOrderValidationError::UnknownOrderCode(_)
                                    | FleetOrderValidationError::MissingCombatShips
                                    | FleetOrderValidationError::MissingScoutShip
                                    | FleetOrderValidationError::MissingEtac
                                    | FleetOrderValidationError::MissingLoadedTroopTransports
                                    | FleetOrderValidationError::InvalidJoinHost
                                    | FleetOrderValidationError::TargetOwnedByFleetEmpire
                            );
                        if !should_sanitize
                            || ecmaint_leaves_unarmed_guard_alone(game_data, fleet_idx, reason)
                        {
                            break;
                        }
                        // ECMAINT holds a patrol that has lost its warships.
                        let seeks_home = capability_loss_requires_abort_and_seek_home(reason)
                            && game_data.fleets[fleet_idx].order != Order::PatrolSector;
                        let retreat_target = seeks_home
                            .then(|| super::combat::nearest_friendly_world(game_data, fleet_idx))
                            .flatten();
                        if seeks_home {
                            super::combat::abort_mission_to_seek_home_or_hold(
                                game_data,
                                fleet_idx,
                                retreat_target,
                            );
                        } else {
                            super::combat::hold_in_place(&mut game_data.fleets[fleet_idx]);
                        }
                        events.push(InvalidPlayerStateEvent::FleetMission {
                            fleet_idx,
                            owner_empire,
                            order_code,
                            coords,
                            reason,
                        });
                    }
                    FleetPlayerInputValidationError::SpeedExceedsMaximum { max, .. } => {
                        game_data.fleets[fleet_idx].speed = max;
                        events.push(InvalidPlayerStateEvent::FleetInput {
                            fleet_idx,
                            owner_empire,
                            coords,
                            reason,
                        });
                        rescan = true;
                    }
                    FleetPlayerInputValidationError::RulesOfEngagementOutOfRange { .. } => {
                        game_data.fleets[fleet_idx].rules_of_engagement = 10;
                        events.push(InvalidPlayerStateEvent::FleetInput {
                            fleet_idx,
                            owner_empire,
                            coords,
                            reason,
                        });
                        rescan = true;
                    }
                    FleetPlayerInputValidationError::NonCombatFleetMustUseZeroRoe { .. } => {
                        game_data.fleets[fleet_idx].rules_of_engagement = 0;
                        events.push(InvalidPlayerStateEvent::FleetInput {
                            fleet_idx,
                            owner_empire,
                            coords,
                            reason,
                        });
                        rescan = true;
                    }
                }
            }
        }
    }

    for planet_idx in 0..game_data.planets.len() {
        let (owner_empire, coords, reason_opt) = {
            let planet = &game_data.planets[planet_idx];
            (
                planet.owner.0,
                planet.sector.to_array(),
                game_data
                    .validate_planet_player_inputs(planet_idx + 1)
                    .err(),
            )
        };
        if let Some(reason) = reason_opt {
            let planet = &mut game_data.planets[planet_idx];
            for slot in 0..10 {
                let build_count = planet.build_queue[slot].points_owed;
                let build_kind = planet.build_queue[slot].kind.code();
                if build_count == 0 && build_kind != 0
                    || build_count != 0
                        && matches!(
                            ProductionItemKind::from_code(build_kind),
                            ProductionItemKind::Unknown(_)
                        )
                {
                    planet.build_queue[slot].points_owed = 0;
                    planet.build_queue[slot].kind = ProductionItemKind::Unknown(0);
                }
            }
            for slot in 0..crate::STARDOCK_SLOT_COUNT {
                let stardock_count = u16::try_from(planet.stardock[slot]).unwrap_or(u16::MAX);
                let stardock_kind = stardock_slot_kind(planet, slot).code();
                if stardock_count == 0 && stardock_kind != 0
                    || stardock_count != 0
                        && matches!(
                            ProductionItemKind::from_code(stardock_kind),
                            ProductionItemKind::Unknown(_)
                        )
                {
                    planet.stardock[slot] = 0;
                }
            }
            events.push(InvalidPlayerStateEvent::PlanetInput {
                planet_idx,
                owner_empire,
                coords,
                reason,
            });
        }
    }

    for player_idx in 0..game_data.players.len() {
        let tax_rate = game_data.players[player_idx].tax_rate;
        if tax_rate > 100 {
            game_data.players[player_idx].tax_rate = 100;
            events.push(InvalidPlayerStateEvent::PlayerTaxRate {
                player_idx,
                owner_empire: (player_idx + 1) as u8,
                tax_rate,
            });
        }
        let player_count = game_data.players.len() as u8;
        for target_empire in 1..=player_count {
            let reason = {
                let player = &game_data.players[player_idx];
                let raw = stored_diplomatic_relation(player, target_empire)
                    .map_or(0, DiplomaticRelation::code);
                let empire = (player_idx + 1) as u8;
                if target_empire == empire {
                    (raw != 0).then_some(PlayerDiplomacyValidationError::SelfTarget { empire })
                } else if raw != 0x00 && raw != 0x01 {
                    Some(PlayerDiplomacyValidationError::InvalidStoredRelationByte {
                        target_empire,
                        raw,
                    })
                } else {
                    None
                }
            };
            if let Some(reason) = reason {
                let _ = set_diplomatic_relation_toward(
                    &mut game_data.players[player_idx],
                    target_empire,
                    DiplomaticRelation::Neutral,
                );
                events.push(InvalidPlayerStateEvent::DiplomacyInput {
                    player_idx,
                    owner_empire: (player_idx + 1) as u8,
                    reason,
                });
            }
        }
    }

    events
}
