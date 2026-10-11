use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use nc_data::{DiplomaticRelation, Order, ProductionItemKind};

use crate::error::HarnessError;
use crate::spec::{
    CombatScenarioSpec, CombatSweepSpec, CommissionSpec, DiplomacySpec, FleetOrderSpec,
    FleetShipsSpec, FleetSpec, HouseSpec, PlanetSpec, PlanetStatField, QueuedMailSpec,
    ReviewBlockSpec, ScenarioBaseline, ScenarioMetadata, ScenarioSpec, ShipDimensionKind,
    StardockSlotSpec, SweepDimension, TurnFileSpec,
};

impl ScenarioSpec {
    pub fn parse_kdl_str(input: &str, base_dir: &Path) -> Result<Self, HarnessError> {
        let document: kdl::KdlDocument = input
            .parse()
            .map_err(|err| HarnessError::Parse(format!("invalid KDL: {err}")))?;
        parse_scenario_document(&document, base_dir, "scenario")
    }

    pub fn load_kdl(path: &Path) -> Result<Self, HarnessError> {
        let text = fs::read_to_string(path)?;
        let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
        Self::parse_kdl_str(&text, base_dir)
    }
}

impl CombatScenarioSpec {
    pub fn parse_kdl_str(input: &str, base_dir: &Path) -> Result<Self, HarnessError> {
        let document: kdl::KdlDocument = input
            .parse()
            .map_err(|err| HarnessError::Parse(format!("invalid KDL: {err}")))?;
        let scenario = parse_scenario_document(&document, base_dir, "combat-scenario")?;
        let root = document
            .get("combat-scenario")
            .ok_or_else(|| HarnessError::Parse("missing combat-scenario node".to_string()))?;
        let maintenance_turns = opt_prop_u16(root, "turns")?.unwrap_or(1);
        if maintenance_turns == 0 {
            return Err(HarnessError::Validation(
                "combat-scenario turns must be >= 1".to_string(),
            ));
        }
        Ok(Self {
            scenario,
            maintenance_turns,
        })
    }

    pub fn load_kdl(path: &Path) -> Result<Self, HarnessError> {
        let text = fs::read_to_string(path)?;
        let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
        Self::parse_kdl_str(&text, base_dir)
    }
}

impl CombatSweepSpec {
    pub fn parse_kdl_str(input: &str, base_dir: &Path) -> Result<Self, HarnessError> {
        let document: kdl::KdlDocument = input
            .parse()
            .map_err(|err| HarnessError::Parse(format!("invalid KDL: {err}")))?;
        let root = document
            .get("combat-sweep")
            .ok_or_else(|| HarnessError::Parse("missing combat-sweep node".to_string()))?;
        let scenario_path = resolve_path(base_dir, &prop_string(root, "scenario")?);
        let maintenance_turns = opt_prop_u16(root, "turns")?;
        let seed = opt_prop_u64(root, "seed")?.unwrap_or(1515);
        let max_cases = opt_prop_usize(root, "max_cases")?.unwrap_or(128);
        if max_cases == 0 {
            return Err(HarnessError::Validation(
                "combat-sweep max_cases must be >= 1".to_string(),
            ));
        }

        let mut dimensions = Vec::new();
        for node in document.nodes() {
            match node.name().value() {
                "combat-sweep" => {}
                "fleet-ship" => dimensions.push(parse_fleet_ship_dimension(node)?),
                "fleet-roe" => dimensions.push(parse_fleet_roe_dimension(node)?),
                "planet-stat" => dimensions.push(parse_planet_stat_dimension(node)?),
                "relation-variation" => dimensions.push(parse_relation_dimension(node)?),
                other => {
                    return Err(HarnessError::Parse(format!(
                        "unknown combat-sweep node: {other}"
                    )));
                }
            }
        }

        if dimensions.is_empty() {
            return Err(HarnessError::Validation(
                "combat-sweep requires at least one dimension".to_string(),
            ));
        }

        Ok(Self {
            scenario_path,
            maintenance_turns,
            seed,
            max_cases,
            dimensions,
        })
    }

    pub fn load_kdl(path: &Path) -> Result<Self, HarnessError> {
        let text = fs::read_to_string(path)?;
        let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
        Self::parse_kdl_str(&text, base_dir)
    }
}

fn parse_scenario_document(
    document: &kdl::KdlDocument,
    base_dir: &Path,
    root_name: &str,
) -> Result<ScenarioSpec, HarnessError> {
    let root = document
        .get(root_name)
        .ok_or_else(|| HarnessError::Parse(format!("missing {root_name} node")))?;
    let metadata = ScenarioMetadata {
        label: opt_prop_string(root, "label")?,
        player_count: prop_u8(root, "player_count")?,
        year: prop_u16(root, "year")?,
        seed: opt_prop_u64(root, "seed")?.unwrap_or(1515),
        baseline: parse_baseline(root)?,
    };

    if !nc_data::is_valid_player_count(metadata.player_count) {
        return Err(HarnessError::Validation(format!(
            "player_count must be 1-25, got {}",
            metadata.player_count
        )));
    }

    let mut houses = Vec::new();
    let mut diplomacy = Vec::new();
    let mut planets = Vec::new();
    let mut fleets = Vec::new();
    let mut turn_files = Vec::new();
    let mut queued_mail = Vec::new();
    let mut results_blocks = Vec::new();
    let mut message_blocks = Vec::new();

    for node in document.nodes() {
        match node.name().value() {
            "scenario" | "combat-scenario" => {}
            "house" => houses.push(parse_house(node)?),
            "relation" => diplomacy.push(parse_relation(node)?),
            "planet" => planets.push(parse_planet(node)?),
            "fleet" => fleets.push(parse_fleet(node)?),
            "turn-file" => turn_files.push(parse_turn_file(node, base_dir)?),
            "queued-mail" => queued_mail.push(parse_queued_mail(node)?),
            "results-block" => results_blocks.push(parse_review_block(node)?),
            "messages-block" => message_blocks.push(parse_review_block(node)?),
            other => {
                return Err(HarnessError::Parse(format!(
                    "unknown {root_name} node: {other}"
                )));
            }
        }
    }

    validate_unique_indices(
        "house",
        houses.iter().map(|house| house.record_index_1_based),
    )?;
    validate_unique_indices(
        "planet",
        planets.iter().map(|planet| planet.record_index_1_based),
    )?;
    validate_unique_indices(
        "fleet",
        fleets.iter().map(|fleet| fleet.record_index_1_based),
    )?;
    validate_relations(&diplomacy, metadata.player_count)?;

    Ok(ScenarioSpec {
        metadata,
        houses,
        diplomacy,
        planets,
        fleets,
        turn_files,
        queued_mail,
        results_blocks,
        message_blocks,
    })
}

fn validate_unique_indices(
    label: &str,
    indices: impl Iterator<Item = usize>,
) -> Result<(), HarnessError> {
    let mut seen = BTreeSet::new();
    for index in indices {
        if !seen.insert(index) {
            return Err(HarnessError::Validation(format!(
                "duplicate {label} record index: {index}"
            )));
        }
    }
    Ok(())
}

fn validate_relations(relations: &[DiplomacySpec], player_count: u8) -> Result<(), HarnessError> {
    let mut seen = BTreeSet::new();
    for relation in relations {
        if relation.from_empire == 0 || relation.from_empire > player_count {
            return Err(HarnessError::Validation(format!(
                "relation.from must be in 1..={player_count}, got {}",
                relation.from_empire
            )));
        }
        if relation.to_empire == 0 || relation.to_empire > player_count {
            return Err(HarnessError::Validation(format!(
                "relation.to must be in 1..={player_count}, got {}",
                relation.to_empire
            )));
        }
        if relation.from_empire == relation.to_empire {
            return Err(HarnessError::Validation(
                "relation cannot target the same empire".to_string(),
            ));
        }
        if !seen.insert((relation.from_empire, relation.to_empire)) {
            return Err(HarnessError::Validation(format!(
                "duplicate relation {} -> {}",
                relation.from_empire, relation.to_empire
            )));
        }
    }
    Ok(())
}

fn parse_baseline(node: &kdl::KdlNode) -> Result<ScenarioBaseline, HarnessError> {
    match opt_prop_string(node, "baseline")?
        .unwrap_or_else(|| "builder-compatible".to_string())
        .as_str()
    {
        "builder-compatible" => Ok(ScenarioBaseline::BuilderCompatible),
        "joinable-new-game" => Ok(ScenarioBaseline::JoinableNewGame),
        other => Err(HarnessError::Parse(format!("unknown baseline: {other}"))),
    }
}

fn parse_house(node: &kdl::KdlNode) -> Result<HouseSpec, HarnessError> {
    Ok(HouseSpec {
        record_index_1_based: prop_usize_1_based(node, "record")?,
        handle: opt_prop_string(node, "handle")?,
        empire_name: opt_prop_string(node, "empire")?,
        homeworld_name: opt_prop_string(node, "homeworld")?,
        tax_rate: opt_prop_u8(node, "tax")?,
        autopilot: node
            .get("autopilot")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
    })
}

fn parse_relation(node: &kdl::KdlNode) -> Result<DiplomacySpec, HarnessError> {
    Ok(DiplomacySpec {
        from_empire: prop_u8(node, "from")?,
        to_empire: prop_u8(node, "to")?,
        relation: parse_diplomatic_relation(&prop_string(node, "status")?)?,
    })
}

fn parse_planet(node: &kdl::KdlNode) -> Result<PlanetSpec, HarnessError> {
    let mut spec = PlanetSpec {
        record_index_1_based: prop_usize_1_based(node, "record")?,
        ..PlanetSpec::default()
    };

    let children = node
        .children()
        .ok_or_else(|| HarnessError::Parse("planet nodes must have children".to_string()))?;
    for child in children.nodes() {
        match child.name().value() {
            "coords" => spec.coords = Some([prop_u8(child, "x")?, prop_u8(child, "y")?]),
            "owner" => spec.owner_empire = Some(prop_u8(child, "record")?),
            "name" => spec.name = Some(text_value(child, "planet name")?),
            "production" => {
                spec.potential_production = opt_prop_u16(child, "potential")?;
                spec.present_production = opt_prop_u16(child, "present")?;
                spec.stored_production = opt_prop_u32(child, "stored")?;
            }
            "defenses" => {
                spec.armies = opt_prop_u8(child, "armies")?;
                spec.ground_batteries = opt_prop_u8(child, "batteries")?;
            }
            "stardock" => spec.stardock.push(parse_stardock_slot(child)?),
            "commission" => spec.commissions.push(CommissionSpec {
                slot_0_based: prop_usize_1_based(child, "slot")? - 1,
            }),
            other => {
                return Err(HarnessError::Parse(format!(
                    "unknown planet child node: {other}"
                )));
            }
        }
    }
    Ok(spec)
}

fn parse_fleet(node: &kdl::KdlNode) -> Result<FleetSpec, HarnessError> {
    let mut spec = FleetSpec {
        record_index_1_based: prop_usize_1_based(node, "record")?,
        ..FleetSpec::default()
    };

    let children = node
        .children()
        .ok_or_else(|| HarnessError::Parse("fleet nodes must have children".to_string()))?;
    for child in children.nodes() {
        match child.name().value() {
            "owner" => spec.owner_empire = Some(prop_u8(child, "record")?),
            "coords" => spec.coords = Some([prop_u8(child, "x")?, prop_u8(child, "y")?]),
            "ships" => {
                spec.ships = Some(FleetShipsSpec {
                    battleships: opt_prop_u16(child, "bb")?.unwrap_or(0),
                    cruisers: opt_prop_u16(child, "ca")?.unwrap_or(0),
                    destroyers: opt_prop_u16(child, "dd")?.unwrap_or(0),
                    scouts: opt_prop_u16(child, "sc")?.unwrap_or(0),
                    transports: opt_prop_u16(child, "tt")?.unwrap_or(0),
                    loaded_armies: opt_prop_u16(child, "armies")?.unwrap_or(0),
                    etacs: opt_prop_u16(child, "etac")?.unwrap_or(0),
                });
            }
            "roe" => spec.rules_of_engagement = Some(prop_u8(child, "value")?),
            "speed" => spec.current_speed = Some(prop_u8(child, "value")?),
            "invasion" => spec.invasion_armies = Some(prop_u8(child, "armies")?),
            "order" => spec.order = Some(parse_fleet_order(child)?),
            other => {
                return Err(HarnessError::Parse(format!(
                    "unknown fleet child node: {other}"
                )));
            }
        }
    }
    Ok(spec)
}

fn parse_fleet_order(node: &kdl::KdlNode) -> Result<FleetOrderSpec, HarnessError> {
    let kind = if let Some(raw) = opt_prop_u8(node, "code")? {
        Order::from_code(raw)
    } else {
        parse_order_kind(&prop_string(node, "kind")?)?
    };
    if matches!(kind, Order::Unknown(_)) {
        return Err(HarnessError::Parse("unknown fleet order kind".to_string()));
    }
    Ok(FleetOrderSpec {
        kind,
        speed: prop_u8(node, "speed")?,
        target: [prop_u8(node, "x")?, prop_u8(node, "y")?],
        aux0: opt_prop_u8(node, "aux0")?,
        aux1: opt_prop_u8(node, "aux1")?,
    })
}

fn parse_stardock_slot(node: &kdl::KdlNode) -> Result<StardockSlotSpec, HarnessError> {
    let kind_raw = if let Some(kind_raw) = opt_prop_u8(node, "kind_raw")? {
        kind_raw
    } else {
        parse_production_kind_raw(&prop_string(node, "kind")?)?
    };
    Ok(StardockSlotSpec {
        slot_0_based: prop_usize_1_based(node, "slot")? - 1,
        kind_raw,
        count: prop_u16(node, "count")?,
    })
}

fn parse_turn_file(node: &kdl::KdlNode, base_dir: &Path) -> Result<TurnFileSpec, HarnessError> {
    let turn = opt_prop_u16(node, "turn")?.unwrap_or(1);
    if turn == 0 {
        return Err(HarnessError::Validation(
            "turn-file turn must be >= 1".to_string(),
        ));
    }
    Ok(TurnFileSpec {
        path: resolve_path(base_dir, &prop_string(node, "path")?),
        turn,
    })
}

fn parse_queued_mail(node: &kdl::KdlNode) -> Result<QueuedMailSpec, HarnessError> {
    Ok(QueuedMailSpec {
        sender_empire: prop_u8(node, "from")?,
        recipient_empire: prop_u8(node, "to")?,
        year: opt_prop_u16(node, "year")?,
        subject: opt_prop_string(node, "subject")?.unwrap_or_default(),
        body: prop_string(node, "body")?,
    })
}

fn parse_review_block(node: &kdl::KdlNode) -> Result<ReviewBlockSpec, HarnessError> {
    Ok(ReviewBlockSpec {
        player_record_index_1_based: opt_prop_usize_1_based(node, "player")?,
        text: text_value(node, "review block text")?,
    })
}

fn parse_fleet_ship_dimension(node: &kdl::KdlNode) -> Result<SweepDimension, HarnessError> {
    let values = positional_u16_values(node)?;
    if values.is_empty() {
        return Err(HarnessError::Validation(
            "fleet-ship dimensions require at least one value".to_string(),
        ));
    }
    Ok(SweepDimension::FleetShips {
        fleet_record_index_1_based: prop_usize_1_based(node, "fleet")?,
        kind: parse_ship_dimension_kind(&prop_string(node, "kind")?)?,
        values,
    })
}

fn parse_fleet_roe_dimension(node: &kdl::KdlNode) -> Result<SweepDimension, HarnessError> {
    let values = positional_u8_values(node)?;
    if values.is_empty() {
        return Err(HarnessError::Validation(
            "fleet-roe dimensions require at least one value".to_string(),
        ));
    }
    Ok(SweepDimension::FleetRoe {
        fleet_record_index_1_based: prop_usize_1_based(node, "fleet")?,
        values,
    })
}

fn parse_planet_stat_dimension(node: &kdl::KdlNode) -> Result<SweepDimension, HarnessError> {
    let values = positional_u16_values(node)?;
    if values.is_empty() {
        return Err(HarnessError::Validation(
            "planet-stat dimensions require at least one value".to_string(),
        ));
    }
    Ok(SweepDimension::PlanetStat {
        planet_record_index_1_based: prop_usize_1_based(node, "planet")?,
        field: parse_planet_stat_field(&prop_string(node, "field")?)?,
        values,
    })
}

fn parse_relation_dimension(node: &kdl::KdlNode) -> Result<SweepDimension, HarnessError> {
    let mut values = Vec::new();
    let mut idx = 0usize;
    while let Some(value) = node.get(idx).and_then(|entry| entry.as_string()) {
        values.push(parse_diplomatic_relation(value)?);
        idx += 1;
    }
    if values.is_empty() {
        return Err(HarnessError::Validation(
            "relation-variation dimensions require at least one value".to_string(),
        ));
    }
    Ok(SweepDimension::DiplomaticRelation {
        from_empire: prop_u8(node, "from")?,
        to_empire: prop_u8(node, "to")?,
        values,
    })
}

fn parse_ship_dimension_kind(value: &str) -> Result<ShipDimensionKind, HarnessError> {
    match value {
        "bb" | "battleship" | "battleships" => Ok(ShipDimensionKind::Battleships),
        "ca" | "cruiser" | "cruisers" => Ok(ShipDimensionKind::Cruisers),
        "dd" | "destroyer" | "destroyers" => Ok(ShipDimensionKind::Destroyers),
        "sc" | "scout" | "scouts" => Ok(ShipDimensionKind::Scouts),
        "tt" | "transport" | "transports" => Ok(ShipDimensionKind::Transports),
        "armies" | "loaded_armies" => Ok(ShipDimensionKind::LoadedArmies),
        "et" | "etac" | "etacs" => Ok(ShipDimensionKind::Etacs),
        other => Err(HarnessError::Parse(format!(
            "unknown fleet-ship kind: {other}"
        ))),
    }
}

fn parse_planet_stat_field(value: &str) -> Result<PlanetStatField, HarnessError> {
    match value {
        "armies" => Ok(PlanetStatField::Armies),
        "batteries" | "ground_batteries" => Ok(PlanetStatField::GroundBatteries),
        other => Err(HarnessError::Parse(format!(
            "unknown planet-stat field: {other}"
        ))),
    }
}

fn parse_production_kind_raw(value: &str) -> Result<u8, HarnessError> {
    let kind = match value {
        "destroyer" | "dd" => ProductionItemKind::Destroyer,
        "cruiser" | "ca" => ProductionItemKind::Cruiser,
        "battleship" | "bb" => ProductionItemKind::Battleship,
        "scout" | "sc" => ProductionItemKind::Scout,
        "transport" | "tt" => ProductionItemKind::Transport,
        "etac" | "et" => ProductionItemKind::Etac,
        "ground_battery" | "battery" | "gb" => ProductionItemKind::GroundBattery,
        "army" | "armies" => ProductionItemKind::Army,
        "starbase" | "sb" => ProductionItemKind::Starbase,
        other => {
            return Err(HarnessError::Parse(format!(
                "unknown production kind: {other}"
            )));
        }
    };
    Ok(match kind {
        ProductionItemKind::Destroyer => 1,
        ProductionItemKind::Cruiser => 2,
        ProductionItemKind::Battleship => 3,
        ProductionItemKind::Scout => 4,
        ProductionItemKind::Transport => 5,
        ProductionItemKind::Etac => 6,
        ProductionItemKind::GroundBattery => 7,
        ProductionItemKind::Army => 8,
        ProductionItemKind::Starbase => 9,
        ProductionItemKind::Unknown(_) => unreachable!(),
    })
}

fn parse_order_kind(value: &str) -> Result<Order, HarnessError> {
    let order = match value {
        "hold" => Order::HoldPosition,
        "move" => Order::MoveOnly,
        "seek_home" => Order::SeekHome,
        "patrol" => Order::PatrolSector,
        "guard_starbase" => Order::GuardStarbase,
        "guard_blockade" => Order::GuardBlockadeWorld,
        "bombard" => Order::BombardWorld,
        "invade" => Order::InvadeWorld,
        "blitz" => Order::BlitzWorld,
        "view" => Order::ViewWorld,
        "scout_sector" => Order::ScoutSector,
        "scout_system" => Order::ScoutSolarSystem,
        "colonize" => Order::ColonizeWorld,
        "join_fleet" => Order::JoinAnotherFleet,
        "rendezvous" => Order::RendezvousSector,
        "salvage" => Order::Salvage,
        other => {
            return Err(HarnessError::Parse(format!(
                "unknown fleet order kind: {other}"
            )));
        }
    };
    Ok(order)
}

fn parse_diplomatic_relation(value: &str) -> Result<DiplomaticRelation, HarnessError> {
    match value {
        "neutral" => Ok(DiplomaticRelation::Neutral),
        "enemy" => Ok(DiplomaticRelation::Enemy),
        other => Err(HarnessError::Parse(format!(
            "unknown diplomatic relation: {other}"
        ))),
    }
}

fn positional_u16_values(node: &kdl::KdlNode) -> Result<Vec<u16>, HarnessError> {
    let mut out = Vec::new();
    let mut idx = 0usize;
    while let Some(value) = node.get(idx).and_then(|entry| entry.as_integer()) {
        out.push(
            u16::try_from(value)
                .map_err(|_| HarnessError::Parse(format!("value out of u16 range: {value}")))?,
        );
        idx += 1;
    }
    Ok(out)
}

fn positional_u8_values(node: &kdl::KdlNode) -> Result<Vec<u8>, HarnessError> {
    let mut out = Vec::new();
    let mut idx = 0usize;
    while let Some(value) = node.get(idx).and_then(|entry| entry.as_integer()) {
        out.push(
            u8::try_from(value)
                .map_err(|_| HarnessError::Parse(format!("value out of u8 range: {value}")))?,
        );
        idx += 1;
    }
    Ok(out)
}

fn text_value(node: &kdl::KdlNode, label: &str) -> Result<String, HarnessError> {
    if let Some(value) = node.get("text").and_then(|entry| entry.as_string()) {
        return Ok(value.to_string());
    }
    node.get(0)
        .and_then(|entry| entry.as_string())
        .map(str::to_string)
        .ok_or_else(|| HarnessError::Parse(format!("missing {label}")))
}

fn resolve_path(base_dir: &Path, raw: &str) -> PathBuf {
    let candidate = PathBuf::from(raw);
    if candidate.is_absolute() {
        candidate
    } else {
        base_dir.join(candidate)
    }
}

fn prop_string(node: &kdl::KdlNode, name: &str) -> Result<String, HarnessError> {
    node.get(name)
        .and_then(|value| value.as_string())
        .map(str::to_string)
        .ok_or_else(|| HarnessError::Parse(format!("missing or invalid string property: {name}")))
}

fn opt_prop_string(node: &kdl::KdlNode, name: &str) -> Result<Option<String>, HarnessError> {
    Ok(node
        .get(name)
        .and_then(|value| value.as_string())
        .map(str::to_string))
}

fn prop_u8(node: &kdl::KdlNode, name: &str) -> Result<u8, HarnessError> {
    let value = node
        .get(name)
        .and_then(|value| value.as_integer())
        .ok_or_else(|| {
            HarnessError::Parse(format!("missing or invalid integer property: {name}"))
        })?;
    u8::try_from(value)
        .map_err(|_| HarnessError::Parse(format!("property {name} out of u8 range: {value}")))
}

fn opt_prop_u8(node: &kdl::KdlNode, name: &str) -> Result<Option<u8>, HarnessError> {
    let Some(value) = node.get(name).and_then(|value| value.as_integer()) else {
        return Ok(None);
    };
    Ok(Some(u8::try_from(value).map_err(|_| {
        HarnessError::Parse(format!("property {name} out of u8 range: {value}"))
    })?))
}

fn prop_u16(node: &kdl::KdlNode, name: &str) -> Result<u16, HarnessError> {
    let value = node
        .get(name)
        .and_then(|value| value.as_integer())
        .ok_or_else(|| {
            HarnessError::Parse(format!("missing or invalid integer property: {name}"))
        })?;
    u16::try_from(value)
        .map_err(|_| HarnessError::Parse(format!("property {name} out of u16 range: {value}")))
}

fn opt_prop_u16(node: &kdl::KdlNode, name: &str) -> Result<Option<u16>, HarnessError> {
    let Some(value) = node.get(name).and_then(|value| value.as_integer()) else {
        return Ok(None);
    };
    Ok(Some(u16::try_from(value).map_err(|_| {
        HarnessError::Parse(format!("property {name} out of u16 range: {value}"))
    })?))
}

fn opt_prop_u32(node: &kdl::KdlNode, name: &str) -> Result<Option<u32>, HarnessError> {
    let Some(value) = node.get(name).and_then(|value| value.as_integer()) else {
        return Ok(None);
    };
    Ok(Some(u32::try_from(value).map_err(|_| {
        HarnessError::Parse(format!("property {name} out of u32 range: {value}"))
    })?))
}

fn opt_prop_u64(node: &kdl::KdlNode, name: &str) -> Result<Option<u64>, HarnessError> {
    let Some(value) = node.get(name).and_then(|value| value.as_integer()) else {
        return Ok(None);
    };
    Ok(Some(u64::try_from(value).map_err(|_| {
        HarnessError::Parse(format!("property {name} out of u64 range: {value}"))
    })?))
}

fn prop_usize_1_based(node: &kdl::KdlNode, name: &str) -> Result<usize, HarnessError> {
    let value = node
        .get(name)
        .and_then(|value| value.as_integer())
        .ok_or_else(|| {
            HarnessError::Parse(format!("missing or invalid integer property: {name}"))
        })?;
    let converted = usize::try_from(value)
        .map_err(|_| HarnessError::Parse(format!("property {name} out of usize range: {value}")))?;
    if converted == 0 {
        return Err(HarnessError::Parse(format!("{name} must be 1-based")));
    }
    Ok(converted)
}

fn opt_prop_usize(node: &kdl::KdlNode, name: &str) -> Result<Option<usize>, HarnessError> {
    let Some(value) = node.get(name).and_then(|value| value.as_integer()) else {
        return Ok(None);
    };
    Ok(Some(usize::try_from(value).map_err(|_| {
        HarnessError::Parse(format!("property {name} out of usize range: {value}"))
    })?))
}

fn opt_prop_usize_1_based(node: &kdl::KdlNode, name: &str) -> Result<Option<usize>, HarnessError> {
    let Some(value) = opt_prop_usize(node, name)? else {
        return Ok(None);
    };
    if value == 0 {
        return Err(HarnessError::Parse(format!("{name} must be 1-based")));
    }
    Ok(Some(value))
}
