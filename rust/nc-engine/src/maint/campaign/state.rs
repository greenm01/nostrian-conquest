use super::super::{
    CampaignOutcomeEvent, CampaignOutlookEvent, CivilDisorderEvent, EmpireEliminationCause,
    EmpireEliminationEvent, FleetDefectionEvent, GameVictoryNoticeEvent, MaintenanceEvents,
};
use crate::{CoreGameData, DiplomaticRelation};
use nc_data::{
    CampaignOutcome, CampaignOutlook, PlayerLifecycleState, PlayerMode, TerminalOutcome,
    WinnerState, empire_has_recovery_path, is_campaign_contender, is_contesting_empire,
};

const RECOVERY_WINDOW_TURNS: u8 = 3;

#[derive(Debug, Clone, Copy)]
pub(crate) struct EmpireTurnStartState {
    owned_planets: usize,
}

#[derive(Debug, Default)]
pub(crate) struct CampaignTransitionEvents {
    pub civil_disorder_events: Vec<CivilDisorderEvent>,
    pub campaign_outlook_events: Vec<CampaignOutlookEvent>,
    pub campaign_outcome_events: Vec<CampaignOutcomeEvent>,
    pub empire_elimination_events: Vec<EmpireEliminationEvent>,
    pub game_victory_notice_events: Vec<GameVictoryNoticeEvent>,
}

pub(super) fn capture_empire_turn_start_states(
    game_data: &CoreGameData,
    _player_lifecycle_states: &[PlayerLifecycleState],
) -> Vec<EmpireTurnStartState> {
    (1..=game_data.players.len())
        .map(|empire_idx| EmpireTurnStartState {
            owned_planets: owned_planet_count(game_data, empire_idx as u8),
        })
        .collect()
}

pub(super) fn campaign_outlook(
    game_data: &CoreGameData,
    player_lifecycle_states: &[PlayerLifecycleState],
) -> CampaignOutlook {
    match sole_contender(game_data, player_lifecycle_states) {
        Some(empire) => CampaignOutlook::SoleContender(empire),
        None => CampaignOutlook::Contested,
    }
}

pub(super) fn campaign_outcome(
    game_data: &CoreGameData,
    player_lifecycle_states: &[PlayerLifecycleState],
    winner_state: WinnerState,
) -> CampaignOutcome {
    if let Some(emperor_empire) = winner_state.winner_empire {
        return CampaignOutcome::RecognizedEmperor(emperor_empire);
    }

    match sole_contender(game_data, player_lifecycle_states) {
        Some(empire)
            if owns_any_planets(game_data, empire) && !is_rogue_empire(game_data, empire) =>
        {
            CampaignOutcome::RecognizedEmperor(empire)
        }
        _ => CampaignOutcome::Ongoing,
    }
}

pub(super) fn apply_campaign_state_transitions(
    game_data: &mut CoreGameData,
    start_states: &[EmpireTurnStartState],
    ownership_change_events: &[nc_data::PlanetOwnershipChangeEvent],
    fleet_destroyed_events: &[nc_data::FleetDestroyedEvent],
    player_lifecycle_states: &mut [PlayerLifecycleState],
    winner_state: &mut WinnerState,
    initial_outlook: CampaignOutlook,
    initial_outcome: CampaignOutcome,
) -> CampaignTransitionEvents {
    let mut events = CampaignTransitionEvents::default();
    let player_count = game_data.players.len();

    for empire_idx in 1..=player_count {
        let empire = empire_idx as u8;
        let Some(player) = game_data.players.get(empire_idx - 1) else {
            continue;
        };
        let lifecycle = player_lifecycle_states
            .get_mut(empire_idx - 1)
            .expect("lifecycle slice matches player count");
        let start = start_states
            .get(empire_idx - 1)
            .copied()
            .unwrap_or(EmpireTurnStartState { owned_planets: 0 });

        if matches!(
            lifecycle.terminal_outcome,
            TerminalOutcome::Defeated | TerminalOutcome::LostGame | TerminalOutcome::Winner
        ) {
            continue;
        }
        // ECMAINT keeps playing a rogue empire after its last world falls:
        // its warships patrol (`24a3:0dbc`) and the slot stays rogue.
        if !is_contesting_empire(player) || player.mode == PlayerMode::Rogue {
            continue;
        }

        let owned_planets = owned_planet_count(game_data, empire);
        if owned_planets > 0 {
            lifecycle.recovery_window_turns_remaining = 0;
            continue;
        }

        let has_recovery_path = empire_has_recovery_path(game_data, empire);
        if has_recovery_path {
            if start.owned_planets > 0 {
                lifecycle.recovery_window_turns_remaining = RECOVERY_WINDOW_TURNS;
            } else if lifecycle.recovery_window_turns_remaining == 0 {
                lifecycle.recovery_window_turns_remaining = RECOVERY_WINDOW_TURNS;
            } else if lifecycle.recovery_window_turns_remaining > 1 {
                lifecycle.recovery_window_turns_remaining -= 1;
            } else {
                defeat_empire(
                    game_data,
                    lifecycle,
                    empire,
                    EmpireEliminationCause::RecoveryWindowExpired,
                    None,
                    &mut events.empire_elimination_events,
                );
            }
            continue;
        }

        // The window only expires in the has-path branch above; losing the
        // last recovery force inside the window is a force loss.
        let cause = if start.owned_planets > 0 {
            EmpireEliminationCause::LastPlanetLost
        } else {
            EmpireEliminationCause::LastRecoveryForceDestroyed
        };
        let context = elimination_context(
            game_data,
            ownership_change_events,
            fleet_destroyed_events,
            empire,
            cause,
        );
        defeat_empire(
            game_data,
            lifecycle,
            empire,
            cause,
            context,
            &mut events.empire_elimination_events,
        );
    }

    let after_outlook = campaign_outlook(game_data, player_lifecycle_states);
    if !matches!(initial_outlook, CampaignOutlook::SoleContender(_))
        && matches!(after_outlook, CampaignOutlook::SoleContender(_))
    {
        if let CampaignOutlook::SoleContender(empire) = after_outlook {
            events.campaign_outlook_events.push(CampaignOutlookEvent {
                empire,
                stardate_week: None,
            });
        }
    }

    let after_outcome = campaign_outcome(game_data, player_lifecycle_states, *winner_state);
    if winner_state.winner_empire.is_none()
        && !matches!(initial_outcome, CampaignOutcome::RecognizedEmperor(_))
        && matches!(after_outcome, CampaignOutcome::RecognizedEmperor(_))
    {
        let CampaignOutcome::RecognizedEmperor(emperor_empire) = after_outcome else {
            unreachable!();
        };
        winner_state.winner_empire = Some(emperor_empire);
        winner_state.winner_declared_year = Some(game_data.conquest.game_year);
        if let Some(lifecycle) = player_lifecycle_states.get_mut(emperor_empire as usize - 1) {
            lifecycle.terminal_outcome = TerminalOutcome::Winner;
            lifecycle.terminal_review_consumed = false;
        }

        for empire_idx in 1..=player_count {
            let empire = empire_idx as u8;
            let Some(lifecycle) = player_lifecycle_states.get_mut(empire_idx - 1) else {
                continue;
            };
            if empire == emperor_empire {
                events
                    .game_victory_notice_events
                    .push(GameVictoryNoticeEvent {
                        recipient_empire: empire,
                        winner_empire: emperor_empire,
                        stardate_week: None,
                    });
                continue;
            }
            if !matches!(lifecycle.terminal_outcome, TerminalOutcome::None) {
                continue;
            }
            if is_joined_empire(&game_data.players[empire_idx - 1]) {
                lifecycle.terminal_outcome = TerminalOutcome::LostGame;
                lifecycle.terminal_review_consumed = false;
                events
                    .game_victory_notice_events
                    .push(GameVictoryNoticeEvent {
                        recipient_empire: empire,
                        winner_empire: emperor_empire,
                        stardate_week: None,
                    });
            }
        }

        events.campaign_outcome_events.push(CampaignOutcomeEvent {
            emperor_empire,
            stardate_week: None,
        });
    }

    events
}

/// A civil-disorder empire with no world loses one fleet to defection each
/// year. ECMAINT's orders pass turns such an empire into a rogue when the next
/// run starts (`24a3:0c2a`), so this sees only an empire that lost its last
/// world, and was not eliminated, in the run just played.
pub(super) fn apply_civil_disorder_fleet_defections(
    game_data: &mut CoreGameData,
    newly_defeated: &[EmpireEliminationEvent],
) -> Result<Vec<FleetDefectionEvent>, Box<dyn std::error::Error>> {
    let mut to_remove = vec![false; game_data.fleets.len()];
    let mut events = Vec::new();

    for empire in 1..=game_data.players.len() as u8 {
        let Some(player) = game_data.players.get(empire.saturating_sub(1) as usize) else {
            continue;
        };
        if player.mode.code() != 0x00 {
            continue;
        }
        if newly_defeated
            .iter()
            .any(|event| event.defeated_empire == empire)
        {
            continue;
        }
        if game_data
            .planets
            .iter()
            .any(|planet| planet.owner.0 == empire)
        {
            continue;
        }

        let candidate = game_data
            .fleets
            .iter()
            .enumerate()
            .filter(|(_, fleet)| fleet.owner.0 == empire && super::super::fleet_has_presence(fleet))
            .max_by_key(|(_, fleet)| fleet.record.0);

        if let Some((fleet_idx, fleet)) = candidate {
            to_remove[fleet_idx] = true;
            events.push(FleetDefectionEvent {
                reporting_empire: empire,
                fleet_number: fleet.fleet_number.0 as u8,
                stardate_week: None,
            });
        }
    }

    if to_remove.iter().any(|remove| *remove) {
        super::super::remove_selected_fleets(game_data, &to_remove);
    }

    Ok(events)
}

pub(super) fn apply_stored_diplomatic_escalations(
    game_data: &mut CoreGameData,
    events: &MaintenanceEvents,
) -> Result<(), Box<dyn std::error::Error>> {
    // Fleet battles leave stored diplomacy alone, as in classic.
    let mut pairs = Vec::new();

    for event in &events.bombard_events {
        if event.defender_empire != 0 {
            pairs.push((event.attacker_empire, event.defender_empire));
        }
    }

    for event in &events.assault_report_events {
        if event.defender_empire != 0 {
            pairs.push((event.attacker_empire, event.defender_empire));
        }
    }

    for event in &events.diplomatic_escalation_events {
        pairs.push((event.left_empire, event.right_empire));
    }

    for (left, right) in pairs {
        if left == 0 || right == 0 || left == right {
            continue;
        }
        let _ = game_data.set_stored_diplomatic_relation(left, right, DiplomaticRelation::Enemy)?;
        let _ = game_data.set_stored_diplomatic_relation(right, left, DiplomaticRelation::Enemy)?;
    }

    Ok(())
}

/// An eliminated empire goes to civil disorder; the next run's orders pass
/// turns it into a rogue, and its player keeps the final review
/// (`TerminalOutcome::Defeated`).
fn defeat_empire(
    game_data: &mut CoreGameData,
    lifecycle: &mut PlayerLifecycleState,
    empire: u8,
    cause: EmpireEliminationCause,
    context: Option<(Option<u8>, Option<usize>, Option<[u8; 2]>)>,
    events: &mut Vec<EmpireEliminationEvent>,
) {
    lifecycle.recovery_window_turns_remaining = 0;
    lifecycle.terminal_outcome = TerminalOutcome::Defeated;
    lifecycle.terminal_review_consumed = false;
    if let Some(player) = game_data.players.get_mut(empire.saturating_sub(1) as usize) {
        player.mode = PlayerMode::CivilDisorder;
    }
    let (victor_empire, planet_idx, coords) = context.unwrap_or((None, None, None));
    events.push(EmpireEliminationEvent {
        defeated_empire: empire,
        victor_empire,
        cause,
        planet_idx,
        coords,
        stardate_week: None,
    });
}

fn elimination_context(
    game_data: &CoreGameData,
    ownership_change_events: &[nc_data::PlanetOwnershipChangeEvent],
    fleet_destroyed_events: &[nc_data::FleetDestroyedEvent],
    empire: u8,
    cause: EmpireEliminationCause,
) -> Option<(Option<u8>, Option<usize>, Option<[u8; 2]>)> {
    match cause {
        EmpireEliminationCause::LastPlanetLost => ownership_change_events
            .iter()
            .rev()
            .find(|event| event.previous_owner_empire == empire)
            .map(|event| {
                let coords = game_data
                    .planets
                    .get(event.planet_idx)
                    .map(|planet| planet.sector.to_array());
                (Some(event.new_owner_empire), Some(event.planet_idx), coords)
            }),
        EmpireEliminationCause::LastRecoveryForceDestroyed => fleet_destroyed_events
            .iter()
            .rev()
            .find(|event| event.reporting_empire == empire)
            .map(|event| (event.primary_enemy_empire, None, Some(event.coords))),
        EmpireEliminationCause::RecoveryWindowExpired => None,
    }
}

fn sole_contender(
    game_data: &CoreGameData,
    player_lifecycle_states: &[PlayerLifecycleState],
) -> Option<u8> {
    let contenders = (1..=game_data.players.len() as u8)
        .filter(|&empire| is_campaign_contender(game_data, empire, player_lifecycle_states))
        .collect::<Vec<_>>();
    if contenders.len() == 1 {
        contenders.first().copied()
    } else {
        None
    }
}

fn owns_any_planets(game_data: &CoreGameData, empire: u8) -> bool {
    owned_planet_count(game_data, empire) > 0
}

fn owned_planet_count(game_data: &CoreGameData, empire: u8) -> usize {
    game_data
        .planets
        .iter()
        .filter(|planet| planet.owner.0 == empire)
        .count()
}

fn is_joined_empire(player: &nc_data::Player) -> bool {
    !matches!(player.mode.code(), 0x00 | 0xff)
}

fn is_rogue_empire(game_data: &CoreGameData, empire: u8) -> bool {
    game_data
        .players
        .get(empire.saturating_sub(1) as usize)
        .map(|player| player.mode.code() == 0xff)
        .unwrap_or(false)
}
