//! Reading one seat's choices out of the batch arrays, checked against its
//! masks.

use crate::action::{
    BUILD_AMOUNT_CHOICES, BUILD_SLOTS, FleetChoice, PlanetChoice, SPEED_CHOICES, SeatAction,
    SeatMasks, TAX_CHOICES,
};

use crate::goals::{GoalMasks, Goals, POSTURES};

use super::{BatchActions, BatchGoals};

/// One seat's [`SeatAction`] from the batch arrays, checked against its
/// masks. `at` is the flat `[game, seat]` index.
pub(crate) fn seat_action(
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
            let world = (at * worlds + mask.record - 1) * BUILD_SLOTS;
            let mut build = [0; BUILD_SLOTS];
            let mut amount = [0; BUILD_SLOTS];
            for slot in 0..BUILD_SLOTS {
                build[slot] = choice("build", actions.build[world + slot], &mask.build)?;
                amount[slot] = choice(
                    "amount",
                    actions.amount[world + slot],
                    &[true; BUILD_AMOUNT_CHOICES],
                )?;
            }
            Ok(PlanetChoice {
                record: mask.record,
                build,
                amount,
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

pub(crate) fn choice(name: &str, value: i64, mask: &[bool]) -> Result<usize, String> {
    usize::try_from(value)
        .ok()
        .filter(|&idx| mask.get(idx).copied().unwrap_or(false))
        .ok_or_else(|| format!("{name} choice {value} is not allowed"))
}

/// One strategist seat's [`Goals`] from the batch arrays, checked against
/// its goal masks. `at` is the flat `[game, seat]` index.
pub(crate) fn seat_goals(
    masks: &GoalMasks,
    goals: &BatchGoals,
    at: usize,
    seats: usize,
) -> Result<Goals, String> {
    Ok(Goals {
        tax: choice("tax goal", goals.tax[at], &[true; TAX_CHOICES])?,
        diplomacy: masks
            .diplomacy
            .iter()
            .enumerate()
            .map(|(other, mask)| {
                choice("diplomacy goal", goals.diplomacy[at * seats + other], mask)
            })
            .collect::<Result<_, _>>()?,
        posture: choice("posture", goals.posture[at], &[true; POSTURES])?,
        strike: choice("strike", goals.strike[at], &masks.strike)?,
        colonize: choice("colonize", goals.colonize[at], &masks.colonize)?,
    })
}
