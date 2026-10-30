//! A batch of seeded games stepped together, with every seat's observation,
//! masks and actions as padded arrays for a learner.
//!
//! Arrays are row-major and indexed `[game, seat, ...]`. World arrays have
//! one row per world in planet-record order, padded to the largest map in
//! the batch; fleet, contact and starbase arrays are padded to the most rows
//! any seat has. Each padded table has a `valid` flag per row.
//!
//! Each seat is one of three kinds:
//! - learned: the caller chooses its whole turn ([`BatchActions`]);
//! - strategist: the caller chooses its [`Goals`] ([`BatchGoals`]) and the
//!   executor carries them out; [`BatchEnv::goal_labels`] gives the goals
//!   the executor would set itself;
//! - scripted: the executor, Aggressor or Autopilot, played as
//!   [`crate::play_match`] plays them, so a batch of scripted seats plays the
//!   same games `play_match` does.
//!
//! Between steps every live game waits for its learned and strategist seats,
//! with their finished ships already commissioned, so the observation and
//! masks show the fleets the seats will order. Scripted seats commission at
//! their own turn, as `play_match` has them do, and get masks with every row
//! padding. Padding rows allow only "no build", "keep orders"
//! and ROE 0, so sampling within the masks never meets an empty row.

mod arrays;
mod choices;

use std::thread;
use std::time::{Duration, Instant};

pub use arrays::{Table, Tensor};
use arrays::{falses, pad_table, par_for_each_mut, put};
use choices::{fleet_choices, planet_choices, seat_action, seat_goals};

use nc_fuzz::Policy;

use crate::action::{
    BUILD_CHOICES, BUILD_SLOTS, DETACH_CHOICES, DIPLOMACY_CHOICES, FleetChoice, MISSION_CHOICES,
    PlanetChoice, ROE_CHOICES, SeatAction, SeatMasks, commission_submission, decode_action,
    seat_masks,
};
use crate::arena::{
    MatchResult, SeatPolicy, SplitMix, aggressor_choices, executor_turn, match_result, seated_game,
};
use crate::executor::{
    Families, Weights, execute_goals, fleet_family, hand_to_network, network_joins, plan_goals,
};
use crate::game::SeatedGame;
use crate::stored::StoredPosition;
use crate::goals::{GoalMasks, Goals, goal_masks};
use crate::observe::{
    CONTACT_WIDTH, FLEET_WIDTH, GLOBAL_WIDTH, Observation, PLANET_WIDTH, STARBASE_WIDTH,
};

/// Every seat's observation; see [`crate::observe`] for the row layouts.
#[derive(Debug, Clone, PartialEq)]
pub struct BatchObservation {
    /// `[games, seats, GLOBAL_WIDTH]`.
    pub global: Tensor<f32>,
    pub planets: Table,
    pub fleets: Table,
    pub contacts: Table,
    pub starbases: Table,
}

/// Every seat's masks. `W` is the padded world count and `F` the padded
/// fleet count. Tax, build-amount, speed and waypoint choices are always
/// allowed.
#[derive(Debug, Clone, PartialEq)]
pub struct BatchMasks {
    /// `[games, seats, seats, DIPLOMACY_CHOICES]`.
    pub diplomacy: Tensor<bool>,
    /// `[games, seats, W, BUILD_CHOICES]`; a world the seat does not own
    /// allows only "no build".
    pub build: Tensor<bool>,
    /// `[games, seats, F, MISSION_CHOICES]`.
    pub mission: Tensor<bool>,
    /// `[games, seats, F, MISSION_CHOICES, W]`.
    pub target: Tensor<bool>,
    /// `[games, seats, F, ROE_CHOICES]`.
    pub roe: Tensor<bool>,
    /// `[games, seats, F]`.
    pub load: Tensor<bool>,
    /// `[games, seats, F, DETACH_CHOICES]`.
    pub detach: Tensor<bool>,
    /// `[games, seats, F, 1 + F]`: no join, then one flag per fleet row.
    pub join: Tensor<bool>,
    /// `[games, seats, F]`.
    pub unload: Tensor<bool>,
}

/// Every seat's choices, shaped as [`BatchMasks`] describes. Entries for
/// finished games, scripted seats, padding rows and worlds a seat does not
/// own are ignored.
#[derive(Debug, Clone, Copy)]
pub struct BatchActions<'a> {
    /// `[games, seats]`.
    pub tax: &'a [i64],
    /// `[games, seats, seats]`.
    pub diplomacy: &'a [i64],
    /// `[games, seats, W, BUILD_SLOTS]`.
    pub build: &'a [i64],
    pub amount: &'a [i64],
    /// `[games, seats, F]`.
    pub mission: &'a [i64],
    pub target: &'a [i64],
    pub waypoint: &'a [i64],
    pub speed: &'a [i64],
    pub roe: &'a [i64],
    /// Nonzero to load armies.
    pub load: &'a [i64],
    pub detach: &'a [i64],
    /// 0 for no join, or 1 plus the host's fleet row.
    pub join: &'a [i64],
    /// Nonzero to unload armies.
    pub unload: &'a [i64],
}

/// Every strategist seat's goals, as [`Goals`] fields. `[games, seats]`
/// except `diplomacy`, `[games, seats, seats]`. Entries for other seats
/// and finished games are ignored.
#[derive(Debug, Clone, Copy)]
pub struct BatchGoals<'a> {
    pub tax: &'a [i64],
    pub diplomacy: &'a [i64],
    pub posture: &'a [i64],
    pub allocation: &'a [i64],
    pub strike: &'a [i64],
    pub colonize: &'a [i64],
}

/// [`BatchGoals`] as owned arrays.
#[derive(Debug, Clone, PartialEq)]
pub struct GoalArrays {
    pub tax: Tensor<i64>,
    pub diplomacy: Tensor<i64>,
    pub posture: Tensor<i64>,
    pub allocation: Tensor<i64>,
    pub strike: Tensor<i64>,
    pub colonize: Tensor<i64>,
}

impl GoalArrays {
    pub fn view(&self) -> BatchGoals<'_> {
        BatchGoals {
            tax: &self.tax.data,
            diplomacy: &self.diplomacy.data,
            posture: &self.posture.data,
            allocation: &self.allocation.data,
            strike: &self.strike.data,
            colonize: &self.colonize.data,
        }
    }
}

/// What strategist seats may choose as goals; diplomacy is in
/// [`BatchMasks::diplomacy`], and tax and posture are always free.
#[derive(Debug, Clone, PartialEq)]
pub struct BatchGoalMasks {
    /// `[games, seats, 1 + W]`: hold, then one flag per world.
    pub strike: Tensor<bool>,
    /// `[games, seats, 1 + W]`: auto, then one flag per world.
    pub colonize: Tensor<bool>,
}

/// Who plays a seat.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Seat {
    /// The caller chooses the whole turn.
    Learned,
    /// The caller chooses goals; the executor with these weights carries
    /// them out.
    Strategist(Weights),
    /// A strategist whose network also orders the fleets of the given
    /// families (t157); the executor orders the rest and does the building.
    Hybrid(Weights, Families),
    Scripted(SeatPolicy),
}

impl Seat {
    /// The executor weights of a seat that takes goals.
    fn goal_weights(&self) -> Option<Weights> {
        match self {
            Seat::Strategist(weights) | Seat::Hybrid(weights, _) => Some(*weights),
            _ => None,
        }
    }
}

/// Fleet choices by `[games, seats, F]`, shaped as [`BatchActions`]'s fleet
/// fields, with each row's [`Family`]: 0 colonize, 1 scout, 2 strike, and
/// -1 for padding.
#[derive(Debug, Clone, PartialEq)]
pub struct FleetArrays {
    pub mission: Tensor<i64>,
    pub target: Tensor<i64>,
    pub waypoint: Tensor<i64>,
    pub speed: Tensor<i64>,
    pub roe: Tensor<i64>,
    pub load: Tensor<i64>,
    pub detach: Tensor<i64>,
    pub join: Tensor<i64>,
    pub unload: Tensor<i64>,
    pub family: Tensor<i64>,
    /// `[games, seats, W, BUILD_SLOTS]`: the executor's builds at each world.
    pub build: Tensor<i64>,
    /// `[games, seats, W, BUILD_SLOTS]`: their amounts.
    pub amount: Tensor<i64>,
}

/// What one step did.
#[derive(Debug, Clone, PartialEq)]
pub struct StepResult {
    /// `[games, seats]`: the seat's win share when its game ends this step,
    /// otherwise 0. See [`MatchResult::win_share`]. A game that reaches its
    /// year limit without a declared winner pays its production leaders
    /// only [`BatchEnv::set_limit_credit`] of their share.
    pub reward: Tensor<f32>,
    /// `[games]`: the game has ended, now or before.
    pub done: Tensor<bool>,
    /// `[games, seats]`: present production of every world the seat owns.
    pub production: Tensor<u32>,
    /// `[games, seats]`: worlds the seat owns.
    pub worlds: Tensor<u32>,
}

#[derive(Debug, Clone)]
struct Slot {
    game: SeatedGame,
    lineup: Vec<Seat>,
    start_year: u16,
    choices: SplitMix,
    years_played: u16,
    /// The seats' masks for the coming year.
    masks: Vec<SeatMasks>,
    /// Strategist seats' goal masks for the coming year.
    goal_masks: Vec<Option<GoalMasks>>,
    result: Option<MatchResult>,
    mask_time: Duration,
    mask_count: u64,
}

/// Seeded games played in step.
#[derive(Debug, Clone)]
pub struct BatchEnv {
    players: u8,
    years: u16,
    threads: usize,
    slots: Vec<Slot>,
    limit_credit: f32,
}

impl BatchEnv {
    /// Fresh games, one per map seed, each run with its campaign seed for
    /// up to `years` years. `lineups` holds one entry per seat for each game.
    /// `threads` 0 uses every core.
    pub fn new(
        players: u8,
        map_seeds: &[u64],
        campaign_seeds: &[u64],
        lineups: &[Vec<Seat>],
        years: u16,
        threads: usize,
    ) -> Self {
        let starts = vec![None; map_seeds.len()];
        Self::with_starts(players, map_seeds, campaign_seeds, lineups, &starts, years, threads)
    }

    /// [`BatchEnv::new`], with game `i` played on from `starts[i]` when it
    /// is set: a stored year of a played game, which keeps its own campaign
    /// seed and counts its years already played toward `years`.
    pub fn with_starts(
        players: u8,
        map_seeds: &[u64],
        campaign_seeds: &[u64],
        lineups: &[Vec<Seat>],
        starts: &[Option<StoredPosition>],
        years: u16,
        threads: usize,
    ) -> Self {
        assert_eq!(map_seeds.len(), starts.len(), "one start per map seed");
        assert_eq!(
            map_seeds.len(),
            campaign_seeds.len(),
            "one campaign seed per map seed"
        );
        assert_eq!(map_seeds.len(), lineups.len(), "one lineup per map seed");
        assert!(
            lineups
                .iter()
                .all(|lineup| lineup.len() == usize::from(players)),
            "one lineup entry per seat"
        );
        let threads = if threads == 0 {
            thread::available_parallelism().map_or(1, |n| n.get())
        } else {
            threads
        };
        let mut slots: Vec<Option<Slot>> = map_seeds.iter().map(|_| None).collect();
        par_for_each_mut(&mut slots, threads, |idx, slot| {
            let lineup = lineups[idx].clone();
            let policies = lineup
                .iter()
                .map(|seat| match seat {
                    Seat::Scripted(SeatPolicy::Autopilot) => Policy::Autopilot,
                    _ => Policy::Aggressor,
                })
                .collect();
            let (game, start_year) = match &starts[idx] {
                Some(start) => (start.seated_game(), start.start_year),
                None => {
                    let game = seated_game(map_seeds[idx], policies, campaign_seeds[idx], years);
                    let year = game.play.built.game_data.conquest.game_year;
                    (game, year)
                }
            };
            assert_eq!(
                game.play.built.game_data.conquest.player_count, players,
                "a stored start has the batch's players"
            );
            let years_played = game.play.built.game_data.conquest.game_year - start_year;
            let mut new = Slot {
                start_year,
                game,
                lineup,
                choices: aggressor_choices(map_seeds[idx]),
                years_played,
                masks: Vec::new(),
                goal_masks: Vec::new(),
                result: None,
                mask_time: Duration::ZERO,
                mask_count: 0,
            };
            new.prepare();
            *slot = Some(new);
        });
        Self {
            players,
            years,
            threads,
            slots: slots.into_iter().map(Option::unwrap).collect(),
            limit_credit: 1.0,
        }
    }

    /// What a production lead at the year limit is worth, against 1 for
    /// sweeping the map (a declared winner). 1, the default, pays both the
    /// same; less makes the sweep the goal.
    pub fn set_limit_credit(&mut self, credit: f32) {
        self.limit_credit = credit;
    }

    /// A new batch whose game `i` is an exact copy of this batch's game
    /// `games[i]`, mid-game state, random streams and turn log included, so
    /// copies given the same choices play out identically (t216's branch
    /// rollouts). A game may be copied more than once.
    pub fn branch(&self, games: &[usize]) -> Self {
        Self {
            players: self.players,
            years: self.years,
            threads: self.threads,
            slots: games.iter().map(|&game| self.slots[game].clone()).collect(),
            limit_credit: self.limit_credit,
        }
    }

    pub fn games(&self) -> usize {
        self.slots.len()
    }

    pub fn players(&self) -> u8 {
        self.players
    }

    /// The padded world count, `W`.
    pub fn worlds(&self) -> usize {
        self.slots
            .iter()
            .map(|slot| slot.game.play.built.game_data.planets.len())
            .max()
            .unwrap_or(0)
    }

    /// The padded fleet count, `F`: the most fleets any seat owns.
    pub fn fleets(&self) -> usize {
        self.slots
            .iter()
            .flat_map(|slot| {
                let mut owned = vec![0; slot.lineup.len()];
                for fleet in &slot.game.play.built.game_data.fleets {
                    if let Some(count) = owned.get_mut(usize::from(fleet.owner.0).wrapping_sub(1)) {
                        *count += 1;
                    }
                }
                owned
            })
            .max()
            .unwrap_or(0)
    }

    /// Total time spent in [`seat_masks`] and how many seat masks it built.
    pub fn mask_cost(&self) -> (Duration, u64) {
        self.slots.iter().fold((Duration::ZERO, 0), |sum, slot| {
            (sum.0 + slot.mask_time, sum.1 + slot.mask_count)
        })
    }

    /// `[games, seats]`: the seat is learned.
    pub fn learned(&self) -> Tensor<bool> {
        self.seats_where(|seat| *seat == Seat::Learned)
    }

    /// `[games, seats]`: the seat takes goals (a strategist or a hybrid).
    pub fn strategist(&self) -> Tensor<bool> {
        self.seats_where(|seat| seat.goal_weights().is_some())
    }

    /// `[games, seats]`: the seat is a hybrid, also taking fleet choices.
    pub fn hybrid(&self) -> Tensor<bool> {
        self.seats_where(|seat| matches!(seat, Seat::Hybrid(..)))
    }

    /// The executor's fleet choices for every goal-taking seat of every live
    /// game, if it carried out `goals` this year: imitation labels for the
    /// fleet heads (t157). Other rows are 0. Fails when a goal falls outside
    /// the goal masks.
    pub fn fleet_labels(&self, goals: &BatchGoals) -> Result<FleetArrays, String> {
        let (games, seats, fleets) = (self.games(), usize::from(self.players), self.fleets());
        let worlds = self.worlds();
        let slots = || Tensor {
            data: vec![0; games * seats * worlds * BUILD_SLOTS],
            shape: vec![games, seats, worlds, BUILD_SLOTS],
        };
        let zeros = |fill: i64| Tensor {
            data: vec![fill; games * seats * fleets],
            shape: vec![games, seats, fleets],
        };
        let mut out = FleetArrays {
            mission: zeros(0),
            target: zeros(0),
            waypoint: zeros(0),
            speed: zeros(0),
            roe: zeros(0),
            load: zeros(0),
            detach: zeros(0),
            join: zeros(0),
            unload: zeros(0),
            family: zeros(-1),
            build: slots(),
            amount: slots(),
        };
        for (game, slot) in self.slots.iter().enumerate() {
            if slot.result.is_some() {
                continue;
            }
            let game_data = &slot.game.play.built.game_data;
            for (seat, kind) in slot.lineup.iter().enumerate() {
                let Some(weights) = kind.goal_weights() else {
                    continue;
                };
                let at = game * seats + seat;
                let goal_masks = slot.goal_masks[seat]
                    .as_ref()
                    .expect("a goal-taking seat has goal masks");
                let seat_goals = seat_goals(goal_masks, goals, at, seats)
                    .map_err(|err| format!("game {game} seat {seat}: {err}"))?;
                let memory = &slot.game.seats[seat];
                let action =
                    execute_goals(game_data, memory, &slot.masks[seat], &weights, &seat_goals);
                for (row, choice) in action.fleets.iter().enumerate() {
                    let idx = at * fleets + row;
                    out.mission.data[idx] = choice.mission as i64;
                    out.target.data[idx] = choice.target as i64;
                    out.waypoint.data[idx] = choice.waypoint as i64;
                    out.speed.data[idx] = choice.speed as i64;
                    out.roe.data[idx] = choice.roe as i64;
                    out.load.data[idx] = i64::from(choice.load);
                    out.detach.data[idx] = choice.detach as i64;
                    out.join.data[idx] = slot.masks[seat]
                        .fleets
                        .iter()
                        .position(|mask| mask.record == choice.join)
                        .map_or(0, |row| row as i64 + 1);
                    out.unload.data[idx] = i64::from(choice.unload);
                    out.family.data[idx] =
                        fleet_family(&game_data.fleets[choice.record - 1]) as i64;
                }
                for choice in &action.planets {
                    let world = (at * worlds + choice.record - 1) * BUILD_SLOTS;
                    for slot in 0..BUILD_SLOTS {
                        out.build.data[world + slot] = choice.build[slot] as i64;
                        out.amount.data[world + slot] = choice.amount[slot] as i64;
                    }
                }
            }
        }
        Ok(out)
    }

    fn seats_where(&self, pick: impl Fn(&Seat) -> bool) -> Tensor<bool> {
        Tensor {
            data: self
                .slots
                .iter()
                .flat_map(|slot| slot.lineup.iter().map(&pick))
                .collect(),
            shape: vec![self.games(), usize::from(self.players)],
        }
    }

    /// Strategist seats' goal masks; other seats allow only hold and auto.
    pub fn goal_masks(&self) -> BatchGoalMasks {
        let (games, seats, worlds) = (self.games(), usize::from(self.players), self.worlds());
        let mut out = BatchGoalMasks {
            strike: falses(&[games, seats, 1 + worlds]),
            colonize: falses(&[games, seats, 1 + worlds]),
        };
        for (game, slot) in self.slots.iter().enumerate() {
            for (seat, masks) in slot.goal_masks.iter().enumerate() {
                let at = game * seats + seat;
                match masks {
                    Some(masks) => {
                        put(&mut out.strike, at, &masks.strike);
                        put(&mut out.colonize, at, &masks.colonize);
                    }
                    None => {
                        out.strike.data[at * (1 + worlds)] = true;
                        out.colonize.data[at * (1 + worlds)] = true;
                    }
                }
            }
        }
        out
    }

    /// The goals the executor would set itself, with each strategist seat's
    /// weights, in every live game; zero for other seats. These are
    /// imitation labels.
    pub fn goal_labels(&self) -> GoalArrays {
        let (games, seats) = (self.games(), usize::from(self.players));
        let mut out = GoalArrays {
            tax: Tensor {
                data: vec![0; games * seats],
                shape: vec![games, seats],
            },
            diplomacy: Tensor {
                data: vec![0; games * seats * seats],
                shape: vec![games, seats, seats],
            },
            posture: Tensor {
                data: vec![0; games * seats],
                shape: vec![games, seats],
            },
            allocation: Tensor {
                data: vec![0; games * seats],
                shape: vec![games, seats],
            },
            strike: Tensor {
                data: vec![0; games * seats],
                shape: vec![games, seats],
            },
            colonize: Tensor {
                data: vec![0; games * seats],
                shape: vec![games, seats],
            },
        };
        for (game, slot) in self.slots.iter().enumerate() {
            if slot.result.is_some() {
                continue;
            }
            for (seat, kind) in slot.lineup.iter().enumerate() {
                let Some(weights) = kind.goal_weights() else {
                    continue;
                };
                // The allocation label is the executor's rule (t175), which
                // its own play leaves off.
                let mut weights = weights;
                weights.doctrine.allocate = true;
                let goals = plan_goals(
                    &slot.game.play.built.game_data,
                    &slot.game.seats[seat],
                    &slot.masks[seat],
                    &weights,
                    slot.start_year,
                );
                let at = game * seats + seat;
                out.tax.data[at] = goals.tax as i64;
                for (other, &choice) in goals.diplomacy.iter().enumerate() {
                    out.diplomacy.data[at * seats + other] = choice as i64;
                }
                out.posture.data[at] = goals.posture as i64;
                out.allocation.data[at] = goals.allocation as i64;
                out.strike.data[at] = goals.strike as i64;
                out.colonize.data[at] = goals.colonize as i64;
            }
        }
        out
    }

    /// The result of the game in slot `idx`, once it has ended.
    pub fn result(&self, idx: usize) -> Option<&MatchResult> {
        self.slots[idx].result.as_ref()
    }

    /// The game in slot `idx`, for tests and inspection. Its `turns` are
    /// every submission applied, for [`crate::replay`].
    pub fn game(&self, idx: usize) -> &SeatedGame {
        &self.slots[idx].game
    }

    pub fn observe(&self) -> BatchObservation {
        let seats = usize::from(self.players);
        let views: Vec<Observation> = self
            .slots
            .iter()
            .flat_map(|slot| (1..=self.players).map(|empire| slot.game.observe(empire)))
            .collect();
        let lead = [self.games(), seats];
        let pad_rows = |pick: fn(&Observation) -> &Vec<f32>, width: usize, rows: usize| {
            pad_table(
                lead,
                views.iter().map(|view| pick(view).as_slice()),
                width,
                rows,
            )
        };
        let most = |pick: fn(&Observation) -> &Vec<f32>, width: usize| {
            views
                .iter()
                .map(|view| pick(view).len() / width)
                .max()
                .unwrap_or(0)
        };
        BatchObservation {
            global: Tensor {
                data: views.iter().flat_map(|view| view.global.clone()).collect(),
                shape: vec![self.games(), seats, GLOBAL_WIDTH],
            },
            planets: pad_rows(|v| &v.planets, PLANET_WIDTH, self.worlds()),
            fleets: pad_rows(|v| &v.fleets, FLEET_WIDTH, self.fleets()),
            contacts: pad_rows(
                |v| &v.contacts,
                CONTACT_WIDTH,
                most(|v| &v.contacts, CONTACT_WIDTH),
            ),
            starbases: pad_rows(
                |v| &v.starbases,
                STARBASE_WIDTH,
                most(|v| &v.starbases, STARBASE_WIDTH),
            ),
        }
    }

    pub fn masks(&self) -> BatchMasks {
        let (games, seats) = (self.games(), usize::from(self.players));
        let (worlds, fleets) = (self.worlds(), self.fleets());
        let mut out = BatchMasks {
            diplomacy: falses(&[games, seats, seats, DIPLOMACY_CHOICES]),
            build: falses(&[games, seats, worlds, BUILD_CHOICES]),
            mission: falses(&[games, seats, fleets, MISSION_CHOICES]),
            target: falses(&[games, seats, fleets, MISSION_CHOICES, worlds]),
            roe: falses(&[games, seats, fleets, ROE_CHOICES]),
            load: falses(&[games, seats, fleets]),
            detach: falses(&[games, seats, fleets, DETACH_CHOICES]),
            join: falses(&[games, seats, fleets, 1 + fleets]),
            unload: falses(&[games, seats, fleets]),
        };
        for (game, slot) in self.slots.iter().enumerate() {
            let real_worlds = slot.game.play.built.game_data.planets.len();
            for (seat, masks) in slot.masks.iter().enumerate() {
                let at = game * seats + seat;
                for (other, row) in masks.diplomacy.iter().enumerate() {
                    put(&mut out.diplomacy, at * seats + other, row);
                }
                for world in 0..worlds {
                    out.build.data[(at * worlds + world) * BUILD_CHOICES] = true;
                }
                for planet in &masks.planets {
                    put(
                        &mut out.build,
                        at * worlds + planet.record - 1,
                        &planet.build,
                    );
                }
                for row in 0..fleets {
                    let fleet = at * fleets + row;
                    match masks.fleets.get(row) {
                        Some(mask) => {
                            put(&mut out.mission, fleet, &mask.mission);
                            for (mission, targets) in mask.targets.iter().enumerate() {
                                put(&mut out.target, fleet * MISSION_CHOICES + mission, targets);
                            }
                            put(&mut out.roe, fleet, &mask.roe);
                            out.load.data[fleet] = mask.load;
                            put(&mut out.detach, fleet, &mask.detach);
                            // A finished game keeps masks from before its
                            // last battles, which may name lost fleets.
                            put(
                                &mut out.join,
                                fleet,
                                &mask.join[..mask.join.len().min(1 + fleets)],
                            );
                            out.unload.data[fleet] = mask.unload;
                        }
                        None => {
                            out.mission.data[fleet * MISSION_CHOICES] = true;
                            put(
                                &mut out.target,
                                fleet * MISSION_CHOICES,
                                &vec![true; real_worlds],
                            );
                            out.roe.data[fleet * ROE_CHOICES] = true;
                            out.detach.data[fleet * DETACH_CHOICES] = true;
                            out.join.data[fleet * (1 + fleets)] = true;
                        }
                    }
                }
            }
        }
        out
    }

    /// Apply learned seats' `actions` and strategist seats' `goals`, play
    /// every scripted seat, run a maintenance year in each live game, and
    /// prepare the next year. Nothing changes when any choice falls outside
    /// the masks, or the choices a seat needs are missing.
    pub fn step(
        &mut self,
        actions: Option<&BatchActions>,
        goals: Option<&BatchGoals>,
    ) -> Result<StepResult, String> {
        let (games, seats) = (self.games(), usize::from(self.players));
        let (worlds, fleets) = (self.worlds(), self.fleets());
        let mut expect = Vec::new();
        if let Some(actions) = actions {
            expect.extend([
                ("tax", actions.tax.len(), games * seats),
                ("diplomacy", actions.diplomacy.len(), games * seats * seats),
                (
                    "build",
                    actions.build.len(),
                    games * seats * worlds * BUILD_SLOTS,
                ),
                (
                    "amount",
                    actions.amount.len(),
                    games * seats * worlds * BUILD_SLOTS,
                ),
                ("mission", actions.mission.len(), games * seats * fleets),
                ("target", actions.target.len(), games * seats * fleets),
                ("waypoint", actions.waypoint.len(), games * seats * fleets),
                ("speed", actions.speed.len(), games * seats * fleets),
                ("roe", actions.roe.len(), games * seats * fleets),
                ("load", actions.load.len(), games * seats * fleets),
                ("detach", actions.detach.len(), games * seats * fleets),
                ("join", actions.join.len(), games * seats * fleets),
                ("unload", actions.unload.len(), games * seats * fleets),
            ]);
        }
        if let Some(goals) = goals {
            expect.extend([
                ("tax goal", goals.tax.len(), games * seats),
                (
                    "diplomacy goal",
                    goals.diplomacy.len(),
                    games * seats * seats,
                ),
                ("posture", goals.posture.len(), games * seats),
                ("allocation", goals.allocation.len(), games * seats),
                ("strike", goals.strike.len(), games * seats),
                ("colonize", goals.colonize.len(), games * seats),
            ]);
        }
        for (name, got, want) in expect {
            if got != want {
                return Err(format!("{name} has {got} entries, expected {want}"));
            }
        }
        let mut chosen: Vec<Vec<Turn>> = Vec::with_capacity(games);
        for (game, slot) in self.slots.iter().enumerate() {
            let mut turns = Vec::new();
            if slot.result.is_none() {
                for (seat, masks) in slot.masks.iter().enumerate() {
                    let at = game * seats + seat;
                    let turn = match (slot.lineup[seat], actions, goals) {
                        (Seat::Learned, Some(actions), _) => {
                            seat_action(masks, actions, at, seats, worlds, fleets).map(Turn::Action)
                        }
                        (Seat::Strategist(_), _, Some(goals)) => {
                            let goal_masks = slot.goal_masks[seat]
                                .as_ref()
                                .expect("a strategist seat has goal masks");
                            seat_goals(goal_masks, goals, at, seats).map(Turn::Goals)
                        }
                        (Seat::Hybrid(..), Some(actions), Some(goals)) => {
                            let goal_masks = slot.goal_masks[seat]
                                .as_ref()
                                .expect("a hybrid seat has goal masks");
                            seat_goals(goal_masks, goals, at, seats).and_then(|goals| {
                                let fleets = fleet_choices(masks, actions, at, fleets)?;
                                let builds = planet_choices(masks, actions, at, worlds)?;
                                Ok(Turn::Hybrid(goals, fleets, builds))
                            })
                        }
                        (Seat::Scripted(_), _, _) => Ok(Turn::Scripted),
                        (Seat::Learned, None, _) => Err("learned seats need actions".into()),
                        (Seat::Strategist(_), _, None) => Err("strategist seats need goals".into()),
                        (Seat::Hybrid(..), _, _) => {
                            Err("hybrid seats need goals and actions".into())
                        }
                    };
                    turns.push(turn.map_err(|err| format!("game {game} seat {seat}: {err}"))?);
                }
            }
            chosen.push(turns);
        }

        let (years, limit_credit) = (self.years, self.limit_credit);
        let mut rewards: Vec<Option<Vec<f32>>> = vec![None; games];
        let mut pairs: Vec<(&mut Slot, &mut Option<Vec<f32>>)> =
            self.slots.iter_mut().zip(rewards.iter_mut()).collect();
        par_for_each_mut(&mut pairs, self.threads, |idx, (slot, reward)| {
            if slot.result.is_some() {
                return;
            }
            for (seat, turn) in chosen[idx].iter().enumerate() {
                slot.play_seat(seat, turn);
            }
            slot.game.run_maintenance().expect("maintenance should run");
            slot.years_played += 1;
            let winner = slot.game.play.winner_state.winner_empire;
            if winner.is_some() || slot.years_played >= years {
                let result =
                    match_result(&slot.game.play.built.game_data, winner, slot.years_played);
                let credit = if winner.is_some() { 1.0 } else { limit_credit };
                **reward = Some(
                    (1..=slot.game.seats.len() as u8)
                        .map(|empire| credit * result.win_share(empire))
                        .collect(),
                );
                slot.result = Some(result);
            } else {
                slot.prepare();
            }
        });

        let standing = |slot: &Slot| {
            slot.result.clone().unwrap_or_else(|| {
                let game_data = &slot.game.play.built.game_data;
                match_result(game_data, None, slot.years_played)
            })
        };
        let results: Vec<MatchResult> = self.slots.iter().map(standing).collect();
        Ok(StepResult {
            reward: Tensor {
                data: rewards
                    .into_iter()
                    .flat_map(|reward| reward.unwrap_or_else(|| vec![0.0; seats]))
                    .collect(),
                shape: vec![games, seats],
            },
            done: Tensor {
                data: self
                    .slots
                    .iter()
                    .map(|slot| slot.result.is_some())
                    .collect(),
                shape: vec![games],
            },
            production: Tensor {
                data: results.iter().flat_map(|r| r.production.clone()).collect(),
                shape: vec![games, seats],
            },
            worlds: Tensor {
                data: results.iter().flat_map(|r| r.worlds.clone()).collect(),
                shape: vec![games, seats],
            },
        })
    }
}

/// One seat's choices for a step.
enum Turn {
    Action(SeatAction),
    Goals(Goals),
    /// Goals, and the network's choice for every fleet row and world.
    Hybrid(Goals, Vec<FleetChoice>, Vec<PlanetChoice>),
    Scripted,
}

impl Slot {
    /// Commission every learned and strategist seat's finished ships and
    /// build its masks.
    fn prepare(&mut self) {
        let players = self.lineup.len();
        let mut masks = Vec::with_capacity(players);
        let mut goal_mask_rows = Vec::with_capacity(players);
        for (seat, kind) in self.lineup.iter().enumerate() {
            let empire = seat as u8 + 1;
            if let Seat::Scripted(_) = kind {
                goal_mask_rows.push(None);
                masks.push(SeatMasks {
                    empire,
                    diplomacy: vec![[true, false, false]; players],
                    planets: Vec::new(),
                    fleets: Vec::new(),
                });
                continue;
            }
            if let Some(commission) = commission_submission(&self.game.play.built.game_data, empire)
            {
                self.game
                    .submit_turn(&commission)
                    .expect("commissioning should submit");
            }
            let game_data = &self.game.play.built.game_data;
            let started = Instant::now();
            let mut seat_mask = seat_masks(game_data, empire);
            if let Seat::Hybrid(_, families) = kind {
                network_joins(game_data, &mut seat_mask, *families);
            }
            self.mask_time += started.elapsed();
            self.mask_count += 1;
            goal_mask_rows.push(
                kind.goal_weights()
                    .map(|_| goal_masks(game_data, &self.game.seats[seat], &seat_mask)),
            );
            masks.push(seat_mask);
        }
        self.masks = masks;
        self.goal_masks = goal_mask_rows;
    }

    /// Submit seat `seat`'s turn from its choices or its scripted policy.
    fn play_seat(&mut self, seat: usize, turn: &Turn) {
        let empire = seat as u8 + 1;
        if let Seat::Scripted(SeatPolicy::Executor(weights)) = self.lineup[seat] {
            return executor_turn(&mut self.game, empire, &weights, self.start_year);
        }
        let game_data = &self.game.play.built.game_data;
        let submission = match (self.lineup[seat], turn) {
            (_, Turn::Action(action)) => decode_action(game_data, empire, action).submission,
            (Seat::Strategist(weights), Turn::Goals(goals)) => {
                let memory = &self.game.seats[seat];
                let action = execute_goals(game_data, memory, &self.masks[seat], &weights, goals);
                decode_action(game_data, empire, &action).submission
            }
            (Seat::Hybrid(weights, families), Turn::Hybrid(goals, network, builds)) => {
                let memory = &self.game.seats[seat];
                let mut action =
                    execute_goals(game_data, memory, &self.masks[seat], &weights, goals);
                hand_to_network(
                    game_data,
                    memory,
                    &weights,
                    families,
                    &mut action,
                    network,
                    builds,
                );
                decode_action(game_data, empire, &action).submission
            }
            (Seat::Scripted(SeatPolicy::Aggressor), _) => {
                nc_fuzz::plan_turn(Policy::Aggressor, game_data, empire, &mut self.choices)
                    .submission
            }
            (Seat::Scripted(SeatPolicy::Autopilot), _) => return,
            _ => unreachable!("every seat's turn matches its kind"),
        };
        self.game
            .submit_turn(&submission)
            .expect("a seat's turn should submit");
    }
}
