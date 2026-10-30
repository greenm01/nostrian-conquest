//! Stored positions: a played game's saved years load back as the
//! positions it stood at, and play on from there as the game did.

#[allow(dead_code)]
mod common;

use common::seated_game;
use nc_ai::{BatchEnv, Seat, SeatPolicy, Weights, executor_turn, load_position};
use nc_data::{CampaignStore, CoreGameData};

const PLAYERS: u8 = 4;
const MAP: u64 = 31;

/// A game of executors played for `years` years and saved as a door game
/// is: each year's snapshot holds the orders given that year. Returns each
/// year's saved state and the state maintenance left after it.
fn play_and_store(store: &CampaignStore, years: u16) -> Vec<(CoreGameData, CoreGameData)> {
    let mut game = seated_game(PLAYERS, MAP);
    let start_year = game.play.built.game_data.conquest.game_year;
    let mut played = Vec::new();
    for _ in 0..years {
        for empire in 1..=PLAYERS {
            executor_turn(&mut game, empire, &Weights::TUNED, start_year);
        }
        let saved = game.play.built.game_data.clone();
        let intel: Vec<_> = game
            .seats
            .iter()
            .map(|seat| seat.planet_intel.clone())
            .collect();
        store
            .save_year_with_intel_and_seed(
                saved.conquest.game_year,
                &saved,
                &[],
                &[],
                &intel,
                Some(MAP),
            )
            .unwrap();
        game.run_maintenance().unwrap();
        played.push((saved, game.play.built.game_data.clone()));
    }
    played
}

fn temp_store(name: &str) -> CampaignStore {
    let dir = std::env::temp_dir().join(format!("nc-ai-stored-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    CampaignStore::open_default_in_dir(&dir).unwrap()
}

#[test]
fn a_stored_year_loads_back_and_plays_on_as_the_game_did() {
    let store = temp_store("replay");
    let played = play_and_store(&store, 6);
    let start_year = played[0].0.conquest.game_year;
    let (saved, after) = &played[3];

    let position = load_position(store.path(), saved.conquest.game_year).unwrap();
    assert_eq!(&position.game_data, saved);
    assert_eq!(position.year(), start_year + 3);
    assert_eq!(position.start_year, start_year);
    assert_eq!(position.campaign_seed, MAP);
    assert_eq!(position.seats.len(), usize::from(PLAYERS));

    // The year's orders are in the snapshot, so maintenance alone plays
    // the year the game played.
    let mut game = position.seated_game();
    game.run_maintenance().unwrap();
    assert_eq!(&game.play.built.game_data, after);

    assert!(load_position(store.path(), start_year + 40).is_err());
}

#[test]
fn a_batch_started_from_a_stored_year_counts_the_years_already_played() {
    let store = temp_store("batch");
    let played = play_and_store(&store, 4);
    let position = load_position(store.path(), played[2].0.conquest.game_year).unwrap();
    let executors = vec![vec![Seat::Scripted(SeatPolicy::Executor(Weights::TUNED)); 4]; 2];
    let years = 5;
    let mut env = BatchEnv::with_starts(
        PLAYERS,
        &[MAP, MAP + 1],
        &[MAP, MAP + 1],
        &executors,
        &[Some(position.clone()), None],
        years,
        0,
    );
    assert_eq!(&env.game(0).play.built.game_data, &position.game_data);
    // Two years were played before the stored year, so three remain.
    for year in 1..=3 {
        let step = env.step(None, None).unwrap();
        assert_eq!(step.done.data[0], year == 3, "year {year}");
        assert!(!step.done.data[1]);
    }
}
