//! Hybrid seats (t157): a strategist whose network also orders the fleets
//! of handed-over families. Given the executor's own fleet choices it plays
//! exactly as a strategist; fleet choices outside the masks are refused.

#[allow(dead_code)]
mod common;

use common::arrays::tuned_without_stations;
use nc_ai::{BatchActions, BatchEnv, Families, FleetArrays, Seat, Weights};

const PLAYERS: u8 = 4;

/// Batch actions whose fleet fields come from `fleets`; the other fields
/// are ignored for hybrid seats and left zero.
struct Actions {
    zeros_seat: Vec<i64>,
    zeros_diplomacy: Vec<i64>,
    zeros_world: Vec<i64>,
    fleets: FleetArrays,
}

impl Actions {
    fn new(env: &BatchEnv, fleets: FleetArrays) -> Self {
        let seats = env.games() * usize::from(PLAYERS);
        Actions {
            zeros_seat: vec![0; seats],
            zeros_diplomacy: vec![0; seats * usize::from(PLAYERS)],
            zeros_world: vec![0; seats * env.worlds() * nc_ai::BUILD_SLOTS],
            fleets,
        }
    }

    fn view(&self) -> BatchActions<'_> {
        BatchActions {
            tax: &self.zeros_seat,
            diplomacy: &self.zeros_diplomacy,
            build: &self.zeros_world,
            amount: &self.zeros_world,
            mission: &self.fleets.mission.data,
            target: &self.fleets.target.data,
            waypoint: &self.fleets.waypoint.data,
            speed: &self.fleets.speed.data,
            roe: &self.fleets.roe.data,
            load: &self.fleets.load.data,
            detach: &self.fleets.detach.data,
            join: &self.fleets.join.data,
            unload: &self.fleets.unload.data,
        }
    }
}

fn lineup(seat: Seat) -> Vec<Seat> {
    vec![
        seat,
        Seat::Strategist(tuned_without_stations()),
        Seat::Scripted(nc_ai::SeatPolicy::Executor(Weights::TUNED)),
        Seat::Scripted(nc_ai::SeatPolicy::Aggressor),
    ]
}

#[test]
fn a_hybrid_given_the_executors_fleet_orders_plays_as_a_strategist() {
    let maps = [101, 102];
    let years = 40;
    let mut hybrid = BatchEnv::new(
        PLAYERS,
        &maps,
        &maps,
        &[
            lineup(Seat::Hybrid(tuned_without_stations(), Families::ALL)),
            lineup(Seat::Hybrid(tuned_without_stations(), Families::ALL)),
        ],
        years,
        0,
    );
    let mut plain = BatchEnv::new(
        PLAYERS,
        &maps,
        &maps,
        &[
            lineup(Seat::Strategist(tuned_without_stations())),
            lineup(Seat::Strategist(tuned_without_stations())),
        ],
        years,
        0,
    );
    assert_eq!(hybrid.hybrid().data[..4], [true, false, false, false]);
    assert_eq!(hybrid.strategist().data[..4], [true, true, false, false]);
    let mut families = [0usize; 4];
    loop {
        let goals = hybrid.goal_labels();
        let fleets = hybrid
            .fleet_labels(&goals.view())
            .expect("labels are allowed goals");
        for &family in fleets.family.data.iter().filter(|&&f| f >= 0) {
            families[family as usize] += 1;
        }
        let actions = Actions::new(&hybrid, fleets);
        let left = hybrid
            .step(Some(&actions.view()), Some(&goals.view()))
            .expect("the executor's fleet orders are allowed");
        let right = plain
            .step(None, Some(&plain.goal_labels().view()))
            .expect("labels are allowed goals");
        assert_eq!(left, right);
        if left.done.data.iter().all(|&done| done) {
            break;
        }
    }
    assert!(families.iter().all(|&count| count > 0), "{families:?}");
}

#[test]
fn hybrid_fleet_choices_outside_the_masks_are_refused() {
    let mut env = BatchEnv::new(
        PLAYERS,
        &[103],
        &[103],
        &[lineup(Seat::Hybrid(Weights::TUNED, Families::NONE))],
        10,
        0,
    );
    let goals = env.goal_labels();
    assert!(
        env.step(None, Some(&goals.view())).is_err(),
        "needs actions"
    );
    let masks = env.masks();
    let mut fleets = env.fleet_labels(&goals.view()).expect("labels");
    let disallowed = masks.mission.data[..env.fleets() * nc_ai::MISSION_CHOICES]
        .iter()
        .position(|&allowed| !allowed)
        .expect("some mission is not allowed for the hybrid seat");
    fleets.mission.data[disallowed / nc_ai::MISSION_CHOICES] =
        (disallowed % nc_ai::MISSION_CHOICES) as i64;
    let actions = Actions::new(&env, fleets);
    let err = env
        .step(Some(&actions.view()), Some(&goals.view()))
        .unwrap_err();
    assert!(err.contains("mission"), "{err}");
}

#[test]
fn unhanded_families_stay_with_the_executor() {
    // With no family handed over, any allowed fleet choice is ignored: the
    // hybrid plays as a strategist even when its network keeps every fleet.
    let maps = [104];
    let years = 25;
    let mut hybrid = BatchEnv::new(
        PLAYERS,
        &maps,
        &maps,
        &[lineup(Seat::Hybrid(Weights::TUNED, Families::NONE))],
        years,
        0,
    );
    let mut plain = BatchEnv::new(
        PLAYERS,
        &maps,
        &maps,
        &[lineup(Seat::Strategist(Weights::TUNED))],
        years,
        0,
    );
    loop {
        let goals = hybrid.goal_labels();
        let mut fleets = hybrid.fleet_labels(&goals.view()).expect("labels");
        // "Keep orders" is always allowed: mission 0, ROE 0, no load.
        for value in fleets
            .mission
            .data
            .iter_mut()
            .chain(fleets.roe.data.iter_mut())
            .chain(fleets.load.data.iter_mut())
        {
            *value = 0;
        }
        let actions = Actions::new(&hybrid, fleets);
        let left = hybrid
            .step(Some(&actions.view()), Some(&goals.view()))
            .expect("keeping orders is allowed");
        let right = plain
            .step(None, Some(&plain.goal_labels().view()))
            .expect("labels");
        assert_eq!(left, right);
        if left.done.data[0] {
            break;
        }
    }
}
