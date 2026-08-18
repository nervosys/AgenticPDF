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
pub mod glyphnames;
pub mod substitute;
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
    /// Set for a composite (`/Type0`) font, whose codes are character ids
    /// addressing glyphs directly rather than single bytes naming them.
    pub composite: bool,
    /// `/CIDToGIDMap` as a stream: character id to glyph index. `None` means
    /// `/Identity`, where the two are the same.
    pub cid_to_gid: Option<Vec<u16>>,
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

    /// The glyph index a character id maps to in a composite font.
    fn glyph_for_cid(&self, cid: u32) -> u16 {
        match &self.cid_to_gid {
            // A map shorter than the id space leaves the rest at zero, which
            // is `.notdef` -- the same answer the specification gives.
            Some(map) => map.get(cid as usize).copied().unwrap_or(0),
            None => cid as u16,
        }
    }

    /// The outline a character code selects.
    ///
    /// A composite font's codes are character ids, which can exceed a byte and
    /// address glyphs directly. A simple font's codes are bytes that name a
    /// glyph through an encoding.
    pub fn outline(&self, code: u32) -> Option<Glyph> {
        self.outline_for(code, None)
    }

    /// The outline a character code selects, given the character the document
    /// says the code means.
    ///
    /// The character matters for a TrueType font, whose `cmap` is keyed by
    /// Unicode. WinAnsi puts its curly quotes at codes 0x91-0x94, where
    /// Unicode has control characters, so looking up the code as a character
    /// misses every quotation mark, apostrophe and dash on the page. The
    /// engine has already decoded the right character; this is how it reaches
    /// the font.
    pub fn outline_for(&self, code: u32, unicode: Option<char>) -> Option<Glyph> {
        if self.composite {
            let gid = self.glyph_for_cid(code);
            return match &self.program {
                FontProgram::TrueType(font) => font.outline(gid),
                // A CID-keyed CFF maps the character id through its own
                // charset; a subset font is usually the identity, and one that
                // is not says so.
                FontProgram::Cff(font) => font.outline(font.index_for_cid(gid)),
                // A Type 1 program is never the descendant of a composite
                // font, but answering by index keeps this total.
                FontProgram::Type1(_) => None,
            };
        }
        let code = u8::try_from(code).ok()?;
        match &self.program {
            FontProgram::Type1(font) => font.outline(self.glyph_name(code)?),
            FontProgram::TrueType(font) => {
                // The document's own `/Differences` first: it states what a
                // code means, which beats assuming the code is Latin-1.
                let named = self
                    .differences
                    .get(&code)
                    .and_then(|name| glyph_name_to_char(name));
                // The document's own decoding first, then `/Differences`,
                // then the code read as a character -- which is only right
                // for the ASCII range.
                let hint = unicode.or(named).or_else(|| char::from_u32(code as u32));
                // Keep the first candidate that actually draws: a subset can
                // point one route at a blank glyph while another reaches the
                // real one.
                let candidates = font.glyph_candidates(code as u32, hint);
                let drawn = candidates
                    .iter()
                    .filter_map(|index| font.outline(*index))
                    .find(|glyph| !glyph.contours.is_empty());
                drawn.or_else(|| font.outline(*candidates.first()?))
            }
            FontProgram::Cff(font) => {
                // The document's `/Differences` name first, then the font's
                // own encoding. A subset font commonly ships an encoding that
                // is the only route to its glyphs.
                // The document's own name first, then the font's encoding,
                // then the Adobe name for the character the code decoded to.
                // That last route is how the WinAnsi high range is reached at
                // all: its dashes and accented letters are named, not spelled.
                let index = self
                    .differences
                    .get(&code)
                    .and_then(|name| font.index_for_name(name))
                    .or_else(|| font.index_for_code(code))
                    .or_else(|| {
                        let ch = unicode?;
                        glyphnames::names_for(ch)
                            .iter()
                            .find_map(|name| font.index_for_name(name))
                    })?;
                font.outline(index)
            }
        }
    }
}

/// The character a glyph name stands for.
///
/// Delegates to [`glyphnames`], which holds the one table both directions are
/// derived from. Keeping a second copy here is how the two came to disagree
/// about whether `quoteright` was the upright apostrophe or the curly one.
fn glyph_name_to_char(name: &str) -> Option<char> {
    glyphnames::char_for(name)
}

/// Every font program embedded in a PDF, parsed.
///
/// A name may appear more than once. A producer commonly embeds several
/// subsets of one typeface -- one per chunk of the document -- all under the
/// same `/BaseFont`, each carrying only the glyphs its own chunk needs.
/// Keeping the first and discarding the rest loses most of the alphabet for
/// every page the first subset did not cover, so all of them are returned and
/// the caller tries each.
///
/// Fonts that will not parse are dropped rather than reported: a renderer that
/// cannot read one font should fall back for that font, not fail the page.
pub fn embedded_fonts(data: &[u8]) -> Vec<EmbeddedFont> {
    let Ok(doc) = engine::Document::parse(data) else {
        return Vec::new();
    };

    let mut out = Vec::new();

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

        // A CID font is reached through its `/Type0` parent and is never used
        // on its own. Taking it as a font in its own right would offer a
        // second, byte-addressed reading of the same program under the same
        // name, and a lookup could pick the wrong one.
        let subtype = doc
            .get(dict, "Subtype")
            .and_then(|o| o.as_name().map(String::from))
            .unwrap_or_default();
        if subtype.starts_with("CIDFontType") {
            continue;
        }

        let base_font = doc
            .get(dict, "BaseFont")
            .and_then(|o| o.as_name().map(String::from))
            .unwrap_or_default();
        // A composite (`/Type0`) font keeps its descriptor on a descendant,
        // and addresses glyphs by character id rather than by name. Looking
        // only at the outer dictionary finds no descriptor and skips the font
        // entirely, which is what used to happen to every CJK document.
        let composite = subtype == "Type0";

        let owner = match composite {
            false => dict.clone(),
            true => {
                let Some(descendants) = doc.get(dict, "DescendantFonts") else {
                    continue;
                };
                let Object::Array(items) = doc.resolve(&descendants) else {
                    continue;
                };
                let Some(Object::Dict(first)) = items.first().map(|item| doc.resolve(item)) else {
                    continue;
                };
                first
            }
        };

        let Some(descriptor) = doc.get(&owner, "FontDescriptor") else {
            continue;
        };
        let descriptor = doc.resolve(&descriptor);
        let Object::Dict(descriptor) = descriptor else {
            continue;
        };

        // `/FontFile` is Type 1, `/FontFile2` TrueType, `/FontFile3` CFF.
        let Some(program) = read_program(&doc, &descriptor) else {
            continue;
        };

        let cid_to_gid = match composite {
            false => None,
            true => read_cid_to_gid(&doc, &owner),
        };

        out.push(EmbeddedFont {
            base_font,
            program,
            differences: differences(&doc, dict),
            composite,
            cid_to_gid,
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

/// `/CIDToGIDMap`: a stream of big-endian glyph indices, one per character id.
///
/// `/Identity`, or an absent entry, means the two are the same and no table is
/// needed.
fn read_cid_to_gid(doc: &engine::Document<'_>, descendant: &engine::Dict) -> Option<Vec<u16>> {
    let entry = doc.get(descendant, "CIDToGIDMap")?;
    let Object::Stream(stream_dict, raw) = doc.resolve(&entry) else {
        return None;
    };
    let bytes = engine::decode_stream(&stream_dict, &raw).ok()?;
    Some(
        bytes
            .chunks_exact(2)
            .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
            .collect(),
    )
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
        let glyph = font.outline(b'A' as u32).expect("A should have an outline");
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
        let glyph = font.outline(b'A' as u32).expect("A should have an outline");
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

#[cfg(test)]
mod fixture_writer {
    /// Write a PDF that embeds a real TrueType font and draws text with it,
    /// so the whole pipeline can be looked at rather than only unit-tested.
    ///
    /// Ignored by default: it writes a file and depends on a system font. Run
    /// with `cargo test -- --ignored write_truetype_sample`.
    #[test]
    #[ignore = "writes a file; run deliberately"]
    fn write_truetype_sample() {
        let Ok(font_bytes) = std::fs::read("C:/Windows/Fonts/Candara.ttf")
            .or_else(|_| std::fs::read("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"))
        else {
            eprintln!("skipping: no system TrueType font");
            return;
        };
        let font = super::TrueTypeFont::parse(&font_bytes).expect("system font parses");

        // Real advances from the font's own metrics, as a producer would
        // write them: text laid out with the wrong widths would not exercise
        // the renderer honestly.
        let scale = 1000.0 / font.units_per_em;
        let widths: Vec<String> = (32..127u8)
            .map(|code| {
                let advance = font
                    .glyph_index(code as u32, char::from_u32(code as u32))
                    .and_then(|index| font.outline(index))
                    .map(|glyph| glyph.advance as f64 * scale)
                    .unwrap_or(500.0);
                format!("{}", advance.round() as i64)
            })
            .collect();

        let lines = [
            "TrueType rendering check",
            "The quick brown fox jumps over the lazy dog.",
            "Waltz, bad nymph, for quick jigs vex! 0123456789",
            "Kerning pairs: AV To Ta We Yo P. r, y.",
            "Punctuation: (parentheses) [brackets] {braces} @ # $ % &",
        ];
        let mut content = String::from("BT\n/F1 18 Tf\n72 700 Td\n");
        for (index, line) in lines.iter().enumerate() {
            if index > 0 {
                content.push_str("0 -28 Td\n");
            }
            let escaped = line
                .replace('\\', "\\\\")
                .replace('(', "\\(")
                .replace(')', "\\)");
            content.push_str(&format!("({escaped}) Tj\n"));
        }
        content.push_str("ET\n");

        let mut pdf: Vec<u8> = Vec::new();
        let mut offsets = vec![0usize];
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let push = |pdf: &mut Vec<u8>, offsets: &mut Vec<usize>, body: &[u8]| {
            offsets.push(pdf.len());
            let number = offsets.len() - 1;
            pdf.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
            pdf.extend_from_slice(body);
            pdf.extend_from_slice(b"\nendobj\n");
        };

        push(&mut pdf, &mut offsets, b"<< /Type /Catalog /Pages 2 0 R >>");
        push(
            &mut pdf,
            &mut offsets,
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        );
        push(
            &mut pdf,
            &mut offsets,
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
              /Resources << /Font << /F1 4 0 R >> >> /Contents 7 0 R >>",
        );
        push(
            &mut pdf,
            &mut offsets,
            format!(
                "<< /Type /Font /Subtype /TrueType /BaseFont /Candara /FirstChar 32 \
                 /LastChar 126 /Widths [{}] /FontDescriptor 5 0 R /Encoding /WinAnsiEncoding >>",
                widths.join(" ")
            )
            .as_bytes(),
        );
        push(
            &mut pdf,
            &mut offsets,
            b"<< /Type /FontDescriptor /FontName /Candara /Flags 32 /ItalicAngle 0 \
              /Ascent 750 /Descent -250 /CapHeight 700 /StemV 80 \
              /FontBBox [-500 -300 1500 1000] /FontFile2 6 0 R >>",
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
        pdf.extend_from_slice(&font_bytes);
        pdf.extend_from_slice(b"\nendstream\nendobj\n");

        push(
            &mut pdf,
            &mut offsets,
            format!(
                "<< /Length {} >>\nstream\n{content}\nendstream",
                content.len()
            )
            .as_bytes(),
        );

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

        let out =
            std::env::var("APDF_FIXTURE_OUT").unwrap_or_else(|_| "truetype-sample.pdf".to_string());
        std::fs::write(&out, &pdf).expect("write the fixture");
        eprintln!("wrote {out} ({} bytes)", pdf.len());
    }
}

#[cfg(test)]
mod composite_fonts {
    use super::*;

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

    /// A `/Type0` font keeps its descriptor on a descendant dictionary. Looking
    /// only at the outer font finds no descriptor at all, which is how every
    /// composite font -- most CJK documents, and plenty of subset Latin ones --
    /// used to be skipped entirely.
    #[test]
    fn a_composite_font_is_found_through_its_descendant() {
        let Some(font_bytes) = system_font() else {
            eprintln!("skipping: no system TrueType font");
            return;
        };
        let program = TrueTypeFont::parse(&font_bytes).expect("parses");
        // Address a glyph the font really has, by index.
        let gid = program
            .glyph_index(b'A' as u32, Some('A'))
            .expect("a font should map A");

        let pdf = composite_pdf(&font_bytes, None);
        let fonts = embedded_fonts(&pdf);
        assert_eq!(fonts.len(), 1, "the composite font should be found");
        let font = &fonts[0];
        assert!(font.composite, "it should be marked composite");

        // Identity: the character id is the glyph index.
        let glyph = font.outline(gid as u32).expect("the glyph draws");
        assert!(!glyph.contours.is_empty());
    }

    /// With a `/CIDToGIDMap` stream the id is an index into that table, not a
    /// glyph index. Ignoring the map draws the wrong letter for every code.
    #[test]
    fn a_cid_to_gid_map_is_honoured() {
        let Some(font_bytes) = system_font() else {
            eprintln!("skipping: no system TrueType font");
            return;
        };
        let program = TrueTypeFont::parse(&font_bytes).expect("parses");
        let gid = program
            .glyph_index(b'A' as u32, Some('A'))
            .expect("a font should map A");

        // A map sending character id 7 to that glyph, and everything else to
        // .notdef.
        let mut map = vec![0u8; 8 * 2];
        map[14] = (gid >> 8) as u8;
        map[15] = (gid & 0xFF) as u8;

        let pdf = composite_pdf(&font_bytes, Some(&map));
        let fonts = embedded_fonts(&pdf);
        let font = fonts.first().expect("the composite font should be found");
        assert!(font.cid_to_gid.is_some(), "the map should have been read");

        let mapped = font.outline(7).expect("id 7 should draw through the map");
        let direct = program.outline(gid).expect("the same glyph directly");
        assert_eq!(
            mapped.contours.len(),
            direct.contours.len(),
            "the mapped id must reach the same glyph"
        );
        assert!(!mapped.contours.is_empty());

        // An id the map sends to zero must reach glyph 0, not a letter.
        // `.notdef` is not empty -- it is usually a box -- so the check is
        // that it is *that* glyph, and that it is not the letter.
        let unmapped = font.outline(3).expect("an unmapped id still resolves");
        assert_eq!(
            unmapped,
            program.outline(0).expect("glyph 0 exists"),
            "an unmapped id should reach .notdef"
        );
        assert_ne!(unmapped, mapped, "the two ids must reach different glyphs");
    }

    /// Build a `/Type0` font with a TrueType descendant.
    fn composite_pdf(font_bytes: &[u8], cid_to_gid: Option<&[u8]>) -> Vec<u8> {
        let mut pdf: Vec<u8> = Vec::new();
        let mut offsets = vec![0usize];
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let push = |pdf: &mut Vec<u8>, offsets: &mut Vec<usize>, body: &[u8]| {
            offsets.push(pdf.len());
            let number = offsets.len() - 1;
            pdf.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
            pdf.extend_from_slice(body);
            pdf.extend_from_slice(b"\nendobj\n");
        };

        push(&mut pdf, &mut offsets, b"<< /Type /Catalog /Pages 2 0 R >>");
        push(
            &mut pdf,
            &mut offsets,
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        );
        push(
            &mut pdf,
            &mut offsets,
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] \
              /Resources << /Font << /F1 4 0 R >> >> >>",
        );
        push(
            &mut pdf,
            &mut offsets,
            b"<< /Type /Font /Subtype /Type0 /BaseFont /TestCID /Encoding /Identity-H \
              /DescendantFonts [5 0 R] >>",
        );
        let descendant = match cid_to_gid {
            Some(_) => "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /TestCID \
                        /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> \
                        /FontDescriptor 6 0 R /CIDToGIDMap 8 0 R >>"
                .to_string(),
            None => "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /TestCID \
                     /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> \
                     /FontDescriptor 6 0 R /CIDToGIDMap /Identity >>"
                .to_string(),
        };
        push(&mut pdf, &mut offsets, descendant.as_bytes());
        push(
            &mut pdf,
            &mut offsets,
            b"<< /Type /FontDescriptor /FontName /TestCID /Flags 4 /FontFile2 7 0 R >>",
        );

        offsets.push(pdf.len());
        pdf.extend_from_slice(b"7 0 obj\n");
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

        if let Some(map) = cid_to_gid {
            offsets.push(pdf.len());
            pdf.extend_from_slice(b"8 0 obj\n");
            pdf.extend_from_slice(format!("<< /Length {} >>\nstream\n", map.len()).as_bytes());
            pdf.extend_from_slice(map);
            pdf.extend_from_slice(b"\nendstream\nendobj\n");
        }

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
}
