use nc_data::{CoreGameData, MaintenanceEvents, Mission, MissionOutcome};

use crate::maint::results::combat::*;
use crate::maint::results::entries::intel::{contact_location, contact_speed_text};
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

    // ----- Scorch-Earth events -----
    // The yearly pass runs before the first tick, so these report week 1.
    for event in &events.planet_scorch_events {
        let Some(planet) = game_data.planets.get(event.planet_idx) else {
            continue;
        };
        let [x, y] = planet.sector.to_array();
        let source = format!("From planet \"{}\" in System({x},{y}):", planet.name.trim());
        let header = report_header(&source, Some(1), year);
        let stored = if event.goods_before > 0 {
            format!(
                " and its stored points from {} to {}",
                event.goods_before, event.goods_after
            )
        } else {
            String::new()
        };
        let body = format!(
            " Scorch-Earth is in force. Our production fell from {} to {}{stored}, and the world neither grew nor paid tax. The order stays in force, and the decay repeats each year, until you rescind it with the Scorch command.",
            event.production_before, event.production_after,
        );
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x05,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both {
                recipient: event.owner_empire,
            },
            repeat_next_pointer: false,
            stardate_week: Some(1),
            narrative_phase: narrative_phase_for_report_text(&body),
        });
    }

    // ----- Ship recovery -----
    // The recovery step runs after the year's last tick. Reports say how
    // many ships came back, and mention a starbase's help only when it
    // brought some back early.
    for event in &events.ship_recovery_events {
        let [x, y] = event.coords;
        let place = format!(
            "{}({x},{y})",
            if event.in_system { "System" } else { "Sector" }
        );
        let (source, body) = match event.unit {
            nc_data::RecoveredUnit::Fleet { fleet_number } => {
                let ships = match event.hulls {
                    1 => "1 ship".to_string(),
                    n => format!("{n} ships"),
                };
                let help = match event.early {
                    0 => String::new(),
                    n => format!(", {n} of them a year early with the help of the starbase"),
                };
                (
                    owned_fleet_source_clause(Some(fleet_number as u8), &place),
                    format!(" {ships} recovered to full strength{help}."),
                )
            }
            nc_data::RecoveredUnit::Starbase { base_id } => (
                format!("From Starbase {base_id}, located in {place}:"),
                " We have recovered to full strength.".to_string(),
            ),
        };
        let header = report_header(&source, Some(52), year);
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x05,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both {
                recipient: event.owner_empire,
            },
            repeat_next_pointer: false,
            stardate_week: Some(52),
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

    // ----- Inactivity autopilot -----
    for &empire in &events.autopilot_engaged_empires {
        let source = "From your Fleet Command Center:";
        let header = report_header(source, Some(1), year);
        let body = " With no orders from you for several years, autopilot has taken charge of the empire. Idle fleets will gather at our best worlds and warships will guard them; fleets on other missions carry on. Log in or submit a turn to take command again.";
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x06,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both { recipient: empire },
            repeat_next_pointer: false,
            stardate_week: Some(1),
            narrative_phase: narrative_phase_for_report_text(body),
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
                armies_landed,
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
                        " Salvage mission report: We have arrived at planet \"{planet_name}\" in System({x},{y}) and have begun salvaging our fleet. We estimate that our fleet will yield {recovered_points} production point(s).{}",
                        match armies_landed {
                            0 => String::new(),
                            1 => " In addition, we have transfered the 1 army we were transporting to the planet's surface.".to_string(),
                            n => format!(" In addition, we have transfered the {n} armies we were transporting to the planet's surface."),
                        }
                    ),
                )
            }
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
                order_code,
                coords,
                reason,
            } => {
                let order_name = nc_data::Order::from_code(order_code)
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

        let mut rendezvous_groups: std::collections::BTreeMap<
            (u8, [u8; 2], u8),
            std::collections::BTreeMap<u8, Vec<u8>>,
        > = std::collections::BTreeMap::new();
        let mut rendezvous_meta: std::collections::HashMap<(u8, [u8; 2], u8), usize> =
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
                .entry(event.stardate_week.unwrap_or(1))
                .or_default()
                .push(event.absorbed_fleet_number);
            rendezvous_meta.entry(key).or_insert(event.fleet_idx);
        }

        for (key, absorbed_by_week) in &rendezvous_groups {
            let (owner_empire, coords, host_fleet_number) = *key;
            let [x, y] = coords;
            let host_fleet_idx = rendezvous_meta[key];
            // Dated by the first arrival; the weeks are named when they differ.
            let stardate_week = absorbed_by_week.keys().next().copied();
            let source =
                owned_fleet_source_clause(Some(host_fleet_number), &format!("Sector({x},{y})"));
            let absorbed_text = if absorbed_by_week.len() == 1 {
                compact_fleet_number_list(absorbed_by_week.values().next().unwrap())
            } else {
                let parts: Vec<String> = absorbed_by_week
                    .iter()
                    .map(|(week, numbers)| {
                        format!("{} in week {week}", compact_fleet_number_list(numbers))
                    })
                    .collect();
                match parts.split_last() {
                    Some((last, [])) => last.clone(),
                    Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
                    None => String::new(),
                }
            };
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
    // ECMAINT names a world by the planet at its sector, blank when there is
    // none.
    let world_name = |coords: [u8; 2]| {
        game_data
            .planets
            .iter()
            .find(|planet| planet.sector.to_array() == coords)
            .map_or("", |planet| planet.name.as_str())
    };
    for event in &events.mission_retarget_events {
        let source = match *event {
            nc_data::MissionRetargetEvent::Retargeted {
                reporting_fleet_number,
                current_coords,
                in_system,
                ..
            } => mission_retarget_source(reporting_fleet_number, current_coords, in_system),
            nc_data::MissionRetargetEvent::Abandoned {
                reporting_fleet_number,
                coords,
                in_system,
                ..
            } => mission_retarget_source(reporting_fleet_number, coords, in_system),
        };
        let (recipient, body) = match *event {
            nc_data::MissionRetargetEvent::Retargeted {
                owner_empire,
                mission: Mission::SeekHome,
                previous_target_coords: [px, py],
                new_target_coords: [nx, ny],
                ..
            } => (
                owner_empire,
                format!(
                    " Seek-Home mission report: Since we have lost control of planet \"{}\" in System({px},{py}), we are changing course and heading for a different friendly world. Our new destination is planet \"{}\" in System({nx},{ny}).",
                    world_name([px, py]),
                    world_name([nx, ny])
                ),
            ),
            nc_data::MissionRetargetEvent::Abandoned {
                owner_empire,
                mission: Mission::SeekHome,
                previous_target_coords: [px, py],
                ..
            } => (
                owner_empire,
                format!(
                    " Seek-Home mission report: Since we have lost control of planet \"{}\" in System({px},{py}), we are holding our current position and awaiting new orders.",
                    world_name([px, py])
                ),
            ),
            nc_data::MissionRetargetEvent::Retargeted {
                owner_empire,
                mission: Mission::Salvage,
                previous_target_coords: [px, py],
                new_target_coords: [nx, ny],
                ..
            } => (
                owner_empire,
                format!(
                    " Salvage mission report: Since we no longer own the world in System({px},{py}), we are changing course. Until ordered otherwise, we are going to salvage our fleet at a friendly controlled world. Our new destination is planet \"{}\" in System({nx},{ny}).",
                    world_name([nx, ny])
                ),
            ),
            nc_data::MissionRetargetEvent::Abandoned {
                owner_empire,
                mission: Mission::Salvage,
                previous_target_coords: [px, py],
                ..
            } => (
                owner_empire,
                format!(
                    " Salvage mission report: Since we no longer own the world in System({px},{py}), we are aborting our salvage mission. Until ordered otherwise, we are going to hold our current position and await new orders."
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
        let (nc_data::MissionRetargetEvent::Retargeted { stardate_week, .. }
        | nc_data::MissionRetargetEvent::Abandoned { stardate_week, .. }) = *event;
        let header = report_header(&source, stardate_week, year);
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x05,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both { recipient },
            repeat_next_pointer: false,
            stardate_week,
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
                    // The sighting that led here gives the layer and speed.
                    let contact = events.scout_contact_events.iter().find(|contact| {
                        contact.identified
                            && contact.viewer_empire == owner_empire
                            && contact.target_empire == target_empire
                            && contact.coords == coords
                    });
                    let location = contact.map(contact_location).unwrap_or_else(|| {
                        mission
                            .map(|mission| mission_location_phrase(mission, coords))
                            .unwrap_or_else(|| format!("Sector({},{})", coords[0], coords[1]))
                    });
                    let speed = contact
                        .map(|contact| {
                            format!(" traveling at {}", contact_speed_text(contact.target_speed))
                        })
                        .unwrap_or_default();
                    format!(
                        "{prefix} We have {}. We have located and identified the alien fleet in {location}{speed}. {enemy} Their fleet contains {fleet_desc}. {outcome}",
                        fleet_force_summary(friendly_initial, friendly_loaded_armies_initial),
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
                resumed,
                losses_sustained,
                enemy_losses_inflicted,
                reason,
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
                        "{prefix} We engaged {}. We had {}. The alien force contained {}. {}, we {} {}. {}",
                        classic_enemy_reference(game_data, target_fleet_number, target_empire),
                        fleet_force_summary(friendly_initial, friendly_loaded_armies_initial),
                        fleet_force_summary(enemy_initial, 0),
                        withdrawal_reason_clause(reason),
                        withdrawal_heading(game_data, retreat_target_coords, resumed),
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
                resumed,
                losses_sustained,
                enemy_losses_inflicted,
                reason,
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
                        "{prefix} We had {}. {}, we attempted to disengage from {} but suffered pursuit fire from an alien force containing {}. We {} {}. {}",
                        fleet_force_summary(friendly_initial, friendly_loaded_armies_initial),
                        withdrawal_reason_clause(reason),
                        classic_enemy_reference(game_data, target_fleet_number, target_empire),
                        fleet_force_summary(enemy_initial, 0),
                        withdrawal_heading(game_data, retreat_target_coords, resumed),
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

/// Where a fleet that left a battle headed: home, or on toward its own
/// target when it broke off en route and carried on.
fn withdrawal_heading(game_data: &CoreGameData, coords: [u8; 2], resumed: bool) -> String {
    let [x, y] = coords;
    if !resumed {
        return format!("withdrew toward System({x},{y})");
    }
    let place = if game_data
        .planets
        .iter()
        .any(|p| p.sector.to_array() == coords)
    {
        "System"
    } else {
        "Sector"
    };
    format!("broke away and continued toward {place}({x},{y})")
}
