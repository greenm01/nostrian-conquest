//! Python bindings for [`nc_ai::BatchEnv`]: a batch of seeded games whose
//! observations, masks and results come back as numpy arrays that own the
//! Rust buffers, without a copy. Choices go in as int64 arrays shaped as
//! the masks describe. Games run with the GIL released.

use nc_ai::{
    BatchActions, BatchEnv, BatchGoals, Families, MatchResult, Seat, SeatPolicy, StoredPosition,
    Table, Tensor, Weights, load_position,
};
use nc_data::TurnSubmission;
use numpy::{Element, PyArray1, PyArrayMethods, PyReadonlyArrayDyn};
use pyo3::exceptions::{PyKeyError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// Seeded games stepped together; every seat is chosen by the caller.
#[pyclass(module = "nc_ai_env")]
struct Env {
    players: u8,
    years: u16,
    threads: usize,
    limit_credit: f32,
    batch: Option<BatchEnv>,
}

/// Each game's stored start, every distinct `(path, year)` loaded once. No
/// starts at all means every game starts fresh.
fn stored_starts(
    starts: Vec<Option<(String, u16)>>,
    games: usize,
    players: u8,
) -> PyResult<Vec<Option<StoredPosition>>> {
    if starts.is_empty() {
        return Ok(vec![None; games]);
    }
    if starts.len() != games {
        return Err(PyValueError::new_err("starts must hold one entry per game"));
    }
    let mut loaded: std::collections::HashMap<(String, u16), StoredPosition> = Default::default();
    starts
        .into_iter()
        .map(|start| {
            let Some((path, year)) = start else {
                return Ok(None);
            };
            if !loaded.contains_key(&(path.clone(), year)) {
                let position = load_position(std::path::Path::new(&path), year)
                    .map_err(|err| PyValueError::new_err(format!("{path}: {err}")))?;
                if position.game_data.conquest.player_count != players {
                    return Err(PyValueError::new_err(format!(
                        "{path} is a game of {} players, not {players}",
                        position.game_data.conquest.player_count
                    )));
                }
                loaded.insert((path.clone(), year), position);
            }
            Ok(Some(loaded[&(path, year)].clone()))
        })
        .collect()
}

#[pymethods]
impl Env {
    /// `players` seats per game, each game ending after `years` years or a
    /// declared winner. `threads` 0 uses every core. `limit_credit` is what
    /// a production lead at the year limit pays, against 1 for sweeping
    /// the map.
    #[new]
    #[pyo3(signature = (players, years, threads = 0, limit_credit = 1.0))]
    fn new(players: u8, years: u16, threads: usize, limit_credit: f32) -> PyResult<Self> {
        if !(2..=25).contains(&players) {
            return Err(PyValueError::new_err("players must be 2 to 25"));
        }
        Ok(Self {
            players,
            years,
            threads,
            limit_credit,
            batch: None,
        })
    }

    /// Start one fresh game per map seed. Campaign seeds default to the map
    /// seeds. `seats` gives each game's lineup, one name per seat:
    /// `learned`, `strategist` (goals carried out by the tuned executor),
    /// `hybrid:FAMILIES` (a strategist whose network also orders the fleets
    /// of the comma-separated families `colonize`, `scout`, `strike` and
    /// `convoy`, and with `builds` also builds at its worlds),
    /// `executor` (tuned weights), `baseline` (the executor's hand-set
    /// weights), `classic` (t154's executor), `aggressor` or `autopilot`. Every seat is learned by
    /// default. A personality (t167) names its style's executor, and
    /// `strategist`, `hybrid:...` and `executor` take a style as
    /// `@NAME`, e.g. `hybrid:strike@rusher`; the default is `land-grab`.
    /// `starts` gives each game `None` or `(path, year)`: play on from that
    /// year of the stored game database at `path`, with its own campaign
    /// seed, its years already played counting toward `years`.
    #[pyo3(signature = (map_seeds, campaign_seeds = None, seats = None, starts = None))]
    fn reset(
        &mut self,
        py: Python<'_>,
        map_seeds: Vec<u64>,
        campaign_seeds: Option<Vec<u64>>,
        seats: Option<Vec<Vec<String>>>,
        starts: Option<Vec<Option<(String, u16)>>>,
    ) -> PyResult<()> {
        let campaign_seeds = campaign_seeds.unwrap_or_else(|| map_seeds.clone());
        if campaign_seeds.len() != map_seeds.len() {
            return Err(PyValueError::new_err(
                "campaign_seeds must match map_seeds in length",
            ));
        }
        let lineups = match seats {
            None => vec![vec![Seat::Learned; usize::from(self.players)]; map_seeds.len()],
            Some(seats) => seats
                .iter()
                .map(|lineup| lineup.iter().map(|name| seat(name)).collect())
                .collect::<PyResult<Vec<Vec<_>>>>()?,
        };
        if lineups.len() != map_seeds.len()
            || lineups
                .iter()
                .any(|lineup| lineup.len() != usize::from(self.players))
        {
            return Err(PyValueError::new_err(
                "seats must hold one lineup per game and one name per seat",
            ));
        }
        let starts = stored_starts(starts.unwrap_or_default(), map_seeds.len(), self.players)?;
        let (players, years, threads) = (self.players, self.years, self.threads);
        let mut batch = py.detach(|| {
            BatchEnv::with_starts(
                players,
                &map_seeds,
                &campaign_seeds,
                &lineups,
                &starts,
                years,
                threads,
            )
        });
        batch.set_limit_credit(self.limit_credit);
        self.batch = Some(batch);
        Ok(())
    }

    /// `[games, seats]` bools: the seat is learned and takes `actions`.
    #[getter]
    fn learned<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        array(py, self.batch()?.learned())
    }

    /// `[games, seats]` bools: the seat takes `goals` (a strategist or a
    /// hybrid).
    #[getter]
    fn strategist<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        array(py, self.batch()?.strategist())
    }

    /// `[games, seats]` bools: the seat is a hybrid, also taking the fleet
    /// fields of `actions` for its handed-over families.
    #[getter]
    fn hybrid<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        array(py, self.batch()?.hybrid())
    }

    /// The executor's fleet choices for every goal-taking seat if it
    /// carried out `goals` this year: `mission`, `target`, `waypoint`,
    /// `speed`, `roe`,
    /// `load`, `detach`, `join`, `unload` and each row's `family` (0 colonize, 1
    /// scout, 2 strike, 3 convoy, -1 padding), all `[games, seats, F]` int64;
    /// and the executor's `build` and `amount`, `[games, seats, W,
    /// BUILD_SLOTS]` int64.
    fn fleet_labels<'py>(
        &self,
        py: Python<'py>,
        goals: &Bound<'py, PyDict>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let goal_arrays = arrays(
            goals,
            &[
                "tax",
                "diplomacy",
                "posture",
                "allocation",
                "strike",
                "colonize",
            ],
        )?;
        let s = slices(&goal_arrays)?;
        let goals = BatchGoals {
            tax: s[0],
            diplomacy: s[1],
            posture: s[2],
            allocation: s[3],
            strike: s[4],
            colonize: s[5],
        };
        let batch = self.batch()?;
        let labels = py
            .detach(|| batch.fleet_labels(&goals))
            .map_err(PyValueError::new_err)?;
        let out = PyDict::new(py);
        out.set_item("mission", array(py, labels.mission)?)?;
        out.set_item("target", array(py, labels.target)?)?;
        out.set_item("waypoint", array(py, labels.waypoint)?)?;
        out.set_item("speed", array(py, labels.speed)?)?;
        out.set_item("roe", array(py, labels.roe)?)?;
        out.set_item("load", array(py, labels.load)?)?;
        out.set_item("detach", array(py, labels.detach)?)?;
        out.set_item("join", array(py, labels.join)?)?;
        out.set_item("unload", array(py, labels.unload)?)?;
        out.set_item("family", array(py, labels.family)?)?;
        out.set_item("build", array(py, labels.build)?)?;
        out.set_item("amount", array(py, labels.amount)?)?;
        Ok(out)
    }

    /// Strategist goal masks: `strike` and `colonize`, `[games, seats,
    /// 1 + W]` (index 0 is hold and auto). Diplomacy is in `masks()`.
    fn goal_masks<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let batch = self.batch()?;
        let masks = py.detach(|| batch.goal_masks());
        let out = PyDict::new(py);
        out.set_item("strike", array(py, masks.strike)?)?;
        out.set_item("colonize", array(py, masks.colonize)?)?;
        Ok(out)
    }

    /// The goals the executor would set itself for every strategist seat:
    /// imitation labels shaped as `step`'s `goals`.
    fn goal_labels<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let batch = self.batch()?;
        let labels = py.detach(|| batch.goal_labels());
        let out = PyDict::new(py);
        out.set_item("tax", array(py, labels.tax)?)?;
        out.set_item("diplomacy", array(py, labels.diplomacy)?)?;
        out.set_item("posture", array(py, labels.posture)?)?;
        out.set_item("allocation", array(py, labels.allocation)?)?;
        out.set_item("strike", array(py, labels.strike)?)?;
        out.set_item("colonize", array(py, labels.colonize)?)?;
        Ok(out)
    }

    /// Every seat's observation: `global`, then `planets`, `fleets`,
    /// `contacts` and `starbases` with a `*_valid` row flag each.
    fn observe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let batch = self.batch()?;
        let obs = py.detach(|| batch.observe());
        let out = PyDict::new(py);
        out.set_item("global", array(py, obs.global)?)?;
        for (name, table) in [
            ("planets", obs.planets),
            ("fleets", obs.fleets),
            ("contacts", obs.contacts),
            ("starbases", obs.starbases),
        ] {
            put_table(&out, name, table)?;
        }
        Ok(out)
    }

    /// Every seat's legal choices as bool arrays: `diplomacy`, `build`,
    /// `mission`, `target`, `roe`, `load`, `detach`, `join` (`[games, seats, F,
    /// 1 + F]`) and `unload`.
    fn masks<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let batch = self.batch()?;
        let masks = py.detach(|| batch.masks());
        let out = PyDict::new(py);
        for (name, tensor) in [
            ("diplomacy", masks.diplomacy),
            ("build", masks.build),
            ("mission", masks.mission),
            ("target", masks.target),
            ("roe", masks.roe),
            ("load", masks.load),
            ("detach", masks.detach),
            ("join", masks.join),
            ("unload", masks.unload),
        ] {
            out.set_item(name, array(py, tensor)?)?;
        }
        Ok(out)
    }

    /// Play a year. `actions` is a dict of int64 arrays for learned seats
    /// (`tax`, `diplomacy`, `build`, `amount`, `mission`, `target`,
    /// `waypoint`, `speed`,
    /// `roe`, `load`, `detach`, and optionally `join` and `unload`, which default
    /// to none); `goals` one for strategist seats (`tax`, `diplomacy`,
    /// `posture`, `allocation`, `strike`, `colonize`). Returns `reward`, `done`,
    /// `production` and `worlds`. A choice outside the masks, or missing
    /// choices, raise ValueError and change nothing.
    #[pyo3(signature = (actions = None, goals = None))]
    fn step<'py>(
        &mut self,
        py: Python<'py>,
        actions: Option<&Bound<'py, PyDict>>,
        goals: Option<&Bound<'py, PyDict>>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let action_arrays = actions
            .map(|dict| {
                arrays(
                    dict,
                    &[
                        "tax",
                        "diplomacy",
                        "build",
                        "amount",
                        "mission",
                        "target",
                        "waypoint",
                        "speed",
                        "roe",
                        "load",
                        "detach",
                    ],
                )
            })
            .transpose()?;
        let goal_arrays = goals
            .map(|dict| {
                arrays(
                    dict,
                    &[
                        "tax",
                        "diplomacy",
                        "posture",
                        "allocation",
                        "strike",
                        "colonize",
                    ],
                )
            })
            .transpose()?;
        let action_slices = action_arrays.as_deref().map(slices).transpose()?;
        // Join and unload default to none, for callers that predate them.
        let extras = match (actions, &action_slices) {
            (Some(dict), Some(required)) => {
                let rows = required[4].len();
                let mut out = [vec![0; rows], vec![0; rows]];
                for (slot, name) in out.iter_mut().zip(["join", "unload"]) {
                    if let Some(item) = dict.get_item(name)? {
                        let array: PyReadonlyArrayDyn<'py, i64> = item
                            .extract()
                            .map_err(|err| PyValueError::new_err(format!("{name}: {err}")))?;
                        *slot = array
                            .as_slice()
                            .map_err(|_| PyValueError::new_err("choice arrays must be contiguous"))?
                            .to_vec();
                    }
                }
                Some(out)
            }
            _ => None,
        };
        let goal_slices = goal_arrays.as_deref().map(slices).transpose()?;
        let actions = action_slices
            .zip(extras.as_ref())
            .map(|(s, extras)| BatchActions {
                tax: s[0],
                diplomacy: s[1],
                build: s[2],
                amount: s[3],
                mission: s[4],
                target: s[5],
                waypoint: s[6],
                speed: s[7],
                roe: s[8],
                load: s[9],
                detach: s[10],
                join: &extras[0],
                unload: &extras[1],
            });
        let goals = goal_slices.map(|s| BatchGoals {
            tax: s[0],
            diplomacy: s[1],
            posture: s[2],
            allocation: s[3],
            strike: s[4],
            colonize: s[5],
        });
        let batch = self
            .batch
            .as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("call reset first"))?;
        let result = py
            .detach(|| batch.step(actions.as_ref(), goals.as_ref()))
            .map_err(PyValueError::new_err)?;
        let out = PyDict::new(py);
        out.set_item("reward", array(py, result.reward)?)?;
        out.set_item("done", array(py, result.done)?)?;
        out.set_item("production", array(py, result.production)?)?;
        out.set_item("worlds", array(py, result.worlds)?)?;
        Ok(out)
    }

    /// A new environment whose game `i` is an exact copy of this one's game
    /// `games[i]` as it stands, so copies stepped with the same choices stay
    /// identical and differ only where their choices do (t216).
    fn branch(&self, games: Vec<usize>) -> PyResult<Self> {
        let batch = self.batch()?;
        if games.iter().any(|&game| game >= batch.games()) {
            return Err(PyValueError::new_err("no such game"));
        }
        Ok(Self {
            players: self.players,
            years: self.years,
            threads: self.threads,
            limit_credit: self.limit_credit,
            batch: Some(batch.branch(&games)),
        })
    }

    /// Every submission applied in game `game`, in order, as turn KDL: with
    /// the game's seeds, enough for `replay` to rebuild it.
    fn turns(&self, game: usize) -> PyResult<Vec<String>> {
        let batch = self.batch()?;
        if game >= batch.games() {
            return Err(PyValueError::new_err("no such game"));
        }
        Ok(batch
            .game(game)
            .turns
            .iter()
            .map(TurnSubmission::to_kdl_string)
            .collect())
    }

    /// Game `game`'s result once it has ended: `years`, `winner` (0 for
    /// none), and `production` and `worlds` by seat. None while it runs.
    fn result<'py>(&self, py: Python<'py>, game: usize) -> PyResult<Option<Bound<'py, PyDict>>> {
        let batch = self.batch()?;
        if game >= batch.games() {
            return Err(PyValueError::new_err("no such game"));
        }
        batch
            .result(game)
            .map(|result| result_dict(py, result))
            .transpose()
    }

    #[getter]
    fn players(&self) -> u8 {
        self.players
    }

    #[getter]
    fn games(&self) -> PyResult<usize> {
        Ok(self.batch()?.games())
    }

    /// The padded world count, `W`.
    #[getter]
    fn worlds(&self) -> PyResult<usize> {
        Ok(self.batch()?.worlds())
    }

    /// The padded fleet count, `F`, for the coming year.
    #[getter]
    fn fleets(&self) -> PyResult<usize> {
        Ok(self.batch()?.fleets())
    }

    /// Seconds spent building seat masks since reset, and how many seat
    /// masks were built.
    #[getter]
    fn mask_cost(&self) -> PyResult<(f64, u64)> {
        let (time, count) = self.batch()?.mask_cost();
        Ok((time.as_secs_f64(), count))
    }
}

impl Env {
    fn batch(&self) -> PyResult<&BatchEnv> {
        self.batch
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("call reset first"))
    }
}

/// Rebuild a recorded game from its seeds, the seats the engine played on
/// autopilot, and its turns as KDL; return its result, as `Env.result`.
#[pyfunction]
fn replay<'py>(
    py: Python<'py>,
    map_seed: u64,
    campaign_seed: u64,
    autopilot: Vec<bool>,
    years: u16,
    turns: Vec<String>,
) -> PyResult<Bound<'py, PyDict>> {
    let turns = turns
        .iter()
        .map(|kdl| TurnSubmission::parse_kdl_str(kdl))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| PyValueError::new_err(format!("turn KDL: {err}")))?;
    let result = py
        .detach(|| nc_ai::replay(map_seed, campaign_seed, &autopilot, years, &turns))
        .map_err(PyValueError::new_err)?;
    result_dict(py, &result)
}

fn result_dict<'py>(py: Python<'py>, result: &MatchResult) -> PyResult<Bound<'py, PyDict>> {
    let out = PyDict::new(py);
    out.set_item("years", result.years)?;
    out.set_item("winner", result.winner.unwrap_or(0))?;
    out.set_item("production", result.production.clone())?;
    out.set_item("worlds", result.worlds.clone())?;
    Ok(out)
}

fn seat(name: &str) -> PyResult<Seat> {
    let (name, weights) = match name.split_once('@') {
        Some((name, style)) => (name, style_weights(style)?),
        None => (name, Weights::TUNED),
    };
    if let Some(families) = name.strip_prefix("hybrid:") {
        let mut handed = Families::NONE;
        for family in families.split(',').filter(|family| !family.is_empty()) {
            match family {
                "colonize" => handed.colonize = true,
                "scout" => handed.scout = true,
                "strike" => handed.strike = true,
                "convoy" => handed.convoy = true,
                "builds" => handed.builds = true,
                other => {
                    return Err(PyValueError::new_err(format!("unknown family {other:?}")));
                }
            }
        }
        return Ok(Seat::Hybrid(weights, handed));
    }
    Ok(match name {
        "learned" => Seat::Learned,
        "strategist" => Seat::Strategist(weights),
        "executor" => Seat::Scripted(SeatPolicy::Executor(weights)),
        "baseline" => Seat::Scripted(SeatPolicy::Executor(Weights::BASELINE)),
        "classic" => Seat::Scripted(SeatPolicy::Executor(Weights::CLASSIC)),
        "aggressor" => Seat::Scripted(SeatPolicy::Aggressor),
        "autopilot" => Seat::Scripted(SeatPolicy::Autopilot),
        style => Seat::Scripted(SeatPolicy::Executor(style_weights(style)?)),
    })
}

/// A personality's executor weights.
fn style_weights(style: &str) -> PyResult<Weights> {
    nc_ai::personality(style)
        .map(|style| style.weights)
        .ok_or_else(|| PyValueError::new_err(format!("unknown seat or personality {style:?}")))
}

/// The named int64 arrays from `dict`.
fn arrays<'py>(
    dict: &Bound<'py, PyDict>,
    names: &[&str],
) -> PyResult<Vec<PyReadonlyArrayDyn<'py, i64>>> {
    names
        .iter()
        .map(|&name| {
            dict.get_item(name)?
                .ok_or_else(|| PyKeyError::new_err(name.to_owned()))?
                .extract()
                .map_err(|err| PyValueError::new_err(format!("{name}: {err}")))
        })
        .collect()
}

fn slices<'a>(arrays: &'a [PyReadonlyArrayDyn<'_, i64>]) -> PyResult<Vec<&'a [i64]>> {
    arrays
        .iter()
        .map(|array| array.as_slice())
        .collect::<Result<_, _>>()
        .map_err(|_| PyValueError::new_err("choice arrays must be contiguous"))
}

/// Hand `tensor`'s buffer to numpy and give it its shape.
fn array<'py, T: Element>(py: Python<'py>, tensor: Tensor<T>) -> PyResult<Bound<'py, PyAny>> {
    Ok(PyArray1::from_vec(py, tensor.data)
        .reshape(tensor.shape)?
        .into_any())
}

fn put_table(out: &Bound<'_, PyDict>, name: &str, table: Table) -> PyResult<()> {
    let py = out.py();
    out.set_item(name, array(py, table.rows)?)?;
    out.set_item(format!("{name}_valid"), array(py, table.valid)?)
}

#[pymodule]
fn nc_ai_env(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Env>()?;
    m.add_function(wrap_pyfunction!(replay, m)?)?;
    m.add("TAX_CHOICES", nc_ai::TAX_CHOICES)?;
    m.add("TAX_STEP", nc_ai::TAX_STEP)?;
    m.add("POSTURES", nc_ai::POSTURES)?;
    m.add("DIPLOMACY_CHOICES", nc_ai::DIPLOMACY_CHOICES)?;
    m.add("BUILD_CHOICES", nc_ai::BUILD_CHOICES)?;
    m.add("BUILD_EXECUTOR", nc_ai::BUILD_EXECUTOR)?;
    m.add("BUILD_AMOUNT_CHOICES", nc_ai::BUILD_AMOUNT_CHOICES)?;
    m.add("BUILD_SLOTS", nc_ai::BUILD_SLOTS)?;
    m.add("MISSION_CHOICES", nc_ai::MISSION_CHOICES)?;
    m.add("SPEED_CHOICES", nc_ai::SPEED_CHOICES)?;
    m.add("ROE_CHOICES", nc_ai::ROE_CHOICES)?;
    m.add("DETACH_CHOICES", nc_ai::DETACH_CHOICES)?;
    m.add("ALLOCATIONS", nc_ai::ALLOCATIONS)?;
    m.add("WAYPOINT_CHOICES", nc_ai::WAYPOINT_CHOICES)?;
    m.add("GLOBAL_WIDTH", nc_ai::GLOBAL_WIDTH)?;
    m.add("PLANET_WIDTH", nc_ai::PLANET_WIDTH)?;
    m.add("FLEET_WIDTH", nc_ai::FLEET_WIDTH)?;
    m.add("CONTACT_WIDTH", nc_ai::CONTACT_WIDTH)?;
    m.add("STARBASE_WIDTH", nc_ai::STARBASE_WIDTH)?;
    Ok(())
}
