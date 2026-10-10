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
        }
    }
}

impl std::error::Error for ParseError {}

pub fn expect_size(
    data: &[u8],
    expected: usize,
    file_type: &'static str,
) -> Result<(), ParseError> {
    if data.len() == expected {
        Ok(())
    } else {
        Err(ParseError::WrongSize {
            file_type,
            expected,
            actual: data.len(),
        })
    }
}

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

pub fn trim_ascii_field(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    text.trim_matches(char::from(0)).trim().to_string()
}

pub fn decode_real48(data: [u8; 6]) -> Option<f64> {
    let exponent = data[5];
    if exponent == 0 {
        return Some(0.0);
    }

    let sign = if (data[4] & 0x80) != 0 { -1.0 } else { 1.0 };
    let mantissa = u64::from(data[0])
        | (u64::from(data[1]) << 8)
        | (u64::from(data[2]) << 16)
        | (u64::from(data[3]) << 24)
        | (u64::from(data[4] & 0x7f) << 32);
    let fractional = mantissa as f64 / ((1u64 << 39) as f64);
    Some(sign * (1.0 + fractional) * 2f64.powi(i32::from(exponent) - 129))
}

pub fn encode_real48(value: f64) -> Option<[u8; 6]> {
    if !value.is_finite() {
        return None;
    }
    if value == 0.0 {
        return Some([0; 6]);
    }

    let sign_bit = if value.is_sign_negative() { 0x80 } else { 0x00 };
    let abs = value.abs();
    let exponent_unbiased = abs.log2().floor() as i32;
    let normalized = abs / 2f64.powi(exponent_unbiased);
    if !(1.0..2.0).contains(&normalized) {
        return None;
    }

    let mut exponent = exponent_unbiased + 129;
    if !(1..=255).contains(&exponent) {
        return None;
    }

    let mut mantissa = ((normalized - 1.0) * ((1u64 << 39) as f64)).round() as u64;
    if mantissa == (1u64 << 39) {
        mantissa = 0;
        exponent += 1;
        if exponent > 255 {
            return None;
        }
    }

    Some([
        (mantissa & 0xff) as u8,
        ((mantissa >> 8) & 0xff) as u8,
        ((mantissa >> 16) & 0xff) as u8,
        ((mantissa >> 24) & 0xff) as u8,
        (((mantissa >> 32) & 0x7f) as u8) | sign_bit,
        exponent as u8,
    ])
}
