use nc_host::admin::{Request, execute, next_local_midnight};
use serde_json::Value;
use std::path::Path;
fn setup(root: &Path) {
    execute(
        root,
        Request::Create {
            game: "one".into(),
            name: "One".into(),
            players: 2,
            seed: Some(1515),
        },
    )
    .unwrap();
}
fn inspect(root: &Path) -> Value {
    execute(root, Request::Inspect { game: "one".into() }).unwrap()
}
fn configure(root: &Path, zone: Option<&str>) -> Value {
    execute(
        root,
        Request::ConfigureSchedule {
            game: "one".into(),
            time_zone: zone.map(str::to_owned),
        },
    )
    .unwrap()
}
fn timestamp(s: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(s).unwrap().timestamp()
}
#[test]
fn new_york_midnight_observes_both_dst_changes() {
    for (start, end, hours) in [
        ("2026-03-08T00:00:00-05:00", "2026-03-09T00:00:00-04:00", 23),
        ("2026-11-01T00:00:00-04:00", "2026-11-02T00:00:00-05:00", 25),
        ("2026-10-11T00:00:00-04:00", "2026-10-12T00:00:00-04:00", 24),
    ] {
        let next = next_local_midnight("America/New_York", timestamp(start)).unwrap();
        assert_eq!(next, timestamp(end));
        assert_eq!(next - timestamp(start), hours * 3600);
        assert_eq!(
            next_local_midnight("America/New_York", timestamp(start) + 3600).unwrap(),
            next
        );
    }
    assert!(next_local_midnight("not-a-zone", 0).is_err());
}
#[test]
fn configuration_and_publishing_never_start_the_game() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    setup(root);
    assert_eq!(configure(root, Some("America/New_York"))["state"], "draft");
    execute(root, Request::Publish { game: "one".into() }).unwrap();
    let info = inspect(root);
    assert!(info["next_due"].is_null());
    assert_eq!(info["state"], "recruiting");
    assert!(
        execute(root, Request::Due)
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty()
    );
    execute(root, Request::StartConfigured { game: "one".into() }).unwrap();
    let info = inspect(root);
    assert_eq!(info["state"], "running");
    let now = chrono::Utc::now().timestamp();
    assert_eq!(
        info["next_due"],
        next_local_midnight("America/New_York", now).unwrap()
    );
    assert_eq!(info["schedule"]["time_zone"], "America/New_York");
    let due = info["next_due"].clone();
    execute(
        root,
        Request::Maintain {
            game: "one".into(),
            expected_year: 3000,
            request_id: "early-manual".into(),
        },
    )
    .unwrap();
    assert_eq!(inspect(root)["next_due"], due);
    execute(root, Request::Pause { game: "one".into() }).unwrap();
    assert!(inspect(root)["next_due"].is_null());
    execute(root, Request::StartConfigured { game: "one".into() }).unwrap();
    assert_eq!(inspect(root)["next_due"], due);
}
#[test]
fn manual_mode_removes_deadlines_and_survives_manual_turns() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    setup(root);
    execute(root, Request::Publish { game: "one".into() }).unwrap();
    execute(
        root,
        Request::Start {
            game: "one".into(),
            first_due: chrono::Utc::now().timestamp() + 3600,
            interval_minutes: 1440,
        },
    )
    .unwrap();
    let info = configure(root, None);
    assert_eq!(info["state"], "running");
    assert!(info["next_due"].is_null());
    assert_eq!(info["schedule"]["mode"], "manual");
    execute(
        root,
        Request::Maintain {
            game: "one".into(),
            expected_year: 3000,
            request_id: "manual".into(),
        },
    )
    .unwrap();
    assert!(inspect(root)["next_due"].is_null());
    assert_eq!(inspect(root)["year"], 3001);
    assert!(
        execute(root, Request::Due)
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty()
    );
    let conn = rusqlite::Connection::open(root.join("one/hosted.db")).unwrap();
    let enabled: bool = conn
        .query_row("SELECT maintenance_enabled FROM game_metadata", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert!(!enabled);
    execute(root, Request::Pause { game: "one".into() }).unwrap();
    execute(root, Request::StartConfigured { game: "one".into() }).unwrap();
    assert!(inspect(root)["next_due"].is_null());
}
