use nc_matrix::sync::{Action, Rooms, plan};
use serde_json::{Value, json};

const BOT: &str = "@bot:matrix.org";
const LOBBY: &str = "!lobby:matrix.org";

fn rooms() -> Rooms {
    Rooms {
        bot: BOT.into(),
        community: LOBBY.into(),
        direct: Default::default(),
    }
}
fn message(sender: &str, body: &str) -> Value {
    json!({"type":"m.room.message","sender":sender,"content":{"msgtype":"m.text","body":body}})
}
fn member(user: &str, membership: &str) -> Value {
    json!({"type":"m.room.member","sender":user,"state_key":user,"content":{"membership":membership}})
}
fn joined(room: &str, state: Vec<Value>, timeline: Vec<Value>) -> Value {
    json!({"rooms":{"join":{room:{"state":{"events":state},"timeline":{"events":timeline}}}}})
}
fn invite(room: &str, inviter: &str) -> Value {
    json!({"rooms":{"invite":{room:{"invite_state":{"events":[
        {"type":"m.room.member","sender":inviter,"state_key":BOT,"content":{"membership":"invite"}}
    ]}}}}})
}

#[test]
fn login_command_in_community_room_requests_a_code_once() {
    let mut rooms = rooms();
    let sync = joined(
        LOBBY,
        vec![],
        vec![
            message("@alice:matrix.org", "  !LOGIN "),
            message("@alice:matrix.org", "!login"),
            message("@bob:example.org", "hello everyone"),
            message("@carol:example.org", "!login please"),
            message(BOT, "!login"),
        ],
    );
    assert_eq!(
        plan(&mut rooms, &sync, false),
        vec![Action::Request("@alice:matrix.org".into())]
    );
}

#[test]
fn games_and_help_commands_in_community_room() {
    let mut rooms = rooms();
    let sync = joined(
        LOBBY,
        vec![],
        vec![
            message("@alice:matrix.org", " !GAMES "),
            message("@bob:example.org", "!games"),
            message("@bob:example.org", "!help"),
            message("@carol:example.org", "!games please"),
            message(BOT, "!help"),
        ],
    );
    assert_eq!(
        plan(&mut rooms, &sync, false),
        vec![Action::Games, Action::Help]
    );
    assert_eq!(plan(&mut rooms, &sync, true), vec![]);
}

#[test]
fn invitation_is_declined_and_its_sender_gets_a_code() {
    let mut rooms = rooms();
    assert_eq!(
        plan(
            &mut rooms,
            &invite("!encrypted:matrix.org", "@alice:matrix.org"),
            false
        ),
        vec![
            Action::Reject("!encrypted:matrix.org".into()),
            Action::Request("@alice:matrix.org".into())
        ]
    );
}

#[test]
fn any_message_in_the_users_private_room_requests_a_code() {
    let mut rooms = rooms();
    let room = "!private:matrix.org";
    let sync = joined(
        room,
        vec![member(BOT, "join"), member("@alice:matrix.org", "join")],
        vec![message("@alice:matrix.org", "sign me in")],
    );
    assert_eq!(
        plan(&mut rooms, &sync, false),
        vec![Action::Request("@alice:matrix.org".into())]
    );
    assert_eq!(rooms.direct["@alice:matrix.org"], room);
    // A stranger's message in someone else's private room is ignored.
    let sync = joined(room, vec![], vec![message("@mallory:evil.example", "hi")]);
    assert!(plan(&mut rooms, &sync, false).is_empty());
}

#[test]
fn first_sync_rebuilds_rooms_without_answering_old_requests() {
    let mut rooms = rooms();
    let mut sync = joined(
        "!private:matrix.org",
        vec![member("@alice:matrix.org", "join")],
        vec![message("@alice:matrix.org", "old message")],
    );
    sync["rooms"]["join"][LOBBY] =
        json!({"timeline":{"events":[message("@bob:example.org","!login")]}});
    sync["rooms"]["invite"] =
        invite("!old:matrix.org", "@carol:example.org")["rooms"]["invite"].clone();
    assert_eq!(
        plan(&mut rooms, &sync, true),
        vec![Action::Reject("!old:matrix.org".into())]
    );
    assert_eq!(rooms.direct["@alice:matrix.org"], "!private:matrix.org");
}

#[test]
fn bot_leaves_and_forgets_a_private_room_the_user_left() {
    let mut rooms = rooms();
    let room = "!private:matrix.org";
    plan(
        &mut rooms,
        &joined(room, vec![member("@alice:matrix.org", "invite")], vec![]),
        false,
    );
    assert_eq!(rooms.direct["@alice:matrix.org"], room);
    assert_eq!(
        plan(
            &mut rooms,
            &joined(room, vec![], vec![member("@alice:matrix.org", "leave")]),
            false
        ),
        vec![Action::Leave(room.into())]
    );
    assert!(rooms.direct.is_empty());
}
