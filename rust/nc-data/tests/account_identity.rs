use nc_data::AccountIdentity;

#[test]
fn identity_storage_roundtrips_and_rejects_unknown_or_malformed_providers() {
    for identity in [
        AccountIdentity::Discord("123".into()),
        AccountIdentity::Nostr("a".repeat(64)),
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
    ] {
        assert!(AccountIdentity::from_storage_key(value).is_err(), "{value}");
    }
    assert!(
        serde_json::from_str::<AccountIdentity>(r#"{"provider":"unknown","id":"123"}"#).is_err()
    );
}
