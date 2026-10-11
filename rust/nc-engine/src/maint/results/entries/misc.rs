use nc_data::{CoreGameData, MaintenanceEvents, Mission, MissionOutcome};

use crate::maint::results::combat::*;
use crate::maint::results::entries::{
    ReportEntry, ReportTarget, narrative_phase_for_report_text, occupied_result_viewers,
};
use crate::maint::results::format::*;
use crate::maint::results::join::build_join_summary_entries;
use crate::maint::results::mod_constants::*;
use crate::maint::results::render::mission_retarget_source;
use crate::maint::results::validation::*;

pub fn push_misc_entries(
    entries: &mut Vec<ReportEntry>,
    game_data: &CoreGameData,
    events: &MaintenanceEvents,
    year: u16,
) {
    // ----- Colonization events -----
    for event in &events.colonization_events {
        let (planet_idx, colonizer_empire, event_week) = match *event {
            nc_data::ColonizationResolvedEvent::Succeeded {
                planet_idx,
                colonizer_empire,
                stardate_week,
                ..
            } => (planet_idx, colonizer_empire, stardate_week),
            nc_data::ColonizationResolvedEvent::BlockedByOwner {
                planet_idx,
                colonizer_empire,
                stardate_week,
                ..
            } => (planet_idx, colonizer_empire, stardate_week),
            nc_data::ColonizationResolvedEvent::Aborted { .. } => continue,
        };
        let Some(planet) = game_data.planets.get(planet_idx) else {
            continue;
        };
        let [x, y] = planet.sector.to_array();
        let fleet_idx = match *event {
            nc_data::ColonizationResolvedEvent::Succeeded { fleet_idx, .. } => fleet_idx,
            nc_data::ColonizationResolvedEvent::BlockedByOwner { fleet_idx, .. } => fleet_idx,
            nc_data::ColonizationResolvedEvent::Aborted { .. } => continue,
        };
        let source =
            owned_fleet_source_clause_from_idx(game_data, fleet_idx, &format!("System({x},{y})"));
        let header = report_header(&source, event_week, year);
        let body = match *event {
            nc_data::ColonizationResolvedEvent::Succeeded { .. } => {
                " Colonization mission report: We have arrived at our target world, successfully terraformed it, and have started a new colony. We await new orders...".to_string()
            }
            nc_data::ColonizationResolvedEvent::BlockedByOwner { owner_empire, .. } => format!(
                " Colonization mission report: We have entered System({x},{y}) and have determined that aliens are already living on the world found within! We have gone ahead and performed a long range viewing analysis and have determined that the world is owned by {} and has a potential of {} points. We are aborting our mission and are leaving the alien solar system.",
                classic_empire_clause(game_data, owner_empire),
                u16::from(planet.potential_production),
            ),
            nc_data::ColonizationResolvedEvent::Aborted { .. } => continue,
        };
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x09,
            tail: RESULTS_TAIL_COLONIZATION,
            target: ReportTarget::Both {
                recipient: colonizer_empire,
            },
            repeat_next_pointer: false,
            stardate_week: event_week,
            narrative_phase: narrative_phase_for_report_text(&body),
        });
    }

    // ----- Civil disorder events -----
    for event in &events.civil_disorder_events {
        let source = "From your Fleet Command Center:";
        let header = report_header(source, event.stardate_week, year);
        let body = format!(
            " With all of our controlled worlds lost and no immediate means of recovery, the empire of \"{}\" has fallen into civil disorder. Remaining forces are scattered and unreliable.",
            event.prior_label,
        );
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x06,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both {
                recipient: event.reporting_empire,
            },
            repeat_next_pointer: false,
            stardate_week: event.stardate_week,
            narrative_phase: narrative_phase_for_report_text(&body),
        });
    }

    // ----- Campaign outlook events -----
    // Every joined empire hears it; only the leader reads it as its own claim.
    for event in &events.campaign_outlook_events {
        let source = "From your Fleet Command Center:";
        let header = report_header(source, event.stardate_week, year);
        for recipient in occupied_result_viewers(game_data) {
            let body = if recipient == event.empire {
                " We now stand as the sole remaining serious contender for the imperial throne. Other empires may still persist, but none currently appear capable of challenging our claim.".to_string()
            } else {
                format!(
                    " {} now stands as the sole remaining serious contender for the imperial throne. No other empire, ours included, currently appears capable of challenging its claim.",
                    empire_label(game_data, event.empire),
                )
            };
            entries.push(ReportEntry {
                text: format!("{header}{body}"),
                kind: 0x06,
                tail: RESULTS_TAIL_FLEET,
                target: ReportTarget::Both { recipient },
                repeat_next_pointer: false,
                stardate_week: event.stardate_week,
                narrative_phase: narrative_phase_for_report_text(&body),
            });
        }
    }

    // Campaign outcome events get no report of their own: the targeted
    // winner-declaration notices below already reach the emperor and every
    // empire still in the campaign, and already defeated empires must not
    // receive a repeated winner notice.

    // ----- Empire elimination events -----
    for event in &events.empire_elimination_events {
        let source = "From your Fleet Command Center:";
        let header = report_header(source, event.stardate_week, year);
        let defeated = empire_label(game_data, event.defeated_empire);
        let cause_clause = match event.cause {
            nc_data::EmpireEliminationCause::LastPlanetLost => match event.coords {
                Some([x, y]) => format!(
                    " Our last world has been lost at System({x},{y}), and no recovery force remains able to restore our empire."
                ),
                None => " Our last world has been lost, and no recovery force remains able to restore our empire.".to_string(),
            },
            nc_data::EmpireEliminationCause::LastRecoveryForceDestroyed => {
                " Our final remaining recovery force has been destroyed while we are planetless. We can no longer continue the campaign.".to_string()
            }
            nc_data::EmpireEliminationCause::RecoveryWindowExpired => {
                " We failed to reclaim a world before our recovery window expired. We can no longer continue the campaign.".to_string()
            }
        };
        let loser_body = format!(
            " ALERT: Empire defeated! We have been defeated.{cause_clause} Remaining scattered forces are no longer under player command."
        );
        entries.push(ReportEntry {
            text: format!("{header}{loser_body}"),
            kind: 0x06,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both {
                recipient: event.defeated_empire,
            },
            repeat_next_pointer: false,
            stardate_week: event.stardate_week,
            narrative_phase: narrative_phase_for_report_text(&loser_body),
        });

        if let Some(victor_empire) = event.victor_empire {
            let victor_body = match event.cause {
                nc_data::EmpireEliminationCause::LastPlanetLost => match event.coords {
                    Some([x, y]) => format!(
                        " ALERT: Enemy empire defeated! Our attack at System({x},{y}) has delivered the final blow against {defeated}. That empire has now been eliminated from the campaign."
                    ),
                    None => format!(
                        " ALERT: Enemy empire defeated! We have delivered the final blow against {defeated}. That empire has now been eliminated from the campaign."
                    ),
                },
                nc_data::EmpireEliminationCause::LastRecoveryForceDestroyed => format!(
                    " ALERT: Enemy empire defeated! By destroying the last remaining recovery force of {defeated}, we have eliminated that empire from the campaign."
                ),
                nc_data::EmpireEliminationCause::RecoveryWindowExpired => format!(
                    " ALERT: Enemy empire defeated! {defeated} has failed to recover before its final window expired and is now eliminated from the campaign."
                ),
            };
            entries.push(ReportEntry {
                text: format!("{header}{victor_body}"),
                kind: 0x06,
                tail: RESULTS_TAIL_FLEET,
                target: ReportTarget::Both {
                    recipient: victor_empire,
                },
                repeat_next_pointer: false,
                stardate_week: event.stardate_week,
                narrative_phase: narrative_phase_for_report_text(&victor_body),
            });
        }
    }

    // ----- Winner declaration notices -----
    for event in &events.game_victory_notice_events {
        let source = "From your Fleet Command Center:";
        let header = report_header(source, event.stardate_week, year);
        let winner = empire_label(game_data, event.winner_empire);
        let body = if event.recipient_empire == event.winner_empire {
            format!(
                " ALERT: Victory declared! We have been recognized as Emperor. The campaign is over. You may continue to review the final game state in survey mode, but no further orders will be accepted."
            )
        } else {
            format!(
                " ALERT: Campaign ended! {winner} has been recognized as Emperor and the game is over. You may review the final results one last time, but your command access is now closed."
            )
        };
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x06,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both {
                recipient: event.recipient_empire,
            },
            repeat_next_pointer: false,
            stardate_week: event.stardate_week,
            narrative_phase: narrative_phase_for_report_text(&body),
        });
    }

    // ----- Fleet defection events -----
    for event in &events.fleet_defection_events {
        let source = "From your Fleet Command Center:";
        let header = report_header(source, event.stardate_week, year);
        let body = format!(
            " We have lost all contact with the {}. In the chaos of civil disorder, the surviving crews have defected and no longer answer to central command.",
            fleet_label(event.fleet_number),
        );
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x06,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both {
                recipient: event.reporting_empire,
            },
            repeat_next_pointer: false,
            stardate_week: event.stardate_week,
            narrative_phase: narrative_phase_for_report_text(&body),
        });
    }

    // ----- Salvage events -----
    for event in &events.salvage_events {
        let (owner_empire, event_week, source, body) = match *event {
            nc_data::SalvageResolvedEvent::Succeeded {
                fleet_number,
                owner_empire,
                planet_idx,
                coords,
                recovered_points,
                stardate_week,
                ..
            } => {
                let [x, y] = coords;
                let planet_name = game_data
                    .planets
                    .get(planet_idx)
                    .map(|planet| planet.name.clone())
                    .unwrap_or_else(|| "the target world".to_string());
                (
                    owner_empire,
                    stardate_week,
                    owned_fleet_source_clause(Some(fleet_number), &format!("System({x},{y})")),
                    format!(
                        " Salvage mission report: We have arrived at planet \"{planet_name}\" in System({x},{y}) and have begun salvaging our fleet. We estimate that our fleet will yield {recovered_points} production point(s)."
                    ),
                )
            }
            nc_data::SalvageResolvedEvent::Failed {
                fleet_idx,
                owner_empire,
                planet_idx: Some(planet_idx),
                coords,
                reason: nc_data::SalvageFailureReason::PlanetNotOwned,
                stardate_week,
                ..
            } => {
                let [x, y] = coords;
                let planet_name = game_data
                    .planets
                    .get(planet_idx)
                    .map(|planet| planet.name.clone())
                    .unwrap_or_else(|| "the target world".to_string());
                (
                    owner_empire,
                    stardate_week,
                    owned_fleet_source_clause_from_idx(
                        game_data,
                        fleet_idx,
                        &format!("System({x},{y})"),
                    ),
                    format!(
                        " Salvage mission report: We have arrived at planet \"{planet_name}\" in System({x},{y}), but it is not under our control so we cannot salvage our fleet there."
                    ),
                )
            }
            nc_data::SalvageResolvedEvent::Failed {
                fleet_idx,
                owner_empire,
                coords,
                reason: nc_data::SalvageFailureReason::NoPlanetAtTarget,
                stardate_week,
                ..
            } => {
                let [x, y] = coords;
                (
                    owner_empire,
                    stardate_week,
                    owned_fleet_source_clause_from_idx(
                        game_data,
                        fleet_idx,
                        &format!("System({x},{y})"),
                    ),
                    " Salvage mission report: We found no planet to salvage at the assigned destination and are awaiting new orders.".to_string(),
                )
            }
            _ => continue,
        };
        let header = report_header(&source, event_week, year);
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x05,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both {
                recipient: owner_empire,
            },
            repeat_next_pointer: false,
            stardate_week: event_week,
            narrative_phase: narrative_phase_for_report_text(&body),
        });
    }

    // ----- Invalid player state events -----
    for event in &events.invalid_player_state_events {
        let (owner_empire, source, body) = match *event {
            nc_data::InvalidPlayerStateEvent::FleetMission {
                fleet_idx,
                owner_empire,
                order_code_raw,
                coords,
                reason,
            } => {
                let order_name = nc_data::Order::from_code(order_code_raw)
                    .display_label()
                    .to_lowercase();
                let body = if capability_loss_invalid_order_reason(reason) {
                    format!(
                        " Hostile action forced us to abort the {order_name} mission because {}. The fleet is {}.",
                        fleet_order_validation_reason_text(reason),
                        aborted_mission_follow_on_text_from_idx(game_data, fleet_idx, owner_empire)
                    )
                } else {
                    format!(
                        " The {order_name} order was canceled because {}. The fleet is holding position and awaiting orders.",
                        fleet_order_validation_reason_text(reason)
                    )
                };
                (
                    owner_empire,
                    owned_fleet_source_clause_from_idx(
                        game_data,
                        fleet_idx,
                        &format!("Sector({},{})", coords[0], coords[1]),
                    ),
                    body,
                )
            }
            nc_data::InvalidPlayerStateEvent::FleetInput {
                fleet_idx,
                owner_empire,
                coords,
                reason,
                ..
            } => (
                owner_empire,
                owned_fleet_source_clause_from_idx(
                    game_data,
                    fleet_idx,
                    &format!("Sector({},{})", coords[0], coords[1]),
                ),
                format!(
                    " Invalid fleet input was corrected because {}.",
                    fleet_player_input_validation_reason_text(reason)
                ),
            ),
            nc_data::InvalidPlayerStateEvent::PlanetInput {
                owner_empire,
                coords,
                reason,
                ..
            } => (
                owner_empire,
                format!("From planet in System({},{}) :", coords[0], coords[1]),
                format!(
                    " Invalid planetary input was cleared because {}.",
                    planet_input_validation_reason_text(reason)
                ),
            ),
            nc_data::InvalidPlayerStateEvent::PlayerTaxRate {
                owner_empire,
                tax_rate,
                ..
            } => (
                owner_empire,
                "From your central administration:".to_string(),
                format!(
                    " Tax rate input {}% for {} was invalid and has been clamped to 100%.",
                    tax_rate,
                    empire_label(game_data, owner_empire)
                ),
            ),
            nc_data::InvalidPlayerStateEvent::DiplomacyInput {
                owner_empire,
                reason,
                ..
            } => (
                owner_empire,
                "From your foreign ministry:".to_string(),
                format!(
                    " Invalid diplomacy input for {} was reset because {}.",
                    empire_label(game_data, owner_empire),
                    diplomacy_input_validation_reason_text(reason)
                ),
            ),
        };
        if owner_empire == 0 {
            continue;
        }
        // Invalid state events don't have stardate_week on the event itself; use year start.
        let header = report_header(&source, None, year);
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x05,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both {
                recipient: owner_empire,
            },
            repeat_next_pointer: false,
            stardate_week: None,
            narrative_phase: narrative_phase_for_report_text(&body),
        });
    }

    // ----- Fleet merge events -----
    // Rendezvous events: one consolidated report per survivor fleet.
    {
        let rendezvous_arrived: std::collections::HashSet<usize> = events
            .mission_events
            .iter()
            .filter(|e| e.kind == Mission::RendezvousSector && e.outcome == MissionOutcome::Arrived)
            .map(|e| e.fleet_idx)
            .collect();

        let mut rendezvous_groups: std::collections::BTreeMap<(u8, [u8; 2], u8), Vec<u8>> =
            std::collections::BTreeMap::new();
        let mut rendezvous_meta: std::collections::HashMap<(u8, [u8; 2], u8), (usize, Option<u8>)> =
            std::collections::HashMap::new();
        for event in events
            .fleet_merge_events
            .iter()
            .filter(|e| e.kind == Mission::RendezvousSector && e.survivor_side)
        {
            let key = (event.owner_empire, event.coords, event.host_fleet_number);
            rendezvous_groups
                .entry(key)
                .or_default()
                .push(event.absorbed_fleet_number);
            rendezvous_meta
                .entry(key)
                .or_insert((event.fleet_idx, event.stardate_week));
        }

        for (key, absorbed_numbers) in &rendezvous_groups {
            let (owner_empire, coords, host_fleet_number) = *key;
            let [x, y] = coords;
            let (host_fleet_idx, stardate_week) = rendezvous_meta[key];
            let source =
                owned_fleet_source_clause(Some(host_fleet_number), &format!("Sector({x},{y})"));
            let absorbed_text = compact_fleet_number_list(absorbed_numbers);
            let arrived_this_turn = rendezvous_arrived.contains(&host_fleet_idx);
            let body = if arrived_this_turn {
                format!(
                    " Rendezvous mission report: We have arrived at our rendezvous point and are absorbing {absorbed_text}."
                )
            } else {
                format!(
                    " Rendezvous mission report: We are on station at our rendezvous point and are absorbing {absorbed_text}."
                )
            };
            let header = report_header(&source, stardate_week, year);
            entries.push(ReportEntry {
                text: format!("{header}{body}"),
                kind: 0x05,
                tail: RESULTS_TAIL_FLEET,
                target: ReportTarget::Both {
                    recipient: owner_empire,
                },
                repeat_next_pointer: false,
                stardate_week: stardate_week,
                narrative_phase: narrative_phase_for_report_text(&body),
            });
        }
    }

    // ----- Join mission summary -----
    entries.extend(build_join_summary_entries(game_data, events, year));

    // ----- Mission retarget events -----
    for event in &events.mission_retarget_events {
        let source = match *event {
            nc_data::MissionRetargetEvent::Retargeted {
                reporting_fleet_number,
                current_coords,
                ..
            } => mission_retarget_source(reporting_fleet_number, current_coords),
            nc_data::MissionRetargetEvent::Abandoned {
                reporting_fleet_number,
                coords,
                ..
            } => owned_fleet_source_clause(
                reporting_fleet_number,
                &format!("Sector({},{})", coords[0], coords[1]),
            ),
        };
        let (recipient, body) = match *event {
            nc_data::MissionRetargetEvent::Retargeted {
                owner_empire,
                mission: Mission::SeekHome,
                previous_target_coords,
                new_target_coords,
                ..
            } => (
                owner_empire,
                format!(
                    " Seek-Home mission report: Our original destination at Sector({},{}) was lost. We are seeking refuge at Sector({},{}).",
                    previous_target_coords[0],
                    previous_target_coords[1],
                    new_target_coords[0],
                    new_target_coords[1]
                ),
            ),
            nc_data::MissionRetargetEvent::Abandoned {
                owner_empire,
                mission: Mission::SeekHome,
                coords,
                ..
            } => (
                owner_empire,
                format!(
                    " Seek-Home mission report: With no friendly planets remaining, we are holding our position in Sector({},{}) and awaiting orders.",
                    coords[0], coords[1]
                ),
            ),
            nc_data::MissionRetargetEvent::Retargeted {
                owner_empire,
                mission: Mission::GuardStarbase,
                previous_target_coords: _,
                new_target_coords,
                ..
            } => (
                owner_empire,
                format!(
                    " Guard Starbase mission report: The guarded starbase relocated. We are moving to Sector({},{}) to resume escort duty.",
                    new_target_coords[0], new_target_coords[1]
                ),
            ),
            nc_data::MissionRetargetEvent::Abandoned {
                owner_empire,
                mission: Mission::GuardStarbase,
                coords,
                ..
            } => (
                owner_empire,
                format!(
                    " Guard Starbase mission report: The guarded starbase was destroyed or lost. We are holding our position in Sector({},{}) and awaiting orders.",
                    coords[0], coords[1]
                ),
            ),
            _ => continue,
        };
        let header = report_header(&source, None, year);
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x05,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both { recipient },
            repeat_next_pointer: false,
            stardate_week: None,
            narrative_phase: narrative_phase_for_report_text(&body),
        });
    }
}

pub fn push_roe_entries(
    entries: &mut Vec<ReportEntry>,
    game_data: &CoreGameData,
    events: &MaintenanceEvents,
    year: u16,
    consumed_roe_disposition_indices: &std::collections::BTreeSet<usize>,
) {
    // Deduplicate NoEngagement: one report per outcome per enemy per location per turn.
    let mut seen_avoidance: std::collections::HashSet<(
        u8,
        u8,
        [u8; 2],
        nc_data::EncounterDispositionReason,
    )> = std::collections::HashSet::new();
    for (disposition_idx, event) in events.encounter_disposition_events.iter().enumerate() {
        if consumed_roe_disposition_indices.contains(&disposition_idx) {
            continue;
        }
        let (owner_empire, event_week, source, body) = match *event {
            nc_data::EncounterDispositionEvent::NoEngagement {
                fleet_idx,
                owner_empire,
                mission,
                coords,
                friendly_initial,
                friendly_loaded_armies_initial,
                target_empire,
                target_fleet_number,
                small_vessels,
                medium_vessels,
                large_vessels,
                reason,
                stardate_week,
            } => (
                owner_empire,
                stardate_week,
                owned_fleet_source_clause_from_idx(
                    game_data,
                    fleet_idx,
                    &format!("Sector({},{})", coords[0], coords[1]),
                ),
                {
                    let prefix = mission.map(mission_report_prefix).unwrap_or_default();
                    let enemy = if let Some(enemy) =
                        known_hostile_fleet_label(game_data, target_fleet_number, target_empire)
                    {
                        format!("It is {enemy}.")
                    } else {
                        format!(
                            "It belongs to {}.",
                            classic_empire_clause(game_data, target_empire)
                        )
                    };
                    let size_summary = contact_size_summary_from_counts(
                        small_vessels,
                        medium_vessels,
                        large_vessels,
                    );
                    let fleet_desc =
                        if small_vessels == 0 && medium_vessels == 0 && large_vessels == 0 {
                            size_summary
                        } else {
                            format!("{size_summary} of unknown type")
                        };
                    let outcome = match reason {
                        nc_data::EncounterDispositionReason::InterceptFailed => {
                            "We are unable to intercept the enemy."
                        }
                        _ => "In accordance to our ROE, we are avoiding this enemy fleet...",
                    };
                    format!(
                        "{prefix} We had {}. We have located and identified the alien fleet in System({},{}) {} Their fleet contains {fleet_desc}. {outcome}",
                        fleet_force_summary(friendly_initial, friendly_loaded_armies_initial),
                        coords[0],
                        coords[1],
                        enemy,
                    )
                },
            ),
            nc_data::EncounterDispositionEvent::Retreated {
                fleet_idx,
                owner_empire,
                mission,
                coords,
                friendly_initial,
                friendly_loaded_armies_initial,
                target_empire,
                target_fleet_number,
                enemy_initial,
                retreat_target_coords,
                losses_sustained,
                enemy_losses_inflicted,
                stardate_week,
                ..
            } => (
                owner_empire,
                stardate_week,
                owned_fleet_source_clause_from_idx(
                    game_data,
                    fleet_idx,
                    &format!("Sector({},{})", coords[0], coords[1]),
                ),
                {
                    let prefix = mission.map(mission_report_prefix).unwrap_or_default();
                    let loss_clause = roe_retreat_loss_clause(losses_sustained);
                    format!(
                        "{prefix} We engaged {}. We had {}. The alien force contained {}. In accordance with our ROE, we withdrew toward System({},{}) {}. {}",
                        classic_enemy_reference(game_data, target_fleet_number, target_empire),
                        fleet_force_summary(friendly_initial, friendly_loaded_armies_initial),
                        fleet_force_summary(enemy_initial, 0),
                        retreat_target_coords[0],
                        retreat_target_coords[1],
                        loss_clause,
                        enemy_losses_sentence(enemy_losses_inflicted),
                    )
                },
            ),
            nc_data::EncounterDispositionEvent::PursuitFire {
                fleet_idx,
                owner_empire,
                mission,
                coords,
                friendly_initial,
                friendly_loaded_armies_initial,
                target_empire,
                target_fleet_number,
                enemy_initial,
                retreat_target_coords,
                losses_sustained,
                enemy_losses_inflicted,
                stardate_week,
                ..
            } => (
                owner_empire,
                stardate_week,
                owned_fleet_source_clause_from_idx(
                    game_data,
                    fleet_idx,
                    &format!("Sector({},{})", coords[0], coords[1]),
                ),
                {
                    let prefix = mission.map(mission_report_prefix).unwrap_or_default();
                    let loss_clause = roe_retreat_loss_clause(losses_sustained);
                    format!(
                        "{prefix} We had {}. We attempted to disengage from {} but suffered pursuit fire from an alien force containing {}. We withdrew toward System({},{}) {}. {}",
                        fleet_force_summary(friendly_initial, friendly_loaded_armies_initial),
                        classic_enemy_reference(game_data, target_fleet_number, target_empire),
                        fleet_force_summary(enemy_initial, 0),
                        retreat_target_coords[0],
                        retreat_target_coords[1],
                        loss_clause,
                        enemy_losses_sentence(enemy_losses_inflicted),
                    )
                },
            ),
        };
        // Deduplicate NoEngagement: one avoidance report per enemy per location.
        if let nc_data::EncounterDispositionEvent::NoEngagement {
            target_empire,
            coords,
            reason,
            ..
        } = event
        {
            if !seen_avoidance.insert((owner_empire, *target_empire, *coords, *reason)) {
                continue;
            }
        }
        let header = report_header(&source, event_week, year);
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x05,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both {
                recipient: owner_empire,
            },
            repeat_next_pointer: false,
            stardate_week: event_week,
            narrative_phase: narrative_phase_for_report_text(&body),
        });
    }
}
