//! Local, transport-independent operations shared by nc-admin and the bridge.
mod account_links;
mod announcements;
mod campaign;
mod exports;
mod handles;
mod invites;
mod maintenance;
mod recovery;
mod scheduling;
mod seats;
pub use scheduling::next_local_midnight;

pub use nc_data::AccountIdentity;
use nc_data::hosted::HostedStore;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::Read;
use std::path::{Path, PathBuf};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    InviteOnly {
        game: String,
    },
    CreateInvites {
        game: String,
        count: u8,
    },
    Membership {
        game: String,
        account: AccountIdentity,
    },
    ListInvites {
        game: String,
    },
    RevokeInvite {
        game: String,
        invite_id: String,
    },
    ClaimInvite {
        game: String,
        account: AccountIdentity,
        code: String,
        /// The claimant's public name, which becomes the locked player handle.
        #[serde(default)]
        display_name: Option<String>,
    },
    ConfigureSchedule {
        game: String,
        time_zone: Option<String>,
    },
    StartConfigured {
        game: String,
    },
    SyncPlayerHandle {
        game: String,
        account: AccountIdentity,
        display_name: String,
    },
    List,
    Catalog,
    Announcements,
    AcknowledgeAnnouncement {
        game: String,
        id: String,
    },
    Export {
        game: String,
        account: AccountIdentity,
        format: String,
    },
    VerifyBackup {
        path: String,
    },
    Restore {
        path: String,
        game: String,
    },
    Inspect {
        game: String,
    },
    Create {
        game: String,
        name: String,
        players: u8,
        seed: Option<u64>,
    },
    Publish {
        game: String,
    },
    Start {
        game: String,
        first_due: i64,
        interval_minutes: u32,
    },
    Pause {
        game: String,
    },
    Resume {
        game: String,
        first_due: i64,
    },
    Archive {
        game: String,
    },
    Schedule {
        game: String,
        first_due: i64,
        interval_minutes: u32,
    },
    Join {
        game: String,
        account: AccountIdentity,
        #[serde(default)]
        display_name: Option<String>,
    },
    Rename {
        game: String,
        seat: u32,
        handle: String,
    },
    Authorize {
        game: String,
        account: AccountIdentity,
    },
    Replace {
        game: String,
        seat: u32,
        account: AccountIdentity,
    },
    LinkAccount {
        game: String,
        seat: u32,
        account: AccountIdentity,
    },
    UnlinkAccount {
        game: String,
        seat: u32,
        account: AccountIdentity,
    },
    Maintain {
        game: String,
        expected_year: u16,
        request_id: String,
    },
    Due,
    DailyBackups,
    Backup {
        game: String,
    },
    History {
        game: String,
    },
}

pub fn cli(args: &[&str]) -> std::result::Result<(), Box<dyn std::error::Error>> {
    if args.len() != 2 || args[0] != "--root" {
        return Err("usage: nc-host admin --root <games-root> (JSON request on stdin)".into());
    }
    let mut input = String::new();
    std::io::stdin().take(65537).read_to_string(&mut input)?;
    let result = if input.len() > 65536 {
        Err("request too large".into())
    } else {
        serde_json::from_str::<Request>(&input)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            .and_then(|request| execute(Path::new(args[1]), request))
    };
    match result {
        Ok(data) => {
            println!("{}", json!({"version":1,"ok":true,"data":data}));
            Ok(())
        }
        Err(error) => {
            println!(
                "{}",
                json!({"version":1,"ok":false,"error":error.to_string()})
            );
            Err(error.to_string().into())
        }
    }
}

pub fn execute(root: &Path, request: Request) -> Result<Value> {
    if !root.exists() {
        if matches!(&request, Request::Create { .. } | Request::Restore { .. }) {
            std::fs::create_dir_all(root)?;
        } else if matches!(&request, Request::List) {
            return Ok(json!([]));
        } else {
            return Err("games root does not exist".into());
        }
    }
    let root = root.canonicalize()?;
    // Reads that sessions make when a terminal opens need no lock, so they
    // answer at once while a turn runs under it (t205); a game in
    // maintenance cannot be played, so its sync is refused at once too.
    match &request {
        Request::Authorize { game, account } => return seats::authorize(&root, game, account),
        Request::Membership { game, account } => return seats::membership(&root, game, account),
        Request::SyncPlayerHandle { game, account, .. }
            if seats::authorize(&root, game, account)?["can_play"] != true =>
        {
            return Err("campaign is not accepting sessions".into());
        }
        _ => {}
    }
    // Administrative changes cannot interleave across processes. Player writes
    // use SQLite revision checks and the per-game write gate instead.
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(root.join(".admin.lock"))?;
    lock.lock()?;
    match request {
        Request::InviteOnly { game } => invites::restrict(&root, &game),
        Request::CreateInvites { game, count } => invites::create(&root, &game, count),
        Request::ListInvites { game } => invites::list(&root, &game),
        Request::RevokeInvite { game, invite_id } => invites::revoke(&root, &game, &invite_id),
        Request::ClaimInvite {
            game,
            account,
            code,
            display_name,
        } => seats::join(
            &root,
            &game,
            &account,
            None,
            Some(&code),
            display_name.as_deref(),
        ),
        Request::ConfigureSchedule { game, time_zone } => {
            scheduling::configure(&root, &game, time_zone.as_deref())
        }
        Request::StartConfigured { game } => scheduling::start(&root, &game),
        Request::SyncPlayerHandle {
            game,
            account,
            display_name,
        } => handles::on_play(&root, &game, &account, &display_name),
        Request::Membership { game, account } => seats::membership(&root, &game, &account),
        Request::List => campaign::list(&root),
        Request::Catalog => campaign::catalog(&root),
        Request::Announcements => announcements::pending(&root),
        Request::AcknowledgeAnnouncement { game, id } => {
            announcements::acknowledge(&root, &game, &id)
        }
        Request::Export {
            game,
            account,
            format,
        } => exports::render(&root, &game, &account, &format),
        Request::VerifyBackup { path } => recovery::verify(Path::new(&path)),
        Request::Restore { path, game } => recovery::restore(&root, Path::new(&path), &game),
        Request::Inspect { game } => campaign::inspect(&root, &game),
        Request::Create {
            game,
            name,
            players,
            seed,
        } => campaign::create(&root, &game, &name, players, seed),
        Request::Publish { game } => campaign::transition(&root, &game, "recruiting", None, None),
        Request::Start {
            game,
            first_due,
            interval_minutes,
        } => campaign::transition(
            &root,
            &game,
            "running",
            Some(first_due),
            Some(interval_minutes),
        ),
        Request::Pause { game } => campaign::transition(&root, &game, "paused", None, None),
        Request::Resume { game, first_due } => {
            campaign::transition(&root, &game, "running", Some(first_due), None)
        }
        Request::Archive { game } => campaign::transition(&root, &game, "archived", None, None),
        Request::Schedule {
            game,
            first_due,
            interval_minutes,
        } => campaign::transition(
            &root,
            &game,
            "running",
            Some(first_due),
            Some(interval_minutes),
        ),
        Request::Join {
            game,
            account,
            display_name,
        } => seats::join(&root, &game, &account, None, None, display_name.as_deref()),
        Request::Rename { game, seat, handle } => handles::rename(&root, &game, seat, &handle),
        Request::Replace {
            game,
            seat,
            account,
        } => seats::join(&root, &game, &account, Some(seat), None, None),
        Request::Authorize { game, account } => seats::authorize(&root, &game, &account),
        Request::LinkAccount {
            game,
            seat,
            account,
        } => account_links::change(&root, &game, seat, &account, true),
        Request::UnlinkAccount {
            game,
            seat,
            account,
        } => account_links::change(&root, &game, seat, &account, false),
        Request::Maintain {
            game,
            expected_year,
            request_id,
        } => maintenance::run(&root, &game, expected_year, &request_id),
        Request::Due => maintenance::due(&root),
        Request::DailyBackups => maintenance::daily_backups(&root),
        Request::Backup { game } => maintenance::backup(&root, &game),
        Request::History { game } => maintenance::history(&root, &game),
    }
}

pub fn game_dir(root: &Path, game: &str) -> Result<PathBuf> {
    if game.is_empty()
        || game.len() > 64
        || !game
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err("game ID must be 1-64 lowercase letters, digits or hyphens".into());
    }
    let path = root.join(game);
    if path
        .symlink_metadata()
        .is_ok_and(|m| m.file_type().is_symlink())
    {
        return Err("game directory must not be a symlink".into());
    }
    Ok(path)
}

fn store(root: &Path, game: &str) -> Result<(PathBuf, HostedStore)> {
    let dir = game_dir(root, game)?;
    for name in ["hosted.db", "ncgame.db"] {
        let meta = dir.join(name).symlink_metadata()?;
        if !meta.is_file() || meta.file_type().is_symlink() {
            return Err("campaign database must be a regular file".into());
        }
    }
    let store = HostedStore::open(&dir.join("hosted.db"))?;
    let managed: bool = store.connection().query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='web_seats')",
        [],
        |r| r.get(0),
    )?;
    if !managed {
        return Err("campaign is not managed by nc-admin".into());
    }
    Ok((dir, store))
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}
