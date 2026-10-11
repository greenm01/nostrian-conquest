use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    WrongSize {
        file_type: &'static str,
        expected: usize,
        actual: usize,
    },
    WrongRecordMultiple {
        file_type: &'static str,
        record_size: usize,
        actual: usize,
    },
    /// A PLANETS.DAT world with units ordered in build slot 8, which classic
    /// reserves and Rust has no unit for.
    ReservedBuildSlot {
        planet: String,
        sector: [u8; 2],
        units: u16,
    },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongSize {
                file_type,
                expected,
                actual,
            } => write!(
                f,
                "{file_type} had wrong size: expected {expected} bytes, got {actual}"
            ),
            Self::WrongRecordMultiple {
                file_type,
                record_size,
                actual,
            } => write!(
                f,
                "{file_type} had wrong size: expected a multiple of {record_size} bytes, got {actual}"
            ),
            Self::ReservedBuildSlot {
                planet,
                sector,
                units,
            } => write!(
                f,
                "planet \"{planet}\" at {},{} has {units} units ordered in reserved build slot 8, which Rust has no unit for; clear the slot to import",
                sector[0], sector[1]
            ),
        }
    }
}

impl std::error::Error for ParseError {}

/// Classic text fields (planet names, `RESULTS.DAT` lines) hold printable
/// ASCII only: space through `~`.
pub fn is_classic_text_char(ch: char) -> bool {
    ch == ' ' || ch.is_ascii_graphic()
}

/// Text as a classic field stores it: any other character becomes `?`.
pub fn classic_text(value: &str) -> String {
    value
        .chars()
        .map(|ch| if is_classic_text_char(ch) { ch } else { '?' })
        .collect()
}

pub fn copy_array<const N: usize>(data: &[u8]) -> [u8; N] {
    let mut out = [0u8; N];
    out.copy_from_slice(data);
    out
}
