use super::InvalidPlayerStateEvent;
use crate::{
    CoreGameData, FleetOrderValidationError, FleetPlayerInputValidationError, Order,
    PlayerDiplomacyValidationError, ProductionItemKind,
};
use nc_data::{
    Coords, DiplomaticRelation, set_diplomatic_relation_toward, set_mission_target,
    stardock_slot_kind, stored_diplomatic_relation,
};

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
            let order_code = fleet.order.to_raw();
            let target = fleet.target.to_array();
            let owner_empire_raw = fleet.owner.0;
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
                        // A JoinAnotherFleet fleet with join_host_fleet_id_raw == 0 was
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
                        if !should_sanitize {
                            break;
                        }
                        let retreat_target = capability_loss_requires_abort_and_seek_home(reason)
                            .then(|| super::combat::nearest_friendly_world(game_data, fleet_idx))
                            .flatten();
                        let fleet = &mut game_data.fleets[fleet_idx];
                        if capability_loss_requires_abort_and_seek_home(reason) {
                            super::combat::abort_mission_to_seek_home_or_hold(
                                fleet,
                                retreat_target,
                            );
                        } else {
                            fleet.order = Order::HoldPosition;
                            fleet.speed = 0;
                            fleet.target = Coords::from_array(coords);
                            set_mission_target(fleet, 0, 0);
                        }
                        events.push(InvalidPlayerStateEvent::FleetMission {
                            fleet_idx,
                            owner_empire_raw,
                            order_code_raw: order_code,
                            coords,
                            reason,
                        });
                    }
                    FleetPlayerInputValidationError::SpeedExceedsMaximum { max, .. } => {
                        game_data.fleets[fleet_idx].speed = max;
                        events.push(InvalidPlayerStateEvent::FleetInput {
                            fleet_idx,
                            owner_empire_raw,
                            coords,
                            reason,
                        });
                        rescan = true;
                    }
                    FleetPlayerInputValidationError::RulesOfEngagementOutOfRange { .. } => {
                        game_data.fleets[fleet_idx].rules_of_engagement = 10;
                        events.push(InvalidPlayerStateEvent::FleetInput {
                            fleet_idx,
                            owner_empire_raw,
                            coords,
                            reason,
                        });
                        rescan = true;
                    }
                    FleetPlayerInputValidationError::NonCombatFleetMustUseZeroRoe { .. } => {
                        game_data.fleets[fleet_idx].rules_of_engagement = 0;
                        events.push(InvalidPlayerStateEvent::FleetInput {
                            fleet_idx,
                            owner_empire_raw,
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
        let (owner_empire_raw, coords, reason_opt) = {
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
                let build_kind = planet.build_queue[slot].kind.to_raw();
                if build_count == 0 && build_kind != 0
                    || build_count != 0
                        && matches!(
                            ProductionItemKind::from_raw(build_kind),
                            ProductionItemKind::Unknown(_)
                        )
                {
                    planet.build_queue[slot].points_owed = 0;
                    planet.build_queue[slot].kind = ProductionItemKind::Unknown(0);
                }
            }
            for slot in 0..crate::STARDOCK_SLOT_COUNT {
                let stardock_count = u16::try_from(planet.stardock[slot]).unwrap_or(u16::MAX);
                let stardock_kind = stardock_slot_kind(planet, slot).to_raw();
                if stardock_count == 0 && stardock_kind != 0
                    || stardock_count != 0
                        && matches!(
                            ProductionItemKind::from_raw(stardock_kind),
                            ProductionItemKind::Unknown(_)
                        )
                {
                    planet.stardock[slot] = 0;
                }
            }
            events.push(InvalidPlayerStateEvent::PlanetInput {
                planet_idx,
                owner_empire_raw,
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
                owner_empire_raw: (player_idx + 1) as u8,
                tax_rate,
            });
        }
        let player_count = game_data.players.len() as u8;
        for target_empire_raw in 1..=player_count {
            let reason = {
                let player = &game_data.players[player_idx];
                let raw = stored_diplomatic_relation(player, target_empire_raw)
                    .map_or(0, DiplomaticRelation::to_raw);
                let empire_raw = (player_idx + 1) as u8;
                if target_empire_raw == empire_raw {
                    (raw != 0).then_some(PlayerDiplomacyValidationError::SelfTarget { empire_raw })
                } else if raw != 0x00 && raw != 0x01 {
                    Some(PlayerDiplomacyValidationError::InvalidStoredRelationByte {
                        target_empire_raw,
                        raw,
                    })
                } else {
                    None
                }
            };
            if let Some(reason) = reason {
                let _ = set_diplomatic_relation_toward(
                    &mut game_data.players[player_idx],
                    target_empire_raw,
                    DiplomaticRelation::Neutral,
                );
                events.push(InvalidPlayerStateEvent::DiplomacyInput {
                    player_idx,
                    owner_empire_raw: (player_idx + 1) as u8,
                    reason,
                });
            }
        }
    }

    events
}
