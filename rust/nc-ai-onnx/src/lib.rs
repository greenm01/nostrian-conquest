//! Learned AI seats for real games (t165): a fleet-level agent exported to
//! ONNX by `tools/ai/export_onnx.py`, run in Rust through tract, so a host
//! needs no Python and no GPU.
//!
//! A [`NetworkSeat`] plays one empire as the agent was trained to: the
//! network sets the seat's goals and orders the fleets of its handed-over
//! [`Families`], and the executor with the seat's [`Weights`] carries out
//! the goals for everything else, building included. The seat sees only
//! what a [`SeatMemory`] and the empire's own records hold, through the
//! same observation and masks the agent trained on.
//!
//! Each head chooses the argmax of its masked logits plus noise. Zero noise
//! plays greedy, the agent's strongest play. Gumbel noise times a
//! temperature samples at that temperature, a weaker and less predictable
//! opponent.

use std::path::Path;

use nc_ai::{
    DETACH_CHOICES, DIPLOMACY_CHOICES, Families, FleetChoice, GoalMasks, Goals, MISSION_CHOICES,
    Observation, ROE_CHOICES, SPEED_CHOICES, SeatMasks, SeatMemory, WAYPOINT_CHOICES, Weights,
    decode_action, execute_goals, fleet_family, goal_masks, observe, seat_masks,
};
use nc_data::{CoreGameData, TurnSubmission};
use tract_onnx::prelude::*;

/// Goal head widths the exported graph was built with; the diplomacy,
/// strike and colonize heads follow the game.
const TAX_CHOICES: usize = nc_ai::TAX_CHOICES;
const POSTURES: usize = nc_ai::POSTURES;
const ALLOCATIONS: usize = nc_ai::ALLOCATIONS;

type Plan = SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>;

/// A loaded agent and how its seat plays.
pub struct NetworkSeat {
    plan: Plan,
    families: Families,
    weights: Weights,
}

/// One seat's choices: its goals, and an order for each fleet row of its
/// masks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkChoices {
    pub goals: Goals,
    pub fleets: Vec<FleetChoice>,
}

impl NetworkSeat {
    /// Load an exported agent whose network orders `families`, with the
    /// executor playing `weights` for the rest.
    pub fn load(path: &Path, families: Families, weights: Weights) -> TractResult<Self> {
        let plan = tract_onnx::onnx()
            .model_for_path(path)?
            .into_optimized()?
            .into_runnable()?;
        Ok(Self {
            plan,
            families,
            weights,
        })
    }

    /// `memory`'s empire's turn, after its finished ships are commissioned
    /// (see [`nc_ai::commission_submission`]). `temperature` 0 plays greedy;
    /// `seed` draws the noise otherwise.
    pub fn turn(
        &self,
        game_data: &CoreGameData,
        memory: &SeatMemory,
        temperature: f32,
        seed: u64,
    ) -> TractResult<TurnSubmission> {
        let empire = memory.empire;
        let masks = seat_masks(game_data, empire);
        let goal_mask = goal_masks(game_data, memory, &masks);
        let observation = observe(game_data, memory);
        let chosen = self.choose(&observation, &masks, &goal_mask, temperature, seed)?;
        let mut action = execute_goals(game_data, memory, &masks, &self.weights, &chosen.goals);
        for (choice, mine) in action.fleets.iter_mut().zip(&chosen.fleets) {
            let fleet = &game_data.fleets[choice.record - 1];
            if self.families.contains(fleet_family(fleet)) {
                *choice = *mine;
            }
        }
        Ok(decode_action(game_data, empire, &action).submission)
    }

    /// Run the network on one seat's observation and masks.
    pub fn choose(
        &self,
        observation: &Observation,
        masks: &SeatMasks,
        goal_mask: &GoalMasks,
        temperature: f32,
        seed: u64,
    ) -> TractResult<NetworkChoices> {
        let worlds = observation.planets.len() / nc_ai::PLANET_WIDTH;
        let players = masks.diplomacy.len();
        let real_fleets = masks.fleets.len();
        // Every table keeps at least one row; padded rows are invalid.
        let fleets = real_fleets.max(1);
        let mut noise = Noise::new(seed, temperature);

        let table = |rows: &[f32], width: usize| -> (Tensor, Tensor) {
            let count = rows.len() / width;
            let padded = count.max(1);
            let mut data = rows.to_vec();
            data.resize(padded * width, 0.0);
            let valid: Vec<bool> = (0..padded).map(|row| row < count).collect();
            (
                tensor(&[1, padded, width], data),
                tensor(&[1, padded], valid),
            )
        };
        let (planets, planets_valid) = table(&observation.planets, nc_ai::PLANET_WIDTH);
        let (fleet_rows, fleets_valid) = table(&observation.fleets, nc_ai::FLEET_WIDTH);
        let (contacts, contacts_valid) = table(&observation.contacts, nc_ai::CONTACT_WIDTH);
        let (starbases, starbases_valid) = table(&observation.starbases, nc_ai::STARBASE_WIDTH);

        let per_fleet = |width: usize, row: &dyn Fn(usize) -> Vec<bool>| -> Tensor {
            let data: Vec<bool> = (0..fleets)
                .flat_map(|idx| {
                    if idx < real_fleets {
                        first_if_empty(row(idx))
                    } else {
                        (0..width).map(|choice| choice == 0).collect()
                    }
                })
                .collect();
            tensor(&[1, fleets, width], data)
        };
        let fleet_target: Vec<bool> = (0..fleets)
            .flat_map(|idx| {
                (0..MISSION_CHOICES).flat_map(move |mission| match masks.fleets.get(idx) {
                    Some(mask) => first_if_empty(mask.targets[mission].clone()),
                    None => (0..worlds).map(|world| world == 0).collect(),
                })
            })
            .collect();
        let inputs: TVec<TValue> = tvec![
            tensor(&[1, nc_ai::GLOBAL_WIDTH], observation.global.clone()).into(),
            planets.into(),
            planets_valid.into(),
            fleet_rows.into(),
            fleets_valid.into(),
            contacts.into(),
            contacts_valid.into(),
            starbases.into(),
            starbases_valid.into(),
            tensor(
                &[1, players, DIPLOMACY_CHOICES],
                masks
                    .diplomacy
                    .iter()
                    .flatten()
                    .copied()
                    .collect::<Vec<_>>(),
            )
            .into(),
            tensor(&[1, 1 + worlds], goal_mask.strike.clone()).into(),
            tensor(&[1, 1 + worlds], goal_mask.colonize.clone()).into(),
            per_fleet(MISSION_CHOICES, &|idx| masks.fleets[idx].mission.to_vec()).into(),
            tensor(&[1, fleets, MISSION_CHOICES, worlds], fleet_target).into(),
            per_fleet(ROE_CHOICES, &|idx| masks.fleets[idx].roe.to_vec()).into(),
            tensor(
                &[1, fleets],
                (0..fleets)
                    .map(|idx| masks.fleets.get(idx).is_some_and(|mask| mask.load))
                    .collect::<Vec<_>>(),
            )
            .into(),
            per_fleet(DETACH_CHOICES, &|idx| masks.fleets[idx].detach.to_vec()).into(),
            noise.tensor(&[1, TAX_CHOICES]).into(),
            noise.tensor(&[1, POSTURES]).into(),
            noise.tensor(&[1, ALLOCATIONS]).into(),
            noise.tensor(&[1, players, DIPLOMACY_CHOICES]).into(),
            noise.tensor(&[1, 1 + worlds]).into(),
            noise.tensor(&[1, 1 + worlds]).into(),
            noise.tensor(&[1, fleets, MISSION_CHOICES]).into(),
            noise.tensor(&[1, fleets, worlds]).into(),
            noise.tensor(&[1, fleets, WAYPOINT_CHOICES]).into(),
            noise.tensor(&[1, fleets, SPEED_CHOICES]).into(),
            noise.tensor(&[1, fleets, ROE_CHOICES]).into(),
            noise.tensor(&[1, fleets, 2]).into(),
            noise.tensor(&[1, fleets, DETACH_CHOICES]).into(),
        ];
        let outputs = self.plan.run(inputs)?;
        let ints = |idx: usize| -> TractResult<Vec<usize>> {
            Ok(outputs[idx]
                .cast_to::<i64>()?
                .as_slice::<i64>()?
                .iter()
                .map(|&value| value as usize)
                .collect())
        };
        let goals = Goals {
            tax: ints(0)?[0],
            posture: ints(1)?[0],
            allocation: ints(2)?[0],
            diplomacy: ints(3)?,
            strike: ints(4)?[0],
            colonize: ints(5)?[0],
        };
        let heads = (6..13).map(ints).collect::<TractResult<Vec<_>>>()?;
        let [mission, target, waypoint, speed, roe, load, detach] = &heads[..] else {
            unreachable!("the graph has seven fleet heads")
        };
        let fleets = masks
            .fleets
            .iter()
            .enumerate()
            .map(|(row, mask)| FleetChoice {
                record: mask.record,
                mission: mission[row],
                target: target[row],
                waypoint: waypoint[row],
                speed: speed[row],
                roe: roe[row],
                load: load[row] != 0,
                detach: detach[row],
            })
            .collect();
        Ok(NetworkChoices { goals, fleets })
    }
}

/// A mask row that allows at least its first choice, as the learner's
/// batching pads it.
fn first_if_empty(mut row: Vec<bool>) -> Vec<bool> {
    if !row.contains(&true) {
        row[0] = true;
    }
    row
}

fn tensor<T: Datum + Copy>(shape: &[usize], data: Vec<T>) -> Tensor {
    tract_ndarray::ArrayD::from_shape_vec(shape, data)
        .expect("the shape fits the data")
        .into()
}

/// Gumbel noise times a temperature, from seeded splitmix64.
struct Noise {
    state: u64,
    temperature: f32,
}

impl Noise {
    fn new(seed: u64, temperature: f32) -> Self {
        Self {
            state: seed,
            temperature,
        }
    }

    fn tensor(&mut self, shape: &[usize]) -> Tensor {
        let count = shape.iter().product();
        let data = (0..count)
            .map(|_| {
                if self.temperature == 0.0 {
                    return 0.0;
                }
                // Uniform in (0, 1), then -ln(-ln u).
                let uniform = ((self.next() >> 11) as f64 + 0.5) / (1u64 << 53) as f64;
                self.temperature * (-(-uniform.ln()).ln()) as f32
            })
            .collect();
        tensor(shape, data)
    }

    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}
