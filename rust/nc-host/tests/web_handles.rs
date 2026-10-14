use nc_host::admin::{Request, execute};
use serde_json::{Value, json};

fn call(root: &std::path::Path, request: Value) -> Result<Value, String> {
    let request: Request = serde_json::from_value(request).map_err(|e| e.to_string())?;
    execute(root, request).map_err(|e| e.to_string())
}
fn play(root: &std::path::Path, account: &Value, name: &str) -> String {
    call(
        root,
        json!({"action":"sync_player_handle","game":"one","account":account,"display_name":name}),
    )
    .unwrap()["handle"]
        .as_str()
        .unwrap()
        .to_owned()
}
fn game(root: &std::path::Path, invite_only: bool) {
    call(
        root,
        json!({"action":"create","game":"one","name":"one","players":4,"seed":1515}),
    )
    .unwrap();
    call(root, json!({"action":"publish","game":"one"})).unwrap();
    if invite_only {
        call(root, json!({"action":"invite_only","game":"one"})).unwrap();
    }
}

#[test]
fn claiming_a_seat_locks_the_claimants_public_name() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    game(root, true);
    let alice = json!({"provider":"matrix","id":"@alice:example.org"});
    let code = call(
        root,
        json!({"action":"create_invites","game":"one","count":1}),
    )
    .unwrap()["invites"][0]["code"]
        .clone();
    call(
        root,
        json!({"action":"claim_invite","game":"one","account":alice,"code":code,"display_name":"Alice\u{7}"}),
    )
    .unwrap();
    assert_eq!(play(root, &alice, "Someone Else"), "Alice");
    // A linked identity with another name plays the same locked handle.
    let nostr = json!({"provider":"nostr","id":"79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798"});
    call(
        root,
        json!({"action":"link_account","game":"one","seat":1,"account":nostr}),
    )
    .unwrap();
    assert_eq!(play(root, &nostr, "anon1234"), "Alice");
}

#[test]
fn open_join_locks_and_unnamed_claims_lock_at_first_play() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    game(root, false);
    let bob = json!({"provider":"discord","id":"123"});
    call(
        root,
        json!({"action":"join","game":"one","account":bob,"display_name":"Bob"}),
    )
    .unwrap();
    assert_eq!(play(root, &bob, "Robert"), "Bob");
    // Claims without a name (as before handles were locked) take the first Play name.
    let carol = json!({"provider":"discord","id":"456"});
    call(root, json!({"action":"join","game":"one","account":carol})).unwrap();
    assert_eq!(play(root, &carol, "Carol"), "Carol");
    assert_eq!(play(root, &carol, "Caroline"), "Carol");
}

#[test]
fn replacement_unlocks_and_operator_rename_overrides_the_lock() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    game(root, false);
    let bob = json!({"provider":"discord","id":"123"});
    let dave = json!({"provider":"discord","id":"789"});
    call(
        root,
        json!({"action":"join","game":"one","account":bob,"display_name":"Bob"}),
    )
    .unwrap();
    call(
        root,
        json!({"action":"replace","game":"one","seat":1,"account":dave}),
    )
    .unwrap();
    assert_eq!(play(root, &dave, "Dave"), "Dave");
    let renamed = call(
        root,
        json!({"action":"rename","game":"one","seat":1,"handle":"Admiral Dave"}),
    )
    .unwrap();
    assert_eq!(renamed["handle"], "Admiral Dave");
    assert_eq!(play(root, &dave, "Dave"), "Admiral Dave");
    let history = call(root, json!({"action":"history","game":"one"})).unwrap();
    assert!(history.to_string().contains("rename"));
    for bad in [
        json!({"action":"rename","game":"one","seat":0,"handle":"X"}),
        json!({"action":"rename","game":"one","seat":5,"handle":"X"}),
        json!({"action":"rename","game":"one","seat":1,"handle":"  \u{7}"}),
    ] {
        assert!(call(root, bad).is_err());
    }
    call(root, json!({"action":"archive","game":"one"})).unwrap();
    assert!(
        call(
            root,
            json!({"action":"rename","game":"one","seat":1,"handle":"Late"})
        )
        .is_err()
    );
}
