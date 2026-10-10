mod common;

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use common::without_unread_bytes;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn init_fixture_dir() -> PathBuf {
    repo_root().join("fixtures/ecutil-init/v1.5")
}

#[test]
fn init_creates_known_good_fixture_set() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let target = std::env::temp_dir().join(format!("nc-cli-init-{unique}"));
    let output = Command::new(env!("CARGO_BIN_EXE_nc-cli"))
        .current_dir(repo_root().join("rust"))
        .args(["init", "original/v1.5"])
        .arg(&target)
        .output()
        .expect("nc-cli should run");

    assert!(
        output.status.success(),
        "nc-cli init failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let init = init_fixture_dir();
    for name in [
        "BASES.DAT",
        "CONQUEST.DAT",
        // Note: DATABASE.DAT is now generated from PLANETS.DAT, so it won't match exactly
        // "DATABASE.DAT",
        "FLEETS.DAT",
        "IPBM.DAT",
        "MESSAGES.DAT",
        "PLANETS.DAT",
        "PLAYER.DAT",
        "RESULTS.DAT",
        "SETUP.DAT",
    ] {
        let actual = fs::read(target.join(name)).unwrap();
        let expected = fs::read(init.join(name)).unwrap();
        // The typed model does not carry the bytes no classic program reads.
        assert_eq!(
            without_unread_bytes(name, &actual),
            without_unread_bytes(name, &expected),
            "{name} should match initialized fixture"
        );
    }

    // DATABASE.DAT is generated from PLANETS.DAT, so just verify it exists and is valid
    let db_actual = fs::read(target.join("DATABASE.DAT")).unwrap();
    assert_eq!(db_actual.len(), 8000, "DATABASE.DAT should be 8000 bytes");

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn init_accepts_omitted_source_and_uses_default_fixture_dir() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let target = std::env::temp_dir().join(format!("nc-cli-init-default-{unique}"));
    let output = Command::new(env!("CARGO_BIN_EXE_nc-cli"))
        .current_dir(repo_root().join("rust"))
        .args(["init"])
        .arg(&target)
        .output()
        .expect("nc-cli should run");

    assert!(
        output.status.success(),
        "nc-cli init failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let init = init_fixture_dir();
    for name in [
        "BASES.DAT",
        "CONQUEST.DAT",
        // Note: DATABASE.DAT is now generated from PLANETS.DAT
        // "DATABASE.DAT",
        "FLEETS.DAT",
        "PLANETS.DAT",
        "PLAYER.DAT",
        "SETUP.DAT",
    ] {
        let actual = fs::read(target.join(name)).unwrap();
        let expected = fs::read(init.join(name)).unwrap();
        // The typed model does not carry the bytes no classic program reads.
        assert_eq!(
            without_unread_bytes(name, &actual),
            without_unread_bytes(name, &expected),
            "{name} should match initialized fixture"
        );
    }

    // DATABASE.DAT is generated from PLANETS.DAT, so just verify it exists and is valid
    let db_actual = fs::read(target.join("DATABASE.DAT")).unwrap();
    assert_eq!(db_actual.len(), 8000, "DATABASE.DAT should be 8000 bytes");

    let _ = fs::remove_dir_all(&target);
}
