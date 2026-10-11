use std::fmt;
use std::fs;
use std::path::Path;

use crate::{
    CampaignRuntimeState, CampaignStore, CampaignStoreError, CoreGameData, DiplomaticRelation,
    FleetDetachSelection, GameStateMutationError, QueuedPlayerMail,
};

mod apply;
mod parser;
mod render;
mod runtime;

pub use crate::player_mail::{MAX_MESSAGE_BODY_CHARS, MAX_MESSAGE_SUBJECT_CHARS};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnSubmission {
    pub player_record_index_1_based: usize,
    pub year: u16,
    pub tax_rate: Option<u8>,
    pub diplomacy: Vec<TurnDiplomacyDirective>,
    pub planets: Vec<PlanetTurnBlock>,
    pub fleets: Vec<FleetTurnBlock>,
    pub messages: Vec<TurnMessage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurnDiplomacyDirective {
    pub to_empire: u8,
    pub relation: DiplomaticRelation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanetTurnBlock {
    pub planet_record_index_1_based: usize,
    pub actions: Vec<PlanetTurnAction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanetTurnAction {
    Rename { name: String },
    ClearBuildQueue,
    ClearBuildKind { kind: u8 },
    RemoveBuild { qty: u16, kind: u8 },
    Build { points_remaining: u32, kind: u8 },
    Commission { slot_0_based: usize },
    AutoCommission,
    Scorch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetTurnBlock {
    pub fleet_record_index_1_based: usize,
    pub actions: Vec<FleetTurnAction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FleetTurnAction {
    Order {
        speed: u8,
        order_code: u8,
        /// `None` only for Seek Home and Salvage: the nearest owned world.
        target: Option<[u8; 2]>,
        aux0: Option<u8>,
        aux1: Option<u8>,
    },
    RulesOfEngagement {
        value: u8,
    },
    Speed {
        value: u8,
    },
    Join {
        host_fleet_record_index_1_based: usize,
    },
    Detach {
        selection: FleetDetachSelection,
        donor_speed: Option<u8>,
        new_fleet_roe: u8,
    },
    Transfer {
        host_fleet_record_index_1_based: usize,
        selection: FleetDetachSelection,
    },
    LoadArmies {
        planet_record_index_1_based: usize,
        qty: u16,
    },
    UnloadArmies {
        planet_record_index_1_based: usize,
        qty: u16,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnMessage {
    pub recipient_empire: u8,
    pub subject: String,
    pub body: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurnSubmissionReport {
    pub player_record_index_1_based: usize,
    pub year: u16,
    pub tax_changed: bool,
    pub diplomacy_updates: usize,
    pub planet_blocks: usize,
    pub planet_actions: usize,
    pub fleet_blocks: usize,
    pub fleet_actions: usize,
    pub messages_queued: usize,
}

#[derive(Debug)]
pub enum TurnSubmissionError {
    Io(std::io::Error),
    Parse(String),
    Validation(String),
    Mutation(GameStateMutationError),
    Storage(CampaignStoreError),
}

impl fmt::Display for TurnSubmissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(source) => write!(f, "{source}"),
            Self::Parse(message) | Self::Validation(message) => write!(f, "{message}"),
            Self::Mutation(source) => write!(f, "{source}"),
            Self::Storage(source) => write!(f, "{source}"),
        }
    }
}

impl std::error::Error for TurnSubmissionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(source) => Some(source),
            Self::Mutation(source) => Some(source),
            Self::Storage(source) => Some(source),
            Self::Parse(_) | Self::Validation(_) => None,
        }
    }
}

impl From<std::io::Error> for TurnSubmissionError {
    fn from(source: std::io::Error) -> Self {
        Self::Io(source)
    }
}

impl From<GameStateMutationError> for TurnSubmissionError {
    fn from(source: GameStateMutationError) -> Self {
        Self::Mutation(source)
    }
}

impl From<CampaignStoreError> for TurnSubmissionError {
    fn from(source: CampaignStoreError) -> Self {
        Self::Storage(source)
    }
}

impl TurnSubmission {
    pub fn parse_kdl_str(input: &str) -> Result<Self, TurnSubmissionError> {
        parser::parse_turn_submission(input)
    }

    pub fn load_kdl(path: &Path) -> Result<Self, TurnSubmissionError> {
        Self::parse_kdl_str(&fs::read_to_string(path)?)
    }

    pub fn to_kdl_string(&self) -> String {
        render::render_turn_submission(self)
    }

    pub fn apply_to(
        &self,
        game_data: &mut CoreGameData,
        queued_mail: &mut Vec<QueuedPlayerMail>,
    ) -> Result<TurnSubmissionReport, TurnSubmissionError> {
        apply::apply_turn_submission(self, game_data, queued_mail)
    }

    pub fn submit_kdl_file_to_campaign_dir(
        dir: &Path,
        player_record_index_1_based: usize,
        file: &Path,
        check_only: bool,
    ) -> Result<TurnSubmissionReport, TurnSubmissionError> {
        runtime::submit_turn_kdl_file(dir, player_record_index_1_based, file, check_only)
    }

    /// Apply this submission to runtime state the caller already loaded and
    /// save it. If maintenance has written a newer year since `state` was
    /// loaded, nothing is saved and the error is
    /// [`CampaignStoreError::StaleSnapshot`].
    pub fn submit_to_runtime_state(
        &self,
        store: &CampaignStore,
        state: CampaignRuntimeState,
    ) -> Result<TurnSubmissionReport, TurnSubmissionError> {
        runtime::submit_to_runtime_state(store, state, self)
    }
}
