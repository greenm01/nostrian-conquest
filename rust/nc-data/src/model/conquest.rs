/// Which weekdays ECMAINT runs a scheduled pass on, Sunday first.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MaintenanceDays(pub [bool; 7]);

/// CONQUEST.DAT: the game clock and the previous year's rankings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Conquest {
    /// 0x000: current game year.
    pub game_year: u16,
    /// 0x002: number of empire slots (4, 9, 16, or 25).
    pub player_count: u8,
    /// 0x003: the maintenance schedule.
    pub maintenance_days: MaintenanceDays,
    /// 0x00A: each empire's production as it stood at the start of the most
    /// recent maintenance pass, shown in the rankings in parentheses.
    pub previous_production: [u16; 25],
    /// 0x03C: each empire's planet count at the start of the most recent
    /// pass.
    pub previous_planet_count: [u8; 25],
}
