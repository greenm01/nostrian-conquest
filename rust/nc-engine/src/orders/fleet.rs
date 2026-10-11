use nc_data::Fleet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetMissionRequirement {
    Any,
    CombatShips,
    CombatAndLoadedTransports,
    LoadedTransports,
    AtLeastOneScout,
    AtLeastOneEtac,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FleetMissionOption {
    pub code: u8,
    pub mission: &'static str,
    pub requirements: &'static str,
    pub requirement: FleetMissionRequirement,
}

pub const FLEET_MISSION_OPTIONS: [FleetMissionOption; 16] = [
    FleetMissionOption {
        code: 0,
        mission: "None (hold position)",
        requirements: "Any ships",
        requirement: FleetMissionRequirement::Any,
    },
    FleetMissionOption {
        code: 1,
        mission: "Move Fleet (only)",
        requirements: "Any",
        requirement: FleetMissionRequirement::Any,
    },
    FleetMissionOption {
        code: 2,
        mission: "Seek Home",
        requirements: "Any",
        requirement: FleetMissionRequirement::Any,
    },
    FleetMissionOption {
        code: 3,
        mission: "Patrol a Sector",
        requirements: "Any",
        requirement: FleetMissionRequirement::Any,
    },
    FleetMissionOption {
        code: 4,
        mission: "Guard a Starbase",
        requirements: "Combat ships",
        requirement: FleetMissionRequirement::CombatShips,
    },
    FleetMissionOption {
        code: 5,
        mission: "Guard/Blockade a World",
        requirements: "Combat ships",
        requirement: FleetMissionRequirement::CombatShips,
    },
    FleetMissionOption {
        code: 6,
        mission: "Bombard a World",
        requirements: "Combat ships",
        requirement: FleetMissionRequirement::CombatShips,
    },
    FleetMissionOption {
        code: 7,
        mission: "Invade a World",
        requirements: "Combat + loaded transports",
        requirement: FleetMissionRequirement::CombatAndLoadedTransports,
    },
    FleetMissionOption {
        code: 8,
        mission: "Blitz a World",
        requirements: "Loaded transports (combat recommended)",
        requirement: FleetMissionRequirement::LoadedTransports,
    },
    FleetMissionOption {
        code: 9,
        mission: "View a World",
        requirements: "Any",
        requirement: FleetMissionRequirement::Any,
    },
    FleetMissionOption {
        code: 10,
        mission: "Scout a Sector",
        requirements: "At least one scout",
        requirement: FleetMissionRequirement::AtLeastOneScout,
    },
    FleetMissionOption {
        code: 11,
        mission: "Scout a Solar System",
        requirements: "At least one scout",
        requirement: FleetMissionRequirement::AtLeastOneScout,
    },
    FleetMissionOption {
        code: 12,
        mission: "Colonize a World",
        requirements: "At least one ETAC",
        requirement: FleetMissionRequirement::AtLeastOneEtac,
    },
    FleetMissionOption {
        code: 13,
        mission: "Join another fleet",
        requirements: "Any",
        requirement: FleetMissionRequirement::Any,
    },
    FleetMissionOption {
        code: 14,
        mission: "Rendezvous at Sector",
        requirements: "Any",
        requirement: FleetMissionRequirement::Any,
    },
    FleetMissionOption {
        code: 15,
        mission: "Salvage",
        requirements: "Any",
        requirement: FleetMissionRequirement::Any,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetTargetInputKind {
    None,
    Coordinates,
    StarbaseId,
    FleetId,
}

pub fn fleet_mission_option(order_code: u8) -> Option<FleetMissionOption> {
    FLEET_MISSION_OPTIONS
        .iter()
        .copied()
        .find(|option| option.code == order_code)
}

pub fn fleet_record_supports_requirement(
    fleet: &Fleet,
    requirement: FleetMissionRequirement,
) -> bool {
    let has_combat =
        fleet.battleship_count() > 0 || fleet.cruiser_count() > 0 || fleet.destroyer_count() > 0;
    let has_loaded_troops = fleet.army_count() > 0;
    let has_scout = fleet.scout_count() > 0;
    let has_etac = fleet.etac_count() > 0;

    match requirement {
        FleetMissionRequirement::Any => true,
        FleetMissionRequirement::CombatShips => has_combat,
        FleetMissionRequirement::CombatAndLoadedTransports => has_combat && has_loaded_troops,
        FleetMissionRequirement::LoadedTransports => has_loaded_troops,
        FleetMissionRequirement::AtLeastOneScout => has_scout,
        FleetMissionRequirement::AtLeastOneEtac => has_etac,
    }
}

pub fn fleet_record_supports_mission_code(fleet: &Fleet, order_code: u8) -> bool {
    fleet_mission_option(order_code)
        .map(|option| fleet_record_supports_requirement(fleet, option.requirement))
        .unwrap_or(false)
}

pub fn fleet_target_input_kind(order_code: Option<u8>) -> FleetTargetInputKind {
    match order_code {
        Some(4) => FleetTargetInputKind::StarbaseId,
        Some(13) => FleetTargetInputKind::FleetId,
        Some(_) => FleetTargetInputKind::Coordinates,
        None => FleetTargetInputKind::None,
    }
}

pub fn fleet_target_status_line(order_code: Option<u8>) -> String {
    match order_code {
        Some(4) => "Enter the starbase number for Guard a Starbase.".to_string(),
        Some(13) => "Enter the host fleet number for Join another fleet.".to_string(),
        Some(0) => "Enter the target coordinates for None (hold position).".to_string(),
        Some(1) => "Enter the target coordinates for Move Fleet (only).".to_string(),
        Some(2) => "Enter the target coordinates for Seek Home.".to_string(),
        Some(3) => "Enter the target coordinates for Patrol a Sector.".to_string(),
        Some(5) => "Enter the target coordinates for Guard/Blockade a World.".to_string(),
        Some(6) => "Enter the target coordinates for Bombard a World.".to_string(),
        Some(7) => "Enter the target coordinates for Invade a World.".to_string(),
        Some(8) => "Enter the target coordinates for Blitz a World.".to_string(),
        Some(9) => "Enter the target coordinates for View a World.".to_string(),
        Some(10) => "Enter the target coordinates for Scout a Sector.".to_string(),
        Some(11) => "Enter the target coordinates for Scout a Solar System.".to_string(),
        Some(12) => "Enter the target coordinates for Colonize a World.".to_string(),
        Some(14) => "Enter the target coordinates for Rendezvous at Sector.".to_string(),
        Some(15) => "Enter the target coordinates for Salvage.".to_string(),
        _ => "Enter the target for the selected fleet mission.".to_string(),
    }
}
