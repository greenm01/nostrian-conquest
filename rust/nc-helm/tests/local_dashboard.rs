use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use nc_data::{CampaignStore, GameStateBuilder};
use nc_helm::{App, Effect, Route};

fn temp_game_dir() -> PathBuf {
    std::env::temp_dir().join(format!(
        "nc-helm-local-dashboard-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ))
}

#[test]
fn local_dashboard_launch_opens_hosted_route_without_boot_effects() {
    let game_dir = temp_game_dir();
    std::fs::create_dir_all(&game_dir).expect("create temp game dir");
    let store = CampaignStore::open_default_in_dir(&game_dir).expect("open campaign store");
    let game_data = GameStateBuilder::new()
        .with_player_count(4)
        .build_initialized_baseline()
        .expect("baseline game");
    store
        .save_runtime_state_structured(
            game_data.conquest.game_year,
            &game_data,
            &BTreeSet::new(),
            &[],
            &[],
        )
        .expect("save runtime state");

    let (app, effects) = App::new_local_dashboard(&game_dir).expect("build local dashboard app");

    assert!(effects.is_empty());
    assert!(matches!(app.model().route, Route::HostedGame(_)));
    assert_eq!(app.model().relay_url, "ws://localhost:8080");
    assert!(!matches!(effects.as_slice(), [Effect::LoadBoot]));

    std::fs::remove_dir_all(&game_dir).expect("cleanup temp game dir");
}
