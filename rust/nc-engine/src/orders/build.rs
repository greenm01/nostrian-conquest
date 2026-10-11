use nc_data::ProductionItemKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildUnitSpec {
    pub number: u8,
    pub kind: ProductionItemKind,
    pub label: &'static str,
    pub singular_label: &'static str,
    pub cost: u32,
}

pub const BUILD_UNITS: [BuildUnitSpec; 9] = [
    BuildUnitSpec {
        number: 1,
        kind: ProductionItemKind::Destroyer,
        label: "Destroyers",
        singular_label: "destroyers",
        cost: 5,
    },
    BuildUnitSpec {
        number: 2,
        kind: ProductionItemKind::Cruiser,
        label: "Cruisers",
        singular_label: "cruisers",
        cost: 15,
    },
    BuildUnitSpec {
        number: 3,
        kind: ProductionItemKind::Battleship,
        label: "Battleships",
        singular_label: "battleships",
        cost: 45,
    },
    BuildUnitSpec {
        number: 4,
        kind: ProductionItemKind::Scout,
        label: "Scouts",
        singular_label: "scouts",
        cost: 15,
    },
    BuildUnitSpec {
        number: 5,
        kind: ProductionItemKind::Transport,
        label: "Troop transports",
        singular_label: "troop transports",
        cost: 5,
    },
    BuildUnitSpec {
        number: 6,
        kind: ProductionItemKind::Etac,
        label: "ETACs",
        singular_label: "ETACs",
        cost: 20,
    },
    BuildUnitSpec {
        number: 7,
        kind: ProductionItemKind::Starbase,
        label: "Starbases",
        singular_label: "starbases",
        cost: 50,
    },
    BuildUnitSpec {
        number: 9,
        kind: ProductionItemKind::Army,
        label: "Armies",
        singular_label: "armies",
        cost: 2,
    },
    BuildUnitSpec {
        number: 10,
        kind: ProductionItemKind::GroundBattery,
        label: "Ground batteries",
        singular_label: "ground batteries",
        cost: 20,
    },
];

pub fn build_unit_spec(number: u8) -> Option<BuildUnitSpec> {
    BUILD_UNITS
        .iter()
        .copied()
        .find(|unit| unit.number == number)
}

pub fn build_unit_spec_by_kind(kind: ProductionItemKind) -> Option<BuildUnitSpec> {
    BUILD_UNITS.iter().copied().find(|unit| unit.kind == kind)
}

pub fn build_kind_name(kind: ProductionItemKind) -> &'static str {
    match kind {
        ProductionItemKind::Destroyer => "Destroyers",
        ProductionItemKind::Cruiser => "Cruisers",
        ProductionItemKind::Battleship => "Battleships",
        ProductionItemKind::Scout => "Scouts",
        ProductionItemKind::Transport => "Troop transports",
        ProductionItemKind::Etac => "ETACs",
        ProductionItemKind::GroundBattery => "Ground batteries",
        ProductionItemKind::Army => "Armies",
        ProductionItemKind::Starbase => "Starbases",
        ProductionItemKind::Unknown(_) => "Unknown",
    }
}

pub fn build_kind_count_label(kind: ProductionItemKind, quantity: u32) -> &'static str {
    if quantity == 1 {
        match kind {
            ProductionItemKind::Destroyer => "Destroyer",
            ProductionItemKind::Cruiser => "Cruiser",
            ProductionItemKind::Battleship => "Battleship",
            ProductionItemKind::Scout => "Scout",
            ProductionItemKind::Transport => "Troop transport",
            ProductionItemKind::Etac => "ETAC",
            ProductionItemKind::GroundBattery => "Ground battery",
            ProductionItemKind::Army => "Army",
            ProductionItemKind::Starbase => "Starbase",
            ProductionItemKind::Unknown(_) => "Unknown",
        }
    } else {
        build_kind_name(kind)
    }
}

pub fn infer_quantity(kind: ProductionItemKind, points_remaining: u32) -> Option<u32> {
    let cost = build_unit_spec_by_kind(kind)?.cost;
    if cost == 0 {
        return None;
    }
    if points_remaining % cost == 0 {
        Some(points_remaining / cost)
    } else {
        None
    }
}

pub fn build_quantity_from_points(kind: ProductionItemKind, points: u32) -> u32 {
    if points == 0 {
        return 0;
    }
    let cost = build_unit_spec_by_kind(kind)
        .map(|unit| unit.cost)
        .unwrap_or(1);
    if cost == 0 {
        points
    } else {
        ((points.saturating_sub(1)) / cost) + 1
    }
}

pub fn max_quantity(points_left: u32, cost: u32) -> u32 {
    if cost == 0 { 0 } else { points_left / cost }
}
