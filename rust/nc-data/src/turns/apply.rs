use super::{
    FleetTurnAction, PlanetTurnAction, TurnMessage, TurnSubmission, TurnSubmissionError,
    TurnSubmissionReport,
};
use crate::{
    CoreGameData, Order, ProductionItemKind, QueuedPlayerMail, validate_queued_player_message,
};

pub(super) fn apply_turn_submission(
    submission: &TurnSubmission,
    game_data: &mut CoreGameData,
    queued_mail: &mut Vec<QueuedPlayerMail>,
) -> Result<TurnSubmissionReport, TurnSubmissionError> {
    let player_record_index_1_based = submission.player_record_index_1_based;
    let player_count = game_data.conquest.player_count as usize;
    if player_record_index_1_based == 0 || player_record_index_1_based > player_count {
        return Err(TurnSubmissionError::Validation(format!(
            "turn player must be in 1..={player_count}, got {}",
            player_record_index_1_based
        )));
    }

    let current_year = game_data.conquest.game_year;
    if submission.year != current_year {
        return Err(TurnSubmissionError::Validation(format!(
            "turn year mismatch: file declares {}, campaign year is {}",
            submission.year, current_year
        )));
    }

    preflight_fleet_order_actions(submission, game_data)?;

    if let Some(tax_rate) = submission.tax_rate {
        game_data.set_player_tax_rate(player_record_index_1_based, tax_rate)?;
    }

    for directive in &submission.diplomacy {
        game_data.set_stored_diplomatic_relation(
            player_record_index_1_based as u8,
            directive.to_empire_raw,
            directive.relation,
        )?;
    }

    for planet in &submission.planets {
        for action in &planet.actions {
            apply_planet_action(game_data, player_record_index_1_based, planet, action)?;
        }
    }

    for fleet in &submission.fleets {
        for action in &fleet.actions {
            apply_fleet_action(game_data, player_record_index_1_based, fleet, action)?;
        }
    }

    for message in &submission.messages {
        queue_message(game_data, queued_mail, player_record_index_1_based, message)?;
    }

    Ok(TurnSubmissionReport {
        player_record_index_1_based,
        year: submission.year,
        tax_changed: submission.tax_rate.is_some(),
        diplomacy_updates: submission.diplomacy.len(),
        planet_blocks: submission.planets.len(),
        planet_actions: submission
            .planets
            .iter()
            .map(|planet| planet.actions.len())
            .sum(),
        fleet_blocks: submission.fleets.len(),
        fleet_actions: submission
            .fleets
            .iter()
            .map(|fleet| fleet.actions.len())
            .sum(),
        messages_queued: submission.messages.len(),
    })
}

fn preflight_fleet_order_actions(
    submission: &TurnSubmission,
    game_data: &CoreGameData,
) -> Result<(), TurnSubmissionError> {
    let mut preview = game_data.clone();
    for fleet in &submission.fleets {
        for action in &fleet.actions {
            let FleetTurnAction::Order {
                speed,
                order_code,
                target,
                aux0,
                aux1,
            } = action
            else {
                continue;
            };
            ensure_player_owns_fleet(
                &preview,
                submission.player_record_index_1_based,
                fleet.fleet_record_index_1_based,
            )?;
            let target = resolve_order_target(&preview, fleet.fleet_record_index_1_based, *target)?;
            ensure_order_target_within_map(&preview, *order_code, target)?;
            preview.set_fleet_order(
                fleet.fleet_record_index_1_based,
                *speed,
                *order_code,
                target,
                *aux0,
                *aux1,
            )?;
        }
    }
    Ok(())
}

fn apply_planet_action(
    game_data: &mut CoreGameData,
    player_record_index_1_based: usize,
    planet: &super::PlanetTurnBlock,
    action: &PlanetTurnAction,
) -> Result<(), TurnSubmissionError> {
    match action {
        PlanetTurnAction::Rename { name } => {
            game_data.rename_owned_planet(
                player_record_index_1_based,
                planet.planet_record_index_1_based,
                name,
            )?;
        }
        PlanetTurnAction::ClearBuildQueue => {
            ensure_player_owns_planet(
                game_data,
                player_record_index_1_based,
                planet.planet_record_index_1_based,
            )?;
            game_data.clear_planet_build_queue(planet.planet_record_index_1_based)?;
        }
        PlanetTurnAction::ClearBuildKind { kind_raw } => {
            ensure_player_owns_planet(
                game_data,
                player_record_index_1_based,
                planet.planet_record_index_1_based,
            )?;
            game_data.clear_planet_build_orders_by_kind(
                planet.planet_record_index_1_based,
                ProductionItemKind::from_code(*kind_raw),
            )?;
        }
        PlanetTurnAction::RemoveBuild { qty, kind_raw } => {
            ensure_player_owns_planet(
                game_data,
                player_record_index_1_based,
                planet.planet_record_index_1_based,
            )?;
            let kind = ProductionItemKind::from_code(*kind_raw);
            let Some(cost) = kind.build_cost() else {
                return Err(TurnSubmissionError::Validation(format!(
                    "unknown production kind: {kind_raw}"
                )));
            };
            let points_to_remove = u32::from(*qty).saturating_mul(cost);
            game_data.remove_planet_build_points_by_kind(
                planet.planet_record_index_1_based,
                kind,
                points_to_remove,
            )?;
        }
        PlanetTurnAction::Build {
            points_remaining_raw,
            kind_raw,
        } => {
            ensure_player_owns_planet(
                game_data,
                player_record_index_1_based,
                planet.planet_record_index_1_based,
            )?;
            let points_left =
                game_data.planet_build_points_left(planet.planet_record_index_1_based)?;
            if u32::from(*points_remaining_raw) > points_left {
                return Err(TurnSubmissionError::Validation(format!(
                    "build of {points_remaining_raw} points exceeds planet {}'s remaining budget of {points_left}",
                    planet.planet_record_index_1_based
                )));
            }
            game_data.append_planet_build_order(
                planet.planet_record_index_1_based,
                u32::from(*points_remaining_raw),
                *kind_raw,
            )?;
        }
        PlanetTurnAction::Commission { slot_0_based } => {
            game_data.commission_planet_stardock_slot(
                player_record_index_1_based,
                planet.planet_record_index_1_based,
                *slot_0_based,
            )?;
        }
        PlanetTurnAction::AutoCommission => {
            ensure_player_owns_planet(
                game_data,
                player_record_index_1_based,
                planet.planet_record_index_1_based,
            )?;
            game_data.auto_commission_all_stardock_units(player_record_index_1_based)?;
        }
        PlanetTurnAction::Scorch => {
            ensure_player_owns_planet(
                game_data,
                player_record_index_1_based,
                planet.planet_record_index_1_based,
            )?;
            game_data.scorch_planet_surface(planet.planet_record_index_1_based)?;
        }
    }

    Ok(())
}

fn apply_fleet_action(
    game_data: &mut CoreGameData,
    player_record_index_1_based: usize,
    fleet: &super::FleetTurnBlock,
    action: &FleetTurnAction,
) -> Result<(), TurnSubmissionError> {
    match action {
        FleetTurnAction::Order {
            speed,
            order_code,
            target,
            aux0,
            aux1,
        } => {
            ensure_player_owns_fleet(
                game_data,
                player_record_index_1_based,
                fleet.fleet_record_index_1_based,
            )?;
            let target =
                resolve_order_target(game_data, fleet.fleet_record_index_1_based, *target)?;
            game_data.set_fleet_order(
                fleet.fleet_record_index_1_based,
                *speed,
                *order_code,
                target,
                *aux0,
                *aux1,
            )?;
        }
        FleetTurnAction::RulesOfEngagement { value } => {
            game_data.set_fleet_rules_of_engagement(
                player_record_index_1_based,
                fleet.fleet_record_index_1_based,
                *value,
            )?;
        }
        FleetTurnAction::Speed { value } => {
            game_data.set_fleet_speed(
                player_record_index_1_based,
                fleet.fleet_record_index_1_based,
                *value,
            )?;
        }
        FleetTurnAction::Join {
            host_fleet_record_index_1_based,
        } => {
            game_data.set_join_fleet_order(
                player_record_index_1_based,
                fleet.fleet_record_index_1_based,
                *host_fleet_record_index_1_based,
            )?;
        }
        FleetTurnAction::Detach {
            selection,
            donor_speed,
            new_fleet_roe,
        } => {
            game_data.detach_ships_to_new_fleet(
                player_record_index_1_based,
                fleet.fleet_record_index_1_based,
                *selection,
                *donor_speed,
                *new_fleet_roe,
            )?;
        }
        FleetTurnAction::Transfer {
            host_fleet_record_index_1_based,
            selection,
        } => {
            game_data.transfer_ships_between_fleets(
                player_record_index_1_based,
                fleet.fleet_record_index_1_based,
                *host_fleet_record_index_1_based,
                *selection,
            )?;
        }
        FleetTurnAction::LoadArmies {
            planet_record_index_1_based,
            qty,
        } => {
            game_data.load_planet_armies_onto_fleet(
                player_record_index_1_based,
                *planet_record_index_1_based,
                fleet.fleet_record_index_1_based,
                *qty,
            )?;
        }
        FleetTurnAction::UnloadArmies {
            planet_record_index_1_based,
            qty,
        } => {
            game_data.unload_fleet_armies_to_planet(
                player_record_index_1_based,
                *planet_record_index_1_based,
                fleet.fleet_record_index_1_based,
                *qty,
            )?;
        }
    }

    Ok(())
}

fn ensure_player_owns_planet(
    game_data: &CoreGameData,
    player_record_index_1_based: usize,
    planet_record_index_1_based: usize,
) -> Result<(), TurnSubmissionError> {
    let planet = game_data
        .planets
        .get(planet_record_index_1_based - 1)
        .ok_or_else(|| {
            TurnSubmissionError::Validation(format!(
                "missing planet record {}",
                planet_record_index_1_based
            ))
        })?;
    if planet.owner.0 as usize != player_record_index_1_based {
        return Err(TurnSubmissionError::Validation(format!(
            "planet {} is not owned by player {}",
            planet_record_index_1_based, player_record_index_1_based
        )));
    }
    Ok(())
}

fn ensure_player_owns_fleet(
    game_data: &CoreGameData,
    player_record_index_1_based: usize,
    fleet_record_index_1_based: usize,
) -> Result<(), TurnSubmissionError> {
    let fleet = game_data
        .fleets
        .get(fleet_record_index_1_based - 1)
        .ok_or_else(|| {
            TurnSubmissionError::Validation(format!(
                "missing fleet record {}",
                fleet_record_index_1_based
            ))
        })?;
    if fleet.owner.0 as usize != player_record_index_1_based {
        return Err(TurnSubmissionError::Validation(format!(
            "fleet {} is not owned by player {}",
            fleet_record_index_1_based, player_record_index_1_based
        )));
    }
    Ok(())
}

/// Submitted orders get the same 1-based map bounds the player UI enforces.
/// A Seek Home or Salvage order without a target takes the nearest owned
/// world, as ECGAME does.
fn resolve_order_target(
    game_data: &CoreGameData,
    fleet_record_index_1_based: usize,
    target: Option<[u8; 2]>,
) -> Result<[u8; 2], TurnSubmissionError> {
    if let Some(target) = target {
        return Ok(target);
    }
    game_data
        .nearest_owned_world(fleet_record_index_1_based, false)
        .ok_or_else(|| {
            TurnSubmissionError::Validation(format!(
                "fleet record {fleet_record_index_1_based} has no owned world to head for"
            ))
        })
}

fn ensure_order_target_within_map(
    game_data: &CoreGameData,
    order_code: u8,
    target: [u8; 2],
) -> Result<(), TurnSubmissionError> {
    let map_size = crate::map_size_for_player_count(game_data.conquest.player_count);
    if Order::from_code(order_code) == Order::HoldPosition
        || crate::sector_within_map(target, map_size)
    {
        return Ok(());
    }
    Err(TurnSubmissionError::Validation(format!(
        "fleet order target ({},{}) is outside the map (1..={map_size})",
        target[0], target[1]
    )))
}

fn queue_message(
    game_data: &CoreGameData,
    queued_mail: &mut Vec<QueuedPlayerMail>,
    sender_player_record_index_1_based: usize,
    message: &TurnMessage,
) -> Result<(), TurnSubmissionError> {
    let (subject, body) = validate_queued_player_message(
        queued_mail,
        sender_player_record_index_1_based as u8,
        message.recipient_empire_raw,
        game_data.conquest.game_year,
        game_data.conquest.player_count,
        &message.subject,
        &message.body,
    )
    .map_err(TurnSubmissionError::Validation)?;

    queued_mail.push(QueuedPlayerMail {
        sender_empire_id: sender_player_record_index_1_based as u8,
        recipient_empire_id: message.recipient_empire_raw,
        year: game_data.conquest.game_year,
        subject,
        body,
        recipient_deleted: false,
    });
    Ok(())
}
