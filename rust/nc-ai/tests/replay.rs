//! Replays (t156): a game rebuilt from its seeds and recorded turns, kept
//! as turn KDL, ends exactly as it was played.

#[allow(dead_code)]
mod common;

use common::arrays::{Rng, random_arrays};
use nc_ai::{BatchEnv, Seat, SeatPolicy, Weights, play_match, replay};
use nc_data::{FleetTurnAction, TurnSubmission};

const PLAYERS: u8 = 4;

#[test]
fn a_batch_game_replays_from_its_seeds_and_turn_kdl() {
    let lineup = vec![
        Seat::Strategist(Weights::TUNED),
        Seat::Scripted(SeatPolicy::Executor(Weights::TUNED)),
        Seat::Scripted(SeatPolicy::Aggressor),
        Seat::Scripted(SeatPolicy::Autopilot),
    ];
    let autopilot = [false, false, false, true];
    let maps = [91, 92, 93];
    let campaigns = [7, 8, 9];
    let years = 40;
    let mut env = BatchEnv::new(
        PLAYERS,
        &maps,
        &campaigns,
        &vec![lineup; maps.len()],
        years,
        0,
    );
    loop {
        let labels = env.goal_labels();
        let step = env
            .step(None, Some(&labels.view()))
            .expect("labels are allowed goals");
        if step.done.data.iter().all(|&done| done) {
            break;
        }
    }
    for game in 0..maps.len() {
        let played = env.result(game).expect("every game ended").clone();
        let turns = &env.game(game).turns;
        assert!(!turns.is_empty());
        assert_eq!(
            replay(maps[game], campaigns[game], &autopilot, years, turns),
            Ok(played.clone()),
            "game {game} replays from its turns"
        );
        let stored: Vec<TurnSubmission> = turns
            .iter()
            .map(|turn| {
                TurnSubmission::parse_kdl_str(&turn.to_kdl_string()).expect("turn KDL parses")
            })
            .collect();
        assert_eq!(
            replay(maps[game], campaigns[game], &autopilot, years, &stored),
            Ok(played),
            "game {game} replays from turn KDL"
        );
    }
}

/// Random masked play that detaches fleets (t172) replays from its turn
/// KDL.
#[test]
fn random_play_with_detached_fleets_replays_from_turn_kdl() {
    let maps = [61, 62];
    let years = 30;
    let lineup = vec![Seat::Learned; usize::from(PLAYERS)];
    let mut env = BatchEnv::new(PLAYERS, &maps, &maps, &vec![lineup; maps.len()], years, 0);
    let mut rng = Rng(5);
    loop {
        let masks = env.masks();
        let arrays = random_arrays(&env, &masks, &mut rng);
        let step = env
            .step(Some(&arrays.view()), None)
            .expect("random choices within the masks are accepted");
        if step.done.data.iter().all(|&done| done) {
            break;
        }
    }
    for (game, &map) in maps.iter().enumerate() {
        let played = env.result(game).expect("every game ended").clone();
        let turns = &env.game(game).turns;
        let detaches = turns
            .iter()
            .flat_map(|turn| &turn.fleets)
            .flat_map(|fleet| &fleet.actions)
            .filter(|action| matches!(action, FleetTurnAction::Detach { .. }))
            .count();
        assert!(detaches > 0, "game {game} should detach");
        let stored: Vec<TurnSubmission> = turns
            .iter()
            .map(|turn| {
                TurnSubmission::parse_kdl_str(&turn.to_kdl_string()).expect("turn KDL parses")
            })
            .collect();
        assert_eq!(
            replay(map, map, &[false; 4], years, &stored),
            Ok(played),
            "game {game} replays from turn KDL"
        );
    }
}

#[test]
fn a_replay_with_a_turn_missing_differs_or_fails() {
    let policies = [SeatPolicy::Executor(Weights::TUNED); 4];
    let mut env = BatchEnv::new(
        PLAYERS,
        &[95],
        &[95],
        &[policies.map(Seat::Scripted).to_vec()],
        30,
        0,
    );
    while !env
        .step(None, None)
        .expect("scripted seats need no choices")
        .done
        .data[0]
    {}
    let played = env.result(0).expect("the game ended").clone();
    assert_eq!(played, play_match(95, &policies, 30));
    let turns = &env.game(0).turns;
    let mut short = turns.clone();
    short.remove(turns.len() / 2);
    assert_ne!(replay(95, 95, &[false; 4], 30, &short), Ok(played));
}
