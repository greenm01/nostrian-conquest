//! Decides what a `/sync` response asks of the bot. No network or storage here.
use serde_json::Value;
use std::collections::HashMap;

/// What the bot knows about its rooms, rebuilt from the first sync.
pub struct Rooms {
    pub bot: String,
    pub community: String,
    /// Each user's private room with the bot, which the bot created.
    pub direct: HashMap<String, String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    /// Send this user a sign-in code in their private room.
    Request(String),
    /// Decline an invitation; the bot only talks in rooms it creates.
    Reject(String),
    /// Leave a private room the user has left.
    Leave(String),
}

/// The first sync after start only rebuilds room state, so old messages and
/// invitations never produce codes.
pub fn plan(rooms: &mut Rooms, sync: &Value, initial: bool) -> Vec<Action> {
    let mut actions = Vec::new();
    for (room, data) in object(&sync["rooms"]["join"]) {
        let events = list(&data["state"]["events"]).chain(list(&data["timeline"]["events"]));
        for event in events {
            let sender = event["sender"].as_str().unwrap_or_default();
            match event["type"].as_str() {
                Some("m.room.member") if room != &rooms.community => {
                    let Some(user) = event["state_key"].as_str() else {
                        continue;
                    };
                    if user == rooms.bot {
                        continue;
                    }
                    match event["content"]["membership"].as_str() {
                        Some("join" | "invite") => {
                            rooms.direct.insert(user.into(), room.clone());
                        }
                        Some("leave" | "ban") => {
                            if rooms.direct.get(user) == Some(room) {
                                rooms.direct.remove(user);
                            }
                            actions.push(Action::Leave(room.clone()));
                        }
                        _ => (),
                    }
                }
                Some("m.room.message") if !initial && sender != rooms.bot => {
                    let asked = if room == &rooms.community {
                        event["content"]["body"]
                            .as_str()
                            .is_some_and(|body| body.trim().eq_ignore_ascii_case("!login"))
                    } else {
                        rooms.direct.get(sender) == Some(room)
                    };
                    if asked {
                        actions.push(Action::Request(sender.into()));
                    }
                }
                _ => (),
            }
        }
    }
    for (room, data) in object(&sync["rooms"]["invite"]) {
        let inviter = list(&data["invite_state"]["events"]).find_map(|event| {
            (event["type"] == "m.room.member"
                && event["state_key"] == rooms.bot.as_str()
                && event["content"]["membership"] == "invite")
                .then(|| event["sender"].as_str())
                .flatten()
        });
        actions.push(Action::Reject(room.clone()));
        if let Some(inviter) = inviter.filter(|_| !initial) {
            actions.push(Action::Request(inviter.into()));
        }
    }
    for (room, _) in object(&sync["rooms"]["leave"]) {
        rooms.direct.retain(|_, direct| direct != room);
    }
    let mut unique = Vec::new();
    for action in actions {
        if !unique.contains(&action) {
            unique.push(action);
        }
    }
    unique
}

fn object(value: &Value) -> impl Iterator<Item = (&String, &Value)> {
    value.as_object().into_iter().flatten()
}
fn list(value: &Value) -> impl Iterator<Item = &Value> {
    value.as_array().into_iter().flatten()
}
