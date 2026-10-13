use nc_data::AccountIdentity;
use nc_data::matrix_login::{
    FAILURE_LIMIT, Issue, Issued, LIFETIME_SECS, REQUEST_INTERVAL_SECS, Redeem, ensure, issue,
    redeem,
};
use rusqlite::Connection;

const T: i64 = 1_800_000_000;

fn store() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    ensure(&conn).unwrap();
    conn
}
fn alice() -> AccountIdentity {
    AccountIdentity::Matrix("@alice:matrix.org".into())
}
fn issued(conn: &Connection, account: &AccountIdentity, now: i64) -> Issued {
    match issue(conn, account, "Alice", now).unwrap() {
        Issue::Issued(issued) => issued,
        Issue::TooSoon => panic!("unexpected throttle"),
    }
}
fn signed_in(account: AccountIdentity) -> Redeem {
    Redeem::SignedIn {
        account,
        display_name: "Alice".into(),
    }
}

#[test]
fn code_or_link_signs_in_once_and_consumes_both() {
    let conn = store();
    let first = issued(&conn, &alice(), T);
    assert_eq!(first.code.len(), 8);
    assert_eq!(first.link_secret.len(), 64);
    let typed = format!("  {} ", first.code.to_ascii_lowercase());
    assert_eq!(redeem(&conn, &typed, T + 1).unwrap(), signed_in(alice()));
    assert_eq!(redeem(&conn, &first.code, T + 2).unwrap(), Redeem::Invalid);
    assert_eq!(
        redeem(&conn, &first.link_secret, T + 2).unwrap(),
        Redeem::Invalid
    );
    let second = issued(&conn, &alice(), T + REQUEST_INTERVAL_SECS);
    assert_eq!(
        redeem(&conn, &second.link_secret, T + REQUEST_INTERVAL_SECS).unwrap(),
        signed_in(alice())
    );
    assert_eq!(
        redeem(&conn, &second.code, T + REQUEST_INTERVAL_SECS).unwrap(),
        Redeem::Invalid
    );
}

#[test]
fn codes_expire_after_ten_minutes() {
    let conn = store();
    let first = issued(&conn, &alice(), T);
    assert_eq!(
        redeem(&conn, &first.code, T + LIFETIME_SECS).unwrap(),
        Redeem::Invalid
    );
    let second = issued(&conn, &alice(), T + LIFETIME_SECS);
    assert_eq!(
        redeem(&conn, &second.code, T + 2 * LIFETIME_SECS - 1).unwrap(),
        signed_in(alice())
    );
}

#[test]
fn newer_request_cancels_older_code_but_rapid_requests_are_ignored() {
    let conn = store();
    let first = issued(&conn, &alice(), T);
    assert_eq!(
        issue(&conn, &alice(), "Alice", T + REQUEST_INTERVAL_SECS - 1).unwrap(),
        Issue::TooSoon
    );
    let second = issued(&conn, &alice(), T + REQUEST_INTERVAL_SECS);
    assert_ne!(first, second);
    assert_eq!(
        redeem(&conn, &first.code, T + REQUEST_INTERVAL_SECS).unwrap(),
        Redeem::Invalid
    );
    assert_eq!(
        redeem(&conn, &first.link_secret, T + REQUEST_INTERVAL_SECS).unwrap(),
        Redeem::Invalid
    );
    // Another user's request is independent.
    let bob = AccountIdentity::Matrix("@bob:example.org".into());
    let other = issued(&conn, &bob, T + REQUEST_INTERVAL_SECS);
    assert_eq!(
        redeem(&conn, &other.code, T + REQUEST_INTERVAL_SECS).unwrap(),
        Redeem::SignedIn {
            account: bob,
            display_name: "Alice".into()
        }
    );
    assert_eq!(
        redeem(&conn, &second.code, T + REQUEST_INTERVAL_SECS).unwrap(),
        signed_in(alice())
    );
}

#[test]
fn failed_guesses_throttle_redemption_site_wide_for_a_window() {
    let conn = store();
    let pending = issued(&conn, &alice(), T);
    for _ in 0..FAILURE_LIMIT {
        assert_eq!(redeem(&conn, "WRONG234", T).unwrap(), Redeem::Invalid);
    }
    assert_eq!(redeem(&conn, &pending.code, T).unwrap(), Redeem::Throttled);
    assert_eq!(
        redeem(&conn, &pending.code, T + LIFETIME_SECS - 1).unwrap(),
        Redeem::Throttled
    );
    let fresh = issued(&conn, &alice(), T + LIFETIME_SECS);
    assert_eq!(
        redeem(&conn, &fresh.code, T + LIFETIME_SECS).unwrap(),
        signed_in(alice())
    );
}

#[test]
fn only_matrix_identities_receive_codes() {
    let conn = store();
    assert!(issue(&conn, &AccountIdentity::Discord("123".into()), "X", T).is_err());
}
