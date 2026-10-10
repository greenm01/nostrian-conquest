/// Translate a Unicode character to its CP437 byte equivalent.
///
/// Printable ASCII (U+0020..=U+007E) passes through unchanged.
/// Box-drawing characters used by the table renderer are mapped to their
/// standard CP437 single-byte codes.  Everything else maps to `b'?'`.
pub fn unicode_char_to_cp437(ch: char) -> u8 {
    match ch {
        // Printable ASCII passes through.
        '\x20'..='\x7E' => ch as u8,

        // Box-drawing characters used by screen/table.rs:
        '─' => 0xC4, // U+2500  horizontal line
        '│' => 0xB3, // U+2502  vertical line
        '┌' => 0xDA, // U+250C  top-left corner
        '┐' => 0xBF, // U+2510  top-right corner
        '└' => 0xC0, // U+2514  bottom-left corner
        '┘' => 0xD9, // U+2518  bottom-right corner
        '├' => 0xC3, // U+251C  left tee
        '┤' => 0xB4, // U+2524  right tee
        '┬' => 0xC2, // U+252C  top tee
        '┴' => 0xC1, // U+2534  bottom tee
        '┼' => 0xC5, // U+253C  cross / plus

        // Fallback.
        _ => b'?',
    }
}

/// Convert a Unicode string to a CP437 byte vector.
pub fn str_to_cp437(s: &str) -> Vec<u8> {
    s.chars().map(unicode_char_to_cp437).collect()
}

/// Unicode characters for CP437 bytes `0x80..=0xFF`.
const CP437_HIGH: [char; 128] = [
    // 0x80
    'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ì', 'Ä', 'Å',
    // 0x90
    'É', 'æ', 'Æ', 'ô', 'ö', 'ò', 'û', 'ù', 'ÿ', 'Ö', 'Ü', '¢', '£', '¥', '₧', 'ƒ',
    // 0xA0
    'á', 'í', 'ó', 'ú', 'ñ', 'Ñ', 'ª', 'º', '¿', '⌐', '¬', '½', '¼', '¡', '«', '»',
    // 0xB0
    '░', '▒', '▓', '│', '┤', '╡', '╢', '╖', '╕', '╣', '║', '╗', '╝', '╜', '╛', '┐',
    // 0xC0
    '└', '┴', '┬', '├', '─', '┼', '╞', '╟', '╚', '╔', '╩', '╦', '╠', '═', '╬', '╧',
    // 0xD0
    '╨', '╤', '╥', '╙', '╘', '╒', '╓', '╫', '╪', '┘', '┌', '█', '▄', '▌', '▐', '▀',
    // 0xE0
    'α', 'ß', 'Γ', 'π', 'Σ', 'σ', 'µ', 'τ', 'Φ', 'Θ', 'Ω', 'δ', '∞', 'φ', 'ε', '∩',
    // 0xF0
    '≡', '±', '≥', '≤', '⌠', '⌡', '÷', '≈', '°', '∙', '·', '√', 'ⁿ', '²', '■', '\u{a0}',
];

/// Decode CP437 text (for example a BBS dropfile). Bytes below `0x80` are
/// ASCII.
pub fn cp437_to_string(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&byte| match byte {
            0x00..=0x7F => byte as char,
            _ => CP437_HIGH[usize::from(byte - 0x80)],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_passthrough() {
        for byte in 0x20u8..=0x7E {
            assert_eq!(unicode_char_to_cp437(byte as char), byte);
        }
    }

    #[test]
    fn box_drawing_mappings() {
        let cases: &[(char, u8)] = &[
            ('─', 0xC4),
            ('│', 0xB3),
            ('┌', 0xDA),
            ('┐', 0xBF),
            ('└', 0xC0),
            ('┘', 0xD9),
            ('├', 0xC3),
            ('┤', 0xB4),
            ('┬', 0xC2),
            ('┴', 0xC1),
            ('┼', 0xC5),
        ];
        for &(unicode, expected) in cases {
            assert_eq!(
                unicode_char_to_cp437(unicode),
                expected,
                "mismatch for U+{:04X} '{unicode}'",
                unicode as u32,
            );
        }
    }

    #[test]
    fn unknown_maps_to_question_mark() {
        assert_eq!(unicode_char_to_cp437('€'), b'?');
        assert_eq!(unicode_char_to_cp437('你'), b'?');
    }

    #[test]
    fn decodes_cp437_text() {
        assert_eq!(cp437_to_string(b"Jos\x82 St\x94rlord"), "José Störlord");
        assert_eq!(cp437_to_string(b"\xc9\xcd\xbb\xb0\xdb"), "╔═╗░█");
        assert_eq!(cp437_to_string(b"plain"), "plain");
    }

    #[test]
    fn str_conversion() {
        let s = "┌──┐";
        let bytes = str_to_cp437(s);
        assert_eq!(bytes, vec![0xDA, 0xC4, 0xC4, 0xBF]);
    }
}
