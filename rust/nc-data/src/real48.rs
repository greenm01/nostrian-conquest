//! Turbo Pascal 6-byte `Real`, computed bit-for-bit as the ECMAINT runtime
//! library does.
//!
//! Layout: byte 0 is the biased exponent (0 means 0.0); bytes 1..=5 hold the
//! 39-bit mantissa little-endian with the sign in the top bit of byte 5.
//! The value is `(1 + m / 2^39) * 2^(e - 129)`.
//!
//! The arithmetic ports the ECMAINT runtime routines (segment 3374): add/sub
//! align into a 48-bit register with one guard byte and truncate shifted-out
//! bits; mul keeps the top 48 bits of the product (a short path drops one
//! partial product when an operand's low 24 mantissa bits are zero); div
//! produces 42 quotient bits plus a sticky `0x3f`; every result rounds by
//! adding `0x80` to the guard byte. `sqrt` is the runtime's Newton loop.
//! Fleet motion depends on this rounding (a climb ends one ulp short of 1.0,
//! so the exact `Z == 1.0` arrival test fires a year later).

use std::cmp::Ordering;
use std::ops::{Add, Div, Mul, Neg, Sub};

const HIDDEN: u64 = 1 << 39;
const MANTISSA_MASK: u64 = HIDDEN - 1;
const REGISTER_TOP: u64 = 1 << 47;

#[derive(Debug, Clone, Copy)]
pub struct Real48 {
    bytes: [u8; 6],
}

/// Unpacked operand: exponent, sign, 40-bit mantissa with the hidden bit set.
#[derive(Clone, Copy)]
struct Parts {
    exp: i32,
    negative: bool,
    mantissa: u64,
}

impl Real48 {
    pub const ZERO: Self = Self { bytes: [0; 6] };
    pub const HALF: Self = Self {
        bytes: [0x80, 0, 0, 0, 0, 0],
    };
    pub const ONE: Self = Self {
        bytes: [0x81, 0, 0, 0, 0, 0],
    };

    pub const fn from_bytes(bytes: [u8; 6]) -> Self {
        Self { bytes }
    }

    pub const fn to_bytes(self) -> [u8; 6] {
        self.bytes
    }

    pub fn is_zero(self) -> bool {
        self.bytes[0] == 0
    }

    /// Integer to real, exact for every `i32`.
    pub fn from_int(value: i32) -> Self {
        if value == 0 {
            return Self::ZERO;
        }
        let magnitude = u64::from(value.unsigned_abs());
        let bits = 64 - magnitude.leading_zeros() as i32;
        Self::pack(Parts {
            exp: 128 + bits,
            negative: value < 0,
            mantissa: magnitude << (40 - bits),
        })
    }

    /// The nearest real to `value`, rounding the mantissa to nearest. A
    /// value the real cannot hold (not finite, or outside its exponent
    /// range) becomes 0.
    pub fn from_f64(value: f64) -> Self {
        if !value.is_finite() || value == 0.0 {
            return Self::ZERO;
        }
        let abs = value.abs();
        let exponent_unbiased = abs.log2().floor() as i32;
        let normalized = abs / 2f64.powi(exponent_unbiased);
        if !(1.0..2.0).contains(&normalized) {
            return Self::ZERO;
        }
        let mut exp = exponent_unbiased + 129;
        if !(1..=255).contains(&exp) {
            return Self::ZERO;
        }
        let mut mantissa = ((normalized - 1.0) * (HIDDEN as f64)).round() as u64;
        if mantissa == HIDDEN {
            mantissa = 0;
            exp += 1;
            if exp > 255 {
                return Self::ZERO;
            }
        }
        Self::pack(Parts {
            exp,
            negative: value.is_sign_negative(),
            mantissa: mantissa | HIDDEN,
        })
    }

    /// Exact conversion; a 40-bit significand always fits an `f64`.
    pub fn to_f64(self) -> f64 {
        if self.is_zero() {
            return 0.0;
        }
        let parts = self.unpack();
        let value = parts.mantissa as f64 * 2f64.powi(parts.exp - 129 - 39);
        if parts.negative { -value } else { value }
    }

    pub fn abs(self) -> Self {
        let mut bytes = self.bytes;
        bytes[5] &= 0x7f;
        Self { bytes }
    }

    fn negate(self) -> Self {
        if self.is_zero() {
            return self;
        }
        let mut bytes = self.bytes;
        bytes[5] ^= 0x80;
        Self { bytes }
    }

    /// Pascal `Trunc`.
    pub fn trunc(self) -> i32 {
        self.to_f64().trunc() as i32
    }

    /// Pascal `Round`: halves round away from zero.
    pub fn round(self) -> i32 {
        let value = self.to_f64();
        let magnitude = (value.abs() + 0.5).floor();
        if value < 0.0 {
            -(magnitude as i32)
        } else {
            magnitude as i32
        }
    }

    fn sum(self, other: Self) -> Self {
        if other.is_zero() {
            return self;
        }
        if self.is_zero() {
            return other;
        }
        let (small, large) = if self.bytes[0] <= other.bytes[0] {
            (self.unpack(), other.unpack())
        } else {
            (other.unpack(), self.unpack())
        };
        let shift = large.exp - small.exp;
        if shift >= 40 {
            return Self::pack(large);
        }
        let mut exp = large.exp;
        let mut negative = small.negative;
        let aligned_small = (small.mantissa << 8) >> shift;
        let large_register = large.mantissa << 8;
        if small.negative == large.negative {
            let mut sum = aligned_small + large_register;
            if sum >> 48 != 0 {
                sum >>= 1;
                exp += 1;
            }
            return Self::round_register(exp, negative, sum);
        }
        let mut difference = if aligned_small >= large_register {
            aligned_small - large_register
        } else {
            negative = !negative;
            large_register - aligned_small
        };
        if difference == 0 {
            return Self::ZERO;
        }
        while difference & REGISTER_TOP == 0 {
            difference <<= 1;
            exp -= 1;
            if exp == 0 {
                return Self::ZERO;
            }
        }
        Self::round_register(exp, negative, difference)
    }

    fn product(self, other: Self) -> Self {
        if self.is_zero() || other.is_zero() {
            return Self::ZERO;
        }
        let (a, b) = (self.unpack(), other.unpack());
        let low_24 = 0xff_ffff;
        let mut product = if a.mantissa & low_24 == 0 || b.mantissa & low_24 == 0 {
            let (short, long) = if a.mantissa & low_24 == 0 {
                (a.mantissa, b.mantissa)
            } else {
                (b.mantissa, a.mantissa)
            };
            let word = short >> 24;
            let (low, mid, high) = (long & 0xff, (long >> 8) & 0xffff, long >> 24);
            ((word >> 8) * low + word * mid + ((word * high) << 16)) & ((1 << 48) - 1)
        } else {
            ((u128::from(a.mantissa) * u128::from(b.mantissa)) >> 32) as u64
        };
        let mut exp = a.exp + b.exp - 128;
        if product & REGISTER_TOP == 0 {
            product <<= 1;
            exp -= 1;
        }
        Self::round_register(exp, a.negative != b.negative, product)
    }

    fn quotient(self, other: Self) -> Self {
        if self.is_zero() {
            return Self::ZERO;
        }
        let (a, b) = (self.unpack(), other.unpack());
        let mut remainder = a.mantissa;
        let mut quotient = 0u64;
        for _ in 0..42 {
            let bit = remainder >= b.mantissa;
            if bit {
                remainder -= b.mantissa;
            }
            quotient = (quotient << 1) | u64::from(bit);
            remainder <<= 1;
        }
        let mut register = (quotient << 6) | 0x3f;
        let mut exp = a.exp - b.exp + 129;
        if register & REGISTER_TOP == 0 {
            register <<= 1;
            exp -= 1;
        }
        Self::round_register(exp, a.negative != b.negative, register)
    }

    /// The runtime's `Sqrt`: Newton steps from the argument's mantissa at
    /// half its exponent until a correction falls 20 binades below the
    /// first guess.
    pub fn sqrt(self) -> Self {
        if self.is_zero() {
            return self;
        }
        let guess_exp = ((self.bytes[0].wrapping_add(0x80) as i8) >> 1) as u8;
        let guess_exp = guess_exp.wrapping_add(0x80);
        let threshold = guess_exp.wrapping_sub(0x14);
        let mut guess = self;
        guess.bytes[0] = guess_exp;
        loop {
            let mut next = self / guess + guess;
            next.bytes[0] = next.bytes[0].wrapping_sub(1);
            let correction = next - guess;
            guess = next;
            if correction.bytes[0] < threshold {
                return guess;
            }
        }
    }

    fn unpack(self) -> Parts {
        let raw = u64::from_le_bytes([
            self.bytes[1],
            self.bytes[2],
            self.bytes[3],
            self.bytes[4],
            self.bytes[5],
            0,
            0,
            0,
        ]);
        Parts {
            exp: i32::from(self.bytes[0]),
            negative: raw & HIDDEN != 0,
            mantissa: (raw & MANTISSA_MASK) | HIDDEN,
        }
    }

    fn pack(parts: Parts) -> Self {
        if parts.exp <= 0 {
            return Self::ZERO;
        }
        let raw = (parts.mantissa & MANTISSA_MASK) | if parts.negative { HIDDEN } else { 0 };
        let mantissa = raw.to_le_bytes();
        Self {
            bytes: [
                parts.exp.min(255) as u8,
                mantissa[0],
                mantissa[1],
                mantissa[2],
                mantissa[3],
                mantissa[4],
            ],
        }
    }

    /// Round a normalized 48-bit register (top bit set) on its guard byte.
    fn round_register(mut exp: i32, negative: bool, mut register: u64) -> Self {
        register += 0x80;
        if register >> 48 != 0 {
            register >>= 1;
            exp += 1;
        }
        Self::pack(Parts {
            exp,
            negative,
            mantissa: register >> 8,
        })
    }
}

impl Default for Real48 {
    fn default() -> Self {
        Self::ZERO
    }
}

impl PartialEq for Real48 {
    fn eq(&self, other: &Self) -> bool {
        self.to_f64() == other.to_f64()
    }
}

impl Eq for Real48 {}

impl PartialOrd for Real48 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Real48 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_f64().total_cmp(&other.to_f64())
    }
}

impl Add for Real48 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        self.sum(other)
    }
}

impl Sub for Real48 {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        self.sum(other.negate())
    }
}

impl Mul for Real48 {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        self.product(other)
    }
}

impl Div for Real48 {
    type Output = Self;
    fn div(self, other: Self) -> Self {
        self.quotient(other)
    }
}

impl Neg for Real48 {
    type Output = Self;
    fn neg(self) -> Self {
        self.negate()
    }
}
