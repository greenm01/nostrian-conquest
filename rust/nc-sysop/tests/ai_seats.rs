//! `settings ai-seat` and AI seats in `maint` (t165).

use std::path::PathBuf;
use std::process::{Command, Output};

use nc_data::{AiDifficulty, CampaignStore};

fn run(args: &[&str], model: Option<&PathBuf>) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_nc-sysop"));
    cmd.args(args).env_remove("NC_AI_MODEL");
    if let Some(model) = model {
        cmd.env("NC_AI_MODEL", model);
    }
    cmd.output().expect("nc-sysop runs")
}

#[test]
fn ai_seats_play_in_maintenance_with_a_model_and_refuse_without_one() {
    let dir = std::env::temp_dir().join(format!("nc-sysop-ai-seats-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let target = dir.to_str().unwrap();
    // The small seeded network the nc-ai-onnx tests use.
    let model =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../nc-ai-onnx/tests/fixtures/tiny.onnx");

    assert!(
        run(&["new-game", target, "--players", "4", "--seed", "7"], None)
            .status
            .success()
    );
    let set = run(
        &[
            "settings",
            "ai-seat",
            "--dir",
            target,
            "--player",
            "3",
            "--difficulty",
            "normal",
        ],
        None,
    );
    assert!(
        set.status.success(),
        "{}",
        String::from_utf8_lossy(&set.stderr)
    );
    let shown = run(&["settings", "show", "--dir", target], None);
    assert!(String::from_utf8_lossy(&shown.stdout).contains("ai_seat seat=3 difficulty=normal"));
    let settings = CampaignStore::open_default_in_dir(&dir)
        .unwrap()
        .load_campaign_settings()
        .unwrap();
    assert_eq!(settings.ai_seats[0].difficulty, AiDifficulty::Normal);

    let refused = run(&["maint", target, "1"], None);
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("NC_AI_MODEL"));

    let played = run(&["maint", target, "3"], Some(&model));
    assert!(
        played.status.success(),
        "{}",
        String::from_utf8_lossy(&played.stderr)
    );
    let store = CampaignStore::open_default_in_dir(&dir).unwrap();
    let (year, memory) = store.load_ai_seat_memory(3).unwrap().expect("memory saved");
    assert_eq!(year, 3003);
    assert!(memory.contains("\"empire\":3"));
    let player = &store
        .load_latest_runtime_state()
        .unwrap()
        .unwrap()
        .game_data
        .players[2];
    assert_eq!(
        player.mode,
        nc_data::PlayerMode::Human,
        "the AI seat joined"
    );
    assert!(!player.autopilot, "the AI, not autopilot, plays the seat");

    let removed = run(
        &[
            "settings",
            "remove-ai-seat",
            "--dir",
            target,
            "--player",
            "3",
        ],
        None,
    );
    assert!(removed.status.success());
    assert!(run(&["maint", target, "1"], None).status.success());
    let _ = std::fs::remove_dir_all(&dir);
}
