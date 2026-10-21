//! A network seat in Rust makes the turns the trained agent makes in
//! Python (t165). The fixture (`tools/ai/export_onnx.py --fixture`) holds a
//! small seeded network and a game it played greedy in seat 1 against three
//! executors, recorded as turn KDL. Replaying that game, the Rust seat must
//! submit seat 1's recorded turn every year. `tiny.onnx` was exported before
//! the join and unload heads (t190), so it also shows an older graph still
//! plays; `tiny-joins.onnx` has them, drawn at random, so its fleets join.
//! `tiny-builds.onnx` also has random build heads (t207), and its seat
//! hands its builds to the network.

use std::path::Path;

use nc_ai::{Families, match_game};
use nc_ai_onnx::NetworkSeat;
use nc_data::{FleetTurnAction, PlanetTurnAction, TurnSubmission};

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// A submission that only commissions finished ships.
fn commissioning(turn: &TurnSubmission) -> bool {
    turn.fleets.is_empty()
        && turn.planets.iter().all(|planet| {
            planet
                .actions
                .iter()
                .all(|action| matches!(action, PlanetTurnAction::AutoCommission))
        })
}

const STRIKE: Families = Families {
    strike: true,
    ..Families::NONE
};

#[test]
fn the_rust_seat_makes_the_recorded_turns() {
    replay("tiny", STRIKE);
}

#[test]
fn a_seat_with_join_heads_makes_the_recorded_turns() {
    let joins = replay("tiny-joins", STRIKE);
    assert!(joins > 0, "the fixture's fleets should join");
}

#[test]
fn a_seat_that_hands_over_builds_makes_the_recorded_turns() {
    replay(
        "tiny-builds",
        Families {
            builds: true,
            ..STRIKE
        },
    );
}

/// Replay fixture `name`'s game with `families` handed over, checking seat
/// 1's turns; the joins seat 1 ordered.
fn replay(name: &str, families: Families) -> usize {
    let record: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(fixture(&format!("{name}-game.json"))).unwrap(),
    )
    .unwrap();
    let field = |name: &str| record[name].as_u64().unwrap();
    let turns: Vec<TurnSubmission> = record["turns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|turn| TurnSubmission::parse_kdl_str(turn.as_str().unwrap()).unwrap())
        .collect();
    let seat = NetworkSeat::load(
        &fixture(&format!("{name}.onnx")),
        families,
        nc_ai::personality("land-grab").unwrap(),
    )
    .expect("the fixture network loads");
    let years = field("years") as u16;
    let mut game = match_game(
        field("map_seed"),
        field("campaign_seed"),
        &[false; 4],
        years,
    );
    let mut checked = 0;
    let mut joins = 0;
    let mut next = 0;
    for _ in 0..years {
        let year = game.play.built.game_data.conquest.game_year;
        while let Some(turn) = turns.get(next).filter(|turn| turn.year == year) {
            if turn.player_record_index_1_based == 1 && !commissioning(turn) {
                let ours = seat
                    .turn(&game.play.built.game_data, &game.seats[0], 0.0, 0)
                    .expect("the network runs");
                assert_eq!(&ours, turn, "year {year}: seat 1's turn differs");
                checked += 1;
                joins += turn
                    .fleets
                    .iter()
                    .flat_map(|fleet| &fleet.actions)
                    .filter(|action| matches!(action, FleetTurnAction::Join { .. }))
                    .count();
            }
            game.submit_turn(turn).expect("a recorded turn applies");
            next += 1;
        }
        game.run_maintenance().expect("maintenance runs");
        if game.play.winner_state.winner_empire.is_some() {
            break;
        }
    }
    assert_eq!(next, turns.len(), "every recorded turn is replayed");
    assert!(checked >= 20, "seat 1 played most years: {checked}");
    joins
}
