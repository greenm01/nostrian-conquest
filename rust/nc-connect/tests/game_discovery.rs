use nc_connect::connect::game_discovery::{InviteResolution, select_discovered_game_from_events};
use nc_connect::connect::resolve::ResolvedTarget;
use nostr_sdk::{Event, EventBuilder, Keys, Kind, Tag};
use sha2::{Digest, Sha256};

fn build_game_definition_event(ssh_host: &str, ssh_port: u16, slots: Vec<[String; 4]>) -> Event {
    let keys = Keys::generate();
    let mut tags = vec![
        Tag::parse(["d", "friday-night"]).unwrap(),
        Tag::parse(["name", "Friday Night EC"]).unwrap(),
        Tag::parse(["status", "active"]).unwrap(),
        Tag::parse(["ssh-host", ssh_host]).unwrap(),
        Tag::parse(["ssh-port", &ssh_port.to_string()]).unwrap(),
        Tag::parse(["players", "4"]).unwrap(),
    ];
    tags.extend(
        slots
            .into_iter()
            .map(|slot| Tag::parse(["slot", &slot[0], &slot[1], &slot[2], &slot[3]]).unwrap()),
    );
    EventBuilder::new(Kind::Custom(30500), "")
        .tags(tags)
        .sign_with_keys(&keys)
        .unwrap()
}

fn pending_slot(seat: u32, invite_hash: &str) -> [String; 4] {
    [
        seat.to_string(),
        invite_hash.to_string(),
        String::new(),
        "pending".to_string(),
    ]
}

fn claimed_slot(seat: u32, invite_hash: &str, player_npub: &str) -> [String; 4] {
    [
        seat.to_string(),
        invite_hash.to_string(),
        player_npub.to_string(),
        "claimed".to_string(),
    ]
}

fn target() -> ResolvedTarget {
    ResolvedTarget {
        server_host: "play.example.com".to_string(),
        server_port: 2222,
        relay_url: "wss://relay.example.com".to_string(),
        invite_code: Some("amber-river".to_string()),
        game_id: None,
        gate_npub: None,
    }
}

fn local_target() -> ResolvedTarget {
    ResolvedTarget {
        server_host: String::new(),
        server_port: 22,
        relay_url: "ws://localhost:8080".to_string(),
        invite_code: Some("amber-river".to_string()),
        game_id: None,
        gate_npub: None,
    }
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    result.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn discovery_matches_invite_hash_and_ssh_target() {
    let event = build_game_definition_event(
        "play.example.com",
        2222,
        vec![pending_slot(1, &sha256_hex("amber-river"))],
    );

    let discovered = select_discovered_game_from_events([&event], &target(), "amber-river", None)
        .expect("discover game");

    assert_eq!(discovered.game_id, "friday-night");
    assert_eq!(discovered.game_name, "Friday Night EC");
    assert_eq!(discovered.ssh_host, "play.example.com");
    assert_eq!(discovered.ssh_port, 2222);
    assert_eq!(discovered.seat, 1);
    assert_eq!(discovered.resolution, InviteResolution::FirstJoin);
    assert!(discovered.gate_npub.starts_with("npub1"));
}

#[test]
fn discovery_normalizes_invite_with_host_suffix() {
    let event = build_game_definition_event(
        "play.example.com",
        2222,
        vec![pending_slot(1, &sha256_hex("amber-river"))],
    );

    let discovered = select_discovered_game_from_events(
        [&event],
        &target(),
        "amber-river@relay.example.com:7447",
        None,
    )
    .expect("discover game");

    assert_eq!(discovered.game_id, "friday-night");
}

#[test]
fn discovery_falls_back_to_gate_override_message_when_no_match_exists() {
    let event = build_game_definition_event(
        "play.example.com",
        2222,
        vec![pending_slot(
            1,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )],
    );

    let err = select_discovered_game_from_events([&event], &target(), "amber-river", None)
        .expect_err("no matching event");

    assert!(err.contains("relay was reachable"));
    assert!(err.contains("no pending hosted seat matched"));
    assert!(err.contains("republish hosted metadata"));
    assert!(!err.contains("--gate"));
}

#[test]
fn discovery_reports_local_hosted_stack_hint_when_local_relay_has_no_match() {
    let err = select_discovered_game_from_events(
        std::iter::empty::<&Event>(),
        &local_target(),
        "amber-river",
        None,
    )
    .expect_err("no matching event");

    assert!(err.contains("local relay"));
    assert!(err.contains("nc-host serve"));
    assert!(err.contains("ws://localhost:8080"));
}

#[test]
fn discovery_accepts_mismatched_host_when_unique() {
    let event = build_game_definition_event(
        "other.example.com",
        2222,
        vec![pending_slot(1, &sha256_hex("amber-river"))],
    );

    let discovered = select_discovered_game_from_events([&event], &target(), "amber-river", None)
        .expect("should match despite mismatched host because hash is unique");

    assert_eq!(discovered.game_id, "friday-night");
}

#[test]
fn discovery_disambiguates_by_host_and_port_when_multiple_hashes_match() {
    let first = build_game_definition_event(
        "other.example.com",
        2222,
        vec![pending_slot(1, &sha256_hex("amber-river"))],
    );
    let second = build_game_definition_event(
        "play.example.com",
        2222,
        vec![pending_slot(1, &sha256_hex("amber-river"))],
    );

    // Even though two games have the same hash, exactly one matches the target host/port.
    let discovered =
        select_discovered_game_from_events([&first, &second], &target(), "amber-river", None)
            .expect("should disambiguate using exact host/port match");

    assert_eq!(discovered.game_id, "friday-night");
}

#[test]
fn discovery_reports_ambiguous_matches() {
    let first = build_game_definition_event(
        "play.example.com",
        2222,
        vec![pending_slot(1, &sha256_hex("amber-river"))],
    );
    let second = build_game_definition_event(
        "play.example.com",
        2222,
        vec![pending_slot(1, &sha256_hex("amber-river"))],
    );

    let err = select_discovered_game_from_events([&first, &second], &target(), "amber-river", None)
        .expect_err("ambiguous match");

    assert!(err.contains("multiple hosted games matched"));
    assert!(err.contains("open the game from the picker"));
    assert!(!err.contains("--gate"));
}

#[test]
fn discovery_accepts_claimed_invite_bound_to_same_identity() {
    let player_keys = Keys::generate();
    let player_hex = player_keys.public_key().to_hex();
    let event = build_game_definition_event(
        "play.example.com",
        2222,
        vec![claimed_slot(1, &sha256_hex("amber-river"), &player_hex)],
    );

    let discovered = select_discovered_game_from_events(
        [&event],
        &target(),
        "amber-river",
        Some(player_hex.as_str()),
    )
    .expect("claimed invite should reconnect for the same identity");

    assert_eq!(discovered.game_id, "friday-night");
    assert_eq!(discovered.seat, 1);
    assert_eq!(discovered.resolution, InviteResolution::SameIdentityRejoin);
}

#[test]
fn discovery_reports_claimed_invite_taken_by_another_identity() {
    let player_keys = Keys::generate();
    let other_player_hex = Keys::generate().public_key().to_hex();
    let player_hex = player_keys.public_key().to_hex();
    let event = build_game_definition_event(
        "play.example.com",
        2222,
        vec![claimed_slot(
            1,
            &sha256_hex("amber-river"),
            &other_player_hex,
        )],
    );

    let err = select_discovered_game_from_events(
        [&event],
        &target(),
        "amber-river",
        Some(player_hex.as_str()),
    )
    .expect_err("claimed invite should report that another player took it");

    assert!(err.contains("claimed by another player"));
    assert!(err.contains("reissue the seat"));
}
