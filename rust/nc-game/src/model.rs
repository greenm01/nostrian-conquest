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
        let empire_name = record.controlled_empire_name_summary();
        let handle = record.assigned_player_handle_summary();
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
    pub fn from_game_data(game_data: &CoreGameData, empire_raw: u8) -> Self {
        let Some(player) = game_data.players.get(empire_raw.saturating_sub(1) as usize) else {
            return Self::FirstTimeMenu;
        };

        if !player.is_active_human_player() {
            return Self::FirstTimeMenu;
        }

        let Some(homeworld) = homeworld_like_planet(game_data, empire_raw, player) else {
            return Self::ReturningPlayer;
        };

        if homeworld.is_named_homeworld_seed() {
            Self::MatchedPreloadedFirstLogin
        } else {
            Self::ReturningPlayer
        }
    }
}

fn homeworld_like_planet<'a>(
    game_data: &'a CoreGameData,
    empire_raw: u8,
    player: &nc_data::Player,
) -> Option<&'a nc_data::Planet> {
    let index = player.homeworld_planet_index_1_based_raw() as usize;
    if index > 0 {
        if let Some(planet) = game_data.planets.get(index - 1) {
            return Some(planet);
        }
    }

    game_data.planets.iter().find(|planet| {
        planet.owner_empire_slot_raw() == empire_raw && planet.is_named_homeworld_seed()
    })
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
            .filter(|planet| planet.owner_empire_slot_raw() as usize == player_record_index_1_based)
            .count();
        let owned_fleets = game_data
            .fleets
            .iter()
            .filter(|fleet| fleet.owner_empire_raw() as usize == player_record_index_1_based)
            .count();

        let player_record = game_data.players.get(player_record_index_1_based - 1);
        let pending_results = player_record
            .map(|record| {
                record.has_classic_results_chain_state()
                    || record.has_classic_results_review_state()
            })
            .unwrap_or(false)
            || results_present;
        let pending_messages = player_record
            .map(|record| record.has_classic_messages_review_state())
            .unwrap_or(false)
            || runtime_messages_present;

        Self {
            game_year: game_data.conquest.game_year(),
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
