use nc_data::CampaignStore;
use nc_host::{
    admin::{Request, execute},
    map_archive::download_bytes,
};
use std::io::{Cursor, Read};
use zip::ZipArchive;

fn file(zip: &mut ZipArchive<Cursor<Vec<u8>>>, name: &str) -> Vec<u8> {
    let mut data = Vec::new();
    zip.by_name(name).unwrap().read_to_end(&mut data).unwrap();
    data
}

#[test]
fn bundle_is_authorized_and_all_formats_share_the_player_snapshot() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    for game in ["one", "two"] {
        execute(
            root,
            Request::Create {
                game: game.into(),
                name: game.into(),
                players: 4,
                seed: Some(1515),
            },
        )
        .unwrap();
        execute(root, Request::Publish { game: game.into() }).unwrap();
    }
    execute(
        root,
        Request::Join {
            game: "one".into(),
            discord_id: "123".into(),
        },
    )
    .unwrap();
    let store = CampaignStore::open_default_in_dir(&root.join("one")).unwrap();
    let mut snapshot = store.load_latest_runtime_state().unwrap().unwrap();
    for planet in &mut snapshot.game_data.planets {
        planet.name = if planet.owner.0 == 1 {
            "=SUM(1,2)&<test>"
        } else {
            "HIDDEN-ENEMY-NAME"
        }
        .into();
    }
    store
        .expecting_snapshot(snapshot.snapshot_id)
        .save_runtime_state_structured(
            snapshot.game_year,
            &snapshot.game_data,
            &snapshot.report_block_rows,
            &snapshot.queued_mail,
        )
        .unwrap();
    let export = |game: &str, id: &str, format: &str| {
        execute(
            root,
            Request::Export {
                game: game.into(),
                discord_id: id.into(),
                format: format.into(),
            },
        )
    };
    for format in ["map-zip", "map-ods", "map-xlsx"] {
        assert!(export("one", "456", format).is_err());
        assert!(export("two", "123", format).is_err());
    }
    let data = export("one", "123", "map-zip").unwrap();
    assert_eq!(data["filename"], "ECMAP-one-P1-Y3000.zip");
    assert_eq!(data["content_type"], "application/zip");
    let mut zip = ZipArchive::new(Cursor::new(download_bytes(&data).unwrap())).unwrap();
    assert_eq!(zip.len(), 5);
    for (name, format) in [
        ("starmap.txt", "map-txt"),
        ("starmap.csv", "map-csv"),
        ("starmap-DETAILS.csv", "map-details"),
    ] {
        let expected = export("one", "123", format).unwrap();
        let contents = file(&mut zip, name);
        assert_eq!(contents, download_bytes(&expected).unwrap());
        assert!(
            !String::from_utf8(contents)
                .unwrap()
                .contains("HIDDEN-ENEMY-NAME")
        );
    }
    let ods = file(&mut zip, "starmap.ods");
    let mut ods = ZipArchive::new(Cursor::new(ods)).unwrap();
    {
        let mime = ods.by_index(0).unwrap();
        assert_eq!(mime.name(), "mimetype");
        assert_eq!(mime.compression(), zip::CompressionMethod::Stored);
        assert_eq!(mime.extra_data().unwrap_or_default(), b"");
    }
    assert_eq!(
        file(&mut ods, "mimetype"),
        b"application/vnd.oasis.opendocument.spreadsheet"
    );
    let content = String::from_utf8(file(&mut ods, "content.xml")).unwrap();
    assert!(content.contains("Known Worlds"));
    assert!(content.contains("=SUM(1,2)&amp;&lt;test&gt;"));
    assert!(!content.contains("table:formula"));
    assert!(!content.contains("HIDDEN-ENEMY-NAME"));
    assert!(
        String::from_utf8(file(&mut ods, "META-INF/manifest.xml"))
            .unwrap()
            .contains("content.xml")
    );
    let mut xlsx = ZipArchive::new(Cursor::new(file(&mut zip, "starmap.xlsx"))).unwrap();
    let strings = String::from_utf8(file(&mut xlsx, "xl/sharedStrings.xml")).unwrap();
    assert!(strings.contains("=SUM(1,2)&amp;&lt;test&gt;"));
    assert!(!strings.contains("HIDDEN-ENEMY-NAME"));
    let worlds = String::from_utf8(file(&mut xlsx, "xl/worksheets/sheet2.xml")).unwrap();
    assert!(!worlds.contains("<f>"));
    assert!(
        String::from_utf8(file(&mut xlsx, "xl/workbook.xml"))
            .unwrap()
            .contains("Known Worlds")
    );
}

#[test]
fn invalid_binary_payload_is_rejected() {
    assert!(download_bytes(&serde_json::json!({"contents_base64":"not base64"})).is_err());
    assert!(download_bytes(&serde_json::json!({})).is_err());
}
