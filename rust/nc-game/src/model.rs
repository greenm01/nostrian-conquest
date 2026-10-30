use nc_data::{CoreGameData, PlayerAccessMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassicLoginState {
    FirstTimeMenu,
    MatchedPreloadedFirstLogin,
    ReturningPlayer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerContext {
    pub record_index_1_based: usize,
    pub is_joined: bool,
    pub classic_login_state: ClassicLoginState,
    pub empire_name: String,
    pub handle: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MainMenuSummary {
    pub game_year: u16,
    pub player_count: usize,
    pub owned_planets: usize,
    pub owned_fleets: usize,
    pub pending_messages: bool,
    pub pending_results: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneralMenuSummary {
    pub pending_messages: bool,
    pub pending_results: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewSummary {
    pub reviewable_messages: bool,
    pub reviewable_results: bool,
}

impl PlayerContext {
    pub fn from_game_data(
        game_data: &CoreGameData,
        record_index_1_based: usize,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let record = game_data
            .players
            .get(record_index_1_based - 1)
            .ok_or_else(|| format!("PLAYER.DAT missing record {record_index_1_based}"))?;
        let classic_login_state =
            ClassicLoginState::from_game_data(game_data, record_index_1_based as u8);
        let empire_name = record.empire_name.trim().to_string();
        let handle = record.handle.trim().to_string();
        Ok(Self {
            record_index_1_based,
            is_joined: classic_login_state != ClassicLoginState::FirstTimeMenu,
            classic_login_state,
            empire_name,
            handle,
        })
    }

    pub fn apply_access_mode_override(&mut self, access_mode: PlayerAccessMode) {
        if matches!(
            access_mode,
            PlayerAccessMode::ReviewOnly | PlayerAccessMode::SurveyOnly
        ) {
            self.classic_login_state = ClassicLoginState::ReturningPlayer;
            self.is_joined = true;
        }
    }
}

impl ClassicLoginState {
    pub fn from_game_data(game_data: &CoreGameData, empire_id: u8) -> Self {
        let Some(player) = game_data.players.get(empire_id.saturating_sub(1) as usize) else {
            return Self::FirstTimeMenu;
        };

        if !(player.mode == nc_data::PlayerMode::Human) {
            return Self::FirstTimeMenu;
        }

        let Some(homeworld) = homeworld_like_planet(game_data, empire_id, player) else {
            return Self::ReturningPlayer;
        };

        if nc_data::is_named_homeworld_seed(&homeworld) {
            Self::MatchedPreloadedFirstLogin
        } else {
            Self::ReturningPlayer
        }
    }
}

fn homeworld_like_planet<'a>(
    game_data: &'a CoreGameData,
    empire_id: u8,
    player: &nc_data::Player,
) -> Option<&'a nc_data::Planet> {
    let index = player.homeworld as usize;
    if index > 0 {
        if let Some(planet) = game_data.planets.get(index - 1) {
            return Some(planet);
        }
    }

    game_data
        .planets
        .iter()
        .find(|planet| planet.owner.0 == empire_id && nc_data::is_named_homeworld_seed(&planet))
}

impl MainMenuSummary {
    pub fn from_game_data(
        game_data: &CoreGameData,
        player_record_index_1_based: usize,
        results_present: bool,
        runtime_messages_present: bool,
    ) -> Self {
        let owned_planets = game_data
            .planets
            .iter()
            .filter(|planet| planet.owner.0 as usize == player_record_index_1_based)
            .count();
        let owned_fleets = game_data
            .fleets
            .iter()
            .filter(|fleet| fleet.owner.0 as usize == player_record_index_1_based)
            .count();

        let player_record = game_data.players.get(player_record_index_1_based - 1);
        let pending_results = player_record
            .map(|record| nc_data::has_results_chain(&record) || (record.messages_last != 0))
            .unwrap_or(false)
            || results_present;
        // Maintenance sets the classic message pointers whenever reports
        // wait, for ECGAME's login; only real mail counts as messages here.
        let pending_messages = runtime_messages_present;

        Self {
            game_year: game_data.conquest.game_year,
            player_count: game_data.players.len(),
            owned_planets,
            owned_fleets,
            pending_messages,
            pending_results,
        }
    }
}

impl GeneralMenuSummary {
    pub fn from_main_menu(summary: &MainMenuSummary) -> Self {
        Self {
            pending_messages: summary.pending_messages,
            pending_results: summary.pending_results,
        }
    }
}

impl ReviewSummary {
    pub fn from_main_menu(summary: &MainMenuSummary) -> Self {
        Self {
            reviewable_messages: summary.pending_messages,
            reviewable_results: summary.pending_results,
        }
    }
}
