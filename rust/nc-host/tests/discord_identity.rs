use nc_data::CampaignStore;
use nc_host::admin::{Request, execute};

#[test]
fn discord_id_owns_seat_and_names_match_and_lock_without_changing_empires() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    execute(
        root,
        Request::Create {
            game: "test".into(),
            name: "Test".into(),
            players: 4,
            seed: Some(1515),
        },
    )
    .unwrap();
    execute(
        root,
        Request::Publish {
            game: "test".into(),
        },
    )
    .unwrap();
    for id in ["123", "456"] {
        execute(
            root,
            Request::Join {
                game: "test".into(),
                account: nc_data::AccountIdentity::Discord(id.into()),
                display_name: None,
            },
        )
        .unwrap();
    }
    let store = CampaignStore::open_default_in_dir(&root.join("test")).unwrap();
    let before = store.load_latest_runtime_state().unwrap().unwrap();
    let sync = |id: &str, name: &str| {
        execute(
            root,
            Request::SyncPlayerHandle {
                game: "test".into(),
                account: nc_data::AccountIdentity::Discord(id.into()),
                display_name: name.into(),
            },
        )
    };
    assert!(sync("789", "Impostor").is_err());
    assert_eq!(sync("123", "Commander").unwrap()["handle"], "Commander");
    assert_eq!(sync("456", "Commander").unwrap()["seat"], 2);
    // The first Play locked the handle; a later sign-in name does not rename it.
    assert_eq!(sync("123", "New\r\nName\x1b").unwrap()["handle"], "Commander");
    assert_eq!(
        execute(
            root,
            Request::Authorize {
                game: "test".into(),
                account: nc_data::AccountIdentity::Discord("123".into())
            }
        )
        .unwrap()["seat"],
        1
    );
    let after = store.load_latest_runtime_state().unwrap().unwrap();
    assert_eq!(
        after.game_data.players[0].empire_name,
        before.game_data.players[0].empire_name
    );
    assert_eq!(after.game_data.players[1].handle, "Commander");
    assert_eq!(after.report_block_rows, before.report_block_rows);
    assert_eq!(after.queued_mail, before.queued_mail);
    let snapshot = after.snapshot_id;
    sync("123", "Another Name").unwrap();
    assert_eq!(
        store
            .load_latest_runtime_state()
            .unwrap()
            .unwrap()
            .snapshot_id,
        snapshot
    );
    assert!(sync("123", "\r\n").is_err());
    execute(
        root,
        Request::Pause {
            game: "test".into(),
        },
    )
    .unwrap();
    assert!(sync("123", "Paused").is_err());
}
