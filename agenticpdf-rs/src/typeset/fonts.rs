// SPDX-License-Identifier: AGPL-3.0-or-later
//! Font metrics for the typesetter.
//!
//! A reflowable document says "this text is 11pt Calibri"; turning that into
//! line breaks needs to know how wide the text actually is. A PDF carries its
//! own width tables, so [`crate::engine`] reads them from the file — but a
//! `.docx` carries none, and this crate embeds no font files. So widths come
//! from the standard-14 metrics, with any requested font mapped onto the
//! nearest of them.
//!
//! That mapping is an approximation, and deliberately the *same* approximation
//! the renderer makes. `render/webgl-renderer.ts` picks a browser font by
//! pattern-matching the PostScript font name — `courier|mono` → monospace,
//! `times|roman` → serif, `helvetica|arial|sans` → sans-serif — so emitting
//! standard-14 names makes the renderer substitute the same family these widths
//! describe. Layout and rendering therefore agree, which is what keeps text
//! inside the margins it was broken to.
//!
//! Coverage is the ASCII range, where the differences between families are real
//! and worth modelling. Beyond it, widths fall back to per-class estimates:
//! exact metrics for every Latin-1 accent would add bulk for an effect smaller
//! than the substitution error already present.

use crate::doc::TextStyle;

/// Which of the three standard-14 families a font maps onto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Serif,
    SansSerif,
    Monospace,
}

impl Family {
    /// Classify a font name the way the renderer's substitution does.
    ///
    /// The patterns mirror `fontFamily()` in `render/webgl-renderer.ts`; if the
    /// two disagree, text is measured against one font and painted in another.
    pub fn from_name(name: &str) -> Family {
        // Subset prefixes such as `ABCDEF+` are not part of the family name.
        let lower = name
            .split_once('+')
            .map_or(
                name,
                |(prefix, rest)| {
                    if prefix.len() <= 6 { rest } else { name }
                },
            )
            .to_ascii_lowercase();

        if ["courier", "mono", "consol", "menlo"]
            .iter()
            .any(|needle| lower.contains(needle))
        {
            return Family::Monospace;
        }
        if [
            "times", "roman", "georgia", "garamond", "cambria", "book", "serif",
        ]
        .iter()
        .any(|needle| lower.contains(needle))
            && !lower.contains("sans")
        {
            return Family::Serif;
        }
        if [
            "helvetica",
            "arial",
            "calibri",
            "verdana",
            "tahoma",
            "segoe",
            "sans",
        ]
        .iter()
        .any(|needle| lower.contains(needle))
        {
            return Family::SansSerif;
        }
        // Word and its kin default to a sans-serif UI font; an unknown name is
        // far more likely to be one of those than a serif.
        Family::SansSerif
    }
}

/// A resolved font: a standard-14 family plus weight and slope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Font {
    pub family: Family,
    pub bold: bool,
    pub italic: bool,
}

impl Font {
    /// Resolve the font a styled run should be measured and drawn with.
    pub fn resolve(style: &TextStyle) -> Font {
        let family = match &style.font {
            // Inline code is monospaced regardless of the run's own font.
            _ if style.code => Family::Monospace,
            Some(name) => Family::from_name(name),
            None => Family::SansSerif,
        };
        Font {
            family,
            bold: style.bold,
            italic: style.italic,
        }
    }

    /// The PostScript name to put in the display list.
    ///
    /// The renderer derives both family and style from this string, so the
    /// standard-14 spelling — with its `-Bold`, `-Oblique`, `-BoldItalic`
    /// suffixes — is what makes it substitute correctly.
    pub fn postscript_name(&self) -> &'static str {
        match (self.family, self.bold, self.italic) {
            (Family::Serif, false, false) => "Times-Roman",
            (Family::Serif, true, false) => "Times-Bold",
            (Family::Serif, false, true) => "Times-Italic",
            (Family::Serif, true, true) => "Times-BoldItalic",
            (Family::SansSerif, false, false) => "Helvetica",
            (Family::SansSerif, true, false) => "Helvetica-Bold",
            (Family::SansSerif, false, true) => "Helvetica-Oblique",
            (Family::SansSerif, true, true) => "Helvetica-BoldOblique",
            (Family::Monospace, false, false) => "Courier",
            (Family::Monospace, true, false) => "Courier-Bold",
            (Family::Monospace, false, true) => "Courier-Oblique",
            (Family::Monospace, true, true) => "Courier-BoldOblique",
        }
    }

    /// Width of one character in em units (1.0 = the font size).
    pub fn char_width(&self, ch: char) -> f64 {
        // Courier is monospaced: every glyph is 600/1000 em, including the
        // ones the tables below would otherwise disagree about.
        if self.family == Family::Monospace {
            return if is_wide(ch) { 1.2 } else { 0.6 };
        }

        let table = match (self.family, self.bold) {
            (Family::Serif, false) => &TIMES_ROMAN,
            (Family::Serif, true) => &TIMES_BOLD,
            (_, false) => &HELVETICA,
            (_, true) => &HELVETICA_BOLD,
        };

        let code = ch as u32;
        if (0x20..0x7F).contains(&code) {
            return table[(code - 0x20) as usize] as f64 / 1000.0;
        }
        fallback_width(ch, self.family)
    }

    /// Height of a line box at `size`, including leading.
    pub fn line_height(&self, size: f64) -> f64 {
        size * LINE_SPACING
    }

    /// Distance from the top of a line box down to the baseline.
    pub fn ascent(&self, size: f64) -> f64 {
        size * ASCENT_RATIO
    }
}

/// Line height as a multiple of the font size — the conventional single
/// spacing that Word and browsers both approximate.
pub const LINE_SPACING: f64 = 1.2;

/// Baseline position within the line box, as a fraction of the font size.
const ASCENT_RATIO: f64 = 0.95;

/// Width for characters outside the tabulated ASCII range.
fn fallback_width(ch: char, family: Family) -> f64 {
    if is_wide(ch) {
        // CJK ideographs, kana and full-width forms occupy a full em.
        return 1.0;
    }
    if ch.is_control() {
        return 0.0;
    }
    // Combining marks advance nothing; they stack on the previous glyph.
    if matches!(ch as u32, 0x0300..=0x036F | 0x20D0..=0x20FF) {
        return 0.0;
    }
    match family {
        Family::Serif => 0.5,
        _ => 0.556,
    }
}

/// Whether a character occupies a full em (CJK and full-width forms).
pub fn is_wide(ch: char) -> bool {
    matches!(ch as u32,
        0x1100..=0x115F      // Hangul Jamo
        | 0x2E80..=0x303E    // CJK radicals, Kangxi, CJK symbols
        | 0x3041..=0x33FF    // Hiragana, Katakana, Bopomofo, compatibility
        | 0x3400..=0x4DBF    // CJK Extension A
        | 0x4E00..=0x9FFF    // CJK Unified Ideographs
        | 0xA000..=0xA4CF    // Yi
        | 0xAC00..=0xD7A3    // Hangul syllables
        | 0xF900..=0xFAFF    // CJK compatibility ideographs
        | 0xFE30..=0xFE6F    // CJK compatibility forms
        | 0xFF00..=0xFF60    // Full-width forms
        | 0xFFE0..=0xFFE6
        | 0x20000..=0x2FA1F  // CJK Extensions B-F
    )
}

/// Measure a string, returning its total width in points and the per-character
/// advances the display list needs.
///
/// The advances are what `RenderOp::Text` carries, and they are what the
/// renderer positions each glyph by. Returning them from the same function that
/// computes the line-breaking width is deliberate: if the two ever disagreed,
/// text would be broken to one width and painted at another.
pub fn measure(text: &str, font: Font, size: f64) -> (f64, Vec<f64>) {
    let mut advances = Vec::with_capacity(text.len());
    let mut total = 0.0;
    for ch in text.chars() {
        let advance = font.char_width(ch) * size;
        advances.push(advance);
        total += advance;
    }
    (total, advances)
}

/// Width of a string in points, without the per-character detail.
pub fn width_of(text: &str, font: Font, size: f64) -> f64 {
    text.chars().map(|ch| font.char_width(ch)).sum::<f64>() * size
}

// ============================================================================
// Standard-14 width tables
// ============================================================================
//
// Adobe AFM `WX` values for characters 0x20-0x7E, in 1/1000 em. The italic
// variants are measured with the roman widths: the difference is small next to
// the error already introduced by substituting a browser font for the one the
// document asked for.

/// Helvetica, characters 0x20-0x7E.
static HELVETICA: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278,
    278, // ' '..'/'
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, // '0'..'9'
    278, 278, 584, 584, 584, 556, 1015, // ':'..'@'
    667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, 722, 778, // 'A'..'O'
    667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, // 'P'..'Z'
    278, 278, 278, 469, 556, 333, // '['..'`'
    556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, 556, 556, // 'a'..'o'
    556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, // 'p'..'z'
    334, 260, 334, 584, // '{'..'~'
];

/// Helvetica-Bold, characters 0x20-0x7E.
static HELVETICA_BOLD: [u16; 95] = [
    278, 333, 474, 556, 556, 889, 722, 238, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556,
    556, 556, 556, 556, 556, 556, 556, 333, 333, 584, 584, 584, 611, 975, 722, 722, 722, 722, 667,
    611, 778, 722, 278, 556, 722, 611, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667,
    667, 611, 333, 278, 333, 584, 556, 333, 556, 611, 556, 611, 556, 333, 611, 611, 278, 278, 556,
    278, 889, 611, 611, 611, 611, 389, 556, 333, 611, 556, 778, 556, 556, 500, 389, 280, 389, 584,
];

/// Times-Roman, characters 0x20-0x7E.
static TIMES_ROMAN: [u16; 95] = [
    250, 333, 408, 500, 500, 833, 778, 180, 333, 333, 500, 564, 250, 333, 250, 278, 500, 500, 500,
    500, 500, 500, 500, 500, 500, 500, 278, 278, 564, 564, 564, 444, 921, 722, 667, 667, 722, 611,
    556, 722, 722, 333, 389, 722, 611, 889, 722, 722, 556, 722, 667, 556, 611, 722, 722, 944, 722,
    722, 611, 333, 278, 333, 469, 500, 333, 444, 500, 444, 500, 444, 333, 500, 500, 278, 278, 500,
    278, 778, 500, 500, 500, 500, 333, 389, 278, 500, 500, 722, 500, 500, 444, 480, 200, 480, 541,
];

/// Times-Bold, characters 0x20-0x7E.
static TIMES_BOLD: [u16; 95] = [
    250, 333, 555, 500, 500, 1000, 833, 278, 333, 333, 500, 570, 250, 333, 250, 278, 500, 500, 500,
    500, 500, 500, 500, 500, 500, 500, 333, 333, 570, 570, 570, 500, 930, 722, 667, 722, 722, 667,
    611, 778, 778, 389, 500, 778, 667, 944, 722, 778, 611, 778, 722, 556, 667, 722, 722, 1000, 722,
    722, 667, 333, 278, 333, 581, 500, 333, 500, 556, 444, 556, 444, 333, 500, 556, 278, 333, 556,
    278, 833, 556, 500, 556, 556, 444, 389, 333, 556, 500, 722, 500, 500, 444, 394, 220, 394, 520,
];

#[cfg(test)]
mod tests {
    use super::*;

    fn font(family: Family, bold: bool) -> Font {
        Font {
            family,
            bold,
            italic: false,
        }
    }

    #[test]
    fn width_tables_cover_the_whole_ascii_range() {
        for table in [&HELVETICA, &HELVETICA_BOLD, &TIMES_ROMAN, &TIMES_BOLD] {
            assert_eq!(table.len(), 95, "0x20..=0x7E is 95 characters");
            assert!(table.iter().all(|&w| w > 0), "no zero-width ASCII glyph");
        }
    }

    #[test]
    fn known_afm_widths_are_correct() {
        // Spot-check values from the Adobe metrics; a transposed table would
        // shift every character and these would all be wrong.
        let helvetica = font(Family::SansSerif, false);
        assert_eq!(helvetica.char_width(' '), 0.278);
        assert_eq!(helvetica.char_width('A'), 0.667);
        assert_eq!(helvetica.char_width('W'), 0.944);
        assert_eq!(helvetica.char_width('i'), 0.222);
        assert_eq!(helvetica.char_width('~'), 0.584);

        let times = font(Family::Serif, false);
        assert_eq!(times.char_width(' '), 0.25);
        assert_eq!(times.char_width('A'), 0.722);
        assert_eq!(times.char_width('m'), 0.778);
    }

    #[test]
    fn bold_is_never_narrower_than_roman_for_letters() {
        for ch in ('A'..='Z').chain('a'..='z') {
            assert!(
                font(Family::SansSerif, true).char_width(ch)
                    >= font(Family::SansSerif, false).char_width(ch),
                "bold '{ch}' is narrower than roman"
            );
            assert!(
                font(Family::Serif, true).char_width(ch)
                    >= font(Family::Serif, false).char_width(ch),
                "bold serif '{ch}' is narrower than roman"
            );
        }
    }

    #[test]
    fn monospace_is_uniform() {
        let courier = font(Family::Monospace, false);
        let width = courier.char_width('i');
        for ch in " Wm0.@".chars() {
            assert_eq!(courier.char_width(ch), width, "'{ch}' is not monospaced");
        }
    }

    #[test]
    fn classifies_font_names_like_the_renderer_does() {
        assert_eq!(Family::from_name("Courier New"), Family::Monospace);
        assert_eq!(Family::from_name("Consolas"), Family::Monospace);
        assert_eq!(Family::from_name("Times New Roman"), Family::Serif);
        assert_eq!(Family::from_name("Cambria"), Family::Serif);
        assert_eq!(Family::from_name("Georgia"), Family::Serif);
        assert_eq!(Family::from_name("Helvetica"), Family::SansSerif);
        assert_eq!(Family::from_name("Arial"), Family::SansSerif);
        assert_eq!(Family::from_name("Calibri"), Family::SansSerif);
        // "sans" wins over "serif" in "sans-serif".
        assert_eq!(Family::from_name("Open Sans"), Family::SansSerif);
        assert_eq!(Family::from_name("PT Sans Serif"), Family::SansSerif);
    }

    #[test]
    fn strips_subset_prefixes_from_font_names() {
        assert_eq!(Family::from_name("ABCDEF+CourierNew"), Family::Monospace);
        // A long prefix is part of the name, not a subset tag.
        assert_eq!(Family::from_name("NotASubsetTag+Arial"), Family::SansSerif);
    }

    #[test]
    fn resolves_a_style_to_a_postscript_name() {
        let plain = TextStyle::default();
        assert_eq!(Font::resolve(&plain).postscript_name(), "Helvetica");

        let bold_serif = TextStyle {
            bold: true,
            font: Some("Times New Roman".into()),
            ..TextStyle::default()
        };
        assert_eq!(Font::resolve(&bold_serif).postscript_name(), "Times-Bold");

        let italic = TextStyle {
            italic: true,
            ..TextStyle::default()
        };
        assert_eq!(
            Font::resolve(&italic).postscript_name(),
            "Helvetica-Oblique"
        );

        // Inline code is monospaced whatever font the run names.
        let code = TextStyle {
            code: true,
            font: Some("Georgia".into()),
            ..TextStyle::default()
        };
        assert_eq!(Font::resolve(&code).postscript_name(), "Courier");
    }

    #[test]
    fn postscript_names_match_the_renderers_substitution_patterns() {
        // These are the regexes in render/webgl-renderer.ts; if a name stops
        // matching, the renderer silently paints a different family than the
        // one the text was measured in.
        for (name, is_mono, is_serif, is_bold, is_italic) in [
            ("Helvetica", false, false, false, false),
            ("Helvetica-Bold", false, false, true, false),
            ("Helvetica-BoldOblique", false, false, true, true),
            ("Times-Roman", false, true, false, false),
            ("Times-BoldItalic", false, true, true, true),
            ("Courier", true, false, false, false),
            ("Courier-Oblique", true, false, false, true),
        ] {
            let lower = name.to_ascii_lowercase();
            assert_eq!(
                lower.contains("courier") || lower.contains("mono"),
                is_mono,
                "{name}: monospace pattern"
            );
            assert_eq!(
                lower.contains("times") || lower.contains("roman"),
                is_serif,
                "{name}: serif pattern"
            );
            assert_eq!(lower.contains("bold"), is_bold, "{name}: bold pattern");
            assert_eq!(
                lower.contains("ital") || lower.contains("obli"),
                is_italic,
                "{name}: italic pattern"
            );
        }
    }

    #[test]
    fn measure_returns_advances_that_sum_to_the_total() {
        let font = font(Family::SansSerif, false);
        let (total, advances) = measure("Hello, world", font, 12.0);
        assert_eq!(advances.len(), 12);
        let summed: f64 = advances.iter().sum();
        assert!((summed - total).abs() < 1e-9);
        assert!((width_of("Hello, world", font, 12.0) - total).abs() < 1e-9);
    }

    #[test]
    fn width_scales_with_the_font_size() {
        let font = font(Family::Serif, false);
        let small = width_of("measure me", font, 10.0);
        let large = width_of("measure me", font, 20.0);
        assert!((large - small * 2.0).abs() < 1e-9);
    }

    #[test]
    fn cjk_characters_occupy_a_full_em() {
        let font = font(Family::SansSerif, false);
        assert_eq!(font.char_width('\u{4E2D}'), 1.0);
        assert_eq!(font.char_width('\u{3042}'), 1.0);
        assert_eq!(font.char_width('\u{AC00}'), 1.0);
        assert!(is_wide('\u{FF21}'), "full-width forms are wide");
        assert!(!is_wide('A'));
    }

    #[test]
    fn combining_marks_and_controls_advance_nothing() {
        let font = font(Family::SansSerif, false);
        assert_eq!(font.char_width('\u{0301}'), 0.0, "combining acute");
        assert_eq!(font.char_width('\u{0007}'), 0.0, "control character");
        // A precomposed accented letter still advances.
        assert!(font.char_width('é') > 0.0);
    }

    #[test]
    fn line_metrics_scale_with_size() {
        let font = font(Family::SansSerif, false);
        assert_eq!(font.line_height(10.0), 12.0);
        assert!(font.ascent(10.0) < font.line_height(10.0));
    }
}
