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

pub mod type1;

pub use type1::{Glyph, Type1Font};

use crate::engine::{self, Object};
use std::collections::HashMap;

/// An embedded font program, keyed by the `/BaseFont` name it was found under.
#[derive(Debug, Clone)]
pub struct EmbeddedFont {
    /// `/BaseFont`, subset prefix and all (`RDQRPY+CMR10`).
    pub base_font: String,
    /// The glyph source.
    pub program: Type1Font,
    /// `/Differences`: character code to glyph name, which overrides whatever
    /// encoding the font program itself ships with.
    pub differences: HashMap<u8, String>,
}

impl EmbeddedFont {
    /// The glyph name a character code selects: the PDF's `/Differences`
    /// first, then the font's own encoding.
    pub fn glyph_name(&self, code: u8) -> Option<&str> {
        self.differences
            .get(&code)
            .map(String::as_str)
            .or_else(|| self.program.glyph_name(code))
    }

    /// The outline a character code selects.
    pub fn outline(&self, code: u8) -> Option<Glyph> {
        let name = self.glyph_name(code)?;
        self.program.outline(name)
    }
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

        // `/FontFile` is Type 1. `/FontFile2` (TrueType) and `/FontFile3`
        // (CFF) are not read yet; a caller falls back for those.
        let Some(file) = doc.get(&descriptor, "FontFile") else {
            continue;
        };
        let Object::Stream(stream_dict, raw) = doc.resolve(&file) else {
            continue;
        };
        let Ok(bytes) = engine::decode_stream(&stream_dict, &raw) else {
            continue;
        };

        let length1 = doc
            .get(&stream_dict, "Length1")
            .and_then(|o| o.as_int())
            .unwrap_or(0)
            .max(0) as usize;
        let length2 = doc
            .get(&stream_dict, "Length2")
            .and_then(|o| o.as_int())
            .unwrap_or(0)
            .max(0) as usize;

        let Some(program) = Type1Font::parse(&bytes, length1, length2) else {
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

/// Just the parsed programs, for tests and callers that only want glyphs.
pub fn embedded_type1(data: &[u8]) -> Vec<Type1Font> {
    embedded_fonts(data)
        .into_iter()
        .map(|font| font.program)
        .collect()
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
            for name in font.program.glyph_names() {
                match font.program.outline(&name) {
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
