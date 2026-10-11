use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::config::{host_config::HostConfig, relay::RelayConfig};
use crate::status::model::{
    DaemonStatusReport, DaemonStatusTotals, GameStatusRow, RelayStatusReport,
};
use crate::support::paths::hosted_db_path;
use crate::support::time::unix_now;
use nc_data::hosted::{
    HostedStore, OutboxStatus, RecruitingMode, count_by_status, count_pending_requests,
    count_pending_turns, count_unpublished_decisions, get_game_metadata, get_settings, list_seats,
};
use nostr_sdk::{Client, RelayStatus};

pub fn collect_status(
    config: &HostConfig,
    config_path: Option<&Path>,
) -> Result<DaemonStatusReport, Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async { collect_status_async(config, config_path).await })
}

async fn collect_status_async(
    config: &HostConfig,
    config_path: Option<&Path>,
) -> Result<DaemonStatusReport, Box<dyn std::error::Error>> {
    let relay = probe_relay(&config.relay_url).await;
    let mut games = discover_games(&config.games_root)?;
    games.sort_by(|a, b| a.game_id.cmp(&b.game_id));

    let mut totals = DaemonStatusTotals::default();
    for game in &games {
        totals.discovered_games += 1;
        if game.catalog_state == "listed"
            && game.lobby_visibility == "public"
            && game.recruiting != RecruitingMode::None.as_str()
        {
            totals.public_recruiting_games += 1;
        }
        if game.maintenance_due_now {
            totals.due_maintenance_games += 1;
        }
        totals.pending_requests += game.pending_requests;
        totals.pending_decisions += game.pending_decisions;
        totals.pending_turns += game.pending_turns;
        totals.outbox_pending += game.outbox_pending;
        totals.outbox_failed += game.outbox_failed;
    }

    Ok(DaemonStatusReport {
        generated_at: unix_now(),
        config_path: config_path.map(|path| path.display().to_string()),
        games_root: config.games_root.display().to_string(),
        relay,
        totals,
        games,
    })
}

pub fn collect_game_status(game_dir: &Path) -> Result<GameStatusRow, Box<dyn std::error::Error>> {
    let db_path = hosted_db_path(game_dir);
    let store = HostedStore::open(&db_path)?;
    let game_id = game_dir
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("invalid game directory name")?;
    build_game_status_row(game_id.to_string(), game_dir.to_path_buf(), &store)
}

fn discover_games(games_root: &Path) -> Result<Vec<GameStatusRow>, Box<dyn std::error::Error>> {
    let mut games = Vec::new();

    if let Ok(entries) = std::fs::read_dir(games_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let db_path = hosted_db_path(&path);
            if !db_path.exists() {
                continue;
            }

            let Some(game_id) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };

            match HostedStore::open(&db_path) {
                Ok(store) => match build_game_status_row(game_id.to_string(), path.clone(), &store)
                {
                    Ok(game) => games.push(game),
                    Err(err) => tracing::warn!("Failed to collect status for {}: {}", game_id, err),
                },
                Err(err) => tracing::warn!("Failed to open hosted store for {}: {}", game_id, err),
            }
        }
    }

    Ok(games)
}

fn build_game_status_row(
    game_id: String,
    dir: PathBuf,
    store: &HostedStore,
) -> Result<GameStatusRow, Box<dyn std::error::Error>> {
    let metadata = get_game_metadata(store.connection(), &game_id)?;
    let settings = get_settings(store.connection(), &game_id)?;
    let seats = list_seats(store.connection(), &game_id)?;
    let claimed_seats = seats
        .iter()
        .filter(|seat| seat.status == nc_data::hosted::SeatStatus::Claimed)
        .count() as u32;
    let open_seats = seats
        .iter()
        .filter(|seat| seat.status == nc_data::hosted::SeatStatus::Pending)
        .count() as u32;
    let due_seconds = settings.maintenance_next_due_unix_seconds;
    let now = chrono::Utc::now().timestamp();
    let maintenance_due_now =
        settings.maintenance_enabled && due_seconds.is_some_and(|due| now >= due);

    Ok(GameStatusRow {
        game_id,
        dir: dir.display().to_string(),
        name: metadata.name,
        status: metadata.status,
        year: metadata.current_year,
        turn: metadata.current_turn,
        players: metadata.players,
        claimed_seats,
        open_seats,
        recruiting: settings.recruiting.as_str().to_string(),
        lobby_visibility: settings.lobby_visibility.as_str().to_string(),
        catalog_state: settings.catalog_state.as_str().to_string(),
        maintenance_enabled: settings.maintenance_enabled,
        maintenance_due_unix_seconds: due_seconds,
        maintenance_due_now,
        pending_requests: count_pending_requests(store.connection(), &metadata.id)?,
        pending_decisions: count_unpublished_decisions(store.connection(), &metadata.id)?,
        pending_turns: count_pending_turns(store.connection(), &metadata.id)?,
        outbox_pending: count_by_status(store.connection(), &metadata.id, OutboxStatus::Pending)?,
        outbox_failed: count_by_status(store.connection(), &metadata.id, OutboxStatus::Failed)?,
    })
}

async fn probe_relay(relay_url: &str) -> RelayStatusReport {
    if let Err(err) = RelayConfig::validate(relay_url) {
        return RelayStatusReport {
            url: relay_url.to_string(),
            configured: false,
            reachable: false,
            status: "invalid".to_string(),
            latency_ms: None,
            error: Some(err.to_string()),
        };
    }

    let started = Instant::now();
    let client = Client::builder().build();
    if let Err(err) = client.add_relay(relay_url).await {
        return RelayStatusReport {
            url: relay_url.to_string(),
            configured: true,
            reachable: false,
            status: "add-relay-failed".to_string(),
            latency_ms: None,
            error: Some(err.to_string()),
        };
    }

    client.connect().await;
    client.wait_for_connection(Duration::from_secs(2)).await;

    let relay = match client.relay(relay_url).await {
        Ok(relay) => relay,
        Err(err) => {
            client.disconnect().await;
            return RelayStatusReport {
                url: relay_url.to_string(),
                configured: true,
                reachable: false,
                status: "missing".to_string(),
                latency_ms: None,
                error: Some(err.to_string()),
            };
        }
    };

    let status = relay.status();
    let reachable = matches!(status, RelayStatus::Connected) || relay.is_connected();
    let latency_ms = reachable.then(|| started.elapsed().as_millis());
    let error = (!reachable).then(|| format!("relay status: {}", status));
    client.disconnect().await;

    RelayStatusReport {
        url: relay_url.to_string(),
        configured: true,
        reachable,
        status: status.to_string().to_ascii_lowercase(),
        latency_ms,
        error,
    }
}
