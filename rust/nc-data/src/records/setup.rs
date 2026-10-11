use crate::SETUP_DAT_SIZE;
use crate::classic::{decode_setup, encode_setup};
use crate::model::Setup;

impl Setup {
    pub fn from_raw(raw: [u8; SETUP_DAT_SIZE]) -> Self {
        decode_setup(&raw)
    }

    pub fn version_tag(&self) -> &[u8] {
        &self.version_tag
    }
    pub fn set_version_tag(&mut self, value: &[u8; 5]) {
        self.version_tag = *value;
    }
    /// The COM IRQs followed by the COM flow-control flags, SETUP.DAT 5..12.
    pub fn option_prefix(&self) -> [u8; 8] {
        let mut prefix = [0u8; 8];
        prefix[..4].copy_from_slice(&self.com_irq);
        for (slot, enabled) in prefix[4..].iter_mut().zip(self.com_hw_flow_control) {
            *slot = u8::from(enabled);
        }
        prefix
    }
    pub fn set_option_prefix(&mut self, value: &[u8; 8]) {
        self.com_irq.copy_from_slice(&value[..4]);
        for (slot, &byte) in self.com_hw_flow_control.iter_mut().zip(&value[4..]) {
            *slot = byte != 0;
        }
    }
    pub fn com_irq_raw(&self, com_index: usize) -> Option<u8> {
        self.com_irq.get(com_index).copied()
    }
    pub fn set_com_irq_raw(&mut self, com_index: usize, irq: u8) -> bool {
        match self.com_irq.get_mut(com_index) {
            Some(slot) => {
                *slot = irq;
                true
            }
            None => false,
        }
    }
    pub fn com_hardware_flow_control_enabled(&self, com_index: usize) -> Option<bool> {
        self.com_hw_flow_control.get(com_index).copied()
    }
    pub fn set_com_hardware_flow_control_enabled(
        &mut self,
        com_index: usize,
        enabled: bool,
    ) -> bool {
        match self.com_hw_flow_control.get_mut(com_index) {
            Some(slot) => {
                *slot = enabled;
                true
            }
            None => false,
        }
    }
    pub fn snoop_enabled(&self) -> bool {
        self.snoop
    }
    pub fn set_snoop_enabled(&mut self, enabled: bool) {
        self.snoop = enabled;
    }
    /// Idle limit between keystrokes; 0 disables the timeout.
    pub fn max_key_gap_minutes(&self) -> u16 {
        self.max_key_gap_minutes
    }
    pub fn set_max_key_gap_minutes(&mut self, minutes: u16) {
        self.max_key_gap_minutes = minutes;
    }
    /// The key-gap limit as the byte ECUTIL enters.
    // t039: remove in phase 5
    pub fn max_time_between_keys_minutes_raw(&self) -> u8 {
        self.max_key_gap_minutes.min(u16::from(u8::MAX)) as u8
    }
    // t039: remove in phase 5
    pub fn set_max_time_between_keys_minutes_raw(&mut self, minutes: u8) {
        self.max_key_gap_minutes = u16::from(minutes);
    }
    pub fn remote_timeout_enabled(&self) -> bool {
        self.remote_timeout
    }
    pub fn set_remote_timeout_enabled(&mut self, enabled: bool) {
        self.remote_timeout = enabled;
    }
    pub fn local_timeout_enabled(&self) -> bool {
        self.local_timeout
    }
    pub fn set_local_timeout_enabled(&mut self, enabled: bool) {
        self.local_timeout = enabled;
    }
    pub fn minimum_time_granted_minutes_raw(&self) -> u8 {
        self.minimum_time_minutes
    }
    pub fn set_minimum_time_granted_minutes_raw(&mut self, minutes: u8) {
        self.minimum_time_minutes = minutes;
    }
    /// "Purge messages & reports after" this many turns; no v1.5 program
    /// reads it.
    pub fn purge_after_turns(&self) -> u16 {
        self.purge_after_turns
    }
    pub fn set_purge_after_turns(&mut self, turns: u16) {
        self.purge_after_turns = turns;
    }
    // t039: remove in phase 5
    pub fn purge_after_turns_raw(&self) -> u8 {
        self.purge_after_turns.min(u16::from(u8::MAX)) as u8
    }
    // t039: remove in phase 5
    pub fn set_purge_after_turns_raw(&mut self, turns: u8) {
        self.purge_after_turns = u16::from(turns);
    }
    /// Years of inactivity before ECMAINT puts a human empire on autopilot;
    /// 0 never does.
    pub fn autopilot_after_turns(&self) -> u16 {
        self.autopilot_after_turns
    }
    pub fn set_autopilot_after_turns(&mut self, turns: u16) {
        self.autopilot_after_turns = turns;
    }
    // t039: remove in phase 5
    pub fn autopilot_inactive_turns_raw(&self) -> u8 {
        self.autopilot_after_turns.min(u16::from(u8::MAX)) as u8
    }
    // t039: remove in phase 5
    pub fn set_autopilot_inactive_turns_raw(&mut self, turns: u8) {
        self.autopilot_after_turns = u16::from(turns);
    }
    pub fn to_bytes(&self) -> Vec<u8> {
        encode_setup(self).to_vec()
    }
}
