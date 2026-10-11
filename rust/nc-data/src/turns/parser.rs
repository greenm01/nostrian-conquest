use super::{
    FleetTurnAction, FleetTurnBlock, PlanetTurnAction, PlanetTurnBlock, TurnDiplomacyDirective,
    TurnMessage, TurnSubmission, TurnSubmissionError,
};
use crate::{
    DiplomaticRelation, FleetDetachSelection, NEW_FLEET_DEFAULT_ROE, Order, ProductionItemKind,
};

pub(super) fn parse_turn_submission(input: &str) -> Result<TurnSubmission, TurnSubmissionError> {
    let document: ::kdl::KdlDocument = input
        .parse()
        .map_err(|err| TurnSubmissionError::Parse(format!("invalid KDL: {err}")))?;

    let turn_nodes = document
        .nodes()
        .iter()
        .filter(|node| node.name().value() == "turn")
        .collect::<Vec<_>>();
    let [turn_node] = turn_nodes.as_slice() else {
        return Err(TurnSubmissionError::Parse(
            "turn.kdl must contain exactly one top-level turn node".to_string(),
        ));
    };

    let mut submission = TurnSubmission {
        player_record_index_1_based: prop_usize(turn_node, "player")?,
        year: prop_u16(turn_node, "year")?,
        tax_rate: None,
        diplomacy: Vec::new(),
        planets: Vec::new(),
        fleets: Vec::new(),
        messages: Vec::new(),
    };

    for node in document.nodes() {
        match node.name().value() {
            "turn" => {}
            "tax" => {
                if submission
                    .tax_rate
                    .replace(prop_u8(node, "rate")?)
                    .is_some()
                {
                    return Err(TurnSubmissionError::Parse(
                        "turn.kdl may contain at most one tax node".to_string(),
                    ));
                }
            }
            "diplomacy" => {
                submission.diplomacy.push(TurnDiplomacyDirective {
                    to_empire: prop_u8(node, "to")?,
                    relation: parse_diplomatic_relation(node)?,
                });
            }
            "planet" => submission.planets.push(parse_planet_block(node)?),
            "fleet" => submission.fleets.push(parse_fleet_block(node)?),
            "message" => submission.messages.push(parse_message(node)?),
            other => {
                return Err(TurnSubmissionError::Parse(format!(
                    "unknown top-level node: {other}"
                )));
            }
        }
    }

    if submission.player_record_index_1_based == 0 {
        return Err(TurnSubmissionError::Parse(
            "turn.player must be 1-based".to_string(),
        ));
    }

    Ok(submission)
}

fn parse_planet_block(node: &::kdl::KdlNode) -> Result<PlanetTurnBlock, TurnSubmissionError> {
    let children = node.children().ok_or_else(|| {
        TurnSubmissionError::Parse("planet node must have child actions".to_string())
    })?;
    let mut actions = Vec::new();
    for child in children.nodes() {
        match child.name().value() {
            "rename" => actions.push(PlanetTurnAction::Rename {
                name: prop_string(child, "name")?,
            }),
            "clear_build_queue" => actions.push(PlanetTurnAction::ClearBuildQueue),
            "clear_build_kind" => actions.push(PlanetTurnAction::ClearBuildKind {
                kind_raw: parse_production_kind_raw(child)?,
            }),
            "remove_build" => actions.push(PlanetTurnAction::RemoveBuild {
                qty: prop_u16(child, "qty")?,
                kind_raw: parse_production_kind_raw(child)?,
            }),
            "build" => actions.push(PlanetTurnAction::Build {
                points_remaining_raw: prop_u8(child, "points")?,
                kind_raw: parse_production_kind_raw(child)?,
            }),
            "commission" => actions.push(PlanetTurnAction::Commission {
                slot_0_based: prop_usize(child, "slot")?.checked_sub(1).ok_or_else(|| {
                    TurnSubmissionError::Parse("commission.slot must be 1-based".to_string())
                })?,
            }),
            "auto_commission" => actions.push(PlanetTurnAction::AutoCommission),
            "scorch" => actions.push(PlanetTurnAction::Scorch),
            other => {
                return Err(TurnSubmissionError::Parse(format!(
                    "unknown planet action: {other}"
                )));
            }
        }
    }

    if actions.is_empty() {
        return Err(TurnSubmissionError::Parse(
            "planet node must contain at least one action".to_string(),
        ));
    }

    Ok(PlanetTurnBlock {
        planet_record_index_1_based: prop_record_index(node, "record")?,
        actions,
    })
}

fn parse_fleet_block(node: &::kdl::KdlNode) -> Result<FleetTurnBlock, TurnSubmissionError> {
    let children = node.children().ok_or_else(|| {
        TurnSubmissionError::Parse("fleet node must have child actions".to_string())
    })?;
    let mut actions = Vec::new();
    for child in children.nodes() {
        match child.name().value() {
            "order" => {
                let order_code = parse_order_code(child)?;
                actions.push(FleetTurnAction::Order {
                    speed: prop_u8(child, "speed")?,
                    order_code,
                    target: parse_order_target(child, order_code)?,
                    aux0: opt_prop_u8(child, "aux0")?,
                    aux1: opt_prop_u8(child, "aux1")?,
                });
            }
            "roe" => actions.push(FleetTurnAction::RulesOfEngagement {
                value: prop_u8(child, "value")?,
            }),
            "speed" => actions.push(FleetTurnAction::Speed {
                value: prop_u8(child, "value")?,
            }),
            "join" => actions.push(FleetTurnAction::Join {
                host_fleet_record_index_1_based: prop_record_index(child, "host")?,
            }),
            "detach" => actions.push(FleetTurnAction::Detach {
                selection: parse_fleet_detach_selection(child)?,
                donor_speed: opt_prop_u8(child, "donor_speed")?,
                new_fleet_roe: opt_prop_u8(child, "new_roe")?.unwrap_or(NEW_FLEET_DEFAULT_ROE),
            }),
            "transfer" => actions.push(FleetTurnAction::Transfer {
                host_fleet_record_index_1_based: prop_record_index(child, "to")?,
                selection: parse_fleet_detach_selection(child)?,
            }),
            "load_armies" => actions.push(FleetTurnAction::LoadArmies {
                planet_record_index_1_based: prop_record_index(child, "planet")?,
                qty: prop_u16(child, "qty")?,
            }),
            "unload_armies" => actions.push(FleetTurnAction::UnloadArmies {
                planet_record_index_1_based: prop_record_index(child, "planet")?,
                qty: prop_u16(child, "qty")?,
            }),
            other => {
                return Err(TurnSubmissionError::Parse(format!(
                    "unknown fleet action: {other}"
                )));
            }
        }
    }

    if actions.is_empty() {
        return Err(TurnSubmissionError::Parse(
            "fleet node must contain at least one action".to_string(),
        ));
    }

    Ok(FleetTurnBlock {
        fleet_record_index_1_based: prop_record_index(node, "record")?,
        actions,
    })
}

fn parse_message(node: &::kdl::KdlNode) -> Result<TurnMessage, TurnSubmissionError> {
    Ok(TurnMessage {
        recipient_empire: prop_u8(node, "to")?,
        subject: opt_prop_string(node, "subject")?.unwrap_or_default(),
        body: prop_string(node, "body")?,
    })
}

fn parse_diplomatic_relation(
    node: &::kdl::KdlNode,
) -> Result<DiplomaticRelation, TurnSubmissionError> {
    let value = opt_prop_string(node, "relation")?
        .or_else(|| opt_prop_string(node, "status").ok().flatten())
        .ok_or_else(|| {
            TurnSubmissionError::Parse(
                "diplomacy node must set relation=\"neutral|enemy\"".to_string(),
            )
        })?;
    match value.to_ascii_lowercase().as_str() {
        "neutral" => Ok(DiplomaticRelation::Neutral),
        "enemy" => Ok(DiplomaticRelation::Enemy),
        other => Err(TurnSubmissionError::Parse(format!(
            "unknown diplomacy relation: {other}"
        ))),
    }
}

fn parse_order_code(node: &::kdl::KdlNode) -> Result<u8, TurnSubmissionError> {
    if let Some(value) = opt_prop_string(node, "kind")? {
        return order_code_from_name(&value);
    }
    let raw = prop_u8_with_aliases(node, &["code", "order_code"])?;
    match Order::from_code(raw) {
        Order::Unknown(_) => Err(TurnSubmissionError::Parse(format!(
            "unknown fleet order code: {raw}"
        ))),
        _ => Ok(raw),
    }
}

/// `x` and `y` together; Seek Home and Salvage may leave both out to take
/// the nearest owned world, as ECGAME does.
fn parse_order_target(
    node: &::kdl::KdlNode,
    order_code: u8,
) -> Result<Option<[u8; 2]>, TurnSubmissionError> {
    match (opt_prop_u8(node, "x")?, opt_prop_u8(node, "y")?) {
        (Some(x), Some(y)) => Ok(Some([x, y])),
        (None, None)
            if matches!(
                Order::from_code(order_code),
                Order::SeekHome | Order::Salvage
            ) =>
        {
            Ok(None)
        }
        _ => Err(TurnSubmissionError::Parse(
            "order node must set both x and y".to_string(),
        )),
    }
}

fn order_code_from_name(value: &str) -> Result<u8, TurnSubmissionError> {
    let raw = match value.to_ascii_lowercase().as_str() {
        "hold" | "hold_position" => 0,
        "move" | "move_only" => 1,
        "seek_home" => 2,
        "patrol" | "patrol_sector" => 3,
        "guard_starbase" => 4,
        "guard_blockade" | "guard_blockade_world" | "blockade" => 5,
        "bombard" | "bombard_world" => 6,
        "invade" | "invade_world" => 7,
        "blitz" | "blitz_world" => 8,
        "view" | "view_world" => 9,
        "scout_sector" => 10,
        "scout_system" | "scout_solar_system" => 11,
        "colonize" | "colonize_world" => 12,
        "join" | "join_fleet" | "join_another_fleet" => 13,
        "rendezvous" | "rendezvous_sector" => 14,
        "salvage" => 15,
        other => {
            return Err(TurnSubmissionError::Parse(format!(
                "unknown fleet order kind: {other}"
            )));
        }
    };
    Ok(raw)
}

fn parse_production_kind_raw(node: &::kdl::KdlNode) -> Result<u8, TurnSubmissionError> {
    if let Some(value) = opt_prop_string(node, "kind")? {
        return production_kind_raw_from_name(&value);
    }
    let raw = prop_u8_with_aliases(node, &["kind_raw", "raw"])?;
    match ProductionItemKind::from_code(raw) {
        ProductionItemKind::Unknown(_) => Err(TurnSubmissionError::Parse(format!(
            "unknown production kind: {raw}"
        ))),
        _ => Ok(raw),
    }
}

fn production_kind_raw_from_name(value: &str) -> Result<u8, TurnSubmissionError> {
    let raw = match value.to_ascii_lowercase().as_str() {
        "destroyer" | "destroyers" => 1,
        "cruiser" | "cruisers" => 2,
        "battleship" | "battleships" => 3,
        "scout" | "scouts" => 4,
        "transport" | "transports" | "troop_transport" | "troop_transports" => 5,
        "etac" | "etacs" => 6,
        "ground_battery" | "ground_batteries" | "battery" | "batteries" => 7,
        "army" | "armies" => 8,
        "starbase" | "starbases" | "base" | "bases" => 9,
        other => {
            return Err(TurnSubmissionError::Parse(format!(
                "unknown production kind: {other}"
            )));
        }
    };
    Ok(raw)
}

fn parse_fleet_detach_selection(
    node: &::kdl::KdlNode,
) -> Result<FleetDetachSelection, TurnSubmissionError> {
    Ok(FleetDetachSelection {
        battleships: opt_prop_u16_with_aliases(node, &["battleships", "bb"])?.unwrap_or(0),
        cruisers: opt_prop_u16_with_aliases(node, &["cruisers", "ca"])?.unwrap_or(0),
        destroyers: opt_prop_u16_with_aliases(node, &["destroyers", "dd"])?.unwrap_or(0),
        full_transports: opt_prop_u16_with_aliases(node, &["full_transports", "full_tt"])?
            .unwrap_or(0),
        empty_transports: opt_prop_u16_with_aliases(node, &["empty_transports", "empty_tt"])?
            .unwrap_or(0),
        scouts: opt_prop_u16_with_aliases(node, &["scouts", "sc"])?.unwrap_or(0),
        etacs: opt_prop_u16_with_aliases(node, &["etacs", "etac"])?.unwrap_or(0),
    })
}

fn prop_u8(node: &::kdl::KdlNode, name: &str) -> Result<u8, TurnSubmissionError> {
    let value = integer_prop(node, name)?;
    u8::try_from(value).map_err(|_| {
        TurnSubmissionError::Parse(format!("property {name} out of u8 range: {value}"))
    })
}

fn prop_u16(node: &::kdl::KdlNode, name: &str) -> Result<u16, TurnSubmissionError> {
    let value = integer_prop(node, name)?;
    u16::try_from(value).map_err(|_| {
        TurnSubmissionError::Parse(format!("property {name} out of u16 range: {value}"))
    })
}

fn prop_usize(node: &::kdl::KdlNode, name: &str) -> Result<usize, TurnSubmissionError> {
    let value = integer_prop(node, name)?;
    usize::try_from(value).map_err(|_| {
        TurnSubmissionError::Parse(format!("property {name} out of usize range: {value}"))
    })
}

/// A 1-based record index; 0 names no record.
fn prop_record_index(node: &::kdl::KdlNode, name: &str) -> Result<usize, TurnSubmissionError> {
    match prop_usize(node, name)? {
        0 => Err(TurnSubmissionError::Parse(format!(
            "{}.{name} must be 1-based",
            node.name().value()
        ))),
        index => Ok(index),
    }
}

fn opt_prop_u8(node: &::kdl::KdlNode, name: &str) -> Result<Option<u8>, TurnSubmissionError> {
    opt_prop_u8_with_aliases(node, &[name])
}

fn opt_prop_string(
    node: &::kdl::KdlNode,
    name: &str,
) -> Result<Option<String>, TurnSubmissionError> {
    let Some(value) = node.get(name) else {
        return Ok(None);
    };
    let Some(value) = value.as_string() else {
        return Err(TurnSubmissionError::Parse(format!(
            "property {name} must be a string"
        )));
    };
    Ok(Some(value.to_string()))
}

fn prop_string(node: &::kdl::KdlNode, name: &str) -> Result<String, TurnSubmissionError> {
    opt_prop_string(node, name)?.ok_or_else(|| {
        TurnSubmissionError::Parse(format!("missing or invalid string property: {name}"))
    })
}

fn prop_u8_with_aliases(node: &::kdl::KdlNode, names: &[&str]) -> Result<u8, TurnSubmissionError> {
    opt_prop_u8_with_aliases(node, names)?.ok_or_else(|| {
        TurnSubmissionError::Parse(format!(
            "missing or invalid integer property: {}",
            names.join("|")
        ))
    })
}

fn opt_prop_u8_with_aliases(
    node: &::kdl::KdlNode,
    names: &[&str],
) -> Result<Option<u8>, TurnSubmissionError> {
    match opt_integer_prop(node, names)? {
        Some(value) => u8::try_from(value).map(Some).map_err(|_| {
            TurnSubmissionError::Parse(format!(
                "property {} out of u8 range: {value}",
                names.join("|")
            ))
        }),
        None => Ok(None),
    }
}

fn opt_prop_u16_with_aliases(
    node: &::kdl::KdlNode,
    names: &[&str],
) -> Result<Option<u16>, TurnSubmissionError> {
    match opt_integer_prop(node, names)? {
        Some(value) => u16::try_from(value).map(Some).map_err(|_| {
            TurnSubmissionError::Parse(format!(
                "property {} out of u16 range: {value}",
                names.join("|")
            ))
        }),
        None => Ok(None),
    }
}

fn integer_prop(node: &::kdl::KdlNode, name: &str) -> Result<i128, TurnSubmissionError> {
    opt_integer_prop(node, &[name])?.ok_or_else(|| {
        TurnSubmissionError::Parse(format!("missing or invalid integer property: {name}"))
    })
}

fn opt_integer_prop(
    node: &::kdl::KdlNode,
    names: &[&str],
) -> Result<Option<i128>, TurnSubmissionError> {
    for name in names {
        let Some(value) = node.get(*name) else {
            continue;
        };
        let Some(integer) = value.as_integer() else {
            return Err(TurnSubmissionError::Parse(format!(
                "property {name} must be an integer"
            )));
        };
        return Ok(Some(integer));
    }
    Ok(None)
}
