/// Translate a Unicode character to its CP437 byte equivalent.
///
/// Printable ASCII (U+0020..=U+007E) passes through unchanged, and every
/// glyph of CP437 bytes 0x80..=0xFE (box drawing, shading, accented letters,
/// ...) maps to its CP437 byte. Common punctuation with no CP437 glyph maps to the
/// nearest ASCII character. Everything else maps to `b'?'`.
///
/// Each character is one screen cell, so it always becomes exactly one byte.
pub fn unicode_char_to_cp437(ch: char) -> u8 {
    match ch {
        // Printable ASCII passes through.
        '\x20'..='\x7E' => ch as u8,

        // Punctuation without a CP437 glyph.
        '—' | '–' | '‐' | '‑' | '−' => b'-',
        '‘' | '’' | '‚' | '′' => b'\'',
        '“' | '”' | '„' | '″' => b'"',
        '…' => b'.',
        '×' => b'x',
        // CP437 has arrow glyphs only at control codes (0x18..=0x1B).
        '←' => b'<',
        '→' => b'>',
        '↑' => b'^',
        '↓' => b'v',
        // CP437 0xFF is the non-breaking space, but on a telnet socket it is
        // IAC.
        '\u{a0}' => b' ',

        _ => CP437_HIGH
            .iter()
            .position(|&glyph| glyph == ch)
            .map_or(b'?', |idx| 0x80 + idx as u8),
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
    fn every_cp437_glyph_round_trips() {
        for byte in 0x80u8..=0xFE {
            let ch = cp437_to_string(&[byte]).chars().next().expect("one char");
            assert_eq!(unicode_char_to_cp437(ch), byte, "U+{:04X}", ch as u32);
        }
    }

    #[test]
    fn punctuation_without_a_glyph_maps_to_ascii() {
        assert_eq!(
            str_to_cp437("full — commission – “ok” ‘x’ … 3×4"),
            b"full - commission - \"ok\" 'x' . 3x4"
        );
        assert_eq!(str_to_cp437("←→↑↓"), b"<>^v");
    }

    #[test]
    fn non_breaking_space_never_emits_telnet_iac() {
        assert_eq!(unicode_char_to_cp437('\u{a0}'), b' ');
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
