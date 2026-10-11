use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use nc_data::{QueuedPlayerMail, append_mail_queue, clear_mail_queue, load_mail_queue};

fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ec-player-mail-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time ok")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[test]
fn player_mail_queue_round_trips_multiline_body() {
    let dir = temp_dir();
    let mail = QueuedPlayerMail {
        sender_empire_id: 1,
        recipient_empire_id: 2,
        year: 3000,
        subject: "hello".to_string(),
        body: "hello there\nsecond line".to_string(),
        recipient_deleted: true,
        sender_deleted: false,
        cancelled: false,
    };

    append_mail_queue(&dir, &mail).expect("append queue");
    let loaded = load_mail_queue(&dir).expect("load queue");
    assert_eq!(loaded, vec![mail]);

    clear_mail_queue(&dir).expect("clear queue");
    assert!(load_mail_queue(&dir).expect("load empty").is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}
