//! Batch environment (t153): padded arrays decode to the same turns a seat
//! plays by hand, random play within the masks never fails, and choices
//! outside the masks are refused without changing any game.

#[allow(dead_code)]
mod common;

use common::seated_game;
use nc_ai::{
    BUILD_AMOUNT_CHOICES, BUILD_CHOICES, BatchActions, BatchEnv, BatchMasks, MISSION_CHOICES,
    ROE_CHOICES, SPEED_CHOICES, SeatAction, TAX_CHOICES, Weights, commission_submission,
    executor_turn, plan_turn, seat_masks,
};

const PLAYERS: u8 = 4;

/// Owned action arrays, borrowed as [`BatchActions`].
#[derive(Clone)]
struct Arrays {
    tax: Vec<i64>,
    diplomacy: Vec<i64>,
    build: Vec<i64>,
    amount: Vec<i64>,
    mission: Vec<i64>,
    target: Vec<i64>,
    speed: Vec<i64>,
    roe: Vec<i64>,
    load: Vec<i64>,
}

impl Arrays {
    fn zeros(env: &BatchEnv) -> Self {
        let seats = env.games() * usize::from(env.players());
        let fleets = vec![0; seats * env.fleets()];
        Arrays {
            tax: vec![0; seats],
            diplomacy: vec![0; seats * usize::from(env.players())],
            build: vec![0; seats * env.worlds()],
            amount: vec![0; seats * env.worlds()],
            mission: fleets.clone(),
            target: fleets.clone(),
            speed: fleets.clone(),
            roe: fleets.clone(),
            load: fleets,
        }
    }

    fn view(&self) -> BatchActions<'_> {
        BatchActions {
            tax: &self.tax,
            diplomacy: &self.diplomacy,
            build: &self.build,
            amount: &self.amount,
            mission: &self.mission,
            target: &self.target,
            speed: &self.speed,
            roe: &self.roe,
            load: &self.load,
        }
    }

    /// Write `action` for flat seat index `at`.
    fn set(&mut self, env: &BatchEnv, at: usize, action: &SeatAction) {
        let (seats, worlds, fleets) = (usize::from(env.players()), env.worlds(), env.fleets());
        self.tax[at] = action.tax as i64;
        for (other, &choice) in action.diplomacy.iter().enumerate() {
            self.diplomacy[at * seats + other] = choice as i64;
        }
        for planet in &action.planets {
            let world = at * worlds + planet.record - 1;
            self.build[world] = planet.build as i64;
            self.amount[world] = planet.amount as i64;
        }
        for (row, fleet) in action.fleets.iter().enumerate() {
            let idx = at * fleets + row;
            self.mission[idx] = fleet.mission as i64;
            self.target[idx] = fleet.target as i64;
            self.speed[idx] = fleet.speed as i64;
            self.roe[idx] = fleet.roe as i64;
            self.load[idx] = i64::from(fleet.load);
        }
    }
}

/// Seeded splitmix64.
struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) % n as u64) as usize
    }

    fn allowed(&mut self, mask: &[bool]) -> i64 {
        let allowed: Vec<usize> = (0..mask.len()).filter(|&idx| mask[idx]).collect();
        assert!(!allowed.is_empty(), "every mask row allows something");
        allowed[self.below(allowed.len())] as i64
    }
}

/// Uniform random choices within `masks`, padding rows included.
fn random_arrays(env: &BatchEnv, masks: &BatchMasks, rng: &mut Rng) -> Arrays {
    let mut arrays = Arrays::zeros(env);
    let rows = |mask: &[bool], width: usize| mask.chunks(width).map(<[bool]>::to_vec).collect();
    let diplomacy: Vec<Vec<bool>> = rows(&masks.diplomacy.data, 3);
    let build: Vec<Vec<bool>> = rows(&masks.build.data, BUILD_CHOICES);
    let mission: Vec<Vec<bool>> = rows(&masks.mission.data, MISSION_CHOICES);
    let target: Vec<Vec<bool>> = rows(&masks.target.data, env.worlds());
    let roe: Vec<Vec<bool>> = rows(&masks.roe.data, ROE_CHOICES);
    for at in 0..arrays.tax.len() {
        arrays.tax[at] = rng.below(TAX_CHOICES) as i64;
    }
    for (idx, row) in diplomacy.iter().enumerate() {
        // Mostly keep relations, so games are not all-out war.
        arrays.diplomacy[idx] = if rng.below(8) == 0 {
            rng.allowed(row)
        } else {
            0
        };
    }
    for (idx, row) in build.iter().enumerate() {
        arrays.build[idx] = rng.allowed(row);
        arrays.amount[idx] = rng.below(BUILD_AMOUNT_CHOICES) as i64;
    }
    for (idx, row) in mission.iter().enumerate() {
        let chosen = rng.allowed(row);
        arrays.mission[idx] = chosen;
        arrays.target[idx] = rng.allowed(&target[idx * MISSION_CHOICES + chosen as usize]);
        arrays.speed[idx] = rng.below(SPEED_CHOICES) as i64;
        arrays.roe[idx] = rng.allowed(&roe[idx]);
        arrays.load[idx] = i64::from(masks.load.data[idx] && rng.below(2) == 0);
    }
    arrays
}

#[test]
fn batch_arrays_play_the_same_game_as_seats_played_by_hand() {
    let maps = [11, 12];
    let years = 25;
    let mut env = BatchEnv::new(PLAYERS, &maps, &maps, years, 0);
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
            .step(&arrays.view())
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
    let mut env = BatchEnv::new(PLAYERS, &maps, &[1, 2, 3], years, 0);
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
            .step(&arrays.view())
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
    let mut env = BatchEnv::new(PLAYERS, &[31, 32], &[31, 32], 10, 0);
    let before = env.observe();
    let masks = env.masks();
    let mut arrays = Arrays::zeros(&env);

    // Diplomacy toward oneself only allows "keep": game 0, seat 1 toward 2.
    let seat = 1;
    arrays.diplomacy[seat * usize::from(PLAYERS) + seat] = 2;
    let err = env.step(&arrays.view()).unwrap_err();
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
    assert!(env.step(&arrays.view()).is_err());

    let mut arrays = Arrays::zeros(&env);
    arrays.tax.pop();
    assert!(env.step(&arrays.view()).unwrap_err().contains("tax has"));
    assert_eq!(env.observe(), before);
}
