use nc_data::AccountIdentity;
use nc_data::matrix_login::{
    INVITE_LIMIT, REQUEST_COOLDOWN_SECS, Request, ensure, may_invite, record_invite, request,
    take_requests,
};
use rusqlite::Connection;

const T: i64 = 1_800_000_000;

fn store() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    ensure(&conn).unwrap();
    conn
}
fn user(name: &str) -> AccountIdentity {
    AccountIdentity::Matrix(format!("@{name}:example.org"))
}

#[test]
fn each_request_is_delivered_once_and_cooled_down_for_ten_minutes() {
    let conn = store();
    assert_eq!(request(&conn, &user("alice"), T).unwrap(), Request::Queued);
    assert_eq!(request(&conn, &user("bob"), T).unwrap(), Request::Queued);
    let mut taken = take_requests(&conn, T + 1).unwrap();
    taken.sort_by_key(|a| a.storage_key());
    assert_eq!(taken, [user("alice"), user("bob")]);
    assert!(take_requests(&conn, T + 2).unwrap().is_empty());
    assert_eq!(
        request(&conn, &user("alice"), T + REQUEST_COOLDOWN_SECS - 1).unwrap(),
        Request::TooSoon
    );
    assert_eq!(
        request(&conn, &user("alice"), T + REQUEST_COOLDOWN_SECS).unwrap(),
        Request::Queued
    );
    assert_eq!(
        take_requests(&conn, T + REQUEST_COOLDOWN_SECS).unwrap(),
        [user("alice")]
    );
}

#[test]
fn requests_the_bot_misses_for_five_minutes_lapse() {
    let conn = store();
    request(&conn, &user("alice"), T).unwrap();
    assert!(take_requests(&conn, T + 300).unwrap().is_empty());
    // Still cooled down: a lapsed request does not allow an immediate retry.
    assert_eq!(
        request(&conn, &user("alice"), T + 301).unwrap(),
        Request::TooSoon
    );
}

#[test]
fn new_private_rooms_for_site_requests_are_capped_per_hour() {
    let conn = store();
    for _ in 0..INVITE_LIMIT {
        assert!(may_invite(&conn, T).unwrap());
        record_invite(&conn, T).unwrap();
    }
    assert!(!may_invite(&conn, T + 3599).unwrap());
    assert!(may_invite(&conn, T + 3600).unwrap());
}

#[test]
fn only_matrix_identities_can_be_requested() {
    let conn = store();
    assert!(request(&conn, &AccountIdentity::Discord("123".into()), T).is_err());
}

/// The bridge queues requests while the bot takes them, each through its
/// own connection to one file (t205): every request is delivered once.
#[test]
fn requests_queued_while_the_bot_takes_them_are_each_delivered_once() {
    let dir = std::env::temp_dir().join(format!("nc-matrix-requests-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(".web-sessions.db");
    ensure(&nc_data::sqlite::open_for_writes(&path).unwrap()).unwrap();
    let bridges: Vec<_> = (0..4)
        .map(|bridge| {
            let path = path.clone();
            std::thread::spawn(move || {
                let conn = nc_data::sqlite::open_for_writes(&path).unwrap();
                for n in 0..50 {
                    let account = user(&format!("b{bridge}-{n}"));
                    assert_eq!(request(&conn, &account, T).unwrap(), Request::Queued);
                }
            })
        })
        .collect();
    let bot = {
        let path = path.clone();
        std::thread::spawn(move || {
            let conn = nc_data::sqlite::open_for_writes(&path).unwrap();
            let mut taken = Vec::new();
            while taken.len() < 200 {
                taken.extend(take_requests(&conn, T + 1).unwrap());
            }
            taken
        })
    };
    for bridge in bridges {
        bridge.join().unwrap();
    }
    let taken = bot.join().unwrap();
    let mut keys: Vec<String> = taken.iter().map(|a| a.storage_key()).collect();
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), 200, "each request delivered once");
    assert_eq!(taken.len(), 200);
    let _ = std::fs::remove_dir_all(&dir);
}
