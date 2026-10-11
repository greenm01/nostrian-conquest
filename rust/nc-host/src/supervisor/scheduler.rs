//! When hosted maintenance is due. The serve loop polls `check_due_games`
//! and hands each due game to its worker, which runs the year and moves
//! the next due time one interval past now.

use crate::game::effects::GameEffects;
use crate::game::maint::next_due_from_now;
use crate::game::msg::GameMsg;
use crate::supervisor::worker_registry::WorkerRegistry;
use nc_data::hosted::{self, HostedStore};
use std::path::{Path, PathBuf};

pub struct Scheduler {
    games_root: PathBuf,
}

impl Scheduler {
    pub fn new(games_root: PathBuf) -> Self {
        Self { games_root }
    }

    /// The games whose maintenance is due. A game that has never been
    /// scheduled is armed one interval from now instead of running at once,
    /// so a new game gets a full interval before its first year.
    pub fn check_due_games(&self) -> Vec<String> {
        let mut due_games = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&self.games_root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let db_path = path.join("hosted.db");
                    if db_path.exists() {
                        if let Some(game_id) = path.file_name().and_then(|n| n.to_str()) {
                            if self.is_due(game_id, &db_path) {
                                due_games.push(game_id.to_string());
                            }
                        }
                    }
                }
            }
        }

        due_games
    }

    fn is_due(&self, game_id: &str, db_path: &Path) -> bool {
        let store = match HostedStore::open(db_path) {
            Ok(s) => s,
            Err(_) => return false,
        };

        let mut settings = match hosted::get_settings(store.connection(), game_id) {
            Ok(s) => s,
            Err(_) => return false,
        };

        if !settings.maintenance_enabled {
            return false;
        }

        match settings.maintenance_next_due_unix_seconds {
            Some(due) => chrono::Utc::now().timestamp() >= due,
            None => {
                settings.maintenance_next_due_unix_seconds =
                    Some(next_due_from_now(settings.maintenance_interval_minutes));
                if let Err(err) = hosted::update_settings(store.connection(), game_id, &settings) {
                    tracing::warn!("Failed to arm maintenance for {}: {}", game_id, err);
                }
                false
            }
        }
    }

    pub fn get_next_maintenance_time(&self, game_id: &str, db_path: &PathBuf) -> Option<i64> {
        let store = match HostedStore::open(db_path) {
            Ok(s) => s,
            Err(_) => return None,
        };

        let settings = match hosted::get_settings(store.connection(), game_id) {
            Ok(s) => s,
            Err(_) => return None,
        };

        if !settings.maintenance_enabled {
            return None;
        }

        match settings.maintenance_next_due_unix_seconds {
            Some(due) => Some(due),
            None => Some(next_due_from_now(settings.maintenance_interval_minutes)),
        }
    }
}

/// Send each due game's maintenance to its worker, so it runs in turn with
/// that game's other work.
pub async fn dispatch_due_maintenance(scheduler: &Scheduler, registry: &WorkerRegistry) {
    for game_id in scheduler.check_due_games() {
        tracing::info!("Maintenance due for {}", game_id);
        let worker = registry.get_or_create(game_id.clone()).await;
        let msg = GameMsg::HandleEffect(GameEffects::RunMaintenance {
            game_id: game_id.clone(),
        });
        if let Err(err) = worker.send(msg).await {
            tracing::error!(
                "Failed to send maintenance for {} to its worker: {}",
                game_id,
                err
            );
        }
    }
}
