//! Batch environment (t153, t155): padded arrays decode to the same turns a
//! seat plays by hand, scripted seats play the games `play_match` plays,
//! strategist seats given the executor's own goals play as the executor,
//! random play within the masks never fails, and choices outside the masks
//! are refused without changing any game.

#[allow(dead_code)]
mod common;

use common::arrays::{Arrays, Rng, random_arrays};
use common::seated_game;
use nc_ai::{
    BatchEnv, MISSION_CHOICES, Seat, SeatPolicy, Weights, commission_submission, executor_turn,
    plan_turn, play_match, seat_masks,
};

const PLAYERS: u8 = 4;

/// Every seat learned, in `games` games.
fn learned(games: usize) -> Vec<Vec<Seat>> {
    vec![vec![Seat::Learned; usize::from(PLAYERS)]; games]
}

#[test]
fn batch_arrays_play_the_same_game_as_seats_played_by_hand() {
    let maps = [11, 12];
    let years = 25;
    let mut env = BatchEnv::new(PLAYERS, &maps, &maps, &learned(maps.len()), years, 0);
    let mut by_hand: Vec<_> = maps.iter().map(|&map| seated_game(PLAYERS, map)).collect();
    let start_year = by_hand[0].play.built.game_data.conquest.game_year;
    let weights = Weights::TUNED;
    for _ in 0..years {
        let mut arrays = Arrays::zeros(&env);
        for (game, slot) in by_hand.iter_mut().enumerate() {
            for empire in 1..=PLAYERS {
                // Plan from the batch's own state, then play the same seat
                // by hand: commissioning and the plan must agree.
                let batched = env.game(game);
                let game_data = &batched.play.built.game_data;
                let masks = seat_masks(game_data, empire);
                let memory = &batched.seats[usize::from(empire) - 1];
                let action = plan_turn(game_data, memory, &masks, &weights, start_year);
                let at = game * usize::from(PLAYERS) + usize::from(empire) - 1;
                arrays.set(&env, at, &action);
                executor_turn(slot, empire, &weights, start_year);
            }
        }
        let step = env
            .step(Some(&arrays.view()), None)
            .expect("executor choices are allowed");
        for (game, slot) in by_hand.iter_mut().enumerate() {
            if step.done.data[game] {
                continue;
            }
            slot.run_maintenance().expect("maintenance should run");
            // The batch commissions finished ships before seats observe.
            for empire in 1..=PLAYERS {
                if let Some(commission) = commission_submission(&slot.play.built.game_data, empire)
                {
                    slot.submit_turn(&commission)
                        .expect("commissioning should submit");
                }
            }
            for empire in 1..=PLAYERS {
                assert_eq!(
                    env.game(game).observe(empire),
                    slot.observe(empire),
                    "game {game} empire {empire} diverged"
                );
            }
        }
        if step.done.data.iter().all(|&done| done) {
            break;
        }
    }
}

#[test]
fn random_play_within_the_masks_ends_every_game_with_shared_rewards() {
    let maps = [21, 22, 23];
    let years = 30;
    let mut env = BatchEnv::new(PLAYERS, &maps, &[1, 2, 3], &learned(maps.len()), years, 0);
    let mut rng = Rng(7);
    let mut total = vec![0.0f32; maps.len()];
    for year in 1..=years {
        let obs = env.observe();
        let masks = env.masks();
        let seats = maps.len() * usize::from(PLAYERS);
        assert_eq!(
            obs.planets.rows.shape,
            vec![maps.len(), 4, env.worlds(), 31]
        );
        assert_eq!(obs.fleets.valid.shape, vec![maps.len(), 4, env.fleets()]);
        assert_eq!(
            masks.target.shape[2..],
            [env.fleets(), MISSION_CHOICES, env.worlds()]
        );
        // Fleet rows of the observation and the masks are the same fleets.
        let real_fleets = obs.fleets.valid.data.iter().filter(|&&v| v).count();
        assert!(seats > 0 && real_fleets > 0);
        let arrays = random_arrays(&env, &masks, &mut rng);
        let step = env
            .step(Some(&arrays.view()), None)
            .expect("masked choices are allowed");
        for (game, sum) in total.iter_mut().enumerate() {
            *sum += step.reward.data[game * 4..game * 4 + 4].iter().sum::<f32>();
        }
        assert_eq!(step.done.data.iter().all(|&d| d), year == years);
    }
    for sum in total {
        assert!(
            (sum - 1.0).abs() < 1e-5,
            "each game hands out one win: {sum}"
        );
    }
    let (time, count) = env.mask_cost();
    assert_eq!(count, maps.len() as u64 * 4 * u64::from(years));
    assert!(time.as_nanos() > 0);
}

#[test]
fn a_choice_outside_the_masks_is_refused_and_changes_nothing() {
    let mut env = BatchEnv::new(PLAYERS, &[31, 32], &[31, 32], &learned(2), 10, 0);
    let before = env.observe();
    let masks = env.masks();
    let mut arrays = Arrays::zeros(&env);

    // Diplomacy toward oneself only allows "keep": game 0, seat 1 toward 2.
    let seat = 1;
    arrays.diplomacy[seat * usize::from(PLAYERS) + seat] = 2;
    let err = env.step(Some(&arrays.view()), None).unwrap_err();
    assert!(err.contains("game 0 seat 1"), "{err}");
    assert_eq!(env.observe(), before);

    let mut arrays = Arrays::zeros(&env);
    let disallowed = masks
        .mission
        .data
        .iter()
        .position(|&allowed| !allowed)
        .expect("some mission is not allowed");
    arrays.mission[disallowed / MISSION_CHOICES] = (disallowed % MISSION_CHOICES) as i64;
    assert!(env.step(Some(&arrays.view()), None).is_err());

    let mut arrays = Arrays::zeros(&env);
    arrays.tax.pop();
    assert!(
        env.step(Some(&arrays.view()), None)
            .unwrap_err()
            .contains("tax has")
    );
    assert_eq!(env.observe(), before);
}

#[test]
fn scripted_seats_play_the_same_games_as_play_match() {
    let years = 30;
    let lineups = [
        [
            SeatPolicy::Executor(Weights::TUNED),
            SeatPolicy::Aggressor,
            SeatPolicy::Executor(Weights::BASELINE),
            SeatPolicy::Autopilot,
        ],
        [
            SeatPolicy::Aggressor,
            SeatPolicy::Executor(Weights::TUNED),
            SeatPolicy::Aggressor,
            SeatPolicy::Executor(Weights::TUNED),
        ],
    ];
    let maps = [51, 52];
    let mut env = BatchEnv::new(
        PLAYERS,
        &maps,
        &maps,
        &lineups.map(|lineup| lineup.map(Seat::Scripted).to_vec()),
        years,
        0,
    );
    assert!(env.learned().data.iter().all(|&learned| !learned));
    let mut played = [0u16; 2];
    let mut reward = vec![0.0f32; maps.len() * 4];
    let mut done = vec![false; maps.len()];
    let mut last = None;
    while !done.iter().all(|&done| done) {
        let step = env
            .step(Some(&Arrays::zeros(&env).view()), None)
            .expect("scripted seats need no choices");
        for (years, done) in played.iter_mut().zip(&done) {
            *years += u16::from(!done);
        }
        for (sum, r) in reward.iter_mut().zip(&step.reward.data) {
            *sum += r;
        }
        done = step.done.data.clone();
        last = Some(step);
    }
    let step = last.expect("a game was played");
    for (game, (&map, lineup)) in maps.iter().zip(&lineups).enumerate() {
        let result = play_match(map, lineup, years);
        assert_eq!(played[game], result.years, "game {game} length");
        assert_eq!(
            step.production.data[game * 4..game * 4 + 4],
            result.production[..]
        );
        assert_eq!(step.worlds.data[game * 4..game * 4 + 4], result.worlds[..]);
        for empire in 1..=PLAYERS {
            let seat = game * 4 + usize::from(empire) - 1;
            assert_eq!(
                reward[seat],
                result.win_share(empire),
                "game {game} empire {empire}"
            );
        }
    }
}

#[test]
fn learned_seats_play_beside_scripted_ones() {
    let lineup = vec![
        Seat::Learned,
        Seat::Scripted(SeatPolicy::Executor(Weights::TUNED)),
        Seat::Scripted(SeatPolicy::Aggressor),
        Seat::Learned,
    ];
    let mut env = BatchEnv::new(
        PLAYERS,
        &[61, 62],
        &[61, 62],
        &[lineup.clone(), lineup],
        20,
        0,
    );
    assert_eq!(env.learned().data[..4], [true, false, false, true]);
    let mut rng = Rng(9);
    for _ in 0..20 {
        let obs = env.observe();
        let masks = env.masks();
        let fleets = env.fleets();
        for game in 0..2 {
            for seat in [1, 2] {
                // Scripted seats' masks are all padding.
                let at = (game * 4 + seat) * fleets;
                assert!(masks.load.data[at..at + fleets].iter().all(|&load| !load));
                let mission =
                    &masks.mission.data[at * MISSION_CHOICES..(at + fleets) * MISSION_CHOICES];
                assert!(
                    mission
                        .chunks(MISSION_CHOICES)
                        .all(|row| row[0] && !row[1..].contains(&true))
                );
            }
        }
        assert!(obs.fleets.valid.data.iter().any(|&valid| valid));
        let arrays = random_arrays(&env, &masks, &mut rng);
        env.step(Some(&arrays.view()), None)
            .expect("learned choices are allowed");
    }
    let (_, count) = env.mask_cost();
    assert_eq!(count, 2 * 2 * 20, "masks for learned seats only");
}

/// Present production of every world each empire owns, by empire slot.
fn production(game_data: &nc_data::CoreGameData) -> Vec<u32> {
    let mut out = vec![0; usize::from(PLAYERS)];
    for planet in &game_data.planets {
        if let Some(sum) = out.get_mut(usize::from(planet.owner.0).wrapping_sub(1)) {
            *sum += u32::from(nc_data::present_production_points(planet));
        }
    }
    out
}

/// Strategist seats commission at the start of the year, before any seat
/// moves, and new fleets take record numbers in that order. The reference
/// therefore commissions every seat first, then plays each with the
/// executor. The goal labels set the allocation by the executor's rule
/// (t175), so the reference plays with that rule on.
#[test]
fn strategists_given_the_executors_goals_play_as_the_executor() {
    let weights = [
        Weights::TUNED,
        Weights::BASELINE,
        Weights::TUNED,
        Weights::BASELINE,
    ];
    let lineup: Vec<Seat> = weights.iter().map(|&w| Seat::Strategist(w)).collect();
    let maps = [71, 72];
    let years = 40;
    let mut env = BatchEnv::new(PLAYERS, &maps, &maps, &[lineup.clone(), lineup], years, 0);
    assert!(env.strategist().data.iter().all(|&s| s));
    let mut by_hand: Vec<_> = maps.iter().map(|&map| seated_game(PLAYERS, map)).collect();
    let start_year = by_hand[0].play.built.game_data.conquest.game_year;
    let mut done = vec![false; maps.len()];
    while !done.iter().all(|&done| done) {
        let labels = env.goal_labels();
        let masks = env.goal_masks();
        let width = masks.strike.shape[2];
        for (at, (&strike, &colonize)) in labels
            .strike
            .data
            .iter()
            .zip(&labels.colonize.data)
            .enumerate()
        {
            assert!(masks.strike.data[at * width + strike as usize]);
            assert!(masks.colonize.data[at * width + colonize as usize]);
        }
        let step = env
            .step(None, Some(&labels.view()))
            .expect("labels are allowed goals");
        for (game, slot) in by_hand.iter_mut().enumerate() {
            if done[game] {
                continue;
            }
            for empire in 1..=PLAYERS {
                if let Some(commission) = commission_submission(&slot.play.built.game_data, empire)
                {
                    slot.submit_turn(&commission)
                        .expect("commissioning should submit");
                }
            }
            for (seat, weights) in weights.iter().enumerate() {
                let empire = seat as u8 + 1;
                let game_data = &slot.play.built.game_data;
                let masks = seat_masks(game_data, empire);
                let mut weights = *weights;
                weights.doctrine.allocate = true;
                let action = plan_turn(game_data, &slot.seats[seat], &masks, &weights, start_year);
                let decoded = nc_ai::decode_action(game_data, empire, &action);
                slot.submit_turn(&decoded.submission)
                    .expect("executor turn should submit");
            }
            slot.run_maintenance().expect("maintenance should run");
            assert_eq!(
                step.production.data[game * 4..game * 4 + 4],
                production(&slot.play.built.game_data)[..],
                "game {game} diverged"
            );
        }
        done = step.done.data.clone();
    }
}

#[test]
fn strategist_goals_outside_the_masks_or_missing_are_refused() {
    let lineup = vec![Seat::Strategist(Weights::TUNED); usize::from(PLAYERS)];
    let mut env = BatchEnv::new(PLAYERS, &[81], &[81], &[lineup], 10, 0);
    let before = env.observe();
    assert!(env.step(None, None).unwrap_err().contains("need goals"));
    let mut labels = env.goal_labels();
    labels.posture.data[2] = nc_ai::POSTURES as i64;
    assert!(
        env.step(None, Some(&labels.view()))
            .unwrap_err()
            .contains("posture")
    );
    let mut labels = env.goal_labels();
    let width = env.goal_masks().strike.shape[2];
    let masks = env.goal_masks();
    let closed = (0..width)
        .find(|&world| !masks.strike.data[world])
        .expect("some world cannot be struck at the start");
    labels.strike.data[0] = closed as i64;
    assert!(
        env.step(None, Some(&labels.view()))
            .unwrap_err()
            .contains("strike")
    );
    assert_eq!(env.observe(), before);
}

/// A game that reaches its year limit without a declared winner pays its
/// production leaders only the limit credit (t180).
#[test]
fn a_production_lead_at_the_year_limit_pays_the_limit_credit() {
    let lineup = vec![Seat::Scripted(SeatPolicy::Executor(Weights::TUNED)); usize::from(PLAYERS)];
    let total = |credit: f32| {
        let mut env = BatchEnv::new(PLAYERS, &[5], &[5], &[lineup.clone()], 3, 0);
        env.set_limit_credit(credit);
        loop {
            let step = env
                .step(None, None)
                .expect("scripted seats need no choices");
            if step.done.data[0] {
                return step.reward.data.iter().sum::<f32>();
            }
        }
    };
    assert_eq!(total(1.0), 1.0);
    assert_eq!(total(0.25), 0.25);
}
