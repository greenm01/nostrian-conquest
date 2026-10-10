//! Policies: how an empire spends choices on its turn.

mod actions;
mod adversarial;
mod view;

use nc_data::{CoreGameData, TurnSubmission};

use crate::choice::ChoiceSource;
use crate::turn::{TurnAction, TurnBuilder};
use actions::{Aim, Choices};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Policy {
    /// Any action the game accepts, chosen uniformly.
    RandomValid,
    /// Declares war, builds warships and troops, and attacks foreign worlds.
    Aggressor,
    /// Constantly splits, joins, and gathers fleets, churning fleet records.
    Swarm,
    /// Boundary and out-of-range values; the submission path rejects most.
    Adversarial,
    /// Submits nothing; the empire runs on the engine's autopilot AI.
    Autopilot,
}

impl Policy {
    pub const ALL: [Policy; 5] = [
        Policy::RandomValid,
        Policy::Aggressor,
        Policy::Swarm,
        Policy::Adversarial,
        Policy::Autopilot,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Policy::RandomValid => "random",
            Policy::Aggressor => "aggressor",
            Policy::Swarm => "swarm",
            Policy::Adversarial => "adversarial",
            Policy::Autopilot => "autopilot",
        }
    }
}

const MAX_ACTIONS_PER_TURN: u32 = 6;

/// Build-unit numbers (see `nc_engine::BUILD_UNITS`).
const ALL_UNITS: [u8; 9] = [1, 2, 3, 4, 5, 6, 7, 9, 10];
const WAR_UNITS: [u8; 5] = [1, 2, 3, 5, 9];
const SWARM_UNITS: [u8; 2] = [1, 4];

/// One empire's finished turn and any actions whose turn KDL did not parse
/// back to the same submission.
pub struct PlannedTurn {
    pub submission: TurnSubmission,
    pub lossy_round_trips: Vec<String>,
}

pub fn plan_turn(
    policy: Policy,
    state: &CoreGameData,
    empire: u8,
    choices: &mut dyn ChoiceSource,
) -> PlannedTurn {
    let mut builder = TurnBuilder::new(state, empire);
    if policy != Policy::Autopilot {
        for _ in 0..choices.pick(0, MAX_ACTIONS_PER_TURN) {
            if let Some(action) = propose(policy, builder.state(), empire, choices) {
                builder.try_add(action);
            }
            if builder.panicked {
                break;
            }
        }
    }
    let (submission, lossy_round_trips) = builder.finish();
    PlannedTurn {
        submission,
        lossy_round_trips,
    }
}

fn propose(
    policy: Policy,
    state: &CoreGameData,
    empire: u8,
    choices: Choices,
) -> Option<TurnAction> {
    match policy {
        Policy::RandomValid => random_valid(state, empire, choices),
        Policy::Aggressor => aggressor(state, empire, choices),
        Policy::Swarm => swarm(state, empire, choices),
        Policy::Adversarial => adversarial::edge_action(state, empire, choices),
        Policy::Autopilot => None,
    }
}

fn random_valid(state: &CoreGameData, empire: u8, choices: Choices) -> Option<TurnAction> {
    match choices.pick(0, 12) {
        0 => actions::order_fleet(state, empire, choices, Aim::Anywhere),
        1 => actions::build(state, empire, choices, &ALL_UNITS),
        2 => actions::commission(state, empire, choices),
        3 => actions::auto_commission(state, empire, choices),
        4 => actions::detach(state, empire, choices),
        5 => actions::join_fleet(state, empire, choices),
        6 => actions::transfer(state, empire, choices),
        7 => actions::load_armies(state, empire, choices),
        8 => actions::unload_armies(state, empire, choices),
        9 => actions::set_roe(state, empire, choices),
        10 => actions::tax(choices),
        11 => actions::diplomacy(state, empire, choices, false),
        _ => actions::scorch(state, empire, choices),
    }
}

fn aggressor(state: &CoreGameData, empire: u8, choices: Choices) -> Option<TurnAction> {
    match choices.pick(0, 8) {
        0..=2 => actions::order_fleet(state, empire, choices, Aim::Attack),
        3 => actions::diplomacy(state, empire, choices, true),
        4 => actions::build(state, empire, choices, &WAR_UNITS),
        5 => actions::auto_commission(state, empire, choices),
        6 => actions::load_armies(state, empire, choices),
        7 => actions::set_roe(state, empire, choices),
        _ => actions::order_fleet(state, empire, choices, Aim::Anywhere),
    }
}

fn swarm(state: &CoreGameData, empire: u8, choices: Choices) -> Option<TurnAction> {
    match choices.pick(0, 5) {
        0 => actions::detach_one(state, empire, choices),
        1 => actions::join_fleet(state, empire, choices),
        2 => actions::order_fleet(state, empire, choices, Aim::Gather),
        3 => actions::build(state, empire, choices, &SWARM_UNITS),
        4 => actions::auto_commission(state, empire, choices),
        _ => actions::transfer(state, empire, choices),
    }
}
