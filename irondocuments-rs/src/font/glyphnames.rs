// SPDX-License-Identifier: AGPL-3.0-or-later
//! Adobe glyph names, in both directions, from one table.
//!
//! A Type 1 or CFF font addresses glyphs by name, and the name is not the
//! character: U+2013 is `endash`, U+00E9 is `eacute`. WinAnsi puts exactly
//! those in its high range, so a document with no `/Differences` reaches them
//! only through their names. Without this a page loses its dashes, its curly
//! quotes and every accented letter while the ASCII around them draws fine.
//!
//! Both directions are needed -- a name from `/Differences` has to become a
//! character, and a decoded character has to become a name -- and they are
//! derived from a single list here rather than written twice. Two hand-written
//! tables drift: the first pair of them disagreed about whether `quoteright`
//! was the ASCII apostrophe or the curly one, which picks the wrong glyph.
//!
//! Latin-1 and the typographic punctuation, which is what documents in these
//! encodings contain. The full Adobe glyph list is far larger and almost
//! entirely unused here.

/// Name to character. The first entry for a character is its preferred name.
static PAIRS: &[(&str, char)] = &[
    // Typographic punctuation: the WinAnsi range 0x80-0x9F, where reading a
    // code as a character gives a control character instead.
    ("endash", '\u{2013}'),
    ("emdash", '\u{2014}'),
    ("quoteleft", '\u{2018}'),
    ("quoteright", '\u{2019}'),
    ("quotesinglbase", '\u{201A}'),
    ("quotedblleft", '\u{201C}'),
    ("quotedblright", '\u{201D}'),
    ("quotedblbase", '\u{201E}'),
    ("dagger", '\u{2020}'),
    ("daggerdbl", '\u{2021}'),
    ("bullet", '\u{2022}'),
    ("ellipsis", '\u{2026}'),
    ("perthousand", '\u{2030}'),
    ("guilsinglleft", '\u{2039}'),
    ("guilsinglright", '\u{203A}'),
    ("trademark", '\u{2122}'),
    ("minus", '\u{2212}'),
    ("ff", '\u{FB00}'),
    ("fi", '\u{FB01}'),
    ("fl", '\u{FB02}'),
    ("ffi", '\u{FB03}'),
    ("ffl", '\u{FB04}'),
    ("OE", '\u{0152}'),
    ("oe", '\u{0153}'),
    ("Scaron", '\u{0160}'),
    ("scaron", '\u{0161}'),
    ("Ydieresis", '\u{0178}'),
    ("Zcaron", '\u{017D}'),
    ("zcaron", '\u{017E}'),
    ("florin", '\u{0192}'),
    ("circumflex", '\u{02C6}'),
    ("tilde", '\u{02DC}'),
    // Latin-1 symbols.
    ("exclamdown", '\u{00A1}'),
    ("cent", '\u{00A2}'),
    ("sterling", '\u{00A3}'),
    ("currency", '\u{00A4}'),
    ("yen", '\u{00A5}'),
    ("brokenbar", '\u{00A6}'),
    ("section", '\u{00A7}'),
    ("dieresis", '\u{00A8}'),
    ("copyright", '\u{00A9}'),
    ("ordfeminine", '\u{00AA}'),
    ("guillemotleft", '\u{00AB}'),
    ("logicalnot", '\u{00AC}'),
    ("registered", '\u{00AE}'),
    ("macron", '\u{00AF}'),
    ("degree", '\u{00B0}'),
    ("plusminus", '\u{00B1}'),
    ("twosuperior", '\u{00B2}'),
    ("threesuperior", '\u{00B3}'),
    ("acute", '\u{00B4}'),
    ("mu", '\u{00B5}'),
    ("paragraph", '\u{00B6}'),
    ("periodcentered", '\u{00B7}'),
    ("cedilla", '\u{00B8}'),
    ("onesuperior", '\u{00B9}'),
    ("ordmasculine", '\u{00BA}'),
    ("guillemotright", '\u{00BB}'),
    ("onequarter", '\u{00BC}'),
    ("onehalf", '\u{00BD}'),
    ("threequarters", '\u{00BE}'),
    ("questiondown", '\u{00BF}'),
    ("multiply", '\u{00D7}'),
    ("divide", '\u{00F7}'),
    // Latin-1 letters.
    ("Agrave", '\u{00C0}'),
    ("Aacute", '\u{00C1}'),
    ("Acircumflex", '\u{00C2}'),
    ("Atilde", '\u{00C3}'),
    ("Adieresis", '\u{00C4}'),
    ("Aring", '\u{00C5}'),
    ("AE", '\u{00C6}'),
    ("Ccedilla", '\u{00C7}'),
    ("Egrave", '\u{00C8}'),
    ("Eacute", '\u{00C9}'),
    ("Ecircumflex", '\u{00CA}'),
    ("Edieresis", '\u{00CB}'),
    ("Igrave", '\u{00CC}'),
    ("Iacute", '\u{00CD}'),
    ("Icircumflex", '\u{00CE}'),
    ("Idieresis", '\u{00CF}'),
    ("Eth", '\u{00D0}'),
    ("Ntilde", '\u{00D1}'),
    ("Ograve", '\u{00D2}'),
    ("Oacute", '\u{00D3}'),
    ("Ocircumflex", '\u{00D4}'),
    ("Otilde", '\u{00D5}'),
    ("Odieresis", '\u{00D6}'),
    ("Oslash", '\u{00D8}'),
    ("Ugrave", '\u{00D9}'),
    ("Uacute", '\u{00DA}'),
    ("Ucircumflex", '\u{00DB}'),
    ("Udieresis", '\u{00DC}'),
    ("Yacute", '\u{00DD}'),
    ("Thorn", '\u{00DE}'),
    ("germandbls", '\u{00DF}'),
    ("agrave", '\u{00E0}'),
    ("aacute", '\u{00E1}'),
    ("acircumflex", '\u{00E2}'),
    ("atilde", '\u{00E3}'),
    ("adieresis", '\u{00E4}'),
    ("aring", '\u{00E5}'),
    ("ae", '\u{00E6}'),
    ("ccedilla", '\u{00E7}'),
    ("egrave", '\u{00E8}'),
    ("eacute", '\u{00E9}'),
    ("ecircumflex", '\u{00EA}'),
    ("edieresis", '\u{00EB}'),
    ("igrave", '\u{00EC}'),
    ("iacute", '\u{00ED}'),
    ("icircumflex", '\u{00EE}'),
    ("idieresis", '\u{00EF}'),
    ("eth", '\u{00F0}'),
    ("ntilde", '\u{00F1}'),
    ("ograve", '\u{00F2}'),
    ("oacute", '\u{00F3}'),
    ("ocircumflex", '\u{00F4}'),
    ("otilde", '\u{00F5}'),
    ("odieresis", '\u{00F6}'),
    ("oslash", '\u{00F8}'),
    ("ugrave", '\u{00F9}'),
    ("uacute", '\u{00FA}'),
    ("ucircumflex", '\u{00FB}'),
    ("udieresis", '\u{00FC}'),
    ("yacute", '\u{00FD}'),
    ("thorn", '\u{00FE}'),
    ("ydieresis", '\u{00FF}'),
    // ASCII punctuation and digits. Letters name themselves and are handled
    // separately, so they do not bloat this list.
    ("space", ' '),
    ("exclam", '!'),
    ("quotedbl", '"'),
    ("numbersign", '#'),
    ("dollar", '$'),
    ("percent", '%'),
    ("ampersand", '&'),
    ("quotesingle", '\''),
    ("parenleft", '('),
    ("parenright", ')'),
    ("asterisk", '*'),
    ("plus", '+'),
    ("comma", ','),
    ("hyphen", '-'),
    ("period", '.'),
    ("slash", '/'),
    ("zero", '0'),
    ("one", '1'),
    ("two", '2'),
    ("three", '3'),
    ("four", '4'),
    ("five", '5'),
    ("six", '6'),
    ("seven", '7'),
    ("eight", '8'),
    ("nine", '9'),
    ("colon", ':'),
    ("semicolon", ';'),
    ("less", '<'),
    ("equal", '='),
    ("greater", '>'),
    ("question", '?'),
    ("at", '@'),
    ("bracketleft", '['),
    ("backslash", '\\'),
    ("bracketright", ']'),
    ("asciicircum", '^'),
    ("underscore", '_'),
    ("grave", '`'),
    ("braceleft", '{'),
    ("bar", '|'),
    ("braceright", '}'),
    ("asciitilde", '~'),
];

/// The character a glyph name stands for.
///
/// Also accepts the two spellings that carry their own answer: a name that is
/// a single character, and the `uniXXXX` form.
pub fn char_for(name: &str) -> Option<char> {
    let mut chars = name.chars();
    if let (Some(only), None) = (chars.next(), chars.next()) {
        return Some(only);
    }
    if let Some(hex) = name.strip_prefix("uni")
        && hex.len() == 4
        && let Ok(value) = u32::from_str_radix(hex, 16)
    {
        return char::from_u32(value);
    }
    PAIRS
        .iter()
        .find(|(candidate, _)| *candidate == name)
        .map(|(_, ch)| *ch)
}

/// The names a character is known by, preferred first.
///
/// A letter names itself, so it is answered without consulting the table.
pub fn names_for(ch: char) -> Vec<&'static str> {
    if ch.is_ascii_alphabetic() {
        // Safe to leak nothing: the name is a one-character slice of a static
        // table of every ASCII letter.
        return vec![ascii_letter(ch)];
    }
    PAIRS
        .iter()
        .filter(|(_, candidate)| *candidate == ch)
        .map(|(name, _)| *name)
        .collect()
}

/// The one-character name of an ASCII letter, as a static string.
fn ascii_letter(ch: char) -> &'static str {
    const LETTERS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let index = match ch {
        'A'..='Z' => (ch as u8 - b'A') as usize,
        'a'..='z' => 26 + (ch as u8 - b'a') as usize,
        _ => return "",
    };
    &LETTERS[index..index + 1]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The characters this table exists for: the ones WinAnsi hides in a range
    /// where reading the code as a character gives something else entirely.
    #[test]
    fn the_winansi_high_range_is_named() {
        assert_eq!(names_for('\u{2013}'), ["endash"]);
        assert_eq!(names_for('\u{2014}'), ["emdash"]);
        assert_eq!(names_for('\u{2019}'), ["quoteright"]);
        assert_eq!(names_for('\u{201C}'), ["quotedblleft"]);
        assert_eq!(names_for('\u{2022}'), ["bullet"]);
        assert_eq!(names_for('\u{2122}'), ["trademark"]);
    }

    #[test]
    fn accented_letters_are_named() {
        assert_eq!(names_for('\u{00E9}'), ["eacute"]);
        assert_eq!(names_for('\u{00FC}'), ["udieresis"]);
        assert_eq!(names_for('\u{00DF}'), ["germandbls"]);
    }

    #[test]
    fn letters_name_themselves() {
        assert_eq!(names_for('A'), ["A"]);
        assert_eq!(names_for('z'), ["z"]);
        assert_eq!(char_for("A"), Some('A'));
    }

    #[test]
    fn punctuation_is_named_not_spelled() {
        assert_eq!(names_for('.'), ["period"]);
        assert_eq!(names_for('1'), ["one"]);
    }

    /// Adobe distinguishes the upright apostrophe from the curly one. Folding
    /// them together picks the wrong glyph, which is what the first version of
    /// this table did.
    #[test]
    fn the_apostrophe_and_the_curly_quote_are_different() {
        assert_eq!(char_for("quotesingle"), Some('\''));
        assert_eq!(char_for("quoteright"), Some('\u{2019}'));
        assert_ne!(char_for("quotesingle"), char_for("quoteright"));
    }

    #[test]
    fn an_unknown_character_names_nothing() {
        assert!(names_for('\u{4E00}').is_empty());
        assert_eq!(char_for("notaglyphname"), None);
    }

    /// One table, both directions: every entry must round-trip. Two
    /// hand-written tables drift, and the pair this replaced had already
    /// disagreed about the apostrophe.
    #[test]
    fn every_name_round_trips() {
        for (name, ch) in PAIRS {
            assert_eq!(char_for(name), Some(*ch), "{name} should give {ch:?}");
            assert!(
                names_for(*ch).contains(name),
                "{ch:?} should be known as {name}"
            );
        }
    }

    /// A character must not be listed under two names without the preferred
    /// one coming first, or a lookup gets the wrong glyph where a font has
    /// both.
    #[test]
    fn no_character_has_a_duplicate_name() {
        let mut names: Vec<&str> = PAIRS.iter().map(|(name, _)| *name).collect();
        names.sort_unstable();
        let count = names.len();
        names.dedup();
        assert_eq!(names.len(), count, "a name is listed twice");
    }
}
