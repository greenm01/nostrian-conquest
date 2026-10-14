//! Seeded matches between seat policies, scored for tuning and evaluation.

use nc_data::{CoreGameData, present_production_points};
use nc_fuzz::{CaseSetup, ChoiceSource, Policy};
use nc_harness::{ScenarioPlay, build_scenario};

use crate::action::{commission_submission, decode_action, seat_masks};
use crate::executor::{Weights, plan_turn};
use crate::game::SeatedGame;

/// Who plays a seat.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SeatPolicy {
    /// The scripted executor, seeing only its own seat.
    Executor(Weights),
    /// The fuzzer's aggressor policy, which sees the whole map.
    Aggressor,
    /// The engine's autopilot.
    Autopilot,
}

/// How a match ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchResult {
    pub years: u16,
    pub winner: Option<u8>,
    /// Present production of every world each empire owns at the end, by
    /// empire slot.
    pub production: Vec<u32>,
    pub worlds: Vec<u32>,
}

impl MatchResult {
    /// 1 for the declared winner, or, with none, shared equally by the
    /// empires with the most production; otherwise 0.
    pub fn win_share(&self, empire: u8) -> f32 {
        if let Some(winner) = self.winner {
            return if winner == empire { 1.0 } else { 0.0 };
        }
        let best = self.production.iter().copied().max().unwrap_or(0);
        let leaders = self.production.iter().filter(|&&p| p == best).count();
        if self.production[usize::from(empire) - 1] == best {
            1.0 / leaders as f32
        } else {
            0.0
        }
    }

    /// The empire's share of all production at the end.
    pub fn production_share(&self, empire: u8) -> f32 {
        let total: u32 = self.production.iter().sum();
        if total == 0 {
            return 0.0;
        }
        self.production[usize::from(empire) - 1] as f32 / total as f32
    }
}

/// Seeded splitmix64 choices for the aggressor policy.
struct SplitMix(u64);

impl ChoiceSource for SplitMix {
    fn pick(&mut self, lo: u32, hi: u32) -> u32 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        lo + ((z ^ (z >> 31)) % (u64::from(hi - lo) + 1)) as u32
    }

    fn flip(&mut self) -> bool {
        self.pick(0, 1) == 1
    }
}

/// Play `policies` (one per seat) on map `map_seed`, with the same campaign
/// seed, for up to `years` years.
pub fn play_match(map_seed: u64, policies: &[SeatPolicy], years: u16) -> MatchResult {
    let mut game = seated_game(
        map_seed,
        policies
            .iter()
            .map(|policy| match policy {
                SeatPolicy::Autopilot => Policy::Autopilot,
                _ => Policy::Aggressor,
            })
            .collect(),
        map_seed,
        years,
    );
    let start_year = game.play.built.game_data.conquest.game_year;
    let mut choices = SplitMix(map_seed ^ 0xA66E_5503);
    let mut played = 0;
    for _ in 0..years {
        for (idx, policy) in policies.iter().enumerate() {
            let empire = idx as u8 + 1;
            match policy {
                SeatPolicy::Executor(weights) => {
                    executor_turn(&mut game, empire, weights, start_year)
                }
                SeatPolicy::Aggressor => {
                    let planned = nc_fuzz::plan_turn(
                        Policy::Aggressor,
                        &game.play.built.game_data,
                        empire,
                        &mut choices,
                    );
                    game.submit_turn(&planned.submission)
                        .expect("aggressor turn should submit");
                }
                SeatPolicy::Autopilot => {}
            }
        }
        game.run_maintenance().expect("maintenance should run");
        played += 1;
        if game.play.winner_state.winner_empire.is_some() {
            break;
        }
    }
    match_result(
        &game.play.built.game_data,
        game.play.winner_state.winner_empire,
        played,
    )
}

/// One executor turn: commission, choose within the masks, submit.
pub fn executor_turn(game: &mut SeatedGame, empire: u8, weights: &Weights, start_year: u16) {
    if let Some(commission) = commission_submission(&game.play.built.game_data, empire) {
        game.submit_turn(&commission)
            .expect("commissioning should submit");
    }
    let game_data = &game.play.built.game_data;
    let memory = &game.seats[usize::from(empire) - 1];
    let masks = seat_masks(game_data, empire);
    let action = plan_turn(game_data, memory, &masks, weights, start_year);
    let decoded = decode_action(game_data, empire, &action);
    game.submit_turn(&decoded.submission)
        .expect("executor turn should submit");
}

/// A fresh game on map `map_seed`, one seat per policy (only Autopilot
/// matters to setup), run with `campaign_seed`; setup records `turns`.
pub(crate) fn seated_game(
    map_seed: u64,
    policies: Vec<Policy>,
    campaign_seed: u64,
    turns: u16,
) -> SeatedGame {
    let setup = CaseSetup {
        player_count: policies.len() as u8,
        map_seed,
        armadas: vec![None; policies.len()],
        policies,
        collapsed_empire: None,
        rally: false,
        war: false,
        turns,
    };
    let built = build_scenario(&setup.scenario_spec()).expect("setup should build");
    SeatedGame::new(ScenarioPlay::new(built).with_campaign_seed(campaign_seed))
}

/// The standing in `game_data` after `years` years.
pub(crate) fn match_result(
    game_data: &CoreGameData,
    winner: Option<u8>,
    years: u16,
) -> MatchResult {
    let player_count = usize::from(game_data.conquest.player_count);
    let mut production = vec![0; player_count];
    let mut worlds = vec![0; player_count];
    for planet in &game_data.planets {
        let owner = usize::from(planet.owner.0);
        if (1..=player_count).contains(&owner) {
            production[owner - 1] += u32::from(present_production_points(planet));
            worlds[owner - 1] += 1;
        }
    }
    MatchResult {
        years,
        winner,
        production,
        worlds,
    }
}
