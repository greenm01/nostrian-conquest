/// An empire's number, 1 to 25; 0 means none.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EmpireId(pub u8);

/// A fleet's number within its empire ("Fleet 1"); 0 means none.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FleetId(pub u16);

/// A starbase's number within its empire ("Starbase 1"); 0 means none.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StarbaseId(pub u16);

/// A 1-based record number in a chained classic file; 0 ends a chain.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RecordNumber(pub u16);

/// A map sector, 1-based.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Coords {
    pub x: u8,
    pub y: u8,
}

impl Coords {
    pub const fn new(x: u8, y: u8) -> Self {
        Self { x, y }
    }

    pub const fn from_array([x, y]: [u8; 2]) -> Self {
        Self { x, y }
    }

    pub const fn to_array(self) -> [u8; 2] {
        [self.x, self.y]
    }
}
