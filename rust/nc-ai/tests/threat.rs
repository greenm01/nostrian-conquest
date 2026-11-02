//! The threat test (t247): off by default it is the distance-and-recency
//! rule, and under `Doctrine::threat_score` it is the learned score against
//! its cutoff, on every world of a played game.

use nc_ai::{
    Doctrine, THREAT_CUTOFF, THREAT_FEATURE_NAMES, executor_turn, match_game, personality,
    threat_features, threat_score, threatened,
};

const PLAYERS: u8 = 4;

#[test]
fn the_threat_switch_picks_the_rule_or_the_learned_score() {
    let weights = personality("land-grab").expect("built in").weights;
    let mut game = match_game(7, 7, &[false; PLAYERS as usize], 40);
    let start = game.play.built.game_data.conquest.game_year;
    let rule = THREAT_FEATURE_NAMES
        .iter()
        .position(|name| *name == "rule")
        .unwrap();
    let mut scored = Doctrine::CURRENT;
    scored.enable("threat_score").unwrap();
    let (mut flagged_by_rule, mut flagged_by_score) = (0, 0);
    for _ in 0..40 {
        let game_data = &game.play.built.game_data;
        for (planet_idx, planet) in game_data.planets.iter().enumerate() {
            let owner = planet.owner.0;
            if owner == 0 || owner > PLAYERS {
                continue;
            }
            let memory = &game.seats[usize::from(owner) - 1];
            let sector = planet.sector.to_array();
            let by_rule = threatened(game_data, memory, &Doctrine::CURRENT, sector);
            let by_score = threatened(game_data, memory, &scored, sector);
            assert_eq!(
                by_rule,
                threat_features(game_data, memory, planet_idx)[rule] == 1.0
            );
            assert_eq!(
                by_score,
                threat_score(game_data, memory, planet_idx) >= THREAT_CUTOFF
            );
            flagged_by_rule += u32::from(by_rule);
            flagged_by_score += u32::from(by_score);
        }
        for empire in 1..=PLAYERS {
            executor_turn(&mut game, empire, &weights, start);
        }
        game.run_maintenance().expect("maintenance runs");
    }
    // Both tests fire in a game with wars, and the score more sparingly.
    assert!(flagged_by_score > 0 && flagged_by_score < flagged_by_rule);
}
