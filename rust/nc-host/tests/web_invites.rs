use nc_host::admin::{Request, execute};
use serde_json::Value;
use std::path::Path;

fn setup(root: &Path, game: &str, players: u8) {
    execute(
        root,
        Request::Create {
            game: game.into(),
            name: game.into(),
            players,
            seed: Some(1515),
        },
    )
    .unwrap();
    execute(root, Request::InviteOnly { game: game.into() }).unwrap();
    execute(root, Request::Publish { game: game.into() }).unwrap();
}
fn codes(root: &Path, count: u8) -> Value {
    execute(
        root,
        Request::CreateInvites {
            game: "one".into(),
            count,
        },
    )
    .unwrap()["invites"]
        .clone()
}
fn claim(root: &Path, game: &str, account: &str, code: &str) -> nc_host::admin::Result<Value> {
    execute(
        root,
        Request::ClaimInvite {
            game: game.into(),
            account: nc_data::AccountIdentity::Discord(account.into()),
            code: code.into(),
            display_name: None,
        },
    )
}

#[test]
fn codes_bind_once_case_insensitively_without_starting_a_full_game() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    setup(root, "one", 2);
    setup(root, "two", 2);
    let issued = codes(root, 2);
    for (i, row) in issued.as_array().unwrap().iter().enumerate() {
        let code = row["code"].as_str().unwrap();
        assert_eq!(code.len(), 8);
        assert!(
            code.bytes()
                .all(|b| b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789".contains(&b))
        );
        assert!(claim(root, "two", "999", code).is_err());
        let account = format!("10{i}");
        assert_eq!(
            claim(root, "one", &account, &code.to_ascii_lowercase()).unwrap()["seat"],
            i + 1
        );
        assert_eq!(claim(root, "one", &account, code).unwrap()["seat"], i + 1);
        assert!(claim(root, "one", "999", code).is_err());
    }
    let info = execute(root, Request::Inspect { game: "one".into() }).unwrap();
    assert_eq!(info["open"], 0);
    assert_eq!(info["state"], "recruiting");
    assert!(info["next_due"].is_null());
    assert!(
        execute(root, Request::Due)
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty()
    );
    let join = execute(
        root,
        Request::Join {
            game: "one".into(),
            account: nc_data::AccountIdentity::Discord("100".into()),
            display_name: None,
        },
    )
    .unwrap();
    assert_eq!(join["seat"], 1); // Existing seat works without entering the code again.
}

#[test]
fn no_code_cannot_bypass_reservations_and_listing_never_returns_secrets() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    setup(root, "one", 2);
    assert!(
        execute(
            root,
            Request::Join {
                game: "one".into(),
                account: nc_data::AccountIdentity::Discord("123".into()),
                display_name: None,
            }
        )
        .is_err()
    );
    let issued = codes(root, 1);
    let code = issued[0]["code"].as_str().unwrap();
    let hash = blake3::hash(code.as_bytes()).to_hex().to_string();
    for request in [
        Request::ListInvites { game: "one".into() },
        Request::History { game: "one".into() },
        Request::Catalog,
    ] {
        let output = execute(root, request).unwrap().to_string();
        assert!(!output.contains(code));
        assert!(!output.contains(&hash));
    }
    let conn = rusqlite::Connection::open(root.join("one/hosted.db")).unwrap();
    let stored: String = conn
        .query_row("SELECT code_hash FROM web_invites", [], |r| r.get(0))
        .unwrap();
    assert_eq!(stored, hash);
    assert!(
        !std::fs::read(root.join("one/hosted.db"))
            .unwrap()
            .windows(code.len())
            .any(|w| w == code.as_bytes())
    );
}

#[test]
fn revocation_reissues_a_seat_and_replacement_invalidates_its_code() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    setup(root, "one", 2);
    let issued = codes(root, 2);
    execute(
        root,
        Request::RevokeInvite {
            game: "one".into(),
            invite_id: issued[0]["invite_id"].as_str().unwrap().into(),
        },
    )
    .unwrap();
    assert!(claim(root, "one", "123", issued[0]["code"].as_str().unwrap()).is_err());
    let replacement = codes(root, 1);
    assert_eq!(replacement[0]["seat"], 1);
    execute(
        root,
        Request::Replace {
            game: "one".into(),
            seat: 2,
            account: nc_data::AccountIdentity::Discord("234".into()),
        },
    )
    .unwrap();
    assert!(claim(root, "one", "123", issued[1]["code"].as_str().unwrap()).is_err());
    claim(root, "one", "123", replacement[0]["code"].as_str().unwrap()).unwrap();
    assert!(
        execute(
            root,
            Request::RevokeInvite {
                game: "one".into(),
                invite_id: replacement[0]["invite_id"].as_str().unwrap().into()
            }
        )
        .is_err()
    );
    assert!(
        execute(
            root,
            Request::Authorize {
                game: "one".into(),
                account: nc_data::AccountIdentity::Discord("123".into())
            }
        )
        .is_ok()
    );
}

#[test]
fn issuing_too_many_codes_has_no_partial_effect() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    setup(root, "one", 2);
    codes(root, 1);
    assert!(
        execute(
            root,
            Request::CreateInvites {
                game: "one".into(),
                count: 2
            }
        )
        .is_err()
    );
    let rows = execute(root, Request::ListInvites { game: "one".into() }).unwrap();
    assert_eq!(rows["invites"].as_array().unwrap().len(), 1);
}

#[test]
fn concurrent_use_of_one_code_has_exactly_one_winner() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    setup(root, "one", 2);
    let issued = codes(root, 1);
    let code = issued[0]["code"].as_str().unwrap().to_owned();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
    let handles = (0..4)
        .map(|i| {
            let root = root.to_owned();
            let code = code.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                claim(&root, "one", &format!("10{i}"), &code)
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .filter(|r| r.is_ok())
            .count(),
        1
    );
}

#[test]
fn failed_claims_are_rate_limited_and_expire_without_exposing_codes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    setup(root, "one", 2);
    let issued = codes(root, 1);
    let code = issued[0]["code"].as_str().unwrap();
    for _ in 0..5 {
        assert!(
            claim(root, "one", "123", "WRONG123")
                .unwrap_err()
                .to_string()
                .contains("invalid")
        );
    }
    assert!(
        claim(root, "one", "123", code)
            .unwrap_err()
            .to_string()
            .contains("ten minutes")
    );
    let conn = rusqlite::Connection::open(root.join("one/hosted.db")).unwrap();
    conn.execute("UPDATE web_invite_attempts SET at=0", [])
        .unwrap();
    assert!(claim(root, "one", "123", code).is_ok());
    for i in 0..100 {
        conn.execute(
            "INSERT INTO web_invite_attempts VALUES (?1,?2)",
            rusqlite::params![format!("20{i}"), chrono::Utc::now().timestamp()],
        )
        .unwrap();
    }
    assert!(
        claim(root, "one", "999", "WRONG123")
            .unwrap_err()
            .to_string()
            .contains("ten minutes")
    );
}
