//! A batch of seeded games stepped together, with every seat's observation,
//! masks and actions as padded arrays for a learner.
//!
//! Arrays are row-major and indexed `[game, seat, ...]`. World arrays have
//! one row per world in planet-record order, padded to the largest map in
//! the batch; fleet, contact and starbase arrays are padded to the most rows
//! any seat has. Each padded table has a `valid` flag per row.
//!
//! Between steps every live game waits for its seats' actions, with finished
//! ships already commissioned, so the observation and masks show the fleets
//! the seats will order. Padding rows allow only "no build", "keep orders"
//! and ROE 0, so sampling within the masks never meets an empty row.

use std::thread;
use std::time::{Duration, Instant};

use nc_fuzz::Policy;

use crate::action::{
    BUILD_AMOUNT_CHOICES, BUILD_CHOICES, DIPLOMACY_CHOICES, FleetChoice, MISSION_CHOICES,
    PlanetChoice, ROE_CHOICES, SPEED_CHOICES, SeatAction, SeatMasks, TAX_CHOICES,
    commission_submission, decode_action, seat_masks,
};
use crate::arena::{MatchResult, match_result, seated_game};
use crate::game::SeatedGame;
use crate::observe::{
    CONTACT_WIDTH, FLEET_WIDTH, GLOBAL_WIDTH, Observation, PLANET_WIDTH, STARBASE_WIDTH,
};

/// A row-major array and its shape.
#[derive(Debug, Clone, PartialEq)]
pub struct Tensor<T> {
    pub data: Vec<T>,
    pub shape: Vec<usize>,
}

/// One padded entity table and its row flags.
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    /// `[games, seats, rows, width]`.
    pub rows: Tensor<f32>,
    /// `[games, seats, rows]`.
    pub valid: Tensor<bool>,
}

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
/// fleet count. Tax, build-amount and speed choices are always allowed.
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
}

/// Every seat's choices, shaped as [`BatchMasks`] describes. Entries for
/// finished games, padding rows and worlds a seat does not own are ignored.
#[derive(Debug, Clone, Copy)]
pub struct BatchActions<'a> {
    /// `[games, seats]`.
    pub tax: &'a [i64],
    /// `[games, seats, seats]`.
    pub diplomacy: &'a [i64],
    /// `[games, seats, W]`.
    pub build: &'a [i64],
    pub amount: &'a [i64],
    /// `[games, seats, F]`.
    pub mission: &'a [i64],
    pub target: &'a [i64],
    pub speed: &'a [i64],
    pub roe: &'a [i64],
    /// Nonzero to load armies.
    pub load: &'a [i64],
}

/// What one step did.
#[derive(Debug, Clone, PartialEq)]
pub struct StepResult {
    /// `[games, seats]`: the seat's win share when its game ends this step,
    /// otherwise 0. See [`MatchResult::win_share`].
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
    years_played: u16,
    /// The seats' masks for the coming year.
    masks: Vec<SeatMasks>,
    result: Option<MatchResult>,
    mask_time: Duration,
    mask_count: u64,
}

/// Seeded games played in step, every seat chosen by the caller.
#[derive(Debug, Clone)]
pub struct BatchEnv {
    players: u8,
    years: u16,
    threads: usize,
    slots: Vec<Slot>,
}

impl BatchEnv {
    /// Fresh games, one per map seed, each run with its campaign seed for
    /// up to `years` years. `threads` 0 uses every core.
    pub fn new(
        players: u8,
        map_seeds: &[u64],
        campaign_seeds: &[u64],
        years: u16,
        threads: usize,
    ) -> Self {
        assert_eq!(
            map_seeds.len(),
            campaign_seeds.len(),
            "one campaign seed per map seed"
        );
        let threads = if threads == 0 {
            thread::available_parallelism().map_or(1, |n| n.get())
        } else {
            threads
        };
        let mut slots: Vec<Option<Slot>> = map_seeds.iter().map(|_| None).collect();
        par_for_each_mut(&mut slots, threads, |idx, slot| {
            let policies = vec![Policy::Aggressor; usize::from(players)];
            let mut new = Slot {
                game: seated_game(map_seeds[idx], policies, campaign_seeds[idx], years),
                years_played: 0,
                masks: Vec::new(),
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

    /// The padded fleet count, `F`.
    pub fn fleets(&self) -> usize {
        self.slots
            .iter()
            .flat_map(|slot| slot.masks.iter().map(|masks| masks.fleets.len()))
            .max()
            .unwrap_or(0)
    }

    /// Total time spent in [`seat_masks`] and how many seat masks it built.
    pub fn mask_cost(&self) -> (Duration, u64) {
        self.slots.iter().fold((Duration::ZERO, 0), |sum, slot| {
            (sum.0 + slot.mask_time, sum.1 + slot.mask_count)
        })
    }

    /// The game in slot `idx`, for tests and inspection.
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
                        }
                        None => {
                            out.mission.data[fleet * MISSION_CHOICES] = true;
                            put(
                                &mut out.target,
                                fleet * MISSION_CHOICES,
                                &vec![true; real_worlds],
                            );
                            out.roe.data[fleet * ROE_CHOICES] = true;
                        }
                    }
                }
            }
        }
        out
    }

    /// Apply every seat's choices, run a maintenance year in each live game,
    /// and prepare the next year. Nothing changes when any choice falls
    /// outside the masks.
    pub fn step(&mut self, actions: &BatchActions) -> Result<StepResult, String> {
        let (games, seats) = (self.games(), usize::from(self.players));
        let (worlds, fleets) = (self.worlds(), self.fleets());
        let expect = [
            ("tax", actions.tax.len(), games * seats),
            ("diplomacy", actions.diplomacy.len(), games * seats * seats),
            ("build", actions.build.len(), games * seats * worlds),
            ("amount", actions.amount.len(), games * seats * worlds),
            ("mission", actions.mission.len(), games * seats * fleets),
            ("target", actions.target.len(), games * seats * fleets),
            ("speed", actions.speed.len(), games * seats * fleets),
            ("roe", actions.roe.len(), games * seats * fleets),
            ("load", actions.load.len(), games * seats * fleets),
        ];
        for (name, got, want) in expect {
            if got != want {
                return Err(format!("{name} has {got} entries, expected {want}"));
            }
        }
        let mut chosen: Vec<Vec<SeatAction>> = Vec::with_capacity(games);
        for (game, slot) in self.slots.iter().enumerate() {
            let mut seat_actions = Vec::new();
            if slot.result.is_none() {
                for (seat, masks) in slot.masks.iter().enumerate() {
                    let at = game * seats + seat;
                    seat_actions.push(
                        seat_action(masks, actions, at, seats, worlds, fleets)
                            .map_err(|err| format!("game {game} seat {seat}: {err}"))?,
                    );
                }
            }
            chosen.push(seat_actions);
        }

        let years = self.years;
        let mut rewards: Vec<Option<Vec<f32>>> = vec![None; games];
        let mut pairs: Vec<(&mut Slot, &mut Option<Vec<f32>>)> =
            self.slots.iter_mut().zip(rewards.iter_mut()).collect();
        par_for_each_mut(&mut pairs, self.threads, |idx, (slot, reward)| {
            if slot.result.is_some() {
                return;
            }
            for (seat, action) in chosen[idx].iter().enumerate() {
                let empire = seat as u8 + 1;
                let decoded = decode_action(&slot.game.play.built.game_data, empire, action);
                slot.game
                    .submit_turn(&decoded.submission)
                    .expect("a masked action should submit");
            }
            slot.game.run_maintenance().expect("maintenance should run");
            slot.years_played += 1;
            let winner = slot.game.play.winner_state.winner_empire;
            if winner.is_some() || slot.years_played >= years {
                let result =
                    match_result(&slot.game.play.built.game_data, winner, slot.years_played);
                **reward = Some(
                    (1..=slot.game.seats.len() as u8)
                        .map(|empire| result.win_share(empire))
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

impl Slot {
    /// Commission every seat's finished ships and build its masks.
    fn prepare(&mut self) {
        let seats = self.game.seats.len() as u8;
        for empire in 1..=seats {
            if let Some(commission) = commission_submission(&self.game.play.built.game_data, empire)
            {
                self.game
                    .submit_turn(&commission)
                    .expect("commissioning should submit");
            }
        }
        let started = Instant::now();
        self.masks = (1..=seats)
            .map(|empire| seat_masks(&self.game.play.built.game_data, empire))
            .collect();
        self.mask_time += started.elapsed();
        self.mask_count += u64::from(seats);
    }
}

/// One seat's [`SeatAction`] from the batch arrays, checked against its
/// masks. `at` is the flat `[game, seat]` index.
fn seat_action(
    masks: &SeatMasks,
    actions: &BatchActions,
    at: usize,
    seats: usize,
    worlds: usize,
    fleets: usize,
) -> Result<SeatAction, String> {
    let tax = choice("tax", actions.tax[at], &[true; TAX_CHOICES])?;
    let diplomacy = masks
        .diplomacy
        .iter()
        .enumerate()
        .map(|(other, mask)| choice("diplomacy", actions.diplomacy[at * seats + other], mask))
        .collect::<Result<_, _>>()?;
    let planets = masks
        .planets
        .iter()
        .map(|mask| {
            let world = at * worlds + mask.record - 1;
            Ok(PlanetChoice {
                record: mask.record,
                build: choice("build", actions.build[world], &mask.build)?,
                amount: choice(
                    "amount",
                    actions.amount[world],
                    &[true; BUILD_AMOUNT_CHOICES],
                )?,
            })
        })
        .collect::<Result<_, String>>()?;
    let fleets = masks
        .fleets
        .iter()
        .enumerate()
        .map(|(row, mask)| {
            let fleet = at * fleets + row;
            let mission = choice("mission", actions.mission[fleet], &mask.mission)?;
            let load = actions.load[fleet] != 0;
            if load && !mask.load {
                return Err(format!("fleet row {row}: load is not allowed"));
            }
            Ok(FleetChoice {
                record: mask.record,
                mission,
                target: choice("target", actions.target[fleet], &mask.targets[mission])?,
                speed: choice("speed", actions.speed[fleet], &[true; SPEED_CHOICES])?,
                roe: choice("roe", actions.roe[fleet], &mask.roe)?,
                load,
            })
        })
        .collect::<Result<_, String>>()?;
    Ok(SeatAction {
        tax,
        diplomacy,
        planets,
        fleets,
    })
}

fn choice(name: &str, value: i64, mask: &[bool]) -> Result<usize, String> {
    usize::try_from(value)
        .ok()
        .filter(|&idx| mask.get(idx).copied().unwrap_or(false))
        .ok_or_else(|| format!("{name} choice {value} is not allowed"))
}

fn falses(shape: &[usize]) -> Tensor<bool> {
    Tensor {
        data: vec![false; shape.iter().product()],
        shape: shape.to_vec(),
    }
}

/// Copy `row` into row `idx` of a tensor whose last axis is `row.len()` or
/// longer.
fn put(tensor: &mut Tensor<bool>, idx: usize, row: &[bool]) {
    let width = *tensor.shape.last().expect("a tensor has an axis");
    tensor.data[idx * width..idx * width + row.len()].copy_from_slice(row);
}

/// Pad each view's rows of `width` to `rows` rows.
fn pad_table<'a>(
    lead: [usize; 2],
    views: impl Iterator<Item = &'a [f32]>,
    width: usize,
    rows: usize,
) -> Table {
    let mut table = Table {
        rows: Tensor {
            data: Vec::with_capacity(lead[0] * lead[1] * rows * width),
            shape: vec![lead[0], lead[1], rows, width],
        },
        valid: Tensor {
            data: Vec::with_capacity(lead[0] * lead[1] * rows),
            shape: vec![lead[0], lead[1], rows],
        },
    };
    for view in views {
        let count = view.len() / width;
        table.rows.data.extend_from_slice(view);
        table
            .rows
            .data
            .resize(table.rows.data.len() + (rows - count) * width, 0.0);
        table.valid.data.extend((0..rows).map(|row| row < count));
    }
    table
}

/// Run `f` on every item, the items split into contiguous runs over
/// `threads` threads.
fn par_for_each_mut<T: Send>(items: &mut [T], threads: usize, f: impl Fn(usize, &mut T) + Sync) {
    let chunk = items.len().div_ceil(threads.max(1)).max(1);
    thread::scope(|scope| {
        for (run, part) in items.chunks_mut(chunk).enumerate() {
            let f = &f;
            scope.spawn(move || {
                for (offset, item) in part.iter_mut().enumerate() {
                    f(run * chunk + offset, item);
                }
            });
        }
    });
}
