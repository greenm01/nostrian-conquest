use crate::classic::{decode_player, encode_player, parse_player_dat};
use crate::model::{Player, PlayerMode};
use crate::support::{ParseError, classic_text};
use crate::{ClassicResultsChain, PLAYER_RECORD_SIZE};

pub use crate::model::DiplomaticRelation;

const HANDLE_LEN: usize = 25;
const EMPIRE_NAME_LEN: usize = 20;

/// A [`Player`] behind the accessor API call sites still use.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerRecord {
    pub player: Player,
}

impl PlayerRecord {
    pub fn new_zeroed() -> Self {
        Self::default()
    }

    pub fn from_raw(raw: [u8; PLAYER_RECORD_SIZE]) -> Self {
        Self {
            player: decode_player(&raw),
        }
    }

    /// The classic PLAYER.DAT encoding of this record.
    pub fn to_bytes(&self) -> [u8; PLAYER_RECORD_SIZE] {
        encode_player(&self.player)
    }

    /// The player mode byte: 0 civil disorder, 1 human, 0xFF rogue.
    pub fn player_mode_raw(&self) -> u8 {
        self.player.mode.to_raw()
    }

    pub fn occupied_flag(&self) -> u8 {
        self.player_mode_raw()
    }

    pub fn owner_mode_raw(&self) -> u8 {
        self.player_mode_raw()
    }

    pub fn assigned_player_flag_raw(&self) -> u8 {
        self.player_mode_raw()
    }

    pub fn is_civil_disorder_player(&self) -> bool {
        self.player.mode == PlayerMode::CivilDisorder
    }

    pub fn is_active_human_player(&self) -> bool {
        self.player.mode == PlayerMode::Human
    }

    pub fn is_active_player(&self) -> bool {
        self.is_active_human_player()
    }

    pub fn is_rogue_player(&self) -> bool {
        self.player.mode == PlayerMode::Rogue
    }

    pub fn is_active_or_rogue_player(&self) -> bool {
        self.is_active_human_player() || self.is_rogue_player()
    }

    pub fn empire_name_bytes(&self) -> &[u8] {
        self.player.empire_name.as_bytes()
    }

    /// Lines per screen for ECGAME's "More" pause; ECUTIL writes 24.
    pub fn page_lines(&self) -> u8 {
        self.player.page_lines
    }

    pub fn set_page_lines(&mut self, value: u8) {
        self.player.page_lines = value;
    }

    /// The page length, which older code read as a name length.
    // t039: remove in phase 5
    pub fn legacy_status_name_max_len_raw(&self) -> u8 {
        self.player.page_lines
    }

    // t039: remove in phase 5
    pub fn legacy_status_name_len_raw(&self) -> u8 {
        self.controlled_empire_name_len_raw()
    }

    /// The empire name; older code read it as a separate status label.
    // t039: remove in phase 5
    pub fn legacy_status_name_summary(&self) -> String {
        self.controlled_empire_name_summary()
    }

    pub fn assigned_player_handle_summary(&self) -> String {
        self.player.handle.trim().to_string()
    }

    /// Whether `alias` names this handle as `set_assigned_player_handle_raw`
    /// stores it: `?` for each character outside classic text, cut to 25.
    pub fn assigned_player_handle_matches(&self, alias: &str) -> bool {
        let stored = classic_text(alias);
        let stored = &stored[..stored.len().min(HANDLE_LEN)];
        self.assigned_player_handle_summary()
            .eq_ignore_ascii_case(stored.trim())
    }

    pub fn set_assigned_player_handle_raw(&mut self, value: &str) {
        self.player.handle = classic_text(value).chars().take(HANDLE_LEN).collect();
    }

    pub fn controlled_empire_name_len_raw(&self) -> u8 {
        self.player.empire_name.chars().count() as u8
    }

    pub fn controlled_empire_name_summary(&self) -> String {
        self.player.empire_name.trim().to_string()
    }

    /// MESSAGES.DAT record of the first waiting message; 0 for none.
    pub fn messages_first(&self) -> i32 {
        self.player.messages_first
    }

    pub fn set_messages_first(&mut self, value: i32) {
        self.player.messages_first = value;
    }

    /// MESSAGES.DAT record of the last waiting message.
    pub fn messages_last(&self) -> i32 {
        self.player.messages_last
    }

    pub fn set_messages_last(&mut self, value: i32) {
        self.player.messages_last = value;
    }

    /// RESULTS.DAT record of the first report header; 0 for none.
    pub fn results_first(&self) -> i32 {
        self.player.results_first
    }

    pub fn set_results_first(&mut self, value: i32) {
        self.player.results_first = value;
    }

    /// RESULTS.DAT record of the last report header.
    pub fn results_last(&self) -> i32 {
        self.player.results_last
    }

    pub fn set_results_last(&mut self, value: i32) {
        self.player.results_last = value;
    }

    /// The low word of `messages_first`, which older code read as a review
    /// state.
    // t039: remove in phase 5
    pub fn classic_message_review_word_raw(&self) -> u16 {
        self.player.messages_first as u16
    }

    // t039: remove in phase 5
    pub fn set_classic_message_review_word_raw(&mut self, value: u16) {
        self.player.messages_first = with_low_word(self.player.messages_first, value);
    }

    /// The high word of `messages_first`.
    // t039: remove in phase 5
    pub fn classic_message_review_carry_word_raw(&self) -> u16 {
        (self.player.messages_first >> 16) as u16
    }

    // t039: remove in phase 5
    pub fn set_classic_message_review_carry_word_raw(&mut self, value: u16) {
        self.player.messages_first = with_high_word(self.player.messages_first, value);
    }

    /// The low word of `messages_last`, which older code read as the
    /// results review state.
    // t039: remove in phase 5
    pub fn classic_results_review_word_raw(&self) -> u16 {
        self.player.messages_last as u16
    }

    // t039: remove in phase 5
    pub fn set_classic_results_review_word_raw(&mut self, value: u16) {
        self.player.messages_last = with_low_word(self.player.messages_last, value);
    }

    /// The high word of `messages_last`.
    // t039: remove in phase 5
    pub fn classic_results_review_carry_word_raw(&self) -> u16 {
        (self.player.messages_last >> 16) as u16
    }

    // t039: remove in phase 5
    pub fn set_classic_results_review_carry_word_raw(&mut self, value: u16) {
        self.player.messages_last = with_high_word(self.player.messages_last, value);
    }

    pub fn has_classic_messages_review_state(&self) -> bool {
        self.player.messages_first != 0
    }

    pub fn has_classic_results_review_state(&self) -> bool {
        self.player.messages_last != 0
    }

    /// The `results_first` record as a word. Original `ECMAINT` output
    /// (two- and three-human oracle runs, see `fixtures/ecmaint-results-*`)
    /// writes the 1-based record index of this empire's first report header
    /// in the shared `RESULTS.DAT`, or 0 when it has none.
    pub fn classic_results_first_header_raw(&self) -> u16 {
        self.player.results_first as u16
    }

    pub fn set_classic_results_first_header_raw(&mut self, value: u16) {
        self.player.results_first = i32::from(value);
    }

    /// The `results_last` record as a word: the 1-based record index of
    /// this empire's last report header.
    pub fn classic_results_last_header_raw(&self) -> u16 {
        self.player.results_last as u16
    }

    pub fn set_classic_results_last_header_raw(&mut self, value: u16) {
        self.player.results_last = i32::from(value);
    }

    pub fn classic_results_chain(&self) -> Option<ClassicResultsChain> {
        let first_header = self.classic_results_first_header_raw();
        (first_header != 0).then(|| ClassicResultsChain {
            first_header,
            last_header: self.classic_results_last_header_raw(),
        })
    }

    pub fn has_classic_results_chain_state(&self) -> bool {
        self.player.results_first != 0 || self.player.results_last != 0
    }

    pub fn has_any_classic_review_state(&self) -> bool {
        self.has_classic_messages_review_state() || self.has_classic_results_review_state()
    }

    /// The low byte of `messages_last`, which older code read as a flag.
    // t039: remove in phase 5
    pub fn classic_reports_pending_flag_raw(&self) -> u8 {
        self.player.messages_last as u8
    }

    // t039: remove in phase 5
    pub fn set_classic_reports_pending_flag_raw(&mut self, value: u8) {
        self.player.messages_last = with_low_byte(self.player.messages_last, value);
    }

    /// The low byte of `messages_first`, which older code read as a flag.
    // t039: remove in phase 5
    pub fn classic_messages_pending_flag_raw(&self) -> u8 {
        self.player.messages_first as u8
    }

    // t039: remove in phase 5
    pub fn set_classic_messages_pending_flag_raw(&mut self, value: u8) {
        self.player.messages_first = with_low_byte(self.player.messages_first, value);
    }

    pub fn set_classic_results_review_state_present(&mut self, present: bool) {
        self.player.messages_last = i32::from(present);
    }

    pub fn set_classic_messages_review_state_present(&mut self, present: bool) {
        self.player.messages_first = i32::from(present);
    }

    /// Advertise login-time reviewables to classic ECGAME.
    ///
    /// Current compatibility rule:
    /// - classic delivery/review probes use both message records together
    ///   for unread mail
    /// - results-only startup probing does not become reviewable when only
    ///   the results-side record is set
    ///
    /// Until a stricter family split is proven in classic login flow, keep
    /// the mirrored advertisement here and route all startup-review
    /// producers through this helper.
    pub fn set_classic_login_reviewables_present(&mut self, present: bool) {
        self.set_classic_messages_review_state_present(present);
        self.set_classic_results_review_state_present(present);
    }

    /// Point this empire at its chain in the shared `RESULTS.DAT`.  Lay the
    /// file out with [`crate::layout_classic_results`] to get the chain.
    pub fn set_classic_results_chain(&mut self, chain: Option<ClassicResultsChain>) {
        let chain = chain.unwrap_or(ClassicResultsChain {
            first_header: 0,
            last_header: 0,
        });
        self.player.results_first = i32::from(chain.first_header);
        self.player.results_last = i32::from(chain.last_header);
    }

    /// Stores `value` as classic text, cut to the 20 characters an empire
    /// name holds.
    pub fn set_controlled_empire_name_raw(&mut self, value: &str) {
        self.player.empire_name = classic_text(value).chars().take(EMPIRE_NAME_LEN).collect();
    }

    /// Returns the empire to civil disorder as ECUTIL and ECMAINT do: mode
    /// 0, a blank handle, and the name "In Civil Disorder". Every other
    /// field stays.
    pub fn set_civil_disorder_mode(&mut self) {
        self.player.mode = PlayerMode::CivilDisorder;
        self.player.handle.clear();
        self.set_controlled_empire_name_raw("In Civil Disorder");
    }

    pub fn ownership_summary(&self) -> String {
        let legacy = self.controlled_empire_name_summary();
        let handle = self.assigned_player_handle_summary();
        let empire = self.controlled_empire_name_summary();

        if self.is_rogue_player() {
            format!("rogue label='{legacy}'")
        } else if legacy.starts_with("In Civil Disorder") || legacy == "Unowned" {
            format!("unowned label='{legacy}'")
        } else if self.assigned_player_flag_raw() != 0 || !handle.is_empty() || !empire.is_empty() {
            format!("player handle='{}' empire='{}'", handle, empire)
        } else {
            format!("unowned label='{legacy}'")
        }
    }

    pub fn tax_rate(&self) -> u8 {
        self.player.tax_rate
    }

    pub fn set_tax_rate_raw(&mut self, value: u8) {
        self.player.tax_rate = value;
    }

    /// Record number of the empire's first starbase in BASES.DAT, the head
    /// of its chain; 0 for none.
    pub fn first_base_record_raw(&self) -> u16 {
        self.player.bases_first.0
    }

    pub fn set_first_base_record_raw(&mut self, value: u16) {
        self.player.bases_first.0 = value;
    }

    /// Record number of the empire's last starbase, the tail of its chain;
    /// 0 for none.
    pub fn last_base_record_raw(&self) -> u16 {
        self.player.bases_last.0
    }

    pub fn set_last_base_record_raw(&mut self, value: u16) {
        self.player.bases_last.0 = value;
    }

    pub fn fleet_chain_head_raw(&self) -> u16 {
        self.player.fleets_first.0
    }

    pub fn set_fleet_chain_head_raw(&mut self, value: u16) {
        self.player.fleets_first.0 = value;
    }

    pub fn fleet_chain_tail_raw(&self) -> u16 {
        self.player.fleets_last.0
    }

    pub fn set_fleet_chain_tail_raw(&mut self, value: u16) {
        self.player.fleets_last.0 = value;
    }

    /// Record number heading the empire's IPBM chain; 0 for none.
    pub fn ipbm_chain_head_raw(&self) -> u16 {
        self.player.ipbms_first.0
    }

    pub fn set_ipbm_chain_head_raw(&mut self, value: u16) {
        self.player.ipbms_first.0 = value;
    }

    pub fn homeworld_planet_index_1_based_raw(&self) -> u8 {
        self.player.homeworld
    }

    pub fn set_homeworld_planet_index_1_based_raw(&mut self, value: u8) {
        self.player.homeworld = value;
    }

    /// Game year of the empire's last sign-on; 0 before the first. Classic
    /// `ECGAME` shows it as `Last year on:`.
    pub fn last_run_year_raw(&self) -> u16 {
        self.player.last_year_on
    }

    pub fn set_last_run_year_raw(&mut self, value: u16) {
        self.player.last_year_on = value;
    }

    pub fn planet_count_raw(&self) -> u8 {
        self.player.planet_count
    }

    pub fn set_planet_count_raw(&mut self, value: u8) {
        self.player.planet_count = value;
    }

    pub fn production_score_raw(&self) -> u16 {
        self.player.production
    }

    pub fn set_production_score_raw(&mut self, value: u16) {
        self.player.production = value;
    }

    /// Autopilot flag. 1 = autopilot on (computer manages empire, mostly
    /// builds planetary defenses). 0 = human player submitting orders.
    pub fn autopilot_flag(&self) -> u8 {
        u8::from(self.player.autopilot)
    }

    pub fn set_autopilot_flag(&mut self, value: u8) {
        self.player.autopilot = value != 0;
    }

    pub fn set_player_mode_raw(&mut self, value: u8) {
        self.player.mode = PlayerMode::from_raw(value);
    }

    pub fn set_occupied_flag(&mut self, value: u8) {
        self.set_player_mode_raw(value);
    }

    pub fn set_owner_empire_raw(&mut self, value: u8) {
        self.set_player_mode_raw(value);
    }

    /// Stored diplomatic relation toward another empire, or `None` for a
    /// byte no classic program writes.
    pub fn diplomatic_relation_toward(&self, other_empire_raw: u8) -> Option<DiplomaticRelation> {
        match self
            .player
            .diplomacy
            .get(diplomacy_slot(other_empire_raw)?)?
        {
            DiplomaticRelation::Unknown(_) => None,
            relation => Some(*relation),
        }
    }

    pub fn diplomatic_relation_byte_raw(&self, other_empire_raw: u8) -> Option<u8> {
        Some(self.player.diplomacy[diplomacy_slot(other_empire_raw)?].to_raw())
    }

    pub fn set_diplomatic_relation_byte_raw(&mut self, other_empire_raw: u8, raw: u8) -> bool {
        self.set_diplomatic_relation_toward(other_empire_raw, DiplomaticRelation::from_raw(raw))
    }

    pub fn set_diplomatic_relation_toward(
        &mut self,
        other_empire_raw: u8,
        relation: DiplomaticRelation,
    ) -> bool {
        match diplomacy_slot(other_empire_raw) {
            Some(slot) => {
                self.player.diplomacy[slot] = relation;
                true
            }
            None => false,
        }
    }
}

fn diplomacy_slot(other_empire_raw: u8) -> Option<usize> {
    (1..=25)
        .contains(&other_empire_raw)
        .then(|| usize::from(other_empire_raw) - 1)
}

fn with_low_word(value: i32, word: u16) -> i32 {
    ((value as u32 & 0xFFFF_0000) | u32::from(word)) as i32
}

fn with_high_word(value: i32, word: u16) -> i32 {
    ((value as u32 & 0x0000_FFFF) | (u32::from(word) << 16)) as i32
}

fn with_low_byte(value: i32, byte: u8) -> i32 {
    ((value as u32 & 0xFFFF_FF00) | u32::from(byte)) as i32
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerDat {
    pub records: Vec<PlayerRecord>,
}

impl PlayerDat {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        Ok(Self {
            records: parse_player_dat(data)?
                .into_iter()
                .map(|player| PlayerRecord { player })
                .collect(),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.records
            .iter()
            .flat_map(|record| encode_player(&record.player))
            .collect()
    }
}
