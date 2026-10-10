use super::super::{
    CampaignOutcomeEvent, CampaignOutlookEvent, CivilDisorderEvent, EmpireEliminationCause,
    EmpireEliminationEvent, FleetDefectionEvent, GameVictoryNoticeEvent, MaintenanceEvents,
};
use crate::{CoreGameData, DiplomaticRelation};
use nc_data::{
    CampaignOutcome, CampaignOutlook, PlayerLifecycleState, TerminalOutcome, WinnerState,
    empire_has_recovery_path, is_campaign_contender, is_contesting_empire,
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
    (1..=game_data.player.records.len())
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
        Some(empire_raw) => CampaignOutlook::SoleContender(empire_raw),
        None => CampaignOutlook::Contested,
    }
}

pub(super) fn campaign_outcome(
    game_data: &CoreGameData,
    player_lifecycle_states: &[PlayerLifecycleState],
    winner_state: WinnerState,
) -> CampaignOutcome {
    if let Some(emperor_empire_raw) = winner_state.winner_empire_raw {
        return CampaignOutcome::RecognizedEmperor(emperor_empire_raw);
    }

    match sole_contender(game_data, player_lifecycle_states) {
        Some(empire_raw)
            if owns_any_planets(game_data, empire_raw)
                && !is_rogue_empire(game_data, empire_raw) =>
        {
            CampaignOutcome::RecognizedEmperor(empire_raw)
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
    let player_count = game_data.player.records.len();

    for empire_idx in 1..=player_count {
        let empire_raw = empire_idx as u8;
        let Some(player) = game_data.player.records.get(empire_idx - 1) else {
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
        if !is_contesting_empire(player) {
            continue;
        }

        let owned_planets = owned_planet_count(game_data, empire_raw);
        if owned_planets > 0 {
            lifecycle.recovery_window_turns_remaining = 0;
            continue;
        }

        let has_recovery_path = empire_has_recovery_path(game_data, empire_raw);
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
                    empire_raw,
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
            empire_raw,
            cause,
        );
        defeat_empire(
            game_data,
            lifecycle,
            empire_raw,
            cause,
            context,
            &mut events.empire_elimination_events,
        );
    }

    let after_outlook = campaign_outlook(game_data, player_lifecycle_states);
    if !matches!(initial_outlook, CampaignOutlook::SoleContender(_))
        && matches!(after_outlook, CampaignOutlook::SoleContender(_))
    {
        if let CampaignOutlook::SoleContender(empire_raw) = after_outlook {
            events.campaign_outlook_events.push(CampaignOutlookEvent {
                empire_raw,
                stardate_week: None,
            });
        }
    }

    let after_outcome = campaign_outcome(game_data, player_lifecycle_states, *winner_state);
    if winner_state.winner_empire_raw.is_none()
        && !matches!(initial_outcome, CampaignOutcome::RecognizedEmperor(_))
        && matches!(after_outcome, CampaignOutcome::RecognizedEmperor(_))
    {
        let CampaignOutcome::RecognizedEmperor(emperor_empire_raw) = after_outcome else {
            unreachable!();
        };
        winner_state.winner_empire_raw = Some(emperor_empire_raw);
        winner_state.winner_declared_year = Some(game_data.conquest.game_year());
        if let Some(lifecycle) = player_lifecycle_states.get_mut(emperor_empire_raw as usize - 1) {
            lifecycle.terminal_outcome = TerminalOutcome::Winner;
            lifecycle.terminal_review_consumed = false;
        }

        for empire_idx in 1..=player_count {
            let empire_raw = empire_idx as u8;
            let Some(lifecycle) = player_lifecycle_states.get_mut(empire_idx - 1) else {
                continue;
            };
            if empire_raw == emperor_empire_raw {
                events
                    .game_victory_notice_events
                    .push(GameVictoryNoticeEvent {
                        recipient_empire_raw: empire_raw,
                        winner_empire_raw: emperor_empire_raw,
                        stardate_week: None,
                    });
                continue;
            }
            if !matches!(lifecycle.terminal_outcome, TerminalOutcome::None) {
                continue;
            }
            if is_joined_empire(&game_data.player.records[empire_idx - 1]) {
                lifecycle.terminal_outcome = TerminalOutcome::LostGame;
                lifecycle.terminal_review_consumed = false;
                events
                    .game_victory_notice_events
                    .push(GameVictoryNoticeEvent {
                        recipient_empire_raw: empire_raw,
                        winner_empire_raw: emperor_empire_raw,
                        stardate_week: None,
                    });
            }
        }

        events.campaign_outcome_events.push(CampaignOutcomeEvent {
            emperor_empire_raw,
            stardate_week: None,
        });
    }

    events
}

pub(super) fn apply_civil_disorder_fleet_defections(
    game_data: &mut CoreGameData,
    newly_defeated: &[EmpireEliminationEvent],
) -> Result<Vec<FleetDefectionEvent>, Box<dyn std::error::Error>> {
    let mut to_remove = vec![false; game_data.fleets.records.len()];
    let mut events = Vec::new();

    for empire_raw in 1..=game_data.player.records.len() as u8 {
        let Some(player) = game_data
            .player
            .records
            .get(empire_raw.saturating_sub(1) as usize)
        else {
            continue;
        };
        if player.owner_mode_raw() != 0x00 {
            continue;
        }
        if newly_defeated
            .iter()
            .any(|event| event.defeated_empire_raw == empire_raw)
        {
            continue;
        }
        if game_data
            .planets
            .records
            .iter()
            .any(|planet| planet.owner_empire_slot_raw() == empire_raw)
        {
            continue;
        }

        let candidate = game_data
            .fleets
            .records
            .iter()
            .enumerate()
            .filter(|(_, fleet)| {
                fleet.owner_empire_raw() == empire_raw && super::super::fleet_has_presence(fleet)
            })
            .max_by_key(|(_, fleet)| fleet.fleet_id());

        if let Some((fleet_idx, fleet)) = candidate {
            to_remove[fleet_idx] = true;
            events.push(FleetDefectionEvent {
                reporting_empire_raw: empire_raw,
                fleet_number: fleet.local_slot_word_raw() as u8,
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
    let mut pairs = Vec::new();

    for event in &events.fleet_battle_events {
        for &enemy_empire_raw in &event.enemy_empires_raw {
            pairs.push((event.reporting_empire_raw, enemy_empire_raw));
        }
    }

    for event in &events.bombard_events {
        if event.defender_empire_raw != 0 {
            pairs.push((event.attacker_empire_raw, event.defender_empire_raw));
        }
    }

    for event in &events.assault_report_events {
        if event.defender_empire_raw != 0 {
            pairs.push((event.attacker_empire_raw, event.defender_empire_raw));
        }
    }

    for event in &events.diplomatic_escalation_events {
        pairs.push((event.left_empire_raw, event.right_empire_raw));
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

pub(super) fn update_player_starbase_flag(game_data: &mut CoreGameData) {
    for player in game_data.player.records.iter_mut() {
        let sc = player.starbase_count_raw();
        player.set_starbase_presence_flag_raw(if sc > 0 { 0x01 } else { 0x00 });
    }
}

fn defeat_empire(
    game_data: &mut CoreGameData,
    lifecycle: &mut PlayerLifecycleState,
    empire_raw: u8,
    cause: EmpireEliminationCause,
    context: Option<(Option<u8>, Option<usize>, Option<[u8; 2]>)>,
    events: &mut Vec<EmpireEliminationEvent>,
) {
    lifecycle.recovery_window_turns_remaining = 0;
    lifecycle.terminal_outcome = TerminalOutcome::Defeated;
    lifecycle.terminal_review_consumed = false;
    if let Some(player) = game_data
        .player
        .records
        .get_mut(empire_raw.saturating_sub(1) as usize)
    {
        player.set_owner_empire_raw(0x00);
    }
    let (victor_empire_raw, planet_idx, coords) = context.unwrap_or((None, None, None));
    events.push(EmpireEliminationEvent {
        defeated_empire_raw: empire_raw,
        victor_empire_raw,
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
    empire_raw: u8,
    cause: EmpireEliminationCause,
) -> Option<(Option<u8>, Option<usize>, Option<[u8; 2]>)> {
    match cause {
        EmpireEliminationCause::LastPlanetLost => ownership_change_events
            .iter()
            .rev()
            .find(|event| event.previous_owner_empire_raw == empire_raw)
            .map(|event| {
                let coords = game_data
                    .planets
                    .records
                    .get(event.planet_idx)
                    .map(|planet| planet.coords_raw());
                (
                    Some(event.new_owner_empire_raw),
                    Some(event.planet_idx),
                    coords,
                )
            }),
        EmpireEliminationCause::LastRecoveryForceDestroyed => fleet_destroyed_events
            .iter()
            .rev()
            .find(|event| event.reporting_empire_raw == empire_raw)
            .map(|event| (event.primary_enemy_empire_raw, None, Some(event.coords))),
        EmpireEliminationCause::RecoveryWindowExpired => None,
    }
}

fn sole_contender(
    game_data: &CoreGameData,
    player_lifecycle_states: &[PlayerLifecycleState],
) -> Option<u8> {
    let contenders = (1..=game_data.player.records.len() as u8)
        .filter(|&empire_raw| {
            is_campaign_contender(game_data, empire_raw, player_lifecycle_states)
        })
        .collect::<Vec<_>>();
    if contenders.len() == 1 {
        contenders.first().copied()
    } else {
        None
    }
}

fn owns_any_planets(game_data: &CoreGameData, empire_raw: u8) -> bool {
    owned_planet_count(game_data, empire_raw) > 0
}

fn owned_planet_count(game_data: &CoreGameData, empire_raw: u8) -> usize {
    game_data
        .planets
        .records
        .iter()
        .filter(|planet| planet.owner_empire_slot_raw() == empire_raw)
        .count()
}

fn is_joined_empire(player: &nc_data::PlayerRecord) -> bool {
    !matches!(player.owner_mode_raw(), 0x00 | 0xff)
}

fn is_rogue_empire(game_data: &CoreGameData, empire_raw: u8) -> bool {
    game_data
        .player
        .records
        .get(empire_raw.saturating_sub(1) as usize)
        .map(|player| player.owner_mode_raw() == 0xff)
        .unwrap_or(false)
}
