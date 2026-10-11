use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use nc_compat::load_classic_core_files;
use nc_data::{
    CampaignRuntimeState, CampaignStore, Coords, CoreGameData, GameStateBuilder, IntelTier, Order,
    ReportBlockRow, recompute_max_speed, set_armies_aboard, set_troop_transports,
};
use nc_engine::validate_maintenance_state;

static TEMP_DIR_SEQ: AtomicU64 = AtomicU64::new(0);

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "{prefix}-{}-{}-{}",
        std::process::id(),
        TEMP_DIR_SEQ.fetch_add(1, Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time ok")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("create temp dir");
    root
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir should have rust workspace parent")
        .parent()
        .expect("rust workspace should have repo root parent")
        .to_path_buf()
}

fn load_fixture(name: &str) -> CoreGameData {
    let fixture_dir = repo_root().join("fixtures").join(name).join("v1.5");
    load_classic_core_files(&fixture_dir).expect("fixture should load")
}

fn join_player(
    game_data: &mut CoreGameData,
    player: usize,
    empire_name: &str,
    homeworld_name: &str,
) {
    game_data
        .join_player(player, empire_name)
        .expect("join player");
    game_data
        .rename_player_homeworld(player, homeworld_name)
        .expect("rename homeworld");
}

fn hold_all_fleets_in_place(game_data: &mut CoreGameData) {
    for fleet in &mut game_data.fleets {
        let coords = fleet.sector.to_array();
        fleet.order = Order::GuardBlockadeWorld;
        fleet.target = Coords::from_array(coords);
    }
}

fn seed_runtime_snapshot(dir: &Path, game_data: &CoreGameData) {
    let store = CampaignStore::open_default_in_dir(dir).expect("open campaign store");
    store
        .save_runtime_state_structured(game_data, &BTreeSet::new(), &[], &[])
        .expect("seed runtime snapshot");
}

fn run_nc_sysop(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_nc-sysop"))
        .args(args)
        .output()
        .expect("nc-sysop should run");
    assert!(
        output.status.success(),
        "nc-sysop failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("stdout should be utf-8")
}

fn load_runtime_state(dir: &Path) -> CampaignRuntimeState {
    CampaignStore::open_default_in_dir(dir)
        .expect("open campaign store")
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("runtime snapshot should exist")
}

fn first_owned_planet_coords(game_data: &CoreGameData, owner: u8) -> [u8; 2] {
    game_data
        .planets
        .iter()
        .find(|planet| planet.owner.0 == owner)
        .map(|planet| planet.sector.to_array())
        .expect("owned planet should exist")
}

fn first_unowned_planet_coords(game_data: &CoreGameData, excluded: &[[u8; 2]]) -> [u8; 2] {
    game_data
        .planets
        .iter()
        .map(|planet| planet.sector.to_array())
        .find(|coords| {
            !excluded.contains(coords)
                && game_data
                    .planets
                    .iter()
                    .any(|planet| planet.sector.to_array() == *coords && planet.owner.0 == 0)
        })
        .expect("unowned planet should exist")
}

fn assert_runtime_playability_invariants(runtime: &CampaignRuntimeState) {
    validate_maintenance_state(&runtime.game_data).expect("runtime state should remain valid");
    for fleet in &runtime.game_data.fleets {
        assert!(
            fleet.speed <= fleet.max_speed,
            "fleet speed should never exceed max speed"
        );
        if matches!(
            fleet.order,
            Order::BombardWorld | Order::InvadeWorld | Order::BlitzWorld
        ) && fleet.sector != fleet.target
        {
            assert_ne!(
                fleet.depth.to_bytes()[0],
                0x80,
                "off-target hostile orders must not sit in ready-to-execute state"
            );
        }
    }
}

#[test]
fn nc_sysop_maint_persists_view_world_reports_and_runtime_intel() {
    let target = unique_temp_dir("nc-sysop-maint-view-world");
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let viewer = &mut game_data.fleets[0];
    viewer.order = Order::ViewWorld;
    viewer.target = Coords::new(15, 13);
    viewer.ships.scouts = 0;
    viewer.ships.etacs = 0;
    // Cruiser speed settles in orbit one sector away within the year.
    recompute_max_speed(viewer);
    viewer.speed = viewer.max_speed;
    seed_runtime_snapshot(&target, &game_data);

    let stdout = run_nc_sysop(&["maint", target.to_str().expect("utf-8 path"), "1"]);
    assert!(stdout.contains("Rust maintenance complete."));

    let store = CampaignStore::open_default_in_dir(&target).expect("open campaign store");
    let runtime = store
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("runtime snapshot should exist");
    assert!(
        runtime
            .report_block_rows
            .iter()
            .any(|row| row.decoded_text.contains("Viewing mission report"))
    );

    let intel = store
        .latest_planet_intel_for_viewer(1)
        .expect("load viewer intel");
    let target_world = &runtime.game_data.planets[13];
    let snapshot = intel
        .iter()
        .find(|snapshot| snapshot.planet_record_index_1_based == 14)
        .expect("viewed planet intel should be stored");
    assert_eq!(snapshot.intel_tier, IntelTier::Partial);
    assert_eq!(
        snapshot.known_name.as_deref(),
        Some(target_world.name.as_str())
    );
    assert_eq!(snapshot.known_owner_empire_id, Some(target_world.owner.0));
    assert_eq!(
        snapshot.known_potential_production,
        Some(u16::from(target_world.potential_production))
    );
    assert_eq!(snapshot.known_armies, None);
    assert_eq!(snapshot.known_ground_batteries, None);

    let _ = fs::remove_dir_all(&target);
}

/// Reports wait until the player deletes them: a year's maintenance keeps
/// last year's unread reports ahead of its own and drops the deleted ones.
#[test]
fn nc_sysop_maint_keeps_unread_reports_from_earlier_years() {
    let target = unique_temp_dir("nc-sysop-maint-unread-reports");
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let viewer = &mut game_data.fleets[0];
    viewer.order = Order::ViewWorld;
    viewer.target = Coords::new(15, 13);
    viewer.ships.scouts = 0;
    viewer.ships.etacs = 0;
    recompute_max_speed(viewer);
    viewer.speed = viewer.max_speed;
    let last_year = |block_index, text: &str, deleted| ReportBlockRow {
        viewer_empire_id: 1,
        block_index,
        decoded_text: text.to_string(),
        raw_bytes: None,
        recipient_deleted: deleted,
    };
    CampaignStore::open_default_in_dir(&target)
        .expect("open campaign store")
        .save_runtime_state_structured(
            &game_data,
            &BTreeSet::new(),
            &[
                last_year(0, "Read last year", true),
                last_year(1, "Unread last year", false),
            ],
            &[],
        )
        .expect("seed runtime snapshot");

    run_nc_sysop(&["maint", target.to_str().expect("utf-8 path"), "1"]);

    let runtime = load_runtime_state(&target);
    let viewer_one = runtime
        .report_block_rows
        .iter()
        .filter(|row| row.viewer_empire_id == 1)
        .map(|row| (row.block_index, row.decoded_text.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(viewer_one.first(), Some(&(0, "Unread last year")));
    assert!(
        viewer_one.iter().all(|(_, text)| *text != "Read last year"),
        "{viewer_one:?}"
    );
    assert!(
        viewer_one
            .iter()
            .any(|(index, text)| *index > 0 && text.contains("Viewing mission report")),
        "this year's reports should follow: {viewer_one:?}"
    );

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn nc_sysop_maint_persists_scout_system_reports_and_runtime_intel() {
    let target = unique_temp_dir("nc-sysop-maint-scout-system");
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let scout = &mut game_data.fleets[0];
    scout.order = Order::ScoutSolarSystem;
    scout.target = Coords::new(15, 13);
    scout.ships.scouts = 1;
    scout.ships.etacs = 0;
    // Cruiser speed settles in orbit one sector away within the year.
    recompute_max_speed(scout);
    scout.speed = scout.max_speed;
    game_data.planets[13].stardock.destroyers = 2;
    seed_runtime_snapshot(&target, &game_data);

    let stdout = run_nc_sysop(&["maint", target.to_str().expect("utf-8 path"), "1"]);
    assert!(stdout.contains("Rust maintenance complete."));

    let store = CampaignStore::open_default_in_dir(&target).expect("open campaign store");
    let runtime = store
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("runtime snapshot should exist");
    assert!(
        runtime
            .report_block_rows
            .iter()
            .any(|row| row.decoded_text.contains("Scouting mission report"))
    );

    let intel = store
        .latest_planet_intel_for_viewer(1)
        .expect("load viewer intel");
    let target_world = &runtime.game_data.planets[13];
    let snapshot = intel
        .iter()
        .find(|snapshot| snapshot.planet_record_index_1_based == 14)
        .expect("scouted planet intel should be stored");
    assert_eq!(snapshot.intel_tier, IntelTier::Full);
    assert_eq!(snapshot.known_owner_empire_id, Some(target_world.owner.0));
    assert_eq!(
        snapshot.known_potential_production,
        Some(u16::from(target_world.potential_production))
    );
    assert_eq!(
        snapshot.known_armies,
        Some(u8::try_from(target_world.armies).unwrap_or(u8::MAX))
    );
    assert_eq!(
        snapshot.known_ground_batteries,
        Some(u8::try_from(target_world.ground_batteries).unwrap_or(u8::MAX))
    );
    assert_eq!(snapshot.known_starbase_count, Some(0));
    assert_eq!(
        snapshot.known_docked_summary.as_deref(),
        Some("2 destroyers")
    );

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn nc_sysop_maint_persists_non_intel_player_reports() {
    let target = unique_temp_dir("nc-sysop-maint-colonize-report");
    let game_data = load_fixture("ecmaint-fleet-pre");
    seed_runtime_snapshot(&target, &game_data);

    let stdout = run_nc_sysop(&["maint", target.to_str().expect("utf-8 path"), "1"]);
    assert!(stdout.contains("Rust maintenance complete."));

    let store = CampaignStore::open_default_in_dir(&target).expect("open campaign store");
    let runtime = store
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("runtime snapshot should exist");
    assert!(runtime.report_block_rows.iter().any(|row| {
        row.decoded_text.contains("successfully terraformed")
            && row.decoded_text.contains("started a new colony")
    }));

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn nc_sysop_maint_persists_bombardment_report_and_runtime_damage() {
    let target = unique_temp_dir("nc-sysop-maint-bombard-runtime");
    let pre = load_fixture("ecmaint-bombard-arrive");
    let target_coords = pre
        .fleets
        .iter()
        .find(|fleet| fleet.order == Order::BombardWorld)
        .map(|fleet| fleet.target.to_array())
        .expect("bombard fixture should contain a bombardment fleet");
    let pre_target = pre
        .planets
        .iter()
        .find(|planet| planet.sector.to_array() == target_coords)
        .expect("bombard target world should exist");
    let pre_batteries = u8::try_from(pre_target.ground_batteries).unwrap_or(u8::MAX);
    let pre_armies = u8::try_from(pre_target.armies).unwrap_or(u8::MAX);
    seed_runtime_snapshot(&target, &pre);

    let stdout = run_nc_sysop(&["maint", target.to_str().expect("utf-8 path"), "1"]);
    assert!(stdout.contains("Rust maintenance complete."));

    let runtime = load_runtime_state(&target);
    let post_target = runtime
        .game_data
        .planets
        .iter()
        .find(|planet| planet.sector.to_array() == target_coords)
        .expect("bombard target world should remain present");

    assert!(runtime.report_block_rows.iter().any(|row| {
        row.decoded_text
            .contains("ALERT: Orbital bombardment underway!")
    }));
    assert!(
        runtime
            .report_block_rows
            .iter()
            .any(|row| row.decoded_text.contains("Our forces:"))
    );
    assert!(
        runtime
            .report_block_rows
            .iter()
            .any(|row| row.decoded_text.contains("World defenses:"))
    );
    assert!(
        u8::try_from(post_target.ground_batteries).unwrap_or(u8::MAX) < pre_batteries
            || u8::try_from(post_target.armies).unwrap_or(u8::MAX) < pre_armies,
        "bombardment should damage the target world"
    );
    assert_runtime_playability_invariants(&runtime);

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn nc_sysop_maint_persists_join_host_destroyed_report_and_runtime_state() {
    let target = unique_temp_dir("nc-sysop-maint-join-host-destroyed");
    let mut game_data = load_fixture("ecmaint-post");

    let host_number = game_data.fleets[0].fleet_number.0;
    game_data.fleets[0].ships.destroyers = 0;
    game_data.fleets[0].ships.cruisers = 0;
    game_data.fleets[0].ships.battleships = 0;
    game_data.fleets[0].ships.scouts = 0;
    set_troop_transports(&mut game_data.fleets[0], 0);
    game_data.fleets[0].ships.etacs = 0;

    let joiner = &mut game_data.fleets[1];
    joiner.sector = Coords::new(7, 9);
    joiner.order = Order::JoinAnotherFleet;
    joiner.mission_target = host_number;
    joiner.target = Coords::new(10, 10);
    joiner.speed = 3;

    seed_runtime_snapshot(&target, &game_data);

    let stdout = run_nc_sysop(&["maint", target.to_str().expect("utf-8 path"), "1"]);
    assert!(stdout.contains("Rust maintenance complete."));

    let runtime = load_runtime_state(&target);
    let joiner = &runtime.game_data.fleets[0];

    assert!(runtime.report_block_rows.iter().any(|row| {
        row.decoded_text.contains("Join mission summary")
            && row
                .decoded_text
                .contains("Lost hosts: Fleet 2 lost host Fleet 1 and is holding position.")
    }));
    assert_eq!(joiner.order, Order::HoldPosition);
    assert_eq!(joiner.speed, 0);
    assert_eq!(joiner.target.to_array(), joiner.sector.to_array());
    assert_runtime_playability_invariants(&runtime);

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn nc_sysop_maint_multi_turn_canary_preserves_playability_invariants() {
    let target = unique_temp_dir("nc-sysop-maint-multi-turn-canary");
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    for fleet in &mut game_data.fleets {
        let coords = fleet.sector.to_array();
        fleet.order = Order::HoldPosition;
        fleet.target = Coords::from_array(coords);
        fleet.speed = 0;
    }

    let p1_home = first_owned_planet_coords(&game_data, 1);
    let p2_home = first_owned_planet_coords(&game_data, 2);
    let colonize_target = first_unowned_planet_coords(&game_data, &[p1_home, p2_home]);

    let player1_fleets = game_data
        .fleets
        .iter()
        .enumerate()
        .filter(|(_, fleet)| fleet.owner.0 == 1)
        .map(|(idx, _)| idx)
        .collect::<Vec<_>>();
    let player2_fleets = game_data
        .fleets
        .iter()
        .enumerate()
        .filter(|(_, fleet)| fleet.owner.0 == 2)
        .map(|(idx, _)| idx)
        .collect::<Vec<_>>();
    let player3_fleets = game_data
        .fleets
        .iter()
        .enumerate()
        .filter(|(_, fleet)| fleet.owner.0 == 3)
        .map(|(idx, _)| idx)
        .collect::<Vec<_>>();
    let player4_fleets = game_data
        .fleets
        .iter()
        .enumerate()
        .filter(|(_, fleet)| fleet.owner.0 == 4)
        .map(|(idx, _)| idx)
        .collect::<Vec<_>>();

    {
        let viewer = &mut game_data.fleets[player1_fleets[0]];
        viewer.ships.cruisers = 1;
        viewer.ships.destroyers = 0;
        viewer.ships.battleships = 0;
        set_troop_transports(viewer, 0);
        set_armies_aboard(viewer, 0);
        viewer.ships.etacs = 0;
        viewer.ships.scouts = 0;
        recompute_max_speed(viewer);
        viewer.order = Order::ViewWorld;
        viewer.target = Coords::from_array(p2_home);
        viewer.speed = viewer.max_speed;
        viewer.speed = viewer.max_speed;
    }
    let host_number = game_data.fleets[player4_fleets[0]].fleet_number.0;
    {
        let host = &mut game_data.fleets[player4_fleets[0]];
        host.ships.cruisers = 1;
        host.ships.destroyers = 0;
        host.ships.battleships = 0;
        set_troop_transports(host, 0);
        set_armies_aboard(host, 0);
        host.ships.etacs = 0;
        host.ships.scouts = 0;
        recompute_max_speed(host);
        host.sector = Coords::new(4, 4);
        host.order = Order::MoveOnly;
        host.target = Coords::new(8, 4);
        host.speed = host.max_speed;
        host.speed = host.max_speed;
    }
    {
        let joiner = &mut game_data.fleets[player4_fleets[1]];
        joiner.ships.cruisers = 1;
        joiner.ships.destroyers = 0;
        joiner.ships.battleships = 0;
        set_troop_transports(joiner, 0);
        set_armies_aboard(joiner, 0);
        joiner.ships.etacs = 0;
        joiner.ships.scouts = 0;
        recompute_max_speed(joiner);
        joiner.sector = Coords::new(1, 4);
        joiner.order = Order::JoinAnotherFleet;
        joiner.target = Coords::new(4, 4);
        joiner.mission_target = host_number;
        joiner.speed = joiner.max_speed;
        joiner.speed = joiner.max_speed;
    }
    {
        let bombard = &mut game_data.fleets[player2_fleets[0]];
        bombard.ships.destroyers = 1;
        bombard.ships.cruisers = 0;
        bombard.ships.battleships = 0;
        set_troop_transports(bombard, 0);
        set_armies_aboard(bombard, 0);
        bombard.ships.etacs = 0;
        bombard.ships.scouts = 0;
        recompute_max_speed(bombard);
        bombard.sector = Coords::new(p1_home[0].saturating_sub(1).max(1), p1_home[1]);
        bombard.order = Order::BombardWorld;
        bombard.target = Coords::from_array(p1_home);
        bombard.speed = bombard.max_speed;
        bombard.speed = bombard.max_speed;
    }
    {
        let colonizer = &mut game_data.fleets[player3_fleets[0]];
        colonizer.ships.etacs = 3;
        colonizer.ships.cruisers = 0;
        colonizer.ships.destroyers = 0;
        colonizer.ships.battleships = 0;
        set_troop_transports(colonizer, 0);
        set_armies_aboard(colonizer, 0);
        colonizer.ships.scouts = 0;
        recompute_max_speed(colonizer);
        let colonizer_start = if colonize_target[0] > 1 {
            [colonize_target[0] - 1, colonize_target[1]]
        } else {
            [colonize_target[0] + 1, colonize_target[1]]
        };
        colonizer.sector = Coords::from_array(colonizer_start);
        colonizer.order = Order::ColonizeWorld;
        colonizer.target = Coords::from_array(colonize_target);
        colonizer.speed = colonizer.max_speed;
        colonizer.speed = colonizer.max_speed;
    }

    // The home guards of empires 1 and 2 hold fire (ROE 0), so the viewer
    // and the bomber reach orbit whatever the sighting rolls.
    for &idx in player1_fleets[1..].iter().chain(player2_fleets[1..].iter()) {
        game_data.fleets[idx].rules_of_engagement = 0;
    }

    seed_runtime_snapshot(&target, &game_data);

    let mut saw_view = false;
    let mut saw_bombard = false;
    let mut saw_join = false;
    let mut saw_colonize = false;
    for _turn in 1..=4 {
        let stdout = run_nc_sysop(&["maint", target.to_str().expect("utf-8 path"), "1"]);
        assert!(stdout.contains("Rust maintenance complete."));
        let runtime = load_runtime_state(&target);
        assert_runtime_playability_invariants(&runtime);

        for row in &runtime.report_block_rows {
            saw_view |= row.decoded_text.contains("Viewing mission report");
            saw_bombard |= row
                .decoded_text
                .to_ascii_lowercase()
                .contains("bombardment");
            saw_join |= row.decoded_text.contains("Join mission summary");
            saw_colonize |= row.decoded_text.contains("terraformed");
        }
    }

    assert!(
        saw_view,
        "multi-turn canary should produce a viewing report"
    );
    assert!(
        saw_bombard,
        "multi-turn canary should produce a bombardment report"
    );
    assert!(saw_join, "multi-turn canary should produce a join report");
    assert!(
        saw_colonize,
        "multi-turn canary should produce a colonization report"
    );

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn nc_sysop_maint_scopes_runtime_reports_and_intel_per_viewer() {
    let target = unique_temp_dir("nc-sysop-maint-viewer-scoping");
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    join_player(&mut game_data, 1, "Empire One", "Forge");
    join_player(&mut game_data, 4, "Empire Four", "Bastion");
    hold_all_fleets_in_place(&mut game_data);

    let player_1_coords = game_data.planets[13].sector.to_array();
    let player_4_coords = game_data.planets[6].sector.to_array();
    // Cruiser speed settles in orbit one sector away within the year.
    for idx in [0, 12] {
        let fleet = &mut game_data.fleets[idx];
        fleet.ships.etacs = 0;
        recompute_max_speed(fleet);
    }
    let player_1_speed = game_data.fleets[0].max_speed;
    let player_4_speed = game_data.fleets[12].max_speed;
    game_data.fleets[0].order = Order::ViewWorld;
    game_data.fleets[0].target = Coords::from_array(player_1_coords);
    game_data.fleets[0].speed = player_1_speed;
    game_data.fleets[12].order = Order::ViewWorld;
    game_data.fleets[12].target = Coords::from_array(player_4_coords);
    game_data.fleets[12].speed = player_4_speed;
    seed_runtime_snapshot(&target, &game_data);

    let stdout = run_nc_sysop(&["maint", target.to_str().expect("utf-8 path"), "1"]);
    assert!(stdout.contains("Rust maintenance complete."));

    let store = CampaignStore::open_default_in_dir(&target).expect("open campaign store");
    let runtime = store
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("runtime snapshot should exist");
    let player_1_reports = runtime
        .report_block_rows
        .iter()
        .filter(|row| row.is_visible_to_viewer(1))
        .map(|row| row.decoded_text.as_str())
        .collect::<Vec<_>>();
    let player_4_reports = runtime
        .report_block_rows
        .iter()
        .filter(|row| row.is_visible_to_viewer(4))
        .map(|row| row.decoded_text.as_str())
        .collect::<Vec<_>>();
    let viewing_reports = runtime
        .report_block_rows
        .iter()
        .filter(|row| row.decoded_text.contains("Viewing mission report"))
        .collect::<Vec<_>>();

    assert!(runtime.report_block_rows.iter().any(
        |row| row.viewer_empire_id == 1 && row.decoded_text.contains("Viewing mission report")
    ));
    assert_eq!(viewing_reports.len(), 2);
    assert!(
        !player_1_reports
            .iter()
            .any(|text| text.contains("13th Fleet"))
    );
    assert!(runtime.report_block_rows.iter().any(
        |row| row.viewer_empire_id == 4 && row.decoded_text.contains("Viewing mission report")
    ));
    assert!(!player_4_reports.iter().any(|text| text.contains(&format!(
        "System({},{})",
        player_1_coords[0], player_1_coords[1]
    ))));

    let player_1_intel = store
        .latest_planet_intel_for_viewer(1)
        .expect("load player 1 intel");
    assert_eq!(
        player_1_intel
            .iter()
            .find(|snapshot| snapshot.planet_record_index_1_based == 14)
            .expect("player 1 viewed world")
            .intel_tier,
        IntelTier::Partial
    );
    assert_eq!(
        player_1_intel
            .iter()
            .find(|snapshot| snapshot.planet_record_index_1_based == 7)
            .expect("player 4 target should stay unknown for player 1")
            .intel_tier,
        IntelTier::Unknown
    );

    let player_4_intel = store
        .latest_planet_intel_for_viewer(4)
        .expect("load player 4 intel");
    assert_eq!(
        player_4_intel
            .iter()
            .find(|snapshot| snapshot.planet_record_index_1_based == 7)
            .expect("player 4 viewed world")
            .intel_tier,
        IntelTier::Partial
    );
    assert_eq!(
        player_4_intel
            .iter()
            .find(|snapshot| snapshot.planet_record_index_1_based == 14)
            .expect("player 1 target should stay unknown for player 4")
            .intel_tier,
        IntelTier::Unknown
    );

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn nc_sysop_maint_view_world_on_station_generates_report() {
    // Regression test for Bug 1: a ViewWorld fleet whose current position
    // already equals its target (no travel needed) must still emit a viewing
    // mission report and update intel on that turn.
    let target = unique_temp_dir("nc-sysop-maint-view-world-on-station");
    let mut game_data = load_fixture("ecmaint-fleet-pre");

    // Place fleet 0 at the coords of planet 13 with ViewWorld order and
    // speed=0 so it has nothing to travel — it is already on station.
    let planet_coords = game_data.planets[13].sector.to_array();
    let viewer = &mut game_data.fleets[0];
    viewer.sector = Coords::from_array(planet_coords);
    viewer.order = Order::ViewWorld;
    viewer.target = Coords::from_array(planet_coords);
    viewer.speed = 0;
    viewer.ships.scouts = 0;
    viewer.ships.etacs = 0;
    seed_runtime_snapshot(&target, &game_data);

    let stdout = run_nc_sysop(&["maint", target.to_str().expect("utf-8 path"), "1"]);
    assert!(stdout.contains("Rust maintenance complete."));

    let store = CampaignStore::open_default_in_dir(&target).expect("open campaign store");
    let runtime = store
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("runtime snapshot should exist");

    assert!(
        runtime
            .report_block_rows
            .iter()
            .any(|row| row.decoded_text.contains("Viewing mission report")),
        "on-station ViewWorld fleet should generate a viewing report"
    );

    let intel = store
        .latest_planet_intel_for_viewer(1)
        .expect("load viewer intel");
    assert!(
        intel
            .iter()
            .any(|s| s.planet_record_index_1_based == 14 && s.intel_tier == IntelTier::Partial),
        "on-station ViewWorld fleet should refresh planet intel"
    );

    // ViewWorld is one-shot: the order must revert to HoldPosition after firing.
    assert_eq!(
        runtime.game_data.fleets[0].order,
        Order::HoldPosition,
        "on-station ViewWorld must reset to HoldPosition after firing"
    );

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn nc_sysop_maint_scout_sector_on_station_persists_report() {
    // A ScoutSector fleet already on station must keep Order::ScoutSector and
    // not crash.  The on-station ScoutSector event deliberately suppresses the
    // repeating status message ("no news is good news"), so no report block row
    // is expected.
    let target = unique_temp_dir("nc-sysop-maint-scout-sector-on-station");
    let mut game_data = load_fixture("ecmaint-fleet-pre");

    let planet_coords = game_data.planets[13].sector.to_array();
    let scout = &mut game_data.fleets[0];
    scout.sector = Coords::from_array(planet_coords);
    scout.order = Order::ScoutSector;
    scout.target = Coords::from_array(planet_coords);
    scout.speed = 0;
    scout.ships.scouts = 1;
    seed_runtime_snapshot(&target, &game_data);

    let stdout = run_nc_sysop(&["maint", target.to_str().expect("utf-8 path"), "1"]);
    assert!(stdout.contains("Rust maintenance complete."));

    let store = CampaignStore::open_default_in_dir(&target).expect("open campaign store");
    let runtime = store
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("runtime snapshot should exist");

    // On-station ScoutSector suppresses the repeating status message.
    assert!(
        !runtime
            .report_block_rows
            .iter()
            .any(|row| row.decoded_text.contains("scout this sector")),
        "on-station ScoutSector must not emit a repeating status report"
    );
    assert_eq!(
        runtime.game_data.fleets[0].order,
        Order::ScoutSector,
        "ScoutSector must persist on station after firing"
    );

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn nc_sysop_maint_scout_system_on_station_persists_report_and_intel() {
    // A ScoutSolarSystem fleet already on station must emit a scouting report,
    // update planet intel to Full, and keep Order::ScoutSolarSystem.
    let target = unique_temp_dir("nc-sysop-maint-scout-system-on-station");
    let mut game_data = load_fixture("ecmaint-fleet-pre");

    let planet_coords = game_data.planets[13].sector.to_array();
    let scout = &mut game_data.fleets[0];
    scout.sector = Coords::from_array(planet_coords);
    scout.order = Order::ScoutSolarSystem;
    scout.target = Coords::from_array(planet_coords);
    scout.speed = 0;
    scout.ships.scouts = 1;
    seed_runtime_snapshot(&target, &game_data);

    let stdout = run_nc_sysop(&["maint", target.to_str().expect("utf-8 path"), "1"]);
    assert!(stdout.contains("Rust maintenance complete."));

    let store = CampaignStore::open_default_in_dir(&target).expect("open campaign store");
    let runtime = store
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("runtime snapshot should exist");

    assert!(
        runtime
            .report_block_rows
            .iter()
            .any(|row| row.decoded_text.contains("Scouting mission report")),
        "on-station ScoutSolarSystem fleet should generate a scouting report"
    );

    let intel = store
        .latest_planet_intel_for_viewer(1)
        .expect("load viewer intel");
    assert!(
        intel
            .iter()
            .any(|s| s.planet_record_index_1_based == 14 && s.intel_tier == IntelTier::Full),
        "on-station ScoutSolarSystem should produce Full intel for the target planet"
    );

    assert_eq!(
        runtime.game_data.fleets[0].order,
        Order::ScoutSolarSystem,
        "ScoutSolarSystem must persist on station after firing"
    );

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn nc_sysop_maint_view_world_reports_when_the_fleet_settles_not_on_sector_entry() {
    // ECMAINT resolves View World when the fleet has dived fully into the
    // target system, not when it first enters the target sector.
    //
    // Geometry: fleet at (5,3), target (7,4), speed 3, from rest. Year 1
    // climbs out (1) and covers 2 of the 2.236 sectors, entering the target
    // sector short of its centre; year 2 reaches the centre and dives.
    let target_dir = unique_temp_dir("nc-sysop-maint-view-world-settles");
    let mut game_data = load_fixture("ecmaint-fleet-pre");

    let viewer = &mut game_data.fleets[0];
    viewer.sector = Coords::new(5, 3);
    viewer.order = Order::ViewWorld;
    viewer.target = Coords::new(7, 4);
    viewer.speed = 3;
    viewer.ships.scouts = 0;
    viewer.ships.etacs = 0;
    seed_runtime_snapshot(&target_dir, &game_data);

    let viewing_reported = |target_dir: &std::path::Path| {
        let store = CampaignStore::open_default_in_dir(target_dir).expect("open campaign store");
        let runtime = store
            .load_latest_runtime_state()
            .expect("load runtime state")
            .expect("runtime snapshot should exist");
        let in_target_sector = runtime.game_data.fleets[0].sector.to_array() == [7, 4];
        let reported = runtime
            .report_block_rows
            .iter()
            .any(|row| row.decoded_text.contains("Viewing mission report"));
        (in_target_sector, reported)
    };

    let stdout = run_nc_sysop(&["maint", target_dir.to_str().expect("utf-8 path"), "1"]);
    assert!(stdout.contains("Rust maintenance complete."));
    assert_eq!(
        viewing_reported(&target_dir),
        (true, false),
        "year 1 enters the target sector without reporting"
    );

    let stdout = run_nc_sysop(&["maint", target_dir.to_str().expect("utf-8 path"), "1"]);
    assert!(stdout.contains("Rust maintenance complete."));
    assert_eq!(
        viewing_reported(&target_dir),
        (true, true),
        "year 2 settles in the system and reports"
    );

    let _ = fs::remove_dir_all(&target_dir);
}
