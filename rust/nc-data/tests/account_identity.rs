use nc_data::AccountIdentity;

#[test]
fn identity_storage_roundtrips_and_rejects_unknown_or_malformed_providers() {
    for identity in [
        AccountIdentity::Discord("123".into()),
        AccountIdentity::Nostr("a".repeat(64)),
        AccountIdentity::Matrix("@alice:matrix.org".into()),
        AccountIdentity::Matrix("@Old_Name=1:[::1]:8448".into()),
    ] {
        assert_eq!(
            AccountIdentity::from_storage_key(&identity.storage_key()).unwrap(),
            identity
        );
        assert_eq!(
            serde_json::from_value::<AccountIdentity>(serde_json::to_value(&identity).unwrap())
                .unwrap(),
            identity
        );
    }
    for value in [
        "123",
        "discord:",
        "discord:nostr:123",
        "discord:abc",
        "nostr:123",
        "nostr:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        "unknown:123",
        "matrix:",
        "matrix:alice:matrix.org",
        "matrix:@alice",
        "matrix:@:matrix.org",
        "matrix:@alice:",
        "matrix:@al ice:matrix.org",
        "matrix:@alice:matrix.org/evil",
    ] {
        assert!(AccountIdentity::from_storage_key(value).is_err(), "{value}");
    }
    assert!(
        serde_json::from_str::<AccountIdentity>(r#"{"provider":"unknown","id":"123"}"#).is_err()
    );
    // 255 bytes is the longest valid Matrix user ID.
    let key = |local: usize| format!("matrix:@{}:matrix.org", "a".repeat(local));
    assert!(AccountIdentity::from_storage_key(&key(243)).is_ok());
    assert!(AccountIdentity::from_storage_key(&key(244)).is_err());
}
