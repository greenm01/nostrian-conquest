use crate::support::{ParseError, classic_text, copy_array, trim_ascii_field};
use crate::{ClassicResultsChain, PLAYER_RECORD_SIZE};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiplomaticRelation {
    Neutral,
    Enemy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerRecord {
    pub raw: [u8; PLAYER_RECORD_SIZE],
}

impl PlayerRecord {
    pub fn new_zeroed() -> Self {
        Self {
            raw: [0; PLAYER_RECORD_SIZE],
        }
    }

    /// Raw player mode byte at offset `0x00`.
    ///
    /// Confirmed classic meanings:
    /// - `0x00` = civil disorder / unjoined
    /// - `0x01` = active human player
    /// - `0xff` = rogue empire
    pub fn player_mode_raw(&self) -> u8 {
        self.raw[0]
    }

    pub fn occupied_flag(&self) -> u8 {
        self.raw[0]
    }

    /// Compatibility alias for the raw player mode byte.
    pub fn owner_mode_raw(&self) -> u8 {
        self.player_mode_raw()
    }

    pub fn is_civil_disorder_player(&self) -> bool {
        self.player_mode_raw() == 0x00
    }

    pub fn is_active_human_player(&self) -> bool {
        self.player_mode_raw() == 0x01
    }

    pub fn is_active_player(&self) -> bool {
        self.is_active_human_player()
    }

    pub fn is_rogue_player(&self) -> bool {
        self.player_mode_raw() == 0xff
    }

    pub fn is_active_or_rogue_player(&self) -> bool {
        self.is_active_human_player() || self.is_rogue_player()
    }

    pub fn handle_bytes(&self) -> &[u8] {
        &self.raw[1..0x1A]
    }

    pub fn empire_name_bytes(&self) -> &[u8] {
        &self.raw[0x1C..=0x2E]
    }

    pub fn assigned_player_flag_raw(&self) -> u8 {
        self.raw[0]
    }

    pub fn legacy_status_name_max_len_raw(&self) -> u8 {
        self.raw[26]
    }

    pub fn legacy_status_name_len_raw(&self) -> u8 {
        self.raw[27]
    }

    pub fn legacy_status_name_summary(&self) -> String {
        let len = self.legacy_status_name_len_raw() as usize;
        let end = (28 + len).min(self.raw.len());
        trim_ascii_field(&self.raw[28..end])
    }

    pub fn assigned_player_handle_summary(&self) -> String {
        trim_ascii_field(self.handle_bytes())
    }

    pub fn set_assigned_player_handle_raw(&mut self, value: &str) {
        let value = classic_text(value);
        let bytes = value.as_bytes();
        let len = bytes.len().min(25);
        self.raw[1..0x1A].fill(b' ');
        self.raw[1..1 + len].copy_from_slice(&bytes[..len]);
    }

    pub fn controlled_empire_name_len_raw(&self) -> u8 {
        self.raw[27]
    }

    pub fn controlled_empire_name_summary(&self) -> String {
        let len = self.controlled_empire_name_len_raw() as usize;
        let end = (28 + len).min(self.raw.len());
        trim_ascii_field(&self.raw[28..end])
    }

    /// Classic login/review state low word for the message-side family.
    ///
    /// Late-output RE shows classic ECGAME/ECMAINT treating `+0x30/+0x32` and
    /// `+0x34/+0x36` as two review-state families, not standalone booleans.
    pub fn classic_message_review_word_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x30], self.raw[0x31]])
    }

    pub fn set_classic_message_review_word_raw(&mut self, value: u16) {
        self.raw[0x30..0x32].copy_from_slice(&value.to_le_bytes());
    }

    pub fn classic_message_review_carry_word_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x32], self.raw[0x33]])
    }

    pub fn set_classic_message_review_carry_word_raw(&mut self, value: u16) {
        self.raw[0x32..0x34].copy_from_slice(&value.to_le_bytes());
    }

    /// Classic login/review state low word for the results/report-side family.
    pub fn classic_results_review_word_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x34], self.raw[0x35]])
    }

    pub fn set_classic_results_review_word_raw(&mut self, value: u16) {
        self.raw[0x34..0x36].copy_from_slice(&value.to_le_bytes());
    }

    pub fn classic_results_review_carry_word_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x36], self.raw[0x37]])
    }

    pub fn set_classic_results_review_carry_word_raw(&mut self, value: u16) {
        self.raw[0x36..0x38].copy_from_slice(&value.to_le_bytes());
    }

    pub fn has_classic_messages_review_state(&self) -> bool {
        self.classic_message_review_word_raw() != 0
            || self.classic_message_review_carry_word_raw() != 0
    }

    pub fn has_classic_results_review_state(&self) -> bool {
        self.classic_results_review_word_raw() != 0
            || self.classic_results_review_carry_word_raw() != 0
    }

    /// Classic `RESULTS.DAT` chain pointers at `+0x38..+0x3f`.
    ///
    /// Original `ECMAINT` output (two- and three-human oracle runs, see
    /// `fixtures/ecmaint-results-*`) writes:
    ///
    /// - `+0x38..+0x39` = 1-based record index of this empire's first report
    ///   header in the shared `RESULTS.DAT`, or `0` when it has none
    /// - `+0x3c..+0x3d` = 1-based record index of its last report header
    /// - `+0x3a..+0x3b` and `+0x3e..+0x3f` = zero
    pub fn classic_results_first_header_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x38], self.raw[0x39]])
    }

    pub fn set_classic_results_first_header_raw(&mut self, value: u16) {
        self.raw[0x38..0x3A].copy_from_slice(&value.to_le_bytes());
    }

    pub fn classic_results_last_header_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x3C], self.raw[0x3D]])
    }

    pub fn set_classic_results_last_header_raw(&mut self, value: u16) {
        self.raw[0x3C..0x3E].copy_from_slice(&value.to_le_bytes());
    }

    pub fn classic_results_chain(&self) -> Option<ClassicResultsChain> {
        let first_header = self.classic_results_first_header_raw();
        (first_header != 0).then(|| ClassicResultsChain {
            first_header,
            last_header: self.classic_results_last_header_raw(),
        })
    }

    pub fn has_classic_results_chain_state(&self) -> bool {
        self.classic_results_first_header_raw() != 0 || self.classic_results_last_header_raw() != 0
    }

    pub fn has_any_classic_review_state(&self) -> bool {
        self.has_classic_messages_review_state() || self.has_classic_results_review_state()
    }

    pub fn classic_reports_pending_flag_raw(&self) -> u8 {
        self.raw[0x34]
    }

    pub fn set_classic_reports_pending_flag_raw(&mut self, value: u8) {
        self.raw[0x34] = value;
    }

    pub fn classic_messages_pending_flag_raw(&self) -> u8 {
        self.raw[0x30]
    }

    pub fn set_classic_messages_pending_flag_raw(&mut self, value: u8) {
        self.raw[0x30] = value;
    }

    pub fn set_classic_results_review_state_present(&mut self, present: bool) {
        self.set_classic_results_review_word_raw(u16::from(present));
        self.set_classic_results_review_carry_word_raw(0);
    }

    pub fn set_classic_messages_review_state_present(&mut self, present: bool) {
        self.set_classic_message_review_word_raw(u16::from(present));
        self.set_classic_message_review_carry_word_raw(0);
    }

    /// Advertise login-time reviewables to classic ECGAME.
    ///
    /// Current compatibility rule:
    /// - classic delivery/review probes use both low words together for unread
    ///   mail
    /// - results-only startup probing does not become reviewable when only the
    ///   results-side low byte is set
    ///
    /// Until a stricter family split is proven in classic login flow, keep the
    /// mirrored advertisement here and route all startup-review producers
    /// through this helper.
    pub fn set_classic_login_reviewables_present(&mut self, present: bool) {
        self.set_classic_messages_review_state_present(present);
        self.set_classic_results_review_state_present(present);
    }

    /// Point this empire at its chain in the shared `RESULTS.DAT`.  Lay the
    /// file out with [`crate::layout_classic_results`] to get the chain.
    pub fn set_classic_results_chain(&mut self, chain: Option<ClassicResultsChain>) {
        self.raw[0x38..0x40].fill(0);
        if let Some(chain) = chain {
            self.set_classic_results_first_header_raw(chain.first_header);
            self.set_classic_results_last_header_raw(chain.last_header);
        }
    }

    pub fn set_controlled_empire_name_raw(&mut self, value: &str) {
        let value = classic_text(value);
        let bytes = value.as_bytes();
        let len = bytes.len().min(self.empire_name_bytes().len());
        self.raw[26] = self.empire_name_bytes().len() as u8;
        self.raw[27] = len as u8;
        self.raw[0x1C..=0x2E].fill(0);
        self.raw[0x1C..0x1C + len].copy_from_slice(&bytes[..len]);
    }

    pub fn set_legacy_status_name_raw(&mut self, value: &str) {
        let bytes = value.as_bytes();
        let len = bytes.len().min(self.raw.len().saturating_sub(28));
        self.raw[26] = len as u8;
        self.raw[27] = len as u8;
        self.raw[28..].fill(0);
        self.raw[28..28 + len].copy_from_slice(&bytes[..len]);
    }

    pub fn set_legacy_status_name_field_raw(&mut self, max_len: u8, value: &str) {
        let bytes = value.as_bytes();
        let len = bytes.len().min(self.raw.len().saturating_sub(28));
        self.raw[26] = max_len;
        self.raw[27] = len as u8;
        self.raw[28..].fill(0);
        self.raw[28..28 + len].copy_from_slice(&bytes[..len]);
    }

    pub fn set_civil_disorder_mode(&mut self) {
        self.set_player_mode_raw(0x00);
        self.raw[1..0x1A].fill(0);
        self.set_legacy_status_name_raw("In Civil Disorder");
    }

    pub fn ownership_summary(&self) -> String {
        let legacy = self.legacy_status_name_summary();
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
        self.raw[0x51]
    }

    pub fn set_tax_rate_raw(&mut self, value: u8) {
        self.raw[0x51] = value;
    }

    pub fn starbase_count_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x44], self.raw[0x45]])
    }

    pub fn set_starbase_count_raw(&mut self, value: u16) {
        let [lo, hi] = value.to_le_bytes();
        self.raw[0x44] = lo;
        self.raw[0x45] = hi;
    }

    pub fn starbase_presence_flag_raw(&self) -> u8 {
        self.raw[0x46]
    }

    pub fn set_starbase_presence_flag_raw(&mut self, value: u8) {
        self.raw[0x46] = value;
    }

    pub fn fleet_chain_head_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x40], self.raw[0x41]])
    }

    pub fn set_fleet_chain_head_raw(&mut self, value: u16) {
        let [lo, hi] = value.to_le_bytes();
        self.raw[0x40] = lo;
        self.raw[0x41] = hi;
    }

    pub fn fleet_chain_tail_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x42], self.raw[0x43]])
    }

    pub fn set_fleet_chain_tail_raw(&mut self, value: u16) {
        let [lo, hi] = value.to_le_bytes();
        self.raw[0x42] = lo;
        self.raw[0x43] = hi;
    }

    pub fn ipbm_count_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x48], self.raw[0x49]])
    }

    pub fn set_ipbm_count_raw(&mut self, value: u16) {
        let [lo, hi] = value.to_le_bytes();
        self.raw[0x48] = lo;
        self.raw[0x49] = hi;
    }

    pub fn homeworld_planet_index_1_based_raw(&self) -> u8 {
        self.raw[0x4C]
    }

    pub fn set_homeworld_planet_index_1_based_raw(&mut self, value: u8) {
        self.raw[0x4C] = value;
        self.raw[0x4D] = value;
    }

    /// Last year this empire successfully entered the game.
    ///
    /// Classic `ECGAME` displays this as the `Last year on:` status line during
    /// login/startup handling. Fresh pre-loaded slots keep this at `0`, while
    /// established returning players carry the current or prior game year.
    pub fn last_run_year_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x4E], self.raw[0x4F]])
    }

    pub fn set_last_run_year_raw(&mut self, value: u16) {
        self.raw[0x4E..0x50].copy_from_slice(&value.to_le_bytes());
    }

    pub fn planet_count_raw(&self) -> u8 {
        self.raw[0x50]
    }

    pub fn set_planet_count_raw(&mut self, value: u8) {
        self.raw[0x50] = value;
    }

    pub fn production_score_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x52], self.raw[0x53]])
    }

    pub fn set_production_score_raw(&mut self, value: u16) {
        self.raw[0x52..0x54].copy_from_slice(&value.to_le_bytes());
    }

    /// Unknown u16/u32-adjacent region beginning at 0x52. Current evidence:
    /// - `0x52..0x53` still appear to be a stable small constant in preserved
    ///   starts
    /// - `0x54..0x57` now map to per-empire diplomacy flags
    /// Do not treat the full 32-bit span as one settled field anymore.
    pub fn unknown_0x52_raw(&self) -> u32 {
        u32::from_le_bytes([
            self.raw[0x52],
            self.raw[0x53],
            self.raw[0x54],
            self.raw[0x55],
        ])
    }

    /// Autopilot flag. 1 = autopilot on (computer manages empire, mostly
    /// builds planetary defenses). 0 = human player submitting orders.
    /// Confirmed by black-box experiment: clearing this byte eliminates
    /// all autopilot-driven army/battery growth in ECMAINT.
    pub fn autopilot_flag(&self) -> u8 {
        self.raw[0x6D]
    }

    pub fn set_autopilot_flag(&mut self, value: u8) {
        self.raw[0x6D] = value;
    }

    /// Set the raw player mode byte at offset `0x00`.
    pub fn set_player_mode_raw(&mut self, value: u8) {
        self.raw[0] = value;
    }

    /// Set the occupied/present flag at offset `0x00`.
    /// This is a compatibility alias for the raw player mode byte.
    pub fn set_occupied_flag(&mut self, value: u8) {
        self.set_player_mode_raw(value);
    }

    /// Compatibility alias for the raw player mode byte.
    pub fn set_owner_empire_raw(&mut self, value: u8) {
        self.set_player_mode_raw(value);
    }

    /// Stored diplomatic relation toward another empire.
    ///
    /// Black-box confirmation from live `ECGAME` diplomacy menu:
    /// player 1 declaring empire 2 an enemy flips player-record byte `0x55`
    /// from `0x00 -> 0x01`, while the surrounding `0x54..0x57` bytes behave
    /// like one slot per empire.
    ///
    /// Current mapping:
    /// - `raw[0x54 + (other_empire_raw - 1)]`
    /// - `0x00 = Neutral`
    /// - `0x01 = Enemy`
    ///
    /// The player record is 110 bytes long and the known autopilot flag is at
    /// `0x6D`, so the contiguous range `0x54..=0x6C` cleanly provides 25
    /// diplomacy slots for the documented maximum of 25 players.
    pub fn diplomatic_relation_toward(&self, other_empire_raw: u8) -> Option<DiplomaticRelation> {
        if !(1..=25).contains(&other_empire_raw) {
            return None;
        }
        match self.diplomatic_relation_byte_raw(other_empire_raw)? {
            0x00 => Some(DiplomaticRelation::Neutral),
            0x01 => Some(DiplomaticRelation::Enemy),
            _ => None,
        }
    }

    pub fn diplomatic_relation_byte_raw(&self, other_empire_raw: u8) -> Option<u8> {
        if !(1..=25).contains(&other_empire_raw) {
            return None;
        }
        Some(self.raw[0x54 + other_empire_raw as usize - 1])
    }

    pub fn set_diplomatic_relation_byte_raw(&mut self, other_empire_raw: u8, raw: u8) -> bool {
        if !(1..=25).contains(&other_empire_raw) {
            return false;
        }
        self.raw[0x54 + other_empire_raw as usize - 1] = raw;
        true
    }

    pub fn set_diplomatic_relation_toward(
        &mut self,
        other_empire_raw: u8,
        relation: DiplomaticRelation,
    ) -> bool {
        if !(1..=25).contains(&other_empire_raw) {
            return false;
        }
        self.set_diplomatic_relation_byte_raw(
            other_empire_raw,
            match relation {
                DiplomaticRelation::Neutral => 0x00,
                DiplomaticRelation::Enemy => 0x01,
            },
        );
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerDat {
    pub records: Vec<PlayerRecord>,
}

impl PlayerDat {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        if data.len() % PLAYER_RECORD_SIZE != 0 {
            return Err(ParseError::WrongRecordMultiple {
                file_type: "PLAYER.DAT",
                record_size: PLAYER_RECORD_SIZE,
                actual: data.len(),
            });
        }
        Ok(Self {
            records: data
                .chunks_exact(PLAYER_RECORD_SIZE)
                .map(|chunk| PlayerRecord {
                    raw: copy_array(chunk),
                })
                .collect(),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.records
            .iter()
            .flat_map(|record| record.raw)
            .collect::<Vec<_>>()
    }
}
