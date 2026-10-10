use super::RecordNumber;

/// Who plays an empire, PLAYER.DAT 0x00.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PlayerMode {
    /// 0: unowned, "In Civil Disorder".
    #[default]
    CivilDisorder,
    /// 1: a human player.
    Human,
    /// 0xFF: played by ECMAINT.
    Rogue,
    Unknown(u8),
}

impl PlayerMode {
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            0x00 => Self::CivilDisorder,
            0x01 => Self::Human,
            0xFF => Self::Rogue,
            other => Self::Unknown(other),
        }
    }

    pub fn to_raw(self) -> u8 {
        match self {
            Self::CivilDisorder => 0x00,
            Self::Human => 0x01,
            Self::Rogue => 0xFF,
            Self::Unknown(raw) => raw,
        }
    }
}

/// One empire's stance toward another, PLAYER.DAT 0x54 + (empire - 1).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DiplomaticRelation {
    #[default]
    Neutral,
    Enemy,
    /// A byte no classic program writes; the sanitizer resets it.
    Unknown(u8),
}

impl DiplomaticRelation {
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            0x00 => Self::Neutral,
            0x01 => Self::Enemy,
            other => Self::Unknown(other),
        }
    }

    pub fn to_raw(self) -> u8 {
        match self {
            Self::Neutral => 0x00,
            Self::Enemy => 0x01,
            Self::Unknown(raw) => raw,
        }
    }
}

/// One PLAYER.DAT record: an empire slot.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Player {
    /// 0x00
    pub mode: PlayerMode,
    /// 0x01: the owner's BBS log-on name, up to 25 characters; classic pads
    /// it with spaces and upper-cases it.
    pub handle: String,
    /// 0x1A: lines per screen for ECGAME's "More" pause; ECUTIL writes 24.
    pub page_lines: u8,
    /// 0x1B: `String[20]`; "In Civil Disorder" for an unowned empire.
    pub empire_name: String,
    /// 0x30: MESSAGES.DAT record of the first waiting message; 0 for none.
    pub messages_first: i32,
    /// 0x34: MESSAGES.DAT record of the last waiting message.
    pub messages_last: i32,
    /// 0x38: RESULTS.DAT record of the first report header; 0 for none.
    pub results_first: i32,
    /// 0x3C: RESULTS.DAT record of the last report header.
    pub results_last: i32,
    /// 0x40: head of the empire's fleet chain.
    pub fleets_first: RecordNumber,
    /// 0x42: tail of the fleet chain.
    pub fleets_last: RecordNumber,
    /// 0x44: head of the starbase chain.
    pub bases_first: RecordNumber,
    /// 0x46: tail of the starbase chain.
    pub bases_last: RecordNumber,
    /// 0x48: head of the IPBM chain.
    pub ipbms_first: RecordNumber,
    /// 0x4A: tail of the IPBM chain.
    pub ipbms_last: RecordNumber,
    /// 0x4C: PLANETS.DAT record of the starting world (ECUTIL writes the
    /// same number to 0x4D).
    pub homeworld: u8,
    /// 0x4E: game year of the last sign-on; 0 before the first.
    pub last_year_on: u16,
    /// 0x50: planets owned, recounted by ECMAINT.
    pub planet_count: u8,
    /// 0x51: tax rate in percent.
    pub tax_rate: u8,
    /// 0x52: `Trunc` of the summed present production of the empire's
    /// planets, recomputed by ECMAINT.
    pub production: u16,
    /// 0x54: stance toward empires 1 to 25.
    pub diplomacy: [DiplomaticRelation; 25],
    /// 0x6D: ECMAINT plays this human empire.
    pub autopilot: bool,
}
