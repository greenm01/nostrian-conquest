use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeatReservation {
    pub player_record_index_1_based: usize,
    pub alias: String,
}

/// A game's `config.kdl`: the sysop's settings (t218), copied into the
/// campaign store at each login and maintenance run. The seat count, seed
/// and map are set by `nc-sysop new-game` and are not in the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BbsGameConfig {
    pub reservations: Vec<SeatReservation>,
    /// Absent: the store keeps the name it was created with.
    pub game_name: Option<String>,
    /// Absent: the store keeps its theme.
    pub theme_key: Option<String>,
    pub snoop: bool,
    pub session_max_idle_minutes: u8,
    pub session_minimum_time_minutes: u8,
    pub session_local_timeout: bool,
    pub session_remote_timeout: bool,
    /// ECUTIL F4 "Autopilot any empires inactive for"; 0 turns it off.
    pub autopilot_after_turns: u8,
    pub maintenance_enabled: bool,
    pub maintenance_interval_minutes: u32,
    pub ai_seats: Vec<crate::AiSeat>,
}

impl Default for BbsGameConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl BbsGameConfig {
    /// A config with every setting at its default.
    pub fn new() -> Self {
        let defaults = crate::CampaignSettings::new("", "");
        Self {
            reservations: Vec::new(),
            game_name: None,
            theme_key: None,
            snoop: defaults.snoop_enabled,
            session_max_idle_minutes: defaults.session_max_idle_minutes,
            session_minimum_time_minutes: defaults.session_minimum_time_minutes,
            session_local_timeout: defaults.session_local_timeout,
            session_remote_timeout: defaults.session_remote_timeout,
            autopilot_after_turns: defaults.inactivity_autopilot_after_turns,
            maintenance_enabled: defaults.maintenance_enabled,
            maintenance_interval_minutes: defaults.maintenance_interval_minutes,
            ai_seats: Vec::new(),
        }
    }

    /// The config that reproduces `settings`.
    pub fn from_settings(settings: &crate::CampaignSettings) -> Self {
        Self {
            reservations: settings.reservations.clone(),
            game_name: Some(settings.game_name.clone()),
            theme_key: Some(settings.default_theme_key.clone()),
            snoop: settings.snoop_enabled,
            session_max_idle_minutes: settings.session_max_idle_minutes,
            session_minimum_time_minutes: settings.session_minimum_time_minutes,
            session_local_timeout: settings.session_local_timeout,
            session_remote_timeout: settings.session_remote_timeout,
            autopilot_after_turns: settings.inactivity_autopilot_after_turns,
            maintenance_enabled: settings.maintenance_enabled,
            maintenance_interval_minutes: settings.maintenance_interval_minutes,
            ai_seats: settings.ai_seats.clone(),
        }
    }

    /// Copy the file's settings into `settings`, leaving the store's own
    /// state (slug, next maintenance time) alone.
    pub fn apply_to(&self, settings: &mut crate::CampaignSettings) {
        if let Some(name) = &self.game_name {
            settings.game_name = name.clone();
        }
        if let Some(theme) = &self.theme_key {
            settings.default_theme_key = theme.clone();
        }
        settings.snoop_enabled = self.snoop;
        settings.session_max_idle_minutes = self.session_max_idle_minutes;
        settings.session_minimum_time_minutes = self.session_minimum_time_minutes;
        settings.session_local_timeout = self.session_local_timeout;
        settings.session_remote_timeout = self.session_remote_timeout;
        settings.inactivity_autopilot_after_turns = self.autopilot_after_turns;
        settings.maintenance_enabled = self.maintenance_enabled;
        settings.maintenance_interval_minutes = self.maintenance_interval_minutes;
        settings.reservations = self.reservations.clone();
        settings.ai_seats = self.ai_seats.clone();
    }
}

#[derive(Debug)]
pub enum BbsGameConfigError {
    Io(std::io::Error),
    Parse(String),
}

impl fmt::Display for BbsGameConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(source) => write!(f, "{source}"),
            Self::Parse(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for BbsGameConfigError {}

impl From<std::io::Error> for BbsGameConfigError {
    fn from(source: std::io::Error) -> Self {
        Self::Io(source)
    }
}

impl BbsGameConfig {
    pub fn parse_kdl_str(input: &str) -> Result<Self, BbsGameConfigError> {
        let document: kdl::KdlDocument = input
            .parse()
            .map_err(|err| BbsGameConfigError::Parse(format!("invalid KDL: {err}")))?;

        for node in document.nodes() {
            match node.name().value() {
                "reservations" | "game_name" | "theme" | "snoop" | "session" | "inactivity"
                | "maintenance" | "ai_seat" => {}
                other => {
                    return Err(BbsGameConfigError::Parse(format!(
                        "unsupported field '{other}'"
                    )));
                }
            }
        }

        let reservations = if let Some(node) = document.get("reservations") {
            let mut reservations = Vec::new();
            if let Some(children) = node.children() {
                for child in children.nodes() {
                    if child.name().value() != "seat" {
                        return Err(BbsGameConfigError::Parse(format!(
                            "reservations only accepts seat children, found '{}'",
                            child.name().value()
                        )));
                    }
                    let player_record_index_1_based = prop_usize(child, "player")?;
                    let alias = prop_string(child, "alias")?.trim().to_string();
                    if alias.is_empty() {
                        return Err(BbsGameConfigError::Parse(
                            "reservation alias must contain at least one visible character"
                                .to_string(),
                        ));
                    }
                    reservations.push(SeatReservation {
                        player_record_index_1_based,
                        alias,
                    });
                }
            }
            reservations
        } else {
            Vec::new()
        };

        let mut config = Self::new();
        config.reservations = reservations;
        config.game_name = arg_string(&document, "game_name")?;
        config.theme_key = arg_string(&document, "theme")?;
        if let Some(value) = arg_bool(&document, "snoop")? {
            config.snoop = value;
        }
        if let Some(session) = block(
            &document,
            "session",
            &[
                "max_idle_minutes",
                "minimum_time_minutes",
                "local_timeout",
                "remote_timeout",
            ],
        )? {
            if let Some(value) = arg_u8(session, "max_idle_minutes", 120)? {
                config.session_max_idle_minutes = value;
            }
            if let Some(value) = arg_u8(session, "minimum_time_minutes", 120)? {
                config.session_minimum_time_minutes = value;
            }
            if let Some(value) = arg_bool(session, "local_timeout")? {
                config.session_local_timeout = value;
            }
            if let Some(value) = arg_bool(session, "remote_timeout")? {
                config.session_remote_timeout = value;
            }
        }
        if let Some(inactivity) = block(&document, "inactivity", &["autopilot_after_turns"])?
            && let Some(value) = arg_u8(inactivity, "autopilot_after_turns", 100)?
        {
            config.autopilot_after_turns = value;
        }
        if let Some(maintenance) =
            block(&document, "maintenance", &["enabled", "interval_minutes"])?
        {
            if let Some(value) = arg_bool(maintenance, "enabled")? {
                config.maintenance_enabled = value;
            }
            if let Some(node) = maintenance.get("interval_minutes") {
                let value = node
                    .get(0)
                    .and_then(|value| value.as_integer())
                    .ok_or_else(|| {
                        BbsGameConfigError::Parse(
                            "interval_minutes requires an integer".to_string(),
                        )
                    })?;
                config.maintenance_interval_minutes = u32::try_from(value)
                    .ok()
                    .filter(|minutes| *minutes > 0)
                    .ok_or_else(|| {
                        BbsGameConfigError::Parse(format!(
                            "interval_minutes must be a positive number of minutes, got {value}"
                        ))
                    })?;
            }
        }
        for node in document
            .nodes()
            .iter()
            .filter(|node| node.name().value() == "ai_seat")
        {
            let difficulty = match node.get("difficulty").and_then(|value| value.as_string()) {
                Some(text) => crate::AiDifficulty::parse(text)
                    .map_err(|err| BbsGameConfigError::Parse(err.to_string()))?,
                None => crate::AiDifficulty::Normal,
            };
            config.ai_seats.push(crate::AiSeat {
                player_record_index_1_based: prop_usize(node, "player")?,
                difficulty,
                personality: node
                    .get("personality")
                    .and_then(|value| value.as_string())
                    .unwrap_or(crate::DEFAULT_AI_PERSONALITY)
                    .to_string(),
            });
        }
        config.validate()
    }

    pub fn load_kdl(path: &Path) -> Result<Self, BbsGameConfigError> {
        let text = fs::read_to_string(path)?;
        Self::parse_kdl_str(&text)
    }

    pub fn to_kdl_string(&self) -> String {
        let mut out = String::new();
        out.push_str("// Sysop settings for this game. Edits take effect at the next login\n");
        out.push_str("// or maintenance run.\n");
        if let Some(name) = &self.game_name {
            out.push_str(&format!("game_name \"{}\"\n", kdl_escape(name)));
        }
        if let Some(theme) = &self.theme_key {
            out.push_str(&format!("theme \"{}\"\n", kdl_escape(theme)));
        }
        out.push_str(&format!("snoop {}\n", kdl_bool(self.snoop)));
        out.push_str("session {\n");
        out.push_str(&format!(
            "    max_idle_minutes {}\n",
            self.session_max_idle_minutes
        ));
        out.push_str(&format!(
            "    minimum_time_minutes {}\n",
            self.session_minimum_time_minutes
        ));
        out.push_str(&format!(
            "    local_timeout {}\n",
            kdl_bool(self.session_local_timeout)
        ));
        out.push_str(&format!(
            "    remote_timeout {}\n",
            kdl_bool(self.session_remote_timeout)
        ));
        out.push_str("}\n");
        out.push_str("inactivity {\n");
        out.push_str(
            "    // Put an empire on autopilot after this many missed turns; 0 never does.\n",
        );
        out.push_str(&format!(
            "    autopilot_after_turns {}\n",
            self.autopilot_after_turns
        ));
        out.push_str("}\n");
        out.push_str("maintenance {\n");
        out.push_str(&format!(
            "    enabled {}\n",
            kdl_bool(self.maintenance_enabled)
        ));
        out.push_str(&format!(
            "    interval_minutes {}\n",
            self.maintenance_interval_minutes
        ));
        out.push_str("}\n");
        for seat in &self.ai_seats {
            out.push_str(&format!(
                "ai_seat player={} difficulty=\"{}\" personality=\"{}\"\n",
                seat.player_record_index_1_based,
                seat.difficulty.as_str(),
                kdl_escape(&seat.personality),
            ));
        }
        if !self.reservations.is_empty() {
            out.push_str("reservations {\n");
            for reservation in &self.reservations {
                out.push_str(&format!(
                    "    seat player={} alias=\"{}\"\n",
                    reservation.player_record_index_1_based,
                    kdl_escape(&reservation.alias),
                ));
            }
            out.push_str("}\n");
        }
        out
    }

    pub fn save_kdl(&self, path: &Path) -> Result<(), BbsGameConfigError> {
        fs::write(path, self.to_kdl_string())?;
        Ok(())
    }

    pub fn validate(self) -> Result<Self, BbsGameConfigError> {
        validate_reservations_with_error(&self.reservations, BbsGameConfigError::Parse)?;
        let mut seats = std::collections::BTreeSet::new();
        for seat in &self.ai_seats {
            let player = seat.player_record_index_1_based;
            if player == 0 {
                return Err(BbsGameConfigError::Parse(
                    "ai_seat player must be >= 1".to_string(),
                ));
            }
            if !seats.insert(player) {
                return Err(BbsGameConfigError::Parse(format!(
                    "duplicate ai_seat for player {player}"
                )));
            }
        }
        Ok(self)
    }

    pub fn reservation_for_alias(&self, alias: &str) -> Option<&SeatReservation> {
        let alias = alias.trim();
        self.reservations
            .iter()
            .find(|reservation| reservation.alias.eq_ignore_ascii_case(alias))
    }

    pub fn reservation_for_player(
        &self,
        player_record_index_1_based: usize,
    ) -> Option<&SeatReservation> {
        self.reservations.iter().find(|reservation| {
            reservation.player_record_index_1_based == player_record_index_1_based
        })
    }

    pub fn validate_reservations_for_player_count(
        &self,
        player_count: usize,
    ) -> Result<(), BbsGameConfigError> {
        validate_reservation_player_count(
            &self.reservations,
            player_count,
            BbsGameConfigError::Parse,
        )
    }

    /// Check that every reserved and AI seat exists in a game of
    /// `player_count` seats.
    pub fn validate_seats_for_player_count(
        &self,
        player_count: usize,
    ) -> Result<(), BbsGameConfigError> {
        self.validate_reservations_for_player_count(player_count)?;
        if let Some(seat) = self
            .ai_seats
            .iter()
            .find(|seat| seat.player_record_index_1_based > player_count)
        {
            return Err(BbsGameConfigError::Parse(format!(
                "ai_seat player {} is not a seat of a {player_count}-player game",
                seat.player_record_index_1_based
            )));
        }
        Ok(())
    }
}

pub(crate) fn validate_reservations(reservations: &[SeatReservation]) -> Result<(), String> {
    validate_reservations_with_error(reservations, |message| message)
}

pub(crate) fn validate_reservation_player_count(
    reservations: &[SeatReservation],
    player_count: usize,
    wrap: impl Fn(String) -> BbsGameConfigError,
) -> Result<(), BbsGameConfigError> {
    for reservation in reservations {
        if reservation.player_record_index_1_based > player_count {
            return Err(wrap(format!(
                "reservation player {} exceeds player count {}",
                reservation.player_record_index_1_based, player_count
            )));
        }
    }
    Ok(())
}

fn validate_reservations_with_error<E>(
    reservations: &[SeatReservation],
    wrap: impl Fn(String) -> E,
) -> Result<(), E> {
    let mut seen_players = std::collections::BTreeSet::new();
    let mut seen_aliases = std::collections::BTreeSet::new();
    for reservation in reservations {
        if reservation.player_record_index_1_based == 0 {
            return Err(wrap("reservation player must be >= 1".to_string()));
        }
        if !seen_players.insert(reservation.player_record_index_1_based) {
            return Err(wrap(format!(
                "duplicate reservation for player {}",
                reservation.player_record_index_1_based
            )));
        }
        let alias_key = reservation.alias.to_ascii_lowercase();
        if !seen_aliases.insert(alias_key) {
            return Err(wrap(format!(
                "duplicate reservation alias '{}'",
                reservation.alias
            )));
        }
    }
    Ok(())
}

fn prop_string(node: &kdl::KdlNode, name: &str) -> Result<String, BbsGameConfigError> {
    node.get(name)
        .and_then(|value| value.as_string())
        .map(str::to_string)
        .ok_or_else(|| BbsGameConfigError::Parse(format!("{name} requires a string property")))
}

fn prop_usize(node: &kdl::KdlNode, name: &str) -> Result<usize, BbsGameConfigError> {
    let value = node
        .get(name)
        .and_then(|value| value.as_integer())
        .ok_or_else(|| BbsGameConfigError::Parse(format!("{name} requires an integer property")))?;
    usize::try_from(value)
        .map_err(|_| BbsGameConfigError::Parse(format!("{name} out of usize range: {value}")))
}

fn kdl_bool(value: bool) -> &'static str {
    if value { "#true" } else { "#false" }
}

/// The children of block `name`, refusing any child not in `known`.
fn block<'a>(
    document: &'a kdl::KdlDocument,
    name: &str,
    known: &[&str],
) -> Result<Option<&'a kdl::KdlDocument>, BbsGameConfigError> {
    let Some(children) = document.get(name).and_then(|node| node.children()) else {
        return Ok(None);
    };
    for child in children.nodes() {
        if !known.contains(&child.name().value()) {
            return Err(BbsGameConfigError::Parse(format!(
                "unsupported field '{}' in {name}",
                child.name().value()
            )));
        }
    }
    Ok(Some(children))
}

fn arg_string(
    document: &kdl::KdlDocument,
    name: &str,
) -> Result<Option<String>, BbsGameConfigError> {
    let Some(node) = document.get(name) else {
        return Ok(None);
    };
    node.get(0)
        .and_then(|value| value.as_string())
        .map(|text| Some(text.to_string()))
        .ok_or_else(|| BbsGameConfigError::Parse(format!("{name} requires a string")))
}

fn arg_bool(document: &kdl::KdlDocument, name: &str) -> Result<Option<bool>, BbsGameConfigError> {
    let Some(node) = document.get(name) else {
        return Ok(None);
    };
    node.get(0)
        .and_then(|value| value.as_bool())
        .map(Some)
        .ok_or_else(|| BbsGameConfigError::Parse(format!("{name} requires #true or #false")))
}

fn arg_u8(
    document: &kdl::KdlDocument,
    name: &str,
    max: u8,
) -> Result<Option<u8>, BbsGameConfigError> {
    let Some(node) = document.get(name) else {
        return Ok(None);
    };
    let value = node
        .get(0)
        .and_then(|value| value.as_integer())
        .ok_or_else(|| BbsGameConfigError::Parse(format!("{name} requires an integer")))?;
    u8::try_from(value)
        .ok()
        .filter(|value| *value <= max)
        .map(Some)
        .ok_or_else(|| BbsGameConfigError::Parse(format!("{name} must be 0-{max}, got {value}")))
}

fn kdl_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Copy `dir/config.kdl` into the campaign store before a login or
/// maintenance run (t218). A game without the file keeps its stored
/// settings. Changes players would notice are announced to every empire in
/// one broadcast report.
pub fn sync_game_config(
    dir: &Path,
    store: &crate::CampaignStore,
) -> Result<(), crate::CampaignStoreError> {
    let path = dir.join("config.kdl");
    if !path.exists() {
        return Ok(());
    }
    let config = BbsGameConfig::load_kdl(&path)
        .map_err(|err| crate::CampaignStoreError::InvalidState(format!("config.kdl: {err}")))?;
    let Some(runtime) = store.load_latest_runtime_state()? else {
        return Ok(());
    };
    config
        .validate_seats_for_player_count(runtime.game_data.players.len())
        .map_err(|err| crate::CampaignStoreError::InvalidState(format!("config.kdl: {err}")))?;
    let mut settings = store.load_campaign_settings()?;
    let before = settings.clone();
    config.apply_to(&mut settings);
    check_new_ai_seats(&runtime.game_data, &before.ai_seats, &settings.ai_seats)
        .map_err(|err| crate::CampaignStoreError::InvalidState(format!("config.kdl: {err}")))?;
    if settings == before {
        return Ok(());
    }
    store.save_campaign_settings(&settings)?;
    let changes = announced_changes(&before, &settings);
    if !changes.is_empty() {
        let year = runtime.game_year;
        let lines = changes
            .iter()
            .map(|change| format!(" - {change}"))
            .collect::<Vec<_>>()
            .join("\n");
        store.change(year, |state| {
            let block_index = state
                .report_block_rows
                .iter()
                .map(|row| row.block_index + 1)
                .max()
                .unwrap_or(0);
            state.report_block_rows.push(crate::ReportBlockRow {
                viewer_empire_id: 0,
                block_index,
                decoded_text: format!(
                    "Game Notice: Stardate: 01/{year}\n The sysop has changed this game's settings:\n{lines}"
                ),
                raw_bytes: None,
                recipient_deleted: false,
            });
            Ok::<(), crate::CampaignStoreError>(())
        })?;
    }
    Ok(())
}

/// Write `dir/config.kdl` from `settings`, headed by a comment giving the
/// game's fixed seat count, seed and map size for reference (t218).
pub fn write_game_config(
    dir: &Path,
    store: &crate::CampaignStore,
    settings: &crate::CampaignSettings,
) -> Result<(), crate::CampaignStoreError> {
    let runtime = store.load_latest_runtime_state()?.ok_or_else(|| {
        crate::CampaignStoreError::InvalidState("campaign store has no snapshots".to_string())
    })?;
    let players = runtime.game_data.conquest.player_count;
    let size = crate::map_size_for_player_count(players);
    let text = format!(
        "// Set by nc-sysop new-game and fixed for the game; shown for reference only.\n\
         // players {players}, seed {}, map {size}x{size}\n{}",
        runtime.campaign_seed,
        BbsGameConfig::from_settings(settings).to_kdl_string()
    );
    let path = dir.join("config.kdl");
    fs::write(&path, text).map_err(|source| crate::CampaignStoreError::Io { path, source })
}

/// Refuse an AI seat newly placed on an empire a person plays: the computer
/// takes only open seats (owner, 2026-10-24). Seats already AI stay.
pub fn check_new_ai_seats(
    game_data: &crate::CoreGameData,
    before: &[crate::AiSeat],
    after: &[crate::AiSeat],
) -> Result<(), String> {
    for seat in after {
        let player = seat.player_record_index_1_based;
        let was_ai = before
            .iter()
            .any(|old| old.player_record_index_1_based == player);
        let held = game_data
            .players
            .get(player.saturating_sub(1))
            .is_some_and(|record| record.mode == crate::PlayerMode::Human);
        if held && !was_ai {
            return Err(format!(
                "seat {player} is played by a person; the computer can take only an open seat"
            ));
        }
    }
    Ok(())
}

/// The setting changes players are told about, one line each. The theme,
/// snoop and seat reservations are left out (owner, 2026-10-24).
fn announced_changes(was: &crate::CampaignSettings, now: &crate::CampaignSettings) -> Vec<String> {
    let mut changes = Vec::new();
    let on_off = |value: bool| if value { "on" } else { "off" };
    if was.game_name != now.game_name {
        changes.push(format!("The game is now called \"{}\".", now.game_name));
    }
    if was.inactivity_autopilot_after_turns != now.inactivity_autopilot_after_turns {
        changes.push(autopilot_change_text(
            was.inactivity_autopilot_after_turns,
            now.inactivity_autopilot_after_turns,
        ));
    }
    if was.session_max_idle_minutes != now.session_max_idle_minutes {
        changes.push(format!(
            "A session now ends after {} idle minutes (it was {}).",
            now.session_max_idle_minutes, was.session_max_idle_minutes
        ));
    }
    if was.session_minimum_time_minutes != now.session_minimum_time_minutes {
        changes.push(format!(
            "Each session is now granted at least {} minutes (it was {}).",
            now.session_minimum_time_minutes, was.session_minimum_time_minutes
        ));
    }
    if was.session_local_timeout != now.session_local_timeout {
        changes.push(format!(
            "The idle timeout for local sessions is {}.",
            on_off(now.session_local_timeout)
        ));
    }
    if was.session_remote_timeout != now.session_remote_timeout {
        changes.push(format!(
            "The idle timeout for remote sessions is {}.",
            on_off(now.session_remote_timeout)
        ));
    }
    if was.maintenance_enabled != now.maintenance_enabled {
        changes.push(format!(
            "Scheduled maintenance is {}.",
            on_off(now.maintenance_enabled)
        ));
    }
    if was.maintenance_interval_minutes != now.maintenance_interval_minutes {
        changes.push(format!(
            "Maintenance now runs every {} (it was every {}).",
            interval_text(now.maintenance_interval_minutes),
            interval_text(was.maintenance_interval_minutes)
        ));
    }
    for seat in 1..=now
        .ai_seats
        .iter()
        .chain(&was.ai_seats)
        .map(|seat| seat.player_record_index_1_based)
        .max()
        .unwrap_or(0)
    {
        let find = |seats: &[crate::AiSeat]| {
            seats
                .iter()
                .find(|ai| ai.player_record_index_1_based == seat)
                .cloned()
        };
        match (find(&was.ai_seats), find(&now.ai_seats)) {
            (None, Some(_)) => {
                changes.push(format!("Empire {seat} is now played by the computer."))
            }
            (Some(_), None) => changes.push(format!(
                "Empire {seat} is no longer played by the computer."
            )),
            _ => {}
        }
    }
    changes
}

fn interval_text(minutes: u32) -> String {
    match minutes {
        m if m % 1440 == 0 && m / 1440 == 1 => "day".to_string(),
        m if m % 1440 == 0 => format!("{} days", m / 1440),
        m if m % 60 == 0 && m / 60 == 1 => "hour".to_string(),
        m if m % 60 == 0 => format!("{} hours", m / 60),
        m => format!("{m} minutes"),
    }
}

fn autopilot_change_text(was: u8, now: u8) -> String {
    let turns = |n: u8| {
        if n == 1 {
            "1 missed turn".to_string()
        } else {
            format!("{n} missed turns")
        }
    };
    match (was, now) {
        (_, 0) => "Autopilot no longer takes over empires whose players miss turns.".to_string(),
        (0, now) => format!("Autopilot now takes over an empire after {}.", turns(now)),
        (was, now) => format!(
            "Autopilot now takes over an empire after {} (it was {}).",
            turns(now),
            turns(was)
        ),
    }
}
