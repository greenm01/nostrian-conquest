use super::{
    FleetTurnAction, FleetTurnBlock, PlanetTurnAction, PlanetTurnBlock, TurnMessage, TurnSubmission,
};
use crate::{FleetDetachSelection, Order, ProductionItemKind};

pub(super) fn render_turn_submission(submission: &TurnSubmission) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "turn player={} year={}\n",
        submission.player_record_index_1_based, submission.year
    ));

    if let Some(tax_rate) = submission.tax_rate {
        out.push_str(&format!("tax rate={tax_rate}\n"));
    }

    for directive in &submission.diplomacy {
        out.push_str(&format!(
            "diplomacy to={} relation=\"{}\"\n",
            directive.to_empire_raw,
            relation_name(directive.relation)
        ));
    }

    for planet in &submission.planets {
        render_planet_block(&mut out, planet);
    }

    for fleet in &submission.fleets {
        render_fleet_block(&mut out, fleet);
    }

    for message in &submission.messages {
        render_message(&mut out, message);
    }

    out
}

fn render_planet_block(out: &mut String, planet: &PlanetTurnBlock) {
    out.push_str(&format!(
        "planet record={} {{\n",
        planet.planet_record_index_1_based
    ));
    for action in &planet.actions {
        match action {
            PlanetTurnAction::Rename { name } => {
                out.push_str(&format!("  rename name=\"{}\"\n", kdl_escape(name)));
            }
            PlanetTurnAction::ClearBuildQueue => out.push_str("  clear_build_queue\n"),
            PlanetTurnAction::ClearBuildKind { kind_raw } => out.push_str(&format!(
                "  clear_build_kind kind=\"{}\"\n",
                production_kind_name(*kind_raw)
            )),
            PlanetTurnAction::RemoveBuild { qty, kind_raw } => out.push_str(&format!(
                "  remove_build kind=\"{}\" qty={}\n",
                production_kind_name(*kind_raw),
                qty
            )),
            PlanetTurnAction::Build {
                points_remaining_raw,
                kind_raw,
            } => out.push_str(&format!(
                "  build points={} kind=\"{}\"\n",
                points_remaining_raw,
                production_kind_name(*kind_raw)
            )),
            PlanetTurnAction::Commission { slot_0_based } => {
                out.push_str(&format!("  commission slot={}\n", slot_0_based + 1));
            }
            PlanetTurnAction::AutoCommission => out.push_str("  auto_commission\n"),
            PlanetTurnAction::Scorch => out.push_str("  scorch\n"),
        }
    }
    out.push_str("}\n");
}

fn render_fleet_block(out: &mut String, fleet: &FleetTurnBlock) {
    out.push_str(&format!(
        "fleet record={} {{\n",
        fleet.fleet_record_index_1_based
    ));
    for action in &fleet.actions {
        match action {
            FleetTurnAction::Order {
                speed,
                order_code,
                target,
                aux0,
                aux1,
            } => {
                out.push_str(&format!(
                    "  order speed={} kind=\"{}\"",
                    speed,
                    order_name(*order_code)
                ));
                if let Some([x, y]) = target {
                    out.push_str(&format!(" x={x} y={y}"));
                }
                if let Some(value) = aux0 {
                    out.push_str(&format!(" aux0={value}"));
                }
                if let Some(value) = aux1 {
                    out.push_str(&format!(" aux1={value}"));
                }
                out.push('\n');
            }
            FleetTurnAction::RulesOfEngagement { value } => {
                out.push_str(&format!("  roe value={value}\n"));
            }
            FleetTurnAction::Speed { value } => {
                out.push_str(&format!("  speed value={value}\n"));
            }
            FleetTurnAction::Join {
                host_fleet_record_index_1_based,
            } => out.push_str(&format!("  join host={host_fleet_record_index_1_based}\n")),
            FleetTurnAction::Detach {
                selection,
                donor_speed,
                new_fleet_roe,
            } => {
                out.push_str("  detach");
                render_detach_selection(out, selection);
                if let Some(speed) = donor_speed {
                    out.push_str(&format!(" donor_speed={speed}"));
                }
                out.push_str(&format!(" new_roe={new_fleet_roe}\n"));
            }
            FleetTurnAction::Transfer {
                host_fleet_record_index_1_based,
                selection,
            } => {
                out.push_str(&format!("  transfer to={host_fleet_record_index_1_based}"));
                render_detach_selection(out, selection);
                out.push('\n');
            }
            FleetTurnAction::LoadArmies {
                planet_record_index_1_based,
                qty,
            } => out.push_str(&format!(
                "  load_armies planet={} qty={}\n",
                planet_record_index_1_based, qty
            )),
            FleetTurnAction::UnloadArmies {
                planet_record_index_1_based,
                qty,
            } => out.push_str(&format!(
                "  unload_armies planet={} qty={}\n",
                planet_record_index_1_based, qty
            )),
        }
    }
    out.push_str("}\n");
}

fn render_message(out: &mut String, message: &TurnMessage) {
    out.push_str(&format!(
        "message to={} subject=\"{}\" body=\"{}\"\n",
        message.recipient_empire_raw,
        kdl_escape(&message.subject),
        kdl_escape(&message.body)
    ));
}

fn render_detach_selection(out: &mut String, selection: &FleetDetachSelection) {
    if selection.battleships > 0 {
        out.push_str(&format!(" battleships={}", selection.battleships));
    }
    if selection.cruisers > 0 {
        out.push_str(&format!(" cruisers={}", selection.cruisers));
    }
    if selection.destroyers > 0 {
        out.push_str(&format!(" destroyers={}", selection.destroyers));
    }
    if selection.full_transports > 0 {
        out.push_str(&format!(" full_transports={}", selection.full_transports));
    }
    if selection.empty_transports > 0 {
        out.push_str(&format!(" empty_transports={}", selection.empty_transports));
    }
    if selection.scouts > 0 {
        out.push_str(&format!(" scouts={}", selection.scouts));
    }
    if selection.etacs > 0 {
        out.push_str(&format!(" etacs={}", selection.etacs));
    }
}

fn relation_name(relation: crate::DiplomaticRelation) -> &'static str {
    match relation {
        crate::DiplomaticRelation::Neutral | crate::DiplomaticRelation::Unknown(_) => "neutral",
        crate::DiplomaticRelation::Enemy => "enemy",
    }
}

fn order_name(order_code: u8) -> &'static str {
    Order::from_raw(order_code).as_str()
}

fn production_kind_name(kind_raw: u8) -> &'static str {
    match ProductionItemKind::from_raw(kind_raw) {
        ProductionItemKind::Destroyer => "destroyer",
        ProductionItemKind::Cruiser => "cruiser",
        ProductionItemKind::Battleship => "battleship",
        ProductionItemKind::Scout => "scout",
        ProductionItemKind::Transport => "transport",
        ProductionItemKind::Etac => "etac",
        ProductionItemKind::GroundBattery => "ground_battery",
        ProductionItemKind::Army => "army",
        ProductionItemKind::Starbase => "starbase",
        ProductionItemKind::Unknown(_) => "unknown",
    }
}

fn kdl_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}
