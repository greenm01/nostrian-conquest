mod common;

use std::collections::BTreeSet;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use common::production::configured_conquest;
use nc_data::{CampaignStore, DEFAULT_CAMPAIGN_DB_NAME, GameStateBuilder};

fn temp_dir(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time ok")
            .as_nanos()
    ))
}

#[test]
fn owned_since_sidecar_carries_forward_and_resets_on_owner_change() {
    let root = temp_dir("nc-data-owned-since");
    fs::create_dir_all(&root).expect("create temp dir");
    let store =
        CampaignStore::open(root.join(DEFAULT_CAMPAIGN_DB_NAME)).expect("open campaign store");

    let mut game = GameStateBuilder::new()
        .with_player_count(2)
        .build_joinable_new_game_baseline()
        .expect("joinable baseline should build");
    game.join_player(1, "Empire One")
        .expect("player one join should succeed");
    game.join_player(2, "Empire Two")
        .expect("player two join should succeed");

    store
        .save_runtime_state_structured(&game, &BTreeSet::new(), &[], &[])
        .expect("save initial snapshot");

    let initial_year = game.conquest.game_year;
    let homeworld_one = game.players[0].homeworld as usize;
    let homeworld_two = game.players[1].homeworld as usize;
    assert_eq!(
        store
            .latest_owned_planet_years_for_empire(1)
            .expect("load empire one owned years")
            .get(&homeworld_one),
        Some(&initial_year)
    );
    assert_eq!(
        store
            .latest_owned_planet_years_for_empire(2)
            .expect("load empire two owned years")
            .get(&homeworld_two),
        Some(&initial_year)
    );

    let captured_planet = game
        .planets
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner.0 == 0)
        .map(|(idx, _)| idx)
        .expect("baseline should have an unowned planet");
    game.conquest = configured_conquest(2);
    game.conquest.game_year = 3001;
    game.planets[captured_planet].owner.0 = 1;
    game.planets[captured_planet].years_under_owner = 2;

    store
        .save_runtime_state_structured(&game, &BTreeSet::new(), &[], &[])
        .expect("save second snapshot");

    let latest_empire_one = store
        .latest_owned_planet_years_for_empire(1)
        .expect("load empire one owned years after acquisition");
    assert_eq!(latest_empire_one.get(&(captured_planet + 1)), Some(&3001));
    assert_eq!(latest_empire_one.get(&homeworld_one), Some(&initial_year));

    game.conquest.game_year = 3002;
    game.planets[captured_planet].owner.0 = 2;
    game.planets[captured_planet].years_under_owner = 2;

    store
        .save_runtime_state_structured(&game, &BTreeSet::new(), &[], &[])
        .expect("save third snapshot");

    let latest_empire_one = store
        .latest_owned_planet_years_for_empire(1)
        .expect("reload empire one owned years");
    let latest_empire_two = store
        .latest_owned_planet_years_for_empire(2)
        .expect("reload empire two owned years");
    assert!(
        !latest_empire_one.contains_key(&(captured_planet + 1)),
        "former owner should no longer retain the planet in owned-since rows"
    );
    assert_eq!(latest_empire_two.get(&(captured_planet + 1)), Some(&3002));
    assert_eq!(latest_empire_two.get(&homeworld_two), Some(&initial_year));
}
