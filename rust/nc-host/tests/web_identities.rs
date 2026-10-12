use nc_host::admin::{Request, execute};
use serde_json::{Value, json};

fn call(root: &std::path::Path, request: Value) -> Result<Value, String> {
    let request: Request = serde_json::from_value(request).map_err(|e| e.to_string())?;
    execute(root, request).map_err(|e| e.to_string())
}

#[test]
fn discord_and_nostr_share_invites_but_never_authority() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let discord = json!({"provider":"discord","id":"123"});
    let nostr = json!({"provider":"nostr","id":"79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798"});
    for game in ["one", "two"] {
        call(
            root,
            json!({"action":"create","game":game,"name":game,"players":4,"seed":1515}),
        )
        .unwrap();
        call(root, json!({"action":"publish","game":game})).unwrap();
        call(root, json!({"action":"invite_only","game":game})).unwrap();
    }
    let codes = call(
        root,
        json!({"action":"create_invites","game":"one","count":2}),
    )
    .unwrap();
    for (index, account) in [&discord, &nostr].into_iter().enumerate() {
        let claim = json!({"action":"claim_invite","game":"one","account":account,"code":codes["invites"][index]["code"]});
        let seat = call(root, claim.clone()).unwrap();
        assert_eq!(seat["seat"], index + 1);
        assert_eq!(seat["account"], *account);
        assert_eq!(call(root, claim).unwrap()["seat"], index + 1);
        assert!(
            call(
                root,
                json!({"action":"export","game":"one","account":account,"format":"map-zip"})
            )
            .is_ok()
        );
        assert!(
            call(
                root,
                json!({"action":"export","game":"two","account":account,"format":"reports"})
            )
            .is_err()
        );
    }
    let outsider = json!({"provider":"discord","id":"456"});
    assert!(call(root, json!({"action":"claim_invite","game":"one","account":outsider,"code":codes["invites"][0]["code"]})).is_err());
    call(
        root,
        json!({"action":"replace","game":"one","seat":2,"account":outsider}),
    )
    .unwrap();
    assert!(
        call(
            root,
            json!({"action":"authorize","game":"one","account":nostr})
        )
        .is_err()
    );
    let game = call(root, json!({"action":"inspect","game":"one"})).unwrap();
    assert_eq!(game["state"], "recruiting");
    assert_eq!(game["year"], 3000);
    assert!(game["next_due"].is_null());
}

#[test]
fn consumed_code_never_links_the_other_provider_in_either_direction() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let discord = json!({"provider":"discord","id":"123"});
    let nostr = json!({"provider":"nostr","id":"79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798"});
    for (game, owner, other) in [
        ("discord-first", &discord, &nostr),
        ("nostr-first", &nostr, &discord),
    ] {
        call(
            root,
            json!({"action":"create","game":game,"name":game,"players":2,"seed":1515}),
        )
        .unwrap();
        call(root, json!({"action":"publish","game":game})).unwrap();
        call(root, json!({"action":"invite_only","game":game})).unwrap();
        let invites = call(
            root,
            json!({"action":"create_invites","game":game,"count":1}),
        )
        .unwrap();
        let code = &invites["invites"][0]["code"];
        call(
            root,
            json!({"action":"claim_invite","game":game,"account":owner,"code":code}),
        )
        .unwrap();
        assert!(
            call(
                root,
                json!({"action":"claim_invite","game":game,"account":other,"code":code})
            )
            .is_err()
        );
        assert!(
            call(
                root,
                json!({"action":"authorize","game":game,"account":other})
            )
            .is_err()
        );
        assert_eq!(
            call(root, json!({"action":"inspect","game":game})).unwrap()["claimed"],
            1
        );
        let listed = call(root, json!({"action":"list_invites","game":game})).unwrap();
        assert_eq!(listed["invites"][0]["state"], "claimed");
        assert_eq!(listed["invites"][0]["claimed_by"], *owner);
    }
}

#[test]
fn operator_links_both_providers_and_can_revoke_one_without_reopening_seat() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let discord = json!({"provider":"discord","id":"123"});
    let nostr = json!({"provider":"nostr","id":"79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798"});
    let another = json!({"provider":"discord","id":"456"});
    call(
        root,
        json!({"action":"create","game":"one","name":"one","players":4,"seed":1515}),
    )
    .unwrap();
    call(root, json!({"action":"publish","game":"one"})).unwrap();
    assert!(
        call(
            root,
            json!({"action":"link_account","game":"one","seat":1,"account":nostr})
        )
        .is_err()
    );
    call(
        root,
        json!({"action":"join","game":"one","account":discord}),
    )
    .unwrap();
    call(
        root,
        json!({"action":"join","game":"one","account":another}),
    )
    .unwrap();
    assert!(
        call(
            root,
            json!({"action":"link_account","game":"one","seat":1,"account":another})
        )
        .is_err()
    );
    for _ in 0..2 {
        call(
            root,
            json!({"action":"link_account","game":"one","seat":1,"account":nostr}),
        )
        .unwrap();
    }
    let inspected = call(root, json!({"action":"inspect","game":"one"})).unwrap();
    assert_eq!(inspected["claimed"], 2);
    assert_eq!(inspected["open"], 2);
    assert_eq!(
        inspected["seats"][0]["accounts"].as_array().unwrap().len(),
        2
    );
    for account in [&discord, &nostr] {
        assert_eq!(
            call(
                root,
                json!({"action":"authorize","game":"one","account":account})
            )
            .unwrap()["seat"],
            1
        );
    }
    call(
        root,
        json!({"action":"unlink_account","game":"one","seat":1,"account":discord}),
    )
    .unwrap();
    assert!(
        call(
            root,
            json!({"action":"authorize","game":"one","account":discord})
        )
        .is_err()
    );
    assert!(
        call(
            root,
            json!({"action":"unlink_account","game":"one","seat":1,"account":nostr})
        )
        .is_err()
    );
    call(
        root,
        json!({"action":"link_account","game":"one","seat":1,"account":discord}),
    )
    .unwrap();
    // Replacement removes all other bindings even when the replacement already owns this seat.
    call(
        root,
        json!({"action":"replace","game":"one","seat":1,"account":discord}),
    )
    .unwrap();
    assert!(
        call(
            root,
            json!({"action":"authorize","game":"one","account":nostr})
        )
        .is_err()
    );
    let inspected = call(root, json!({"action":"inspect","game":"one"})).unwrap();
    assert_eq!(inspected["claimed"], 2);
    assert_eq!(inspected["year"], 3000);
    assert_eq!(inspected["state"], "recruiting");
    assert!(inspected["next_due"].is_null());
}

#[test]
fn simultaneous_discord_and_nostr_claim_has_exactly_one_winner() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    call(
        root,
        json!({"action":"create","game":"one","name":"one","players":2,"seed":1515}),
    )
    .unwrap();
    call(root, json!({"action":"publish","game":"one"})).unwrap();
    call(root, json!({"action":"invite_only","game":"one"})).unwrap();
    let issued = call(
        root,
        json!({"action":"create_invites","game":"one","count":1}),
    )
    .unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let accounts = [
        json!({"provider":"discord","id":"123"}),
        json!({"provider":"nostr","id":"79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798"}),
    ];
    let threads = accounts
        .into_iter()
        .map(|account| {
            let root = root.to_owned();
            let barrier = barrier.clone();
            let code = issued["invites"][0]["code"].clone();
            std::thread::spawn(move || {
                barrier.wait();
                call(
                    &root,
                    json!({"action":"claim_invite","game":"one","account":account,"code":code}),
                )
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        threads
            .into_iter()
            .filter_map(|t| t.join().unwrap().ok())
            .count(),
        1
    );
}
