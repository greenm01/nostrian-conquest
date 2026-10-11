use nc_data::{
    EmpireEliminationCause, GameStateBuilder, PlayerAccessMode, PlayerActivityState,
    PlayerLifecycleState, PlayerMode, PublicEmpireStatus, TerminalOutcome, WinnerState,
    default_player_lifecycle_states, empire_has_recovery_path, player_access_mode,
    player_public_status, set_armies_aboard, set_troop_transports,
};
use nc_engine::run_maintenance_turn_with_context_seed_and_lifecycle;

fn baseline_game() -> nc_data::CoreGameData {
    GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build")
}

fn strip_empire_planets(game_data: &mut nc_data::CoreGameData, empire_raw: u8) {
    for planet in &mut game_data.planets {
        if planet.owner.0 == empire_raw {
            planet.owner.0 = 0;
            planet.years_under_owner = 0;
        }
    }
}

fn strip_empire_ships(game_data: &mut nc_data::CoreGameData, empire_raw: u8) {
    for fleet in &mut game_data.fleets {
        if fleet.owner.0 == empire_raw {
            fleet.ships.destroyers = 0;
            fleet.ships.cruisers = 0;
            fleet.ships.battleships = 0;
            fleet.ships.scouts = 0;
            set_troop_transports(fleet, 0);
            set_armies_aboard(fleet, 0);
            fleet.ships.etacs = 0;
        }
    }
}

fn run_turn(
    game_data: &mut nc_data::CoreGameData,
    lifecycle: &mut [PlayerLifecycleState],
    winner: &mut WinnerState,
) -> nc_data::MaintenanceEvents {
    run_maintenance_turn_with_context_seed_and_lifecycle(game_data, 1, &[], &[], lifecycle, winner)
        .expect("maintenance should succeed")
}

fn default_activity_states(player_count: usize) -> Vec<PlayerActivityState> {
    (1..=player_count)
        .map(|player_record_index_1_based| PlayerActivityState {
            player_record_index_1_based,
            last_participation_year: 0,
            inactivity_autopilot_pending_clear: false,
        })
        .collect()
}

#[test]
fn public_status_reports_mia_when_inactivity_autopilot_is_active() {
    let mut game_data = baseline_game();
    game_data.players[0].autopilot = true;
    let mut activity = default_activity_states(game_data.players.len());
    activity[0].inactivity_autopilot_pending_clear = true;

    let status = player_public_status(
        &game_data,
        1,
        &activity,
        &default_player_lifecycle_states(game_data.conquest.player_count),
    );

    assert_eq!(status, PublicEmpireStatus::Mia);
}

#[test]
fn public_status_reports_defeated_for_terminal_loss_states() {
    let game_data = baseline_game();
    let activity = default_activity_states(game_data.players.len());
    let mut lifecycle = default_player_lifecycle_states(game_data.conquest.player_count);
    lifecycle[0].terminal_outcome = TerminalOutcome::Defeated;
    lifecycle[1].terminal_outcome = TerminalOutcome::LostGame;

    assert_eq!(
        player_public_status(&game_data, 1, &activity, &lifecycle),
        PublicEmpireStatus::Defeated
    );
    assert_eq!(
        player_public_status(&game_data, 2, &activity, &lifecycle),
        PublicEmpireStatus::Defeated
    );
}

#[test]
fn recovery_path_requires_loaded_transports_or_etac_with_unowned_world() {
    let mut game_data = baseline_game();
    for planet in &mut game_data.planets {
        if planet.owner.0 == 1 {
            planet.owner.0 = 0;
            planet.years_under_owner = 0;
        }
    }
    for fleet in &mut game_data.fleets {
        if fleet.owner.0 == 1 {
            set_troop_transports(fleet, 0);
            set_armies_aboard(fleet, 0);
            fleet.ships.etacs = 0;
        }
    }

    assert!(!empire_has_recovery_path(&game_data, 1));

    let fleet_idx = game_data
        .fleets
        .iter()
        .position(|fleet| fleet.owner.0 == 1)
        .expect("player 1 fleet");
    game_data.fleets[fleet_idx].ships.etacs = 1;
    assert!(empire_has_recovery_path(&game_data, 1));

    for planet in &mut game_data.planets {
        if planet.owner.0 == 0 {
            planet.owner.0 = 2;
            planet.years_under_owner = 2;
        }
    }
    assert!(!empire_has_recovery_path(&game_data, 1));

    let fleet = game_data.fleets.get_mut(fleet_idx).expect("player 1 fleet");
    fleet.ships.etacs = 0;
    set_troop_transports(fleet, 1);
    set_armies_aboard(fleet, 1);
    assert!(empire_has_recovery_path(&game_data, 1));
}

#[test]
fn access_mode_distinguishes_review_survey_and_lockout() {
    let player_count = 4;
    let mut lifecycle = default_player_lifecycle_states(player_count);

    assert_eq!(
        player_access_mode(1, &lifecycle, WinnerState::default()),
        PlayerAccessMode::Normal
    );

    lifecycle[0].terminal_outcome = TerminalOutcome::Defeated;
    assert_eq!(
        player_access_mode(1, &lifecycle, WinnerState::default()),
        PlayerAccessMode::ReviewOnly
    );

    lifecycle[0].terminal_review_consumed = true;
    assert_eq!(
        player_access_mode(1, &lifecycle, WinnerState::default()),
        PlayerAccessMode::LockedOut
    );

    lifecycle[0] = PlayerLifecycleState {
        player_record_index_1_based: 1,
        recovery_window_turns_remaining: 0,
        terminal_outcome: TerminalOutcome::Winner,
        terminal_review_consumed: false,
    };
    assert_eq!(
        player_access_mode(1, &lifecycle, WinnerState::default()),
        PlayerAccessMode::SurveyOnly
    );

    assert_eq!(
        player_access_mode(
            2,
            &lifecycle,
            WinnerState {
                winner_empire_raw: Some(1),
                winner_declared_year: Some(3005),
            }
        ),
        PlayerAccessMode::LockedOut
    );
}

#[test]
fn losing_last_recovery_force_inside_window_is_not_reported_as_expiry() {
    let mut game_data = baseline_game();
    let mut lifecycle = default_player_lifecycle_states(game_data.conquest.player_count);
    let mut winner = WinnerState::default();
    strip_empire_planets(&mut game_data, 2);
    strip_empire_ships(&mut game_data, 2);
    let fleet_idx = game_data
        .fleets
        .iter()
        .position(|fleet| fleet.owner.0 == 2)
        .expect("player 2 fleet");
    set_troop_transports(&mut game_data.fleets[fleet_idx], 1);
    set_armies_aboard(&mut game_data.fleets[fleet_idx], 1);

    let events = run_turn(&mut game_data, &mut lifecycle, &mut winner);
    assert!(events.empire_elimination_events.is_empty());
    assert_eq!(lifecycle[1].recovery_window_turns_remaining, 3);

    let fleet_idx = game_data
        .fleets
        .iter()
        .position(|fleet| fleet.owner.0 == 2)
        .expect("player 2 fleet");
    set_armies_aboard(&mut game_data.fleets[fleet_idx], 0);

    let events = run_turn(&mut game_data, &mut lifecycle, &mut winner);
    let defeat = events
        .empire_elimination_events
        .iter()
        .find(|event| event.defeated_empire_raw == 2)
        .expect("player 2 should be defeated");
    assert_eq!(
        defeat.cause,
        EmpireEliminationCause::LastRecoveryForceDestroyed
    );
}

#[test]
fn planetless_rogue_without_recovery_forces_does_not_block_victory() {
    let mut game_data = baseline_game();
    let mut lifecycle = default_player_lifecycle_states(game_data.conquest.player_count);
    let mut winner = WinnerState::default();
    for empire_raw in 2..=4u8 {
        strip_empire_planets(&mut game_data, empire_raw);
        strip_empire_ships(&mut game_data, empire_raw);
    }
    game_data.players[3].mode = PlayerMode::Rogue;
    // Empire 2 keeps the campaign contested until its recovery window closes.
    lifecycle[1].recovery_window_turns_remaining = 1;

    let events = run_turn(&mut game_data, &mut lifecycle, &mut winner);

    assert_eq!(winner.winner_empire_raw, Some(1));
    assert!(
        events
            .empire_elimination_events
            .iter()
            .any(|event| event.defeated_empire_raw == 4)
    );
}
