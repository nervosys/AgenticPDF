// SPDX-License-Identifier: AGPL-3.0-or-later
//! Stand-ins for fonts a document names but does not embed.
//!
//! The standard fourteen -- Times, Helvetica, Courier, Symbol, ZapfDingbats --
//! may be referenced without being embedded, because every reader is expected
//! to have them. There is nothing in the file to decode, so a renderer either
//! ships its own copies, as PDF.js does, or substitutes what the system has,
//! as Poppler and therefore Okular do.
//!
//! This substitutes. Shipping a megabyte of typefaces is a licensing and size
//! decision rather than a technical one, and matching the local Times is what
//! the reader beside this one on the same desktop will be doing.
//!
//! The document still supplies the advances, so a substituted run occupies
//! exactly the space the original reserved. Only the letterforms differ, and
//! for these fourteen the metric-compatible system faces are close.

use std::collections::HashMap;

use super::{EmbeddedFont, FontProgram, TrueTypeFont};

/// A face to try, in order of preference, for one style.
struct Candidate {
    /// Matched against the lowercased base font name.
    keywords: &'static [&'static str],
    /// Files to try, most preferred first.
    files: &'static [&'static str],
}

/// The mapping from a standard name to whatever the system actually has.
///
/// Ordered from most specific to least: `Times-BoldItalic` must match the
/// bold-italic entry before the plain `times` one, so the bold and italic
/// keywords are checked first.
const CANDIDATES: &[Candidate] = &[
    Candidate {
        keywords: &["courier", "mono"],
        files: &[
            "C:/Windows/Fonts/cour.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
            "/System/Library/Fonts/Menlo.ttc",
        ],
    },
    Candidate {
        keywords: &["symbol"],
        files: &["C:/Windows/Fonts/symbol.ttf"],
    },
    Candidate {
        keywords: &["zapf", "dingbat"],
        files: &["C:/Windows/Fonts/wingding.ttf"],
    },
    Candidate {
        keywords: &["helvetica", "arial", "sans"],
        files: &[
            "C:/Windows/Fonts/arial.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/System/Library/Fonts/Helvetica.ttc",
        ],
    },
    // Times last among the text faces: it is also the sensible default for a
    // name that matches nothing else, and a serif face is the safer guess for
    // an unknown body font.
    Candidate {
        keywords: &["times", "roman", "serif", "georgia", "garamond", "book"],
        files: &[
            "C:/Windows/Fonts/times.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSerif-Regular.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf",
            "/System/Library/Fonts/Times.ttc",
        ],
    },
];

/// Bold and italic variants, tried before the regular file when the name says
/// so. A substituted run in the wrong weight is more obviously wrong than one
/// in the wrong typeface.
const STYLED: &[(&str, &str, &[&str])] = &[
    (
        "times",
        "bolditalic",
        &[
            "C:/Windows/Fonts/timesbi.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSerif-BoldItalic.ttf",
        ],
    ),
    (
        "times",
        "bold",
        &[
            "C:/Windows/Fonts/timesbd.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSerif-Bold.ttf",
        ],
    ),
    (
        "times",
        "italic",
        &[
            "C:/Windows/Fonts/timesi.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSerif-Italic.ttf",
        ],
    ),
    (
        "arial",
        "bolditalic",
        &[
            "C:/Windows/Fonts/arialbi.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-BoldItalic.ttf",
        ],
    ),
    (
        "arial",
        "bold",
        &[
            "C:/Windows/Fonts/arialbd.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
        ],
    ),
    (
        "arial",
        "italic",
        &[
            "C:/Windows/Fonts/ariali.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Italic.ttf",
        ],
    ),
    (
        "courier",
        "bold",
        &[
            "C:/Windows/Fonts/courbd.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationMono-Bold.ttf",
        ],
    ),
];

/// Which family a base font name belongs to, and whether it is bold or italic.
fn classify(base_font: &str) -> (&'static Candidate, bool, bool) {
    let name = base_font.to_ascii_lowercase();
    let bold = name.contains("bold") || name.contains("black") || name.contains("heavy");
    let italic = name.contains("italic") || name.contains("oblique");

    for candidate in CANDIDATES {
        if candidate.keywords.iter().any(|word| name.contains(word)) {
            return (candidate, bold, italic);
        }
    }
    // Nothing matched: a serif face is the safer guess for an unknown body
    // font, and it is the last entry.
    (&CANDIDATES[CANDIDATES.len() - 1], bold, italic)
}

/// The family keyword the styled table is keyed by.
fn family_key(candidate: &Candidate) -> &'static str {
    match candidate.keywords.first() {
        Some(&"courier") => "courier",
        Some(&"helvetica") => "arial",
        _ => "times",
    }
}

/// Load a stand-in for a font the document names but does not embed.
///
/// Returns `None` where no face can be found -- a stripped container, or a
/// browser, which has no filesystem to read. The caller then falls back to
/// laying the run out with whatever font it has.
pub fn load(base_font: &str) -> Option<EmbeddedFont> {
    // A browser has no filesystem to read a face from, and saying so here
    // keeps the file reads out of the wasm bundle entirely.
    if cfg!(target_arch = "wasm32") {
        return None;
    }
    let (candidate, bold, italic) = classify(base_font);
    let family = family_key(candidate);
    let style = match (bold, italic) {
        (true, true) => "bolditalic",
        (true, false) => "bold",
        (false, true) => "italic",
        (false, false) => "",
    };

    let mut paths: Vec<&str> = Vec::new();
    if !style.is_empty() {
        for (entry_family, entry_style, files) in STYLED {
            if *entry_family == family && *entry_style == style {
                paths.extend_from_slice(files);
            }
        }
    }
    paths.extend_from_slice(candidate.files);

    for path in paths {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        if let Some(program) = TrueTypeFont::parse(&bytes) {
            return Some(EmbeddedFont {
                base_font: base_font.to_string(),
                program: FontProgram::TrueType(program),
                differences: HashMap::new(),
                composite: false,
                cid_to_gid: None,
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_standard_names_are_classified_by_family() {
        assert_eq!(family_key(classify("Times-Roman").0), "times");
        assert_eq!(family_key(classify("Helvetica").0), "arial");
        assert_eq!(family_key(classify("Courier").0), "courier");
        assert_eq!(family_key(classify("Arial-BoldMT").0), "arial");
        // An unknown name falls to a serif face rather than to nothing.
        assert_eq!(family_key(classify("SomeUnknownFace").0), "times");
    }

    #[test]
    fn style_is_read_from_the_name() {
        let (_, bold, italic) = classify("Times-BoldItalic");
        assert!(bold && italic);
        let (_, bold, italic) = classify("Helvetica-Oblique");
        assert!(!bold && italic);
        let (_, bold, italic) = classify("Courier-Bold");
        assert!(bold && !italic);
        let (_, bold, italic) = classify("Times-Roman");
        assert!(!bold && !italic);
    }

    /// The substitute must actually draw, or the fallback is no better than
    /// the one it replaces.
    #[test]
    fn a_standard_name_yields_a_drawable_face() {
        let Some(font) = load("Times-Roman") else {
            eprintln!("skipping: no system face to substitute");
            return;
        };
        let glyph = font.outline(b'A' as u32).expect("A should draw");
        assert!(!glyph.contours.is_empty());
        assert!(glyph.advance > 0.0);

        // Lowercase and punctuation too, since a body font needs both.
        assert!(font.outline(b'e' as u32).is_some());
        assert!(font.outline(b'.' as u32).is_some());
    }

    /// A bold name should reach a different face from the regular one where
    /// the system has both. Where it does not, the test says so rather than
    /// asserting against a face that is not installed.
    #[test]
    fn bold_reaches_a_different_face_where_one_exists() {
        let (Some(regular), Some(bold)) = (load("Times-Roman"), load("Times-Bold")) else {
            eprintln!("skipping: no system faces to substitute");
            return;
        };
        let (Some(plain), Some(heavy)) = (regular.outline(b'I' as u32), bold.outline(b'I' as u32))
        else {
            return;
        };
        let width = |glyph: &super::super::Glyph| {
            let points: Vec<[f32; 2]> = glyph.contours.iter().flatten().copied().collect();
            let min = points.iter().map(|p| p[0]).fold(f32::MAX, f32::min);
            let max = points.iter().map(|p| p[0]).fold(f32::MIN, f32::max);
            max - min
        };
        if !std::path::Path::new("C:/Windows/Fonts/timesbd.ttf").exists() {
            eprintln!("skipping: no bold face installed");
            return;
        }
        assert!(
            width(&heavy) > width(&plain),
            "a bold I should have a thicker stem: {} vs {}",
            width(&heavy),
            width(&plain)
        );
    }
}

#[cfg(test)]
mod coverage_probe {
    #[test]
    fn probe_ligature_coverage() {
        let Some(font) = super::load("Times-Roman") else {
            return;
        };
        for (label, ch) in [
            ("fi", '\u{FB01}'),
            ("fl", '\u{FB02}'),
            ("f", 'f'),
            ("i", 'i'),
        ] {
            let found = font.outline(ch as u32).is_some();
            eprintln!("{label} (U+{:04X}): {}", ch as u32, found);
        }
    }
}
