use crate::classic::{decode_player, encode_player};
use crate::model::{Player, PlayerMode};
use crate::{ClassicResultsChain, PLAYER_RECORD_SIZE};

use crate::model::DiplomaticRelation;

impl Player {
    pub fn from_raw(raw: [u8; PLAYER_RECORD_SIZE]) -> Self {
        decode_player(&raw)
    }

    /// The classic PLAYER.DAT encoding of this record.
    pub fn to_bytes(&self) -> [u8; PLAYER_RECORD_SIZE] {
        encode_player(self)
    }

    /// The player mode byte: 0 civil disorder, 1 human, 0xFF rogue.
    pub fn player_mode_raw(&self) -> u8 {
        self.mode.to_raw()
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
        self.mode == PlayerMode::CivilDisorder
    }

    pub fn is_active_human_player(&self) -> bool {
        self.mode == PlayerMode::Human
    }

    pub fn is_active_player(&self) -> bool {
        self.is_active_human_player()
    }

    pub fn is_rogue_player(&self) -> bool {
        self.mode == PlayerMode::Rogue
    }

    pub fn is_active_or_rogue_player(&self) -> bool {
        self.is_active_human_player() || self.is_rogue_player()
    }

    pub fn empire_name_bytes(&self) -> &[u8] {
        self.empire_name.as_bytes()
    }

    /// Lines per screen for ECGAME's "More" pause; ECUTIL writes 24.
    pub fn page_lines(&self) -> u8 {
        self.page_lines
    }

    pub fn set_page_lines(&mut self, value: u8) {
        self.page_lines = value;
    }

    /// The page length, which older code read as a name length.
    // t039: remove in phase 5
    pub fn legacy_status_name_max_len_raw(&self) -> u8 {
        self.page_lines
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
        self.handle.trim().to_string()
    }

    /// Whether `alias` names this handle as `set_assigned_player_handle_raw`
    /// stores it: `?` for each character outside classic text, cut to 25.
    pub fn assigned_player_handle_matches(&self, alias: &str) -> bool {
        crate::names::player_handle_matches(self, alias)
    }

    pub fn set_assigned_player_handle_raw(&mut self, value: &str) {
        crate::names::set_player_handle(self, value)
    }

    pub fn controlled_empire_name_len_raw(&self) -> u8 {
        self.empire_name.chars().count() as u8
    }

    pub fn controlled_empire_name_summary(&self) -> String {
        self.empire_name.trim().to_string()
    }

    /// MESSAGES.DAT record of the first waiting message; 0 for none.
    pub fn messages_first(&self) -> i32 {
        self.messages_first
    }

    pub fn set_messages_first(&mut self, value: i32) {
        self.messages_first = value;
    }

    /// MESSAGES.DAT record of the last waiting message.
    pub fn messages_last(&self) -> i32 {
        self.messages_last
    }

    pub fn set_messages_last(&mut self, value: i32) {
        self.messages_last = value;
    }

    /// RESULTS.DAT record of the first report header; 0 for none.
    pub fn results_first(&self) -> i32 {
        self.results_first
    }

    pub fn set_results_first(&mut self, value: i32) {
        self.results_first = value;
    }

    /// RESULTS.DAT record of the last report header.
    pub fn results_last(&self) -> i32 {
        self.results_last
    }

    pub fn set_results_last(&mut self, value: i32) {
        self.results_last = value;
    }

    /// The low word of `messages_first`, which older code read as a review
    /// state.
    // t039: remove in phase 5
    pub fn classic_message_review_word_raw(&self) -> u16 {
        self.messages_first as u16
    }

    // t039: remove in phase 5
    pub fn set_classic_message_review_word_raw(&mut self, value: u16) {
        self.messages_first = with_low_word(self.messages_first, value);
    }

    /// The high word of `messages_first`.
    // t039: remove in phase 5
    pub fn classic_message_review_carry_word_raw(&self) -> u16 {
        (self.messages_first >> 16) as u16
    }

    // t039: remove in phase 5
    pub fn set_classic_message_review_carry_word_raw(&mut self, value: u16) {
        self.messages_first = with_high_word(self.messages_first, value);
    }

    /// The low word of `messages_last`, which older code read as the
    /// results review state.
    // t039: remove in phase 5
    pub fn classic_results_review_word_raw(&self) -> u16 {
        self.messages_last as u16
    }

    // t039: remove in phase 5
    pub fn set_classic_results_review_word_raw(&mut self, value: u16) {
        self.messages_last = with_low_word(self.messages_last, value);
    }

    /// The high word of `messages_last`.
    // t039: remove in phase 5
    pub fn classic_results_review_carry_word_raw(&self) -> u16 {
        (self.messages_last >> 16) as u16
    }

    // t039: remove in phase 5
    pub fn set_classic_results_review_carry_word_raw(&mut self, value: u16) {
        self.messages_last = with_high_word(self.messages_last, value);
    }

    pub fn has_classic_messages_review_state(&self) -> bool {
        self.messages_first != 0
    }

    pub fn has_classic_results_review_state(&self) -> bool {
        self.messages_last != 0
    }

    /// The `results_first` record as a word. Original `ECMAINT` output
    /// (two- and three-human oracle runs, see `fixtures/ecmaint-results-*`)
    /// writes the 1-based record index of this empire's first report header
    /// in the shared `RESULTS.DAT`, or 0 when it has none.
    pub fn classic_results_first_header_raw(&self) -> u16 {
        self.results_first as u16
    }

    pub fn set_classic_results_first_header_raw(&mut self, value: u16) {
        self.results_first = i32::from(value);
    }

    /// The `results_last` record as a word: the 1-based record index of
    /// this empire's last report header.
    pub fn classic_results_last_header_raw(&self) -> u16 {
        self.results_last as u16
    }

    pub fn set_classic_results_last_header_raw(&mut self, value: u16) {
        self.results_last = i32::from(value);
    }

    pub fn classic_results_chain(&self) -> Option<ClassicResultsChain> {
        crate::classic_results::results_chain(self)
    }

    pub fn has_classic_results_chain_state(&self) -> bool {
        crate::classic_results::has_results_chain(self)
    }

    pub fn has_any_classic_review_state(&self) -> bool {
        self.has_classic_messages_review_state() || self.has_classic_results_review_state()
    }

    /// The low byte of `messages_last`, which older code read as a flag.
    // t039: remove in phase 5
    pub fn classic_reports_pending_flag_raw(&self) -> u8 {
        self.messages_last as u8
    }

    // t039: remove in phase 5
    pub fn set_classic_reports_pending_flag_raw(&mut self, value: u8) {
        self.messages_last = with_low_byte(self.messages_last, value);
    }

    /// The low byte of `messages_first`, which older code read as a flag.
    // t039: remove in phase 5
    pub fn classic_messages_pending_flag_raw(&self) -> u8 {
        self.messages_first as u8
    }

    // t039: remove in phase 5
    pub fn set_classic_messages_pending_flag_raw(&mut self, value: u8) {
        self.messages_first = with_low_byte(self.messages_first, value);
    }

    pub fn set_classic_results_review_state_present(&mut self, present: bool) {
        self.messages_last = i32::from(present);
    }

    pub fn set_classic_messages_review_state_present(&mut self, present: bool) {
        self.messages_first = i32::from(present);
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
        crate::classic_results::set_login_reviewables_present(self, present)
    }

    /// Point this empire at its chain in the shared `RESULTS.DAT`.  Lay the
    /// file out with [`crate::layout_classic_results`] to get the chain.
    pub fn set_classic_results_chain(&mut self, chain: Option<ClassicResultsChain>) {
        crate::classic_results::set_results_chain(self, chain)
    }

    /// Stores `value` as classic text, cut to the 20 characters an empire
    /// name holds.
    pub fn set_controlled_empire_name_raw(&mut self, value: &str) {
        crate::names::set_empire_name(self, value)
    }

    /// Returns the empire to civil disorder as ECUTIL and ECMAINT do: mode
    /// 0, a blank handle, and the name "In Civil Disorder". Every other
    /// field stays.
    pub fn set_civil_disorder_mode(&mut self) {
        crate::player_lifecycle::set_civil_disorder(self)
    }

    pub fn ownership_summary(&self) -> String {
        crate::names::ownership_summary(self)
    }

    pub fn tax_rate(&self) -> u8 {
        self.tax_rate
    }

    pub fn set_tax_rate_raw(&mut self, value: u8) {
        self.tax_rate = value;
    }

    /// Record number of the empire's first starbase in BASES.DAT, the head
    /// of its chain; 0 for none.
    pub fn first_base_record_raw(&self) -> u16 {
        self.bases_first.0
    }

    pub fn set_first_base_record_raw(&mut self, value: u16) {
        self.bases_first.0 = value;
    }

    /// Record number of the empire's last starbase, the tail of its chain;
    /// 0 for none.
    pub fn last_base_record_raw(&self) -> u16 {
        self.bases_last.0
    }

    pub fn set_last_base_record_raw(&mut self, value: u16) {
        self.bases_last.0 = value;
    }

    pub fn fleet_chain_head_raw(&self) -> u16 {
        self.fleets_first.0
    }

    pub fn set_fleet_chain_head_raw(&mut self, value: u16) {
        self.fleets_first.0 = value;
    }

    pub fn fleet_chain_tail_raw(&self) -> u16 {
        self.fleets_last.0
    }

    pub fn set_fleet_chain_tail_raw(&mut self, value: u16) {
        self.fleets_last.0 = value;
    }

    /// Record number heading the empire's IPBM chain; 0 for none.
    pub fn ipbm_chain_head_raw(&self) -> u16 {
        self.ipbms_first.0
    }

    pub fn set_ipbm_chain_head_raw(&mut self, value: u16) {
        self.ipbms_first.0 = value;
    }

    pub fn homeworld_planet_index_1_based_raw(&self) -> u8 {
        self.homeworld
    }

    pub fn set_homeworld_planet_index_1_based_raw(&mut self, value: u8) {
        self.homeworld = value;
    }

    /// Game year of the empire's last sign-on; 0 before the first. Classic
    /// `ECGAME` shows it as `Last year on:`.
    pub fn last_run_year_raw(&self) -> u16 {
        self.last_year_on
    }

    pub fn set_last_run_year_raw(&mut self, value: u16) {
        self.last_year_on = value;
    }

    pub fn planet_count_raw(&self) -> u8 {
        self.planet_count
    }

    pub fn set_planet_count_raw(&mut self, value: u8) {
        self.planet_count = value;
    }

    pub fn production_score_raw(&self) -> u16 {
        self.production
    }

    pub fn set_production_score_raw(&mut self, value: u16) {
        self.production = value;
    }

    /// Autopilot flag. 1 = autopilot on (computer manages empire, mostly
    /// builds planetary defenses). 0 = human player submitting orders.
    pub fn autopilot_flag(&self) -> u8 {
        u8::from(self.autopilot)
    }

    pub fn set_autopilot_flag(&mut self, value: u8) {
        self.autopilot = value != 0;
    }

    pub fn set_player_mode_raw(&mut self, value: u8) {
        self.mode = PlayerMode::from_raw(value);
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
        crate::diplomacy::diplomatic_relation_toward(self, other_empire_raw)
    }

    pub fn diplomatic_relation_byte_raw(&self, other_empire_raw: u8) -> Option<u8> {
        crate::diplomacy::stored_diplomatic_relation(self, other_empire_raw)
            .map(DiplomaticRelation::to_raw)
    }

    pub fn set_diplomatic_relation_byte_raw(&mut self, other_empire_raw: u8, raw: u8) -> bool {
        self.set_diplomatic_relation_toward(other_empire_raw, DiplomaticRelation::from_raw(raw))
    }

    pub fn set_diplomatic_relation_toward(
        &mut self,
        other_empire_raw: u8,
        relation: DiplomaticRelation,
    ) -> bool {
        crate::diplomacy::set_diplomatic_relation_toward(self, other_empire_raw, relation)
    }
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
