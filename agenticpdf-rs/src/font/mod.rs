// SPDX-License-Identifier: AGPL-3.0-or-later
//! Embedded font programs, decoded into glyph outlines.
//!
//! A renderer that substitutes its own font can only approximate a document:
//! the advances differ, so runs either collide or have to be squeezed to fit,
//! and the page is visibly not what the author saw. Drawing the glyphs the
//! document actually carries is what PDF.js and Okular do, and it is the only
//! way the output matches.
//!
//! Outlines come back in font units and already flattened to polygons, because
//! every consumer here -- the desktop painter, the browser canvas -- fills
//! polygons rather than curves.

pub mod cff;
pub mod truetype;
pub mod type1;

pub use cff::CffFont;
pub use truetype::TrueTypeFont;
pub use type1::{Glyph, Type1Font};

use crate::engine::{self, Object};
use std::collections::HashMap;

/// A glyph source: whichever program the document embedded.
#[derive(Debug, Clone)]
pub enum FontProgram {
    /// `/FontFile`: a PostScript Type 1 program. What TeX emits.
    Type1(Type1Font),
    /// `/FontFile2`: TrueType outlines. What nearly everything else emits.
    TrueType(TrueTypeFont),
    /// `/FontFile3`: CFF, which is what Type 1 became. A modern producer
    /// subsetting a PostScript typeface emits this.
    Cff(CffFont),
}

impl FontProgram {
    /// The Type 1 program, where that is what this is. Tests that exercise
    /// glyph *names* need it; nothing else should care which kind it is.
    pub fn as_type1(&self) -> Option<&Type1Font> {
        match self {
            FontProgram::Type1(font) => Some(font),
            _ => None,
        }
    }
}

/// An embedded font program, keyed by the `/BaseFont` name it was found under.
#[derive(Debug, Clone)]
pub struct EmbeddedFont {
    /// `/BaseFont`, subset prefix and all (`RDQRPY+CMR10`).
    pub base_font: String,
    /// The glyph source.
    pub program: FontProgram,
    /// `/Differences`: character code to glyph name, which overrides whatever
    /// encoding the font program itself ships with.
    pub differences: HashMap<u8, String>,
}

impl EmbeddedFont {
    /// The glyph name a character code selects, where names apply at all.
    ///
    /// TrueType addresses glyphs by index rather than by name, so a name only
    /// helps there as a route to a Unicode value the `cmap` can answer.
    pub fn glyph_name(&self, code: u8) -> Option<&str> {
        if let Some(name) = self.differences.get(&code) {
            return Some(name.as_str());
        }
        match &self.program {
            FontProgram::Type1(font) => font.glyph_name(code),
            FontProgram::TrueType(_) => None,
            // CFF names glyphs, but its own encoding maps codes directly, so
            // a name is only consulted when `/Differences` supplies one.
            FontProgram::Cff(_) => None,
        }
    }

    /// The scale from the program's own units into text space.
    pub fn font_matrix(&self) -> [f64; 6] {
        match &self.program {
            FontProgram::Type1(font) => font.font_matrix,
            FontProgram::TrueType(font) => font.font_matrix(),
            FontProgram::Cff(font) => font.font_matrix,
        }
    }

    /// The outline a character code selects.
    pub fn outline(&self, code: u8) -> Option<Glyph> {
        match &self.program {
            FontProgram::Type1(font) => font.outline(self.glyph_name(code)?),
            FontProgram::TrueType(font) => {
                // The document's own `/Differences` first: it states what a
                // code means, which beats assuming the code is Latin-1.
                let named = self
                    .differences
                    .get(&code)
                    .and_then(|name| glyph_name_to_char(name));
                let fallback = char::from_u32(code as u32);
                let index = font.glyph_index(code as u32, named.or(fallback))?;
                font.outline(index)
            }
            FontProgram::Cff(font) => {
                // The document's `/Differences` name first, then the font's
                // own encoding. A subset font commonly ships an encoding that
                // is the only route to its glyphs.
                let index = self
                    .differences
                    .get(&code)
                    .and_then(|name| font.index_for_name(name))
                    .or_else(|| font.index_for_code(code))?;
                font.outline(index)
            }
        }
    }
}

/// The Unicode value a glyph name stands for, where that is knowable.
///
/// Covers the two spellings that carry their own answer -- a single character,
/// and the `uniXXXX` form -- plus the punctuation names common enough that a
/// document using them would otherwise lose its punctuation. The full Adobe
/// glyph list would be a large table for little more coverage.
fn glyph_name_to_char(name: &str) -> Option<char> {
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
    Some(match name {
        "space" => ' ',
        "exclam" => '!',
        "quotedbl" => '"',
        "numbersign" => '#',
        "dollar" => '$',
        "percent" => '%',
        "ampersand" => '&',
        "quotesingle" | "quoteleft" | "quoteright" => '\'',
        "parenleft" => '(',
        "parenright" => ')',
        "asterisk" => '*',
        "plus" => '+',
        "comma" => ',',
        "hyphen" => '-',
        "period" => '.',
        "slash" => '/',
        "zero" => '0',
        "one" => '1',
        "two" => '2',
        "three" => '3',
        "four" => '4',
        "five" => '5',
        "six" => '6',
        "seven" => '7',
        "eight" => '8',
        "nine" => '9',
        "colon" => ':',
        "semicolon" => ';',
        "less" => '<',
        "equal" => '=',
        "greater" => '>',
        "question" => '?',
        "at" => '@',
        "bracketleft" => '[',
        "backslash" => '\\',
        "bracketright" => ']',
        "underscore" => '_',
        "braceleft" => '{',
        "bar" => '|',
        "braceright" => '}',
        "quotedblleft" => '\u{201C}',
        "quotedblright" => '\u{201D}',
        "endash" => '\u{2013}',
        "emdash" => '\u{2014}',
        "fi" => '\u{FB01}',
        "fl" => '\u{FB02}',
        _ => return None,
    })
}

/// Every Type 1 program embedded in a PDF, parsed.
///
/// Fonts that will not parse are dropped rather than reported: a renderer that
/// cannot read one font should fall back for that font, not fail the page.
pub fn embedded_fonts(data: &[u8]) -> Vec<EmbeddedFont> {
    let Ok(doc) = engine::Document::parse(data) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    let mut seen: Vec<String> = Vec::new();

    // Walk every object rather than every page's resources: a font can be
    // shared, reached through nested form XObjects, or referenced from a
    // resource dictionary this code has no other reason to visit.
    for number in doc.object_numbers() {
        let Ok(object) = doc.fetch(number) else {
            continue;
        };
        let Object::Dict(dict) = &object else {
            continue;
        };
        if doc
            .get(dict, "Type")
            .and_then(|o| o.as_name().map(String::from))
            != Some("Font".to_string())
        {
            continue;
        }

        let base_font = doc
            .get(dict, "BaseFont")
            .and_then(|o| o.as_name().map(String::from))
            .unwrap_or_default();
        if seen.contains(&base_font) {
            continue;
        }

        let Some(descriptor) = doc.get(dict, "FontDescriptor") else {
            continue;
        };
        let descriptor = doc.resolve(&descriptor);
        let Object::Dict(descriptor) = descriptor else {
            continue;
        };

        // `/FontFile` is Type 1, `/FontFile2` is TrueType. `/FontFile3` is
        // CFF, which this reader cannot draw yet; a caller falls back there.
        let Some(program) = read_program(&doc, &descriptor) else {
            continue;
        };

        seen.push(base_font.clone());
        out.push(EmbeddedFont {
            base_font,
            program,
            differences: differences(&doc, dict),
        });
    }
    out
}

/// Just the Type 1 programs, for tests that exercise that reader directly.
pub fn embedded_type1(data: &[u8]) -> Vec<Type1Font> {
    embedded_fonts(data)
        .into_iter()
        .filter_map(|font| match font.program {
            FontProgram::Type1(program) => Some(program),
            _ => None,
        })
        .collect()
}

/// Read whichever font program a descriptor embeds.
fn read_program(doc: &engine::Document<'_>, descriptor: &engine::Dict) -> Option<FontProgram> {
    for (key, kind) in [("FontFile", 1u8), ("FontFile2", 2), ("FontFile3", 3)] {
        let Some(file) = doc.get(descriptor, key) else {
            continue;
        };
        let Object::Stream(stream_dict, raw) = doc.resolve(&file) else {
            continue;
        };
        let Ok(bytes) = engine::decode_stream(&stream_dict, &raw) else {
            continue;
        };
        match kind {
            1 => {
                let length = |name: &str| {
                    doc.get(&stream_dict, name)
                        .and_then(|o| o.as_int())
                        .unwrap_or(0)
                        .max(0) as usize
                };
                if let Some(font) = Type1Font::parse(&bytes, length("Length1"), length("Length2")) {
                    return Some(FontProgram::Type1(font));
                }
            }
            2 => {
                if let Some(font) = TrueTypeFont::parse(&bytes) {
                    return Some(FontProgram::TrueType(font));
                }
            }
            _ => {
                // `/FontFile3` is bare CFF for `/Type1C`, or a whole OpenType
                // file for `/OpenType`, in which case the CFF sits in a table.
                let cff = match bytes.starts_with(b"OTTO") {
                    true => opentype_cff_table(&bytes).and_then(CffFont::parse),
                    false => CffFont::parse(&bytes),
                };
                if let Some(font) = cff {
                    return Some(FontProgram::Cff(font));
                }
                // An OpenType wrapper may carry TrueType outlines instead.
                if let Some(font) = TrueTypeFont::parse(&bytes) {
                    return Some(FontProgram::TrueType(font));
                }
            }
        }
    }
    None
}

/// The `CFF ` table inside an OpenType file.
fn opentype_cff_table(data: &[u8]) -> Option<&[u8]> {
    let count = u16::from_be_bytes(data.get(4..6)?.try_into().ok()?) as usize;
    for entry in 0..count.min(512) {
        let at = 12 + entry * 16;
        if data.get(at..at + 4)? == b"CFF " {
            let offset = u32::from_be_bytes(data.get(at + 8..at + 12)?.try_into().ok()?) as usize;
            let length = u32::from_be_bytes(data.get(at + 12..at + 16)?.try_into().ok()?) as usize;
            return data.get(offset..offset.saturating_add(length));
        }
    }
    None
}

/// `/Encoding << /Differences [ 32 /space /exclam ... ] >>`.
fn differences(doc: &engine::Document<'_>, font: &engine::Dict) -> HashMap<u8, String> {
    let mut out = HashMap::new();
    let Some(encoding) = doc.get(font, "Encoding") else {
        return out;
    };
    let Object::Dict(encoding) = doc.resolve(&encoding) else {
        return out;
    };
    let Some(list) = doc.get(&encoding, "Differences") else {
        return out;
    };
    let Object::Array(items) = doc.resolve(&list) else {
        return out;
    };

    // The array alternates a starting code with the names that follow it.
    let mut code: i64 = 0;
    for item in items {
        match doc.resolve(&item) {
            Object::Int(value) => code = value,
            Object::Real(value) => code = value as i64,
            Object::Name(name) => {
                if (0..=255).contains(&code) {
                    out.insert(code as u8, name);
                }
                code += 1;
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod survey {
    /// Walk every glyph of every embedded font, to catch a charstring that
    /// loops or produces nothing. Sampling a few glyphs, as the other tests
    /// do, is exactly how such a case gets missed.
    #[test]
    fn every_glyph_of_every_embedded_font_terminates() {
        let Ok(pdf) = std::fs::read("../demos/sample.pdf") else {
            eprintln!("skipping: demos/sample.pdf not present");
            return;
        };
        let fonts = super::embedded_fonts(&pdf);
        let mut empty = Vec::new();
        let mut drawn = 0usize;
        for font in &fonts {
            for name in font.program.as_type1().expect("type 1").glyph_names() {
                match font.program.as_type1().expect("type 1").outline(&name) {
                    Some(glyph) if glyph.contours.is_empty() && name != ".notdef" => {
                        empty.push(format!("{}/{name}", font.base_font))
                    }
                    Some(_) => drawn += 1,
                    None => empty.push(format!("{}/{name} (missing)", font.base_font)),
                }
            }
        }
        eprintln!(
            "fonts={} glyphs drawn={} empty={}",
            fonts.len(),
            drawn,
            empty.len()
        );
        if !empty.is_empty() {
            eprintln!("first empty: {:?}", &empty[..empty.len().min(10)]);
        }
        assert!(
            drawn > 100,
            "expected a page's worth of glyphs, got {drawn}"
        );
        // A handful of empty glyphs is normal (space, .notdef); a pile of them
        // means the interpreter is failing quietly.
        assert!(
            empty.len() * 4 < drawn,
            "too many empty glyphs: {} empty vs {drawn} drawn",
            empty.len()
        );
    }
}

#[cfg(test)]
mod truetype_document {
    use super::*;

    /// Build a minimal PDF that embeds a real TrueType font, and read it back.
    ///
    /// Synthesised rather than committed: a system font is not ours to
    /// redistribute, and the point of the test is the wiring from
    /// `/FontFile2` through to an outline, not the particular typeface.
    fn pdf_embedding(font_bytes: &[u8]) -> Vec<u8> {
        pdf_embedding_with(font_bytes, "FontFile2")
    }

    fn pdf_embedding_with(font_bytes: &[u8], key: &str) -> Vec<u8> {
        let mut pdf: Vec<u8> = Vec::new();
        let mut offsets = vec![0usize];
        pdf.extend_from_slice(b"%PDF-1.4\n");

        let object = |pdf: &mut Vec<u8>, offsets: &mut Vec<usize>, body: &[u8]| {
            offsets.push(pdf.len());
            let number = offsets.len() - 1;
            pdf.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
            pdf.extend_from_slice(body);
            pdf.extend_from_slice(b"\nendobj\n");
        };

        object(&mut pdf, &mut offsets, b"<< /Type /Catalog /Pages 2 0 R >>");
        object(
            &mut pdf,
            &mut offsets,
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        );
        object(
            &mut pdf,
            &mut offsets,
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] \
              /Resources << /Font << /F1 4 0 R >> >> >>",
        );
        object(
            &mut pdf,
            &mut offsets,
            b"<< /Type /Font /Subtype /TrueType /BaseFont /TestTT \
              /FirstChar 65 /LastChar 66 /Widths [600 600] /FontDescriptor 5 0 R >>",
        );
        object(
            &mut pdf,
            &mut offsets,
            format!("<< /Type /FontDescriptor /FontName /TestTT /Flags 32 /{key} 6 0 R >>")
                .as_bytes(),
        );

        offsets.push(pdf.len());
        pdf.extend_from_slice(b"6 0 obj\n");
        pdf.extend_from_slice(
            format!(
                "<< /Length {} /Length1 {} >>\nstream\n",
                font_bytes.len(),
                font_bytes.len()
            )
            .as_bytes(),
        );
        pdf.extend_from_slice(font_bytes);
        pdf.extend_from_slice(b"\nendstream\nendobj\n");

        let xref = pdf.len();
        pdf.extend_from_slice(format!("xref\n0 {}\n", offsets.len()).as_bytes());
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        for &offset in &offsets[1..] {
            pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        pdf.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
                offsets.len()
            )
            .as_bytes(),
        );
        pdf
    }

    fn system_font() -> Option<Vec<u8>> {
        for path in [
            "C:/Windows/Fonts/Candara.ttf",
            "C:/Windows/Fonts/arial.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ] {
            if let Ok(bytes) = std::fs::read(path) {
                return Some(bytes);
            }
        }
        None
    }

    /// The same wiring, for a CFF program under `/FontFile3`.
    #[test]
    fn a_cff_font_embedded_in_a_pdf_yields_outlines() {
        let cff = crate::font::cff::built_font::build();
        let pdf = pdf_embedding_with(&cff, "FontFile3");
        let fonts = embedded_fonts(&pdf);

        assert_eq!(fonts.len(), 1, "the embedded CFF font should be found");
        let font = &fonts[0];
        assert!(matches!(font.program, FontProgram::Cff(_)));

        // The built font names its one glyph "A" and draws a known rectangle.
        let glyph = font.outline(b'A').expect("A should have an outline");
        let points: Vec<[f32; 2]> = glyph.contours.iter().flatten().copied().collect();
        let min_x = points.iter().map(|p| p[0]).fold(f32::MAX, f32::min);
        let max_x = points.iter().map(|p| p[0]).fold(f32::MIN, f32::max);
        assert_eq!(min_x, 100.0);
        assert_eq!(max_x, 300.0);
    }

    #[test]
    fn a_truetype_font_embedded_in_a_pdf_yields_outlines() {
        let Some(font_bytes) = system_font() else {
            eprintln!("skipping: no system TrueType font");
            return;
        };
        let pdf = pdf_embedding(&font_bytes);
        let fonts = embedded_fonts(&pdf);

        assert_eq!(fonts.len(), 1, "the embedded TrueType font should be found");
        let font = &fonts[0];
        assert!(matches!(font.program, FontProgram::TrueType(_)));
        assert_eq!(font.base_font, "TestTT");

        // A 'A' must come back with an outline, reached by character code.
        let glyph = font.outline(b'A').expect("A should have an outline");
        assert!(!glyph.contours.is_empty());

        // The matrix must reflect the font's own design grid, or every glyph
        // is drawn at the wrong size.
        let matrix = font.font_matrix();
        assert!(
            matrix[0] > 0.0 && matrix[0] <= 0.002,
            "font matrix scale looks wrong: {matrix:?}"
        );
    }
}
