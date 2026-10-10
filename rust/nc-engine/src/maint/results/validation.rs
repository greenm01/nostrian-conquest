use nc_data::{
    FleetOrderValidationError, FleetPlayerInputValidationError, PlanetPlayerInputValidationError,
    PlayerDiplomacyValidationError,
};

pub fn fleet_order_validation_reason_text(reason: FleetOrderValidationError) -> String {
    match reason {
        FleetOrderValidationError::UnknownOrderCode(code) => {
            format!("unknown mission code {code:#04x}")
        }
        FleetOrderValidationError::MissingCombatShips => {
            "the fleet lacks the required combat ships".to_string()
        }
        FleetOrderValidationError::MissingScoutShip => {
            "the fleet lacks the required scout ship".to_string()
        }
        FleetOrderValidationError::MissingEtac => "the fleet lacks the required ETAC".to_string(),
        FleetOrderValidationError::MissingLoadedTroopTransports => {
            "the fleet lacks loaded troop transports".to_string()
        }
        FleetOrderValidationError::MissingPlanetTarget => {
            "the mission target is not a valid world".to_string()
        }
        FleetOrderValidationError::TargetOwnedByFleetEmpire => {
            "the target world belongs to us".to_string()
        }
        FleetOrderValidationError::TargetNotOwnedByFleetEmpire => {
            "the target world is not under our control".to_string()
        }
        FleetOrderValidationError::TargetAlreadyOwned => {
            "the target world is already owned".to_string()
        }
        FleetOrderValidationError::DuplicateFriendlyColonizeTarget {
            target,
            conflicting_fleet_record_index_1_based,
        } => format!(
            "another of our fleets is already set to colonize ({:02},{:02}) (fleet record #{conflicting_fleet_record_index_1_based})",
            target[0], target[1]
        ),
        FleetOrderValidationError::InvalidJoinHost => {
            "the target fleet no longer exists".to_string()
        }
        FleetOrderValidationError::InvalidGuardStarbase => {
            "the selected starbase linkage is invalid".to_string()
        }
        FleetOrderValidationError::NotNearestOwnedWorld => {
            "the target world is not our nearest world".to_string()
        }
    }
}

pub fn capability_loss_invalid_order_reason(reason: FleetOrderValidationError) -> bool {
    matches!(
        reason,
        FleetOrderValidationError::MissingCombatShips
            | FleetOrderValidationError::MissingScoutShip
            | FleetOrderValidationError::MissingEtac
            | FleetOrderValidationError::MissingLoadedTroopTransports
    )
}

pub fn fleet_player_input_validation_reason_text(
    reason: FleetPlayerInputValidationError,
) -> String {
    match reason {
        FleetPlayerInputValidationError::InvalidOrder(order_reason) => {
            fleet_order_validation_reason_text(order_reason)
        }
        FleetPlayerInputValidationError::SpeedExceedsMaximum { speed, max } => {
            format!("fleet speed {speed} exceeded the current maximum speed {max}")
        }
        FleetPlayerInputValidationError::RulesOfEngagementOutOfRange { roe } => {
            format!("rules of engagement {roe} was outside the valid 0-10 range")
        }
        FleetPlayerInputValidationError::NonCombatFleetMustUseZeroRoe { roe } => {
            format!(
                "fleet with only scouts, transports, and ETACs used ROE {roe}; support-only fleets must use ROE 0"
            )
        }
    }
}

pub fn planet_input_validation_reason_text(reason: PlanetPlayerInputValidationError) -> String {
    match reason {
        PlanetPlayerInputValidationError::InvalidBuildKind(kind) => {
            format!("the build queue contains unknown item kind {kind:#04x}")
        }
        PlanetPlayerInputValidationError::InvalidBuildPointsForKind {
            kind_raw,
            points_remaining_raw,
        } => {
            format!(
                "the build queue stores invalid points {points_remaining_raw} for item kind {kind_raw:#04x}"
            )
        }
        PlanetPlayerInputValidationError::MissingBuildKindForCount => {
            "a build queue slot had points remaining but no build kind".to_string()
        }
        PlanetPlayerInputValidationError::MissingBuildCountForKind => {
            "a build queue slot named an item but had zero remaining cost".to_string()
        }
        PlanetPlayerInputValidationError::InvalidStardockKind(kind) => {
            format!("the stardock contains unknown item kind {kind:#04x}")
        }
        PlanetPlayerInputValidationError::MissingStardockKindForCount => {
            "a stardock slot stored units with no item kind".to_string()
        }
        PlanetPlayerInputValidationError::MissingStardockCountForKind => {
            "a stardock slot named an item but stored zero units".to_string()
        }
        PlanetPlayerInputValidationError::InvalidTaxRate(rate) => {
            format!("the attached tax input {rate}% is invalid")
        }
    }
}

pub fn diplomacy_input_validation_reason_text(reason: PlayerDiplomacyValidationError) -> String {
    match reason {
        PlayerDiplomacyValidationError::TargetOutOfRange { target_empire_raw } => {
            format!(
                "target empire {} was outside the active player range",
                target_empire_raw
            )
        }
        PlayerDiplomacyValidationError::SelfTarget { empire_raw } => {
            format!(
                "empire {} attempted to target itself in diplomacy",
                empire_raw
            )
        }
        PlayerDiplomacyValidationError::InvalidStoredRelationByte {
            target_empire_raw,
            raw,
        } => format!(
            "stored diplomacy byte {raw:#04x} toward empire {} was invalid",
            target_empire_raw
        ),
    }
}
