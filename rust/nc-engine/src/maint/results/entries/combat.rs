use crate::maint::FleetBattlePerspective;
use nc_data::{CoreGameData, MaintenanceEvents, Mission, MissionOutcome};

use crate::maint::results::combat::*;
use crate::maint::results::compose::ReportSuppressionPlan;
use crate::maint::results::entries::{NarrativePhase, ReportEntry, ReportTarget};
use crate::maint::results::format::*;
use crate::maint::results::mod_constants::*;
use crate::maint::results::structured::*;

pub fn push_combat_entries(
    entries: &mut Vec<ReportEntry>,
    game_data: &CoreGameData,
    events: &MaintenanceEvents,
    year: u16,
) {
    // ----- Bombard events (defender-side report) -----
    for event in &events.bombard_events {
        if event.defender_empire == 0 {
            continue;
        }
        let Some(planet) = game_data.planets.get(event.planet_idx) else {
            continue;
        };
        let [x, y] = planet.sector.to_array();
        let source = format!(
            "From planet \"{}\" in System({x},{y}):",
            planet.name.clone()
        );
        let header = report_header(&source, event.stardate_week, year);
        let attacker = classic_enemy_reference_titlecase(
            game_data,
            event.attacker_fleet_number,
            event.attacker_empire,
        );
        let context_rows = vec![StructuredBodyItem::Label {
            label: LABEL_ATTACKER.to_string(),
            value: attacker.clone(),
        }];
        let force_rows = vec![
            StructuredBodyItem::Label {
                label: LABEL_ATTACKING_FORCE.to_string(),
                value: fleet_force_summary(
                    event.attacker_initial,
                    event.attacker_loaded_armies_initial,
                ),
            },
            StructuredBodyItem::Label {
                label: LABEL_OUR_DEFENSES.to_string(),
                value: ground_force_value(
                    event.defender_batteries_initial,
                    event.defender_armies_initial,
                    "none",
                ),
            },
        ];
        let mut outcome_rows = Vec::new();
        if event.defender_batteries_initial > 0 || event.defender_armies_initial > 0 {
            outcome_rows.push(StructuredBodyItem::Text(planetary_defense_outcome_line(
                event.defender_batteries_initial,
                event.defender_armies_initial,
                event.defender_battery_losses,
                event.defender_army_losses,
            )));
        }
        outcome_rows.extend(
            bombardment_collateral_damage_lines(
                event.stardock_items_destroyed,
                event.stored_goods_destroyed,
                event.factories_destroyed,
                false,
            )
            .into_iter()
            .map(StructuredBodyItem::Text),
        );
        let items = structured_combat_body(
            structured_bombardment_alert(),
            context_rows,
            force_rows,
            outcome_rows,
        );
        let text = structured_report_text(&header, items);
        entries.push(ReportEntry {
            text,
            kind: 0x08,
            tail: RESULTS_TAIL_BOMBARD,
            target: ReportTarget::Both {
                recipient: event.defender_empire,
            },
            repeat_next_pointer: false,
            stardate_week: event.stardate_week,
            narrative_phase: NarrativePhase::DefenderAftermath,
        });
    }

    // ----- Fleet battle events -----
    let suppression_plan = ReportSuppressionPlan::build(game_data, events);
    for event in &events.fleet_battle_events {
        if event.reporting_fleet_number.is_some_and(|fleet_number| {
            suppression_plan.destroyed_supersedes_battle(event.reporting_empire, fleet_number)
        }) {
            continue;
        }
        if event.reporting_fleet_number.is_some_and(|fleet_number| {
            suppression_plan.disposition_supersedes_battle(event.reporting_empire, fleet_number)
        }) {
            continue;
        }
        let reporting_fleet_survives = event.reporting_fleet_number.is_some_and(|fleet_number| {
            suppression_plan.fleet_survives(event.reporting_empire, fleet_number)
        });
        let enemy_list = join_report_parts(
            &event
                .enemy_empires
                .iter()
                .map(|empire| classic_empire_clause(game_data, *empire))
                .collect::<Vec<_>>(),
        );
        let [x, y] = event.coords;
        let enemy = if event.enemy_empires.len() == 1 {
            classic_enemy_reference_titlecase(
                game_data,
                event.primary_enemy_fleet_number,
                event.enemy_empires[0],
            )
        } else {
            format!("hostile fleets belonging to {enemy_list}")
        };
        let friendly_initial = fleet_force_summary_with_starbases(
            event.friendly_initial,
            event.friendly_loaded_armies_initial,
            event.friendly_initial_starbases,
        );
        let enemy_initial = fleet_force_summary_with_starbases(
            event.enemy_initial,
            event.enemy_loaded_armies_initial,
            event.enemy_initial_starbases,
        );
        let starbase_only_defender = is_starbase_only_force(
            event.friendly_initial,
            event.friendly_loaded_armies_initial,
            event.friendly_initial_starbases,
        );
        if starbase_only_defender
            && event.friendly_starbases_lost == event.friendly_initial_starbases
        {
            continue;
        }
        if !reporting_fleet_survives
            && event.reporting_fleet_number.is_some()
            && event.friendly_initial == event.friendly_losses
        {
            let source = "From your Fleet Command Center:";
            let header = report_header(source, event.stardate_week, year);
            let context_rows = vec![
                StructuredBodyItem::Label {
                    label: "Fleet lost:".to_string(),
                    value: fleet_label(event.reporting_fleet_number.unwrap_or(0)),
                },
                StructuredBodyItem::Label {
                    label: LABEL_LAST_CONTACT.to_string(),
                    value: fleet_command_last_contact_value(
                        &enemy,
                        [x, y],
                        matches!(event.perspective, FleetBattlePerspective::Intercepted),
                    ),
                },
            ];
            let force_rows = vec![
                StructuredBodyItem::Label {
                    label: LABEL_OUR_FORCES.to_string(),
                    value: friendly_initial,
                },
                StructuredBodyItem::Label {
                    label: LABEL_ALIEN_FORCES.to_string(),
                    value: enemy_initial,
                },
            ];
            let mut outcome_rows = Vec::new();
            outcome_rows.extend(surprise_sentence(event).map(StructuredBodyItem::Text));
            outcome_rows.push(StructuredBodyItem::Label {
                label: LABEL_ENEMY_LOSSES.to_string(),
                value: combat_losses_value(event.enemy_losses, event.enemy_starbases_destroyed),
            });
            let items = structured_combat_body(
                structured_fleet_destroyed_alert(),
                context_rows,
                force_rows,
                outcome_rows,
            );
            let text = structured_report_text(&header, items);
            entries.push(ReportEntry {
                text,
                kind: 0x06,
                tail: RESULTS_TAIL_FLEET,
                target: ReportTarget::Both {
                    recipient: event.reporting_empire,
                },
                repeat_next_pointer: false,
                stardate_week: event.stardate_week,
                narrative_phase: NarrativePhase::BattleResolution,
            });
            continue;
        }
        let reporting_fleet = event
            .reporting_fleet_number
            .filter(|_| reporting_fleet_survives);
        let source = if reporting_fleet.is_some() {
            owned_fleet_source_clause(reporting_fleet, &format!("System({x},{y})"))
        } else if starbase_only_defender {
            "From your Fleet Command Center:".to_string()
        } else {
            owned_fleet_source_clause(None, &format!("System({x},{y})"))
        };
        let header = report_header(&source, event.stardate_week, year);
        let outcome_text = battle_outcome_sentence(events, event);
        let context_rows = vec![StructuredBodyItem::Label {
            label: LABEL_ENEMY.to_string(),
            value: enemy.clone(),
        }];
        let force_rows = vec![
            StructuredBodyItem::Label {
                label: if starbase_only_defender {
                    LABEL_OUR_DEFENSES.to_string()
                } else {
                    LABEL_OUR_FORCES.to_string()
                },
                value: friendly_initial.clone(),
            },
            StructuredBodyItem::Label {
                label: LABEL_ALIEN_FORCES.to_string(),
                value: enemy_initial.clone(),
            },
        ];
        let mut outcome_rows = Vec::new();
        if matches!(event.perspective, FleetBattlePerspective::Intercepted) {
            outcome_rows.push(StructuredBodyItem::Text(
                "Interception successful.".to_string(),
            ));
        }
        outcome_rows.extend(surprise_sentence(event).map(StructuredBodyItem::Text));
        outcome_rows.push(StructuredBodyItem::Text(outcome_text.clone()));
        if let Some(starbase_line) = enemy_starbase_outcome_line(event.enemy_starbases_destroyed) {
            outcome_rows.push(StructuredBodyItem::Text(starbase_line));
        }
        outcome_rows.push(StructuredBodyItem::Label {
            label: LABEL_OUR_LOSSES.to_string(),
            value: combat_losses_value(event.friendly_losses, event.friendly_starbases_lost),
        });
        if !outcome_text.contains("completely destroyed") {
            outcome_rows.push(StructuredBodyItem::Label {
                label: LABEL_ENEMY_LOSSES.to_string(),
                value: combat_losses_value(event.enemy_losses, event.enemy_starbases_destroyed),
            });
        }
        let items = structured_combat_body(
            structured_fleet_battle_alert(),
            context_rows,
            force_rows,
            outcome_rows,
        );
        let text = structured_report_text(&header, items);
        entries.push(ReportEntry {
            text,
            kind: 0x06,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both {
                recipient: event.reporting_empire,
            },
            repeat_next_pointer: false,
            stardate_week: event.stardate_week,
            narrative_phase: NarrativePhase::BattleResolution,
        });
    }

    // ----- Fleet destroyed events -----
    for event in &events.fleet_destroyed_events {
        let [x, y] = event.coords;
        let enemy = event
            .primary_enemy_empire
            .and_then(|empire| {
                known_hostile_fleet_label(game_data, event.primary_enemy_fleet_number, empire)
                    .or_else(|| Some(classic_empire_clause(game_data, empire)))
            })
            .unwrap_or_else(|| "an alien fleet".to_string());
        let source = "From your Fleet Command Center:";
        let header = report_header(source, event.stardate_week, year);
        let context_rows = vec![
            StructuredBodyItem::Label {
                label: "Fleet lost:".to_string(),
                value: fleet_label(event.fleet_number),
            },
            StructuredBodyItem::Label {
                label: LABEL_LAST_CONTACT.to_string(),
                value: fleet_command_last_contact_value(&enemy, [x, y], event.was_intercepting),
            },
        ];
        let force_rows = vec![
            StructuredBodyItem::Label {
                label: LABEL_OUR_FORCES.to_string(),
                value: fleet_force_summary(
                    event.friendly_initial,
                    event.friendly_loaded_armies_initial,
                ),
            },
            StructuredBodyItem::Label {
                label: LABEL_ALIEN_FORCES.to_string(),
                value: destroyed_fleet_enemy_force_value(event),
            },
        ];
        let outcome_rows = vec![StructuredBodyItem::Label {
            label: LABEL_ENEMY_LOSSES.to_string(),
            value: destroyed_fleet_enemy_losses_value(event),
        }];
        let items = structured_combat_body(
            structured_fleet_destroyed_alert(),
            context_rows,
            force_rows,
            outcome_rows,
        );
        let text = structured_report_text(&header, items);
        entries.push(ReportEntry {
            text,
            kind: 0x06,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both {
                recipient: event.reporting_empire,
            },
            repeat_next_pointer: false,
            stardate_week: event.stardate_week,
            narrative_phase: NarrativePhase::BattleResolution,
        });
    }

    // ----- Starbase destroyed events -----
    for event in &events.starbase_destroyed_events {
        let [x, y] = event.coords;
        let enemy = event
            .primary_enemy_empire
            .and_then(|empire| {
                known_hostile_fleet_label(game_data, event.primary_enemy_fleet_number, empire)
                    .or_else(|| Some(classic_empire_clause(game_data, empire)))
            })
            .unwrap_or_else(|| "an alien fleet".to_string());
        let source = "From your Fleet Command Center:";
        let header = report_header(source, event.stardate_week, year);
        let context_rows = vec![
            StructuredBodyItem::Label {
                label: "Starbase lost:".to_string(),
                value: format!("Starbase {}", event.starbase_id),
            },
            StructuredBodyItem::Label {
                label: LABEL_LAST_CONTACT.to_string(),
                value: fleet_command_last_contact_value(&enemy, [x, y], false),
            },
        ];
        let force_rows = vec![StructuredBodyItem::Label {
            label: LABEL_ALIEN_FORCES.to_string(),
            value: fleet_force_summary(event.enemy_initial, event.enemy_loaded_armies_initial),
        }];
        let outcome_rows = vec![StructuredBodyItem::Label {
            label: LABEL_ENEMY_LOSSES.to_string(),
            value: ship_loss_summary(event.enemy_losses),
        }];
        let items = structured_combat_body(
            structured_starbase_destroyed_alert(),
            context_rows,
            force_rows,
            outcome_rows,
        );
        let text = structured_report_text(&header, items);
        entries.push(ReportEntry {
            text,
            kind: 0x06,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both {
                recipient: event.reporting_empire,
            },
            repeat_next_pointer: false,
            stardate_week: event.stardate_week,
            narrative_phase: NarrativePhase::BattleResolution,
        });
    }

    // ----- Assault report events (invade/blitz) -----
    for event in &events.assault_report_events {
        let Some(planet) = game_data.planets.get(event.planet_idx) else {
            continue;
        };
        let [x, y] = planet.sector.to_array();
        let attacker_force = assault_attacker_force_summary(event);
        let source =
            owned_fleet_source_clause(event.attacker_fleet_number, &format!("System({x},{y})"));
        let header = report_header(&source, event.stardate_week, year);
        let outcome_text = match (event.kind, event.outcome) {
            (Mission::InvadeWorld, MissionOutcome::Succeeded) => None,
            (Mission::InvadeWorld, MissionOutcome::Failed) => {
                Some("The landing was repulsed.".to_string())
            }
            (Mission::InvadeWorld, MissionOutcome::Aborted) => {
                Some("Enemy ground batteries prevented a landing.".to_string())
            }
            (Mission::BlitzWorld, MissionOutcome::Succeeded) => Some(format!(
                "We have seized planet \"{}\" in a fast assault.",
                planet.name.clone()
            )),
            (Mission::BlitzWorld, MissionOutcome::Failed) => {
                Some("The blitz attack failed.".to_string())
            }
            _ => continue,
        };
        let context_rows = vec![StructuredBodyItem::Label {
            label: LABEL_TARGET_WORLD.to_string(),
            value: format!("planet \"{}\"", planet.name.clone()),
        }];
        let mut force_rows = vec![
            StructuredBodyItem::Label {
                label: LABEL_OUR_FORCES.to_string(),
                value: attacker_force,
            },
            StructuredBodyItem::Label {
                label: LABEL_WORLD_DEFENSES.to_string(),
                value: ground_force_value(
                    event.defender_batteries_initial,
                    event.defender_armies_initial,
                    "undefended",
                ),
            },
        ];
        if event.kind == Mission::BlitzWorld {
            if let Some(cover_fire) = blitz_cover_value(event) {
                force_rows.push(StructuredBodyItem::Label {
                    label: "Cover fire:".to_string(),
                    value: cover_fire,
                });
            }
        }
        let mut outcome_rows = Vec::new();
        if let Some(outcome_text) = outcome_text {
            outcome_rows.push(StructuredBodyItem::Text(outcome_text));
        }
        outcome_rows.push(StructuredBodyItem::Label {
            label: LABEL_OUR_LOSSES.to_string(),
            value: assault_friendly_losses_summary(
                event.attacker_ship_losses,
                event.attacker_army_losses,
                event.transport_army_losses,
            ),
        });
        outcome_rows.push(StructuredBodyItem::Label {
            label: LABEL_ENEMY_LOSSES.to_string(),
            value: assault_enemy_losses_summary(
                event.defender_battery_losses,
                event.defender_army_losses,
            ),
        });
        if let Some(softening_losses) = invasion_softening_losses_summary(event) {
            outcome_rows.push(StructuredBodyItem::Label {
                label: format!("{} ", invasion_softening_losses_label(true)),
                value: softening_losses,
            });
        }
        if let Some(ground_battle_losses) = invasion_ground_battle_losses_summary(event) {
            outcome_rows.push(StructuredBodyItem::Label {
                label: format!("{} ", invasion_ground_battle_losses_label(true)),
                value: ground_battle_losses,
            });
        }
        if event.kind == Mission::BlitzWorld {
            outcome_rows.push(StructuredBodyItem::Label {
                label: "Transport losses:".to_string(),
                value: transport_loss_value(event),
            });
        }
        if let Some(inherited) = &event.inherited {
            if inherited.scorched {
                outcome_rows.push(StructuredBodyItem::Text(
                    "The defenders scorched the world before it fell.".to_string(),
                ));
            }
            outcome_rows.push(StructuredBodyItem::Label {
                label: LABEL_INHERITED.to_string(),
                value: inherited_assets_value(inherited),
            });
        }
        let items = structured_combat_body(
            structured_assault_alert(event.kind, event.outcome),
            context_rows,
            force_rows,
            outcome_rows,
        );
        let text = structured_report_text(&header, items);
        entries.push(ReportEntry {
            text,
            kind: 0x0c,
            tail: RESULTS_TAIL_INVASION,
            target: ReportTarget::Both {
                recipient: event.attacker_empire,
            },
            repeat_next_pointer: false,
            stardate_week: event.stardate_week,
            narrative_phase: NarrativePhase::AttackerAftermath,
        });

        if event.defender_empire == 0 || event.outcome == MissionOutcome::Succeeded {
            continue;
        }

        let defender_source = format!(
            "From planet \"{}\" in System({x},{y}):",
            planet.name.clone()
        );
        let defender_header = report_header(&defender_source, event.stardate_week, year);
        let attacker = classic_enemy_reference_titlecase(
            game_data,
            event.attacker_fleet_number,
            event.attacker_empire,
        );
        let context_rows = vec![StructuredBodyItem::Label {
            label: LABEL_ATTACKER.to_string(),
            value: attacker,
        }];
        let mut force_rows = vec![
            StructuredBodyItem::Label {
                label: LABEL_ATTACKING_FORCE.to_string(),
                value: assault_attacker_force_summary(event),
            },
            StructuredBodyItem::Label {
                label: LABEL_OUR_DEFENSES.to_string(),
                value: ground_force_value(
                    event.defender_batteries_initial,
                    event.defender_armies_initial,
                    "undefended",
                ),
            },
        ];
        if event.kind == Mission::BlitzWorld {
            if let Some(cover_fire) = blitz_cover_value(event) {
                force_rows.push(StructuredBodyItem::Label {
                    label: "Cover fire:".to_string(),
                    value: cover_fire,
                });
            }
        }
        let mut outcome_rows = vec![StructuredBodyItem::Text(
            match (event.kind, event.outcome) {
                (Mission::InvadeWorld, MissionOutcome::Failed) => {
                    "The invaders were thrown back.".to_string()
                }
                (Mission::InvadeWorld, MissionOutcome::Aborted) => {
                    "Our batteries prevented an enemy landing.".to_string()
                }
                (Mission::BlitzWorld, MissionOutcome::Failed) => {
                    "The blitz assault was repulsed.".to_string()
                }
                (Mission::BlitzWorld, MissionOutcome::Aborted) => {
                    "The blitz assault never reached the surface.".to_string()
                }
                _ => unreachable!("only failed or aborted defender assault reports reach here"),
            },
        )];
        if event.defender_batteries_initial > 0 || event.defender_armies_initial > 0 {
            outcome_rows.push(StructuredBodyItem::Text(planetary_defense_outcome_line(
                event.defender_batteries_initial,
                event.defender_armies_initial,
                event.defender_battery_losses,
                event.defender_army_losses,
            )));
        }
        outcome_rows.push(StructuredBodyItem::Label {
            label: LABEL_ENEMY_LOSSES.to_string(),
            value: assault_friendly_losses_summary(
                event.attacker_ship_losses,
                event.attacker_army_losses,
                event.transport_army_losses,
            ),
        });
        if let Some(softening_losses) = invasion_softening_losses_summary(event) {
            outcome_rows.push(StructuredBodyItem::Label {
                label: format!("{} ", invasion_softening_losses_label(false)),
                value: softening_losses,
            });
        }
        if let Some(ground_battle_losses) = invasion_ground_battle_losses_summary(event) {
            outcome_rows.push(StructuredBodyItem::Label {
                label: format!("{} ", invasion_ground_battle_losses_label(false)),
                value: ground_battle_losses,
            });
        }
        let items = structured_combat_body(
            structured_repulsed_assault_alert(event.kind, event.outcome),
            context_rows,
            force_rows,
            outcome_rows,
        );
        let text = structured_report_text(&defender_header, items);
        entries.push(ReportEntry {
            text,
            kind: 0x0c,
            tail: RESULTS_TAIL_INVASION,
            target: ReportTarget::Both {
                recipient: event.defender_empire,
            },
            repeat_next_pointer: false,
            stardate_week: event.stardate_week,
            narrative_phase: NarrativePhase::DefenderAftermath,
        });
    }
}
