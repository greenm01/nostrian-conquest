/// SETUP.DAT: the installation settings ECUTIL keeps for ECGAME and ECMAINT.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Setup {
    /// 0: `"EC"` and the version digits, `"EC151"` for v1.51.
    pub version_tag: [u8; 5],
    /// 5: IRQ for COM1..COM4.
    pub com_irq: [u8; 4],
    /// 9: hardware flow control for COM1..COM4.
    pub com_hw_flow_control: [bool; 4],
    /// 512: mirror a remote caller's session on the local screen.
    pub snoop: bool,
    /// 513: idle limit between keystrokes; 0 disables the timeout.
    pub max_key_gap_minutes: u16,
    /// 515: apply the key-gap timeout to remote callers.
    pub remote_timeout: bool,
    /// 516: apply the key-gap timeout to local users.
    pub local_timeout: bool,
    /// 517: ECGAME raises a session's remaining time to at least this.
    pub minimum_time_minutes: u8,
    /// 518: "Purge messages & reports after" this many turns; no v1.5
    /// program reads it.
    pub purge_after_turns: u16,
    /// 520: ECMAINT puts an inactive human empire on autopilot after this
    /// many years; 0 never does.
    pub autopilot_after_turns: u16,
}
