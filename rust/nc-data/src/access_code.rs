//! Short codes people type: invite codes and Matrix sign-in codes. Only their
//! hashes are stored.
use rand::Rng;

/// Upper-case letters and digits without the look-alikes I, O, 0 and 1.
const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

pub fn generate() -> String {
    let mut rng = rand::thread_rng();
    (0..8)
        .map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char)
        .collect()
}

/// Hash of a code as typed, ignoring surrounding space and letter case.
pub fn hash(code: &str) -> String {
    blake3::hash(code.trim().to_ascii_uppercase().as_bytes())
        .to_hex()
        .to_string()
}
