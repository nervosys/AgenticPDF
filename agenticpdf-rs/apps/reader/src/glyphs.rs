// SPDX-License-Identifier: AGPL-3.0-or-later
//! The document's own glyphs, ready to fill.
//!
//! A renderer that substitutes a font can only approximate a page: its
//! advances differ from the document's, so runs collide, or have to be
//! squeezed to fit, and the result is visibly not what the author saw. PDF.js
//! and Okular draw the glyphs the file carries. So does this, by filling the
//! outlines [`agenticpdf::font`] decodes.
//!
//! Outlines are cached per code, because a charstring is a small program and
//! interpreting one per glyph per frame would be paid sixty times a second for
//! an answer that never changes.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use agenticpdf::font::{self, EmbeddedFont, Glyph};

/// Outline cache: `(font index, character code)`.
type OutlineCache = Mutex<HashMap<(usize, u32), Option<Arc<Glyph>>>>;
/// Raster cache: `(font index, code, size in quarter-pixels, degrees, colour)`.
type RasterCache = Mutex<HashMap<(usize, u32, u32, i32, [u8; 3]), Option<Arc<RasterGlyph>>>>;

/// Every embedded font in one document, with its glyphs cached.
///
/// Shared behind a lock rather than a cell because the mobile shells keep the
/// session in a `static Mutex`, which requires everything in it to be `Send`.
#[derive(Default)]
pub struct FontSet {
    fonts: Vec<EmbeddedFont>,
    /// `/BaseFont` name to index in `fonts`.
    by_name: HashMap<String, usize>,
    /// `(font index, character code)` to outline. `None` is cached too: a code
    /// with no glyph is a lookup that would otherwise be repeated every frame.
    cache: OutlineCache,
    /// `(font index, code, size in 1/4 px, angle in degrees, colour)` to mask.
    /// Rasterising is the expensive half, and a page reuses the same letters at
    /// the same size constantly, so this is what keeps a redraw cheap.
    rasters: RasterCache,
}

impl FontSet {
    /// Read every embedded font from a document.
    ///
    /// A file with none -- a Word document laid out by the typesetter, or a
    /// PDF that references fonts without embedding them -- yields an empty set,
    /// and the caller falls back to laying text out with its own font.
    pub fn load(data: &[u8]) -> FontSet {
        let fonts = font::embedded_fonts(data);
        let by_name = fonts
            .iter()
            .enumerate()
            .map(|(index, font)| (font.base_font.clone(), index))
            .collect();
        FontSet {
            fonts,
            by_name,
            cache: Mutex::new(HashMap::new()),
            rasters: Mutex::new(HashMap::new()),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.fonts.is_empty()
    }

    pub fn len(&self) -> usize {
        self.fonts.len()
    }

    /// The em-space scale for a font's outlines: its `/FontMatrix` reduces
    /// font units to text space, and is very nearly always 1/1000.
    pub fn font_matrix(&self, base_font: &str) -> Option<[f64; 6]> {
        let index = *self.by_name.get(base_font)?;
        Some(self.fonts[index].font_matrix())
    }

    /// The outline a character code selects in a named font.
    pub fn glyph(&self, base_font: &str, code: u32) -> Option<Arc<Glyph>> {
        let index = *self.by_name.get(base_font)?;
        let key = (index, code);
        if let Ok(cache) = self.cache.lock()
            && let Some(hit) = cache.get(&key)
        {
            return hit.clone();
        }
        let outline = u8::try_from(code)
            .ok()
            .and_then(|code| self.fonts[index].outline(code))
            .filter(|glyph| !glyph.contours.is_empty())
            .map(Arc::new);
        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(key, outline.clone());
        }
        outline
    }
}

impl FontSet {
    /// A rasterised glyph, cached.
    ///
    /// Size is quantised to a quarter pixel so that a zoom animation reuses
    /// masks instead of rasterising a fresh set every frame.
    pub fn raster(
        &self,
        base_font: &str,
        code: u32,
        size: f64,
        rot: f64,
        color: [u8; 3],
    ) -> Option<Arc<RasterGlyph>> {
        let index = *self.by_name.get(base_font)?;
        let quantised = (size * 4.0).round().max(0.0) as u32;
        // Angle quantised to a degree: a run shares one angle, so this costs a
        // cache entry per angle actually used rather than per glyph.
        let angle = (rot.to_degrees().round() as i32).rem_euclid(360);
        let key = (index, code, quantised, angle, color);
        if let Ok(cache) = self.rasters.lock()
            && let Some(hit) = cache.get(&key)
        {
            return hit.clone();
        }
        let matrix = self.fonts[index].font_matrix();
        let raster = self
            .glyph(base_font, code)
            .and_then(|glyph| {
                rasterize(
                    &glyph,
                    matrix,
                    quantised as f64 / 4.0,
                    (angle as f64).to_radians(),
                    color,
                )
            })
            .map(Arc::new);
        if let Ok(mut cache) = self.rasters.lock() {
            cache.insert(key, raster.clone());
        }
        raster
    }
}

impl std::fmt::Debug for FontSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FontSet")
            .field("fonts", &self.fonts.len())
            .field("cached", &self.cache.lock().map(|c| c.len()).unwrap_or(0))
            .finish()
    }
}

/// A glyph rasterised to an anti-aliased RGBA mask, positioned relative to the
/// text origin.
///
/// Filling the outline as a polygon is the obvious alternative and it does not
/// work: the painter's filled-path primitive covers a *convex* polygon, so a
/// concave letter -- `L`, `E`, `C` -- fills as its hull and comes out a solid
/// blob. Rasterising here also buys anti-aliasing, without which ten-point
/// text is a staircase.
pub struct RasterGlyph {
    pub width: u32,
    pub height: u32,
    /// Offset from the glyph's origin to the bitmap's top-left, in pixels,
    /// with y increasing downward as the screen does.
    pub left: f32,
    pub top: f32,
    pub pixels: Vec<u8>,
}

/// Supersampling factor per axis. Three is enough to make ten-point text look
/// like text rather than a staircase, and costs nine coverage samples per
/// pixel on a glyph that is cached from then on.
const SUPERSAMPLE: usize = 3;

/// Rasterise a glyph at a device pixel size.
///
/// `contours` arrive in font units; `scale` converts them to pixels. The mask
/// is tinted at draw time by the caller's colour, so the same coverage serves
/// every colour the document uses.
pub fn rasterize(
    glyph: &Glyph,
    matrix: [f64; 6],
    size: f64,
    rot: f64,
    color: [u8; 3],
) -> Option<RasterGlyph> {
    // Font units through /FontMatrix into text space, then to pixels. y is
    // flipped here: glyph space is y-up, a bitmap is y-down.
    let mut contours: Vec<Vec<[f32; 2]>> = Vec::with_capacity(glyph.contours.len());
    for contour in &glyph.contours {
        contours.push(
            contour
                .iter()
                .map(|[gx, gy]| {
                    let tx = matrix[0] * *gx as f64 + matrix[2] * *gy as f64 + matrix[4];
                    let ty = matrix[1] * *gx as f64 + matrix[3] * *gy as f64 + matrix[5];
                    // Rotate in document space, where y is up, then flip for a
                    // bitmap, where y is down. Rotating after the flip turns
                    // the glyph the wrong way.
                    let (dx, dy) = (tx * size, ty * size);
                    let (cos, sin) = (rot.cos(), rot.sin());
                    [(dx * cos - dy * sin) as f32, -(dx * sin + dy * cos) as f32]
                })
                .collect(),
        );
    }

    let points: Vec<[f32; 2]> = contours.iter().flatten().copied().collect();
    if points.len() < 3 {
        return None;
    }
    let min_x = points.iter().map(|p| p[0]).fold(f32::MAX, f32::min).floor() - 1.0;
    let max_x = points.iter().map(|p| p[0]).fold(f32::MIN, f32::max).ceil() + 1.0;
    let min_y = points.iter().map(|p| p[1]).fold(f32::MAX, f32::min).floor() - 1.0;
    let max_y = points.iter().map(|p| p[1]).fold(f32::MIN, f32::max).ceil() + 1.0;

    let width = (max_x - min_x) as i64;
    let height = (max_y - min_y) as i64;
    // A glyph larger than this is a broken outline, not a letter; refusing it
    // keeps a damaged font from asking for a gigabyte.
    if !(1..=4096).contains(&width) || !(1..=4096).contains(&height) {
        return None;
    }
    let (width, height) = (width as usize, height as usize);

    // Coverage by supersampled scanlines, non-zero winding -- the rule
    // PostScript fills with, and what gives a counter its hole.
    let mut coverage = vec![0u16; width * height];
    let samples = height * SUPERSAMPLE;
    let mut crossings: Vec<(f32, i32)> = Vec::new();
    for sample in 0..samples {
        let y = min_y + (sample as f32 + 0.5) / SUPERSAMPLE as f32;
        crossings.clear();
        for contour in &contours {
            for index in 0..contour.len() {
                let a = contour[index];
                let b = contour[(index + 1) % contour.len()];
                if (a[1] > y) == (b[1] > y) {
                    continue;
                }
                let t = (y - a[1]) / (b[1] - a[1]);
                let direction = if b[1] > a[1] { 1 } else { -1 };
                crossings.push((a[0] + t * (b[0] - a[0]), direction));
            }
        }
        if crossings.is_empty() {
            continue;
        }
        crossings.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        let row = sample / SUPERSAMPLE;
        let mut winding = 0;
        for pair in crossings.windows(2) {
            winding += pair[0].1;
            if winding == 0 {
                continue;
            }
            // Horizontal coverage is analytic: a span that covers part of a
            // pixel contributes that fraction, which is what keeps stems from
            // shimmering between one pixel and two.
            let from = pair[0].0.max(min_x);
            let to = pair[1].0.min(max_x);
            if to <= from {
                continue;
            }
            let first = ((from - min_x).floor() as usize).min(width - 1);
            let last = ((to - min_x).ceil() as usize).min(width);
            for column in first..last {
                let left = min_x + column as f32;
                let overlap = (to.min(left + 1.0) - from.max(left)).clamp(0.0, 1.0);
                if overlap > 0.0 {
                    let at = row * width + column;
                    coverage[at] = coverage[at].saturating_add((overlap * 255.0) as u16);
                }
            }
        }
    }

    let mut pixels = vec![0u8; width * height * 4];
    for (index, &value) in coverage.iter().enumerate() {
        let alpha = (value / SUPERSAMPLE as u16).min(255) as u8;
        pixels[index * 4] = color[0];
        pixels[index * 4 + 1] = color[1];
        pixels[index * 4 + 2] = color[2];
        pixels[index * 4 + 3] = alpha;
    }

    Some(RasterGlyph {
        width: width as u32,
        height: height as u32,
        left: min_x,
        top: min_y,
        pixels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Option<Vec<u8>> {
        std::fs::read("../../../demos/sample.pdf")
            .or_else(|_| std::fs::read("../demos/sample.pdf"))
            .or_else(|_| std::fs::read("demos/sample.pdf"))
            .ok()
    }

    #[test]
    fn a_document_without_fonts_yields_an_empty_set() {
        let set = FontSet::load(b"not a pdf");
        assert!(set.is_empty());
        assert!(set.glyph("anything", 65).is_none());
    }

    #[test]
    fn the_documents_own_glyphs_are_found_by_code() {
        let Some(pdf) = sample() else {
            eprintln!("skipping: demos/sample.pdf not present");
            return;
        };
        let set = FontSet::load(&pdf);
        assert!(!set.is_empty(), "the sample embeds fonts");

        // 'I' from the body font, as the title's first character.
        let glyph = set.glyph("MGMJCW+NimbusRomNo9L-Regu", b'I' as u32);
        let glyph = glyph.expect("the body font should have an I");
        assert!(!glyph.contours.is_empty());
    }

    /// The cache must return the same outline rather than re-interpreting the
    /// charstring, which is the whole reason it exists.
    #[test]
    fn a_repeated_glyph_comes_from_the_cache() {
        let Some(pdf) = sample() else {
            eprintln!("skipping: demos/sample.pdf not present");
            return;
        };
        let set = FontSet::load(&pdf);
        let first = set.glyph("MGMJCW+NimbusRomNo9L-Regu", b'e' as u32);
        let second = set.glyph("MGMJCW+NimbusRomNo9L-Regu", b'e' as u32);
        match (first, second) {
            (Some(a), Some(b)) => assert!(Arc::ptr_eq(&a, &b), "the same outline should be shared"),
            _ => panic!("the body font should have an e"),
        }
    }

    /// A miss is cached too, or every frame repeats a lookup that fails.
    #[test]
    fn a_missing_glyph_is_remembered_as_missing() {
        let Some(pdf) = sample() else {
            eprintln!("skipping: demos/sample.pdf not present");
            return;
        };
        let set = FontSet::load(&pdf);
        assert!(set.glyph("MGMJCW+NimbusRomNo9L-Regu", 1).is_none());
        assert!(set.glyph("MGMJCW+NimbusRomNo9L-Regu", 1).is_none());
        assert!(set.glyph("no such font", 65).is_none());
    }
}

#[cfg(test)]
mod raster_tests {
    use super::*;

    fn sample() -> Option<Vec<u8>> {
        std::fs::read("../demos/sample.pdf").ok()
    }

    /// A concave letter must rasterise as that letter, not as its hull.
    ///
    /// This is the failure the rasteriser exists to avoid: the painter's
    /// filled-path primitive covers a convex polygon, so `L` drawn as a path
    /// came out a solid rectangle. The check is that the corner opposite the
    /// stem stays empty.
    #[test]
    fn a_concave_letter_is_not_filled_as_a_blob() {
        let Some(pdf) = sample() else {
            eprintln!("skipping: demos/sample.pdf not present");
            return;
        };
        let set = FontSet::load(&pdf);
        let raster = set
            .raster(
                "MGMJCW+NimbusRomNo9L-Regu",
                b'L' as u32,
                40.0,
                0.0,
                [0, 0, 0],
            )
            .expect("the body font should have an L");

        let alpha = |x: u32, y: u32| -> u8 {
            let at = ((y * raster.width + x) * 4 + 3) as usize;
            raster.pixels.get(at).copied().unwrap_or(0)
        };

        // Bottom-left is the foot of the L: inked.
        let bottom_left = alpha(raster.width / 5, raster.height * 4 / 5);
        // Top-right is the open corner: an L leaves it empty, a hull fills it.
        let top_right = alpha(raster.width * 4 / 5, raster.height / 5);

        assert!(
            bottom_left > 128,
            "the L's foot should be inked: {bottom_left}"
        );
        assert!(
            top_right < 64,
            "the corner opposite the stem should be empty; a filled one means \
             the glyph was drawn as its convex hull: {top_right}"
        );
    }

    /// A counter is a hole, not a second disc drawn over the first.
    #[test]
    fn a_counter_stays_open() {
        let Some(pdf) = sample() else {
            eprintln!("skipping: demos/sample.pdf not present");
            return;
        };
        let set = FontSet::load(&pdf);
        let raster = set
            .raster(
                "MGMJCW+NimbusRomNo9L-Regu",
                b'o' as u32,
                40.0,
                0.0,
                [0, 0, 0],
            )
            .expect("the body font should have an o");

        let centre = {
            let at = (((raster.height / 2) * raster.width + raster.width / 2) * 4 + 3) as usize;
            raster.pixels.get(at).copied().unwrap_or(0)
        };
        assert!(centre < 64, "the middle of an o should be open: {centre}");
    }

    /// Rasters are cached per size and angle, because rasterising per frame
    /// would repeat the expensive half of the work sixty times a second.
    #[test]
    fn a_repeated_raster_is_shared() {
        let Some(pdf) = sample() else {
            eprintln!("skipping: demos/sample.pdf not present");
            return;
        };
        let set = FontSet::load(&pdf);
        let first = set.raster(
            "MGMJCW+NimbusRomNo9L-Regu",
            b'e' as u32,
            12.0,
            0.0,
            [0, 0, 0],
        );
        let second = set.raster(
            "MGMJCW+NimbusRomNo9L-Regu",
            b'e' as u32,
            12.0,
            0.0,
            [0, 0, 0],
        );
        match (first, second) {
            (Some(a), Some(b)) => assert!(Arc::ptr_eq(&a, &b)),
            _ => panic!("the body font should have an e"),
        }
    }

    /// A quarter turn must produce a mask taller than it is wide for a letter
    /// that is wider than it is tall, which is the cheapest proof the rotation
    /// is applied at all.
    #[test]
    fn a_rotated_glyph_is_actually_rotated() {
        let Some(pdf) = sample() else {
            eprintln!("skipping: demos/sample.pdf not present");
            return;
        };
        let set = FontSet::load(&pdf);
        let upright = set
            .raster(
                "MGMJCW+NimbusRomNo9L-Regu",
                b'L' as u32,
                40.0,
                0.0,
                [0, 0, 0],
            )
            .expect("L");
        let turned = set
            .raster(
                "MGMJCW+NimbusRomNo9L-Regu",
                b'L' as u32,
                40.0,
                std::f64::consts::FRAC_PI_2,
                [0, 0, 0],
            )
            .expect("L turned");
        assert!(
            (turned.width as i32 - upright.height as i32).abs() <= 2,
            "a quarter turn should swap the mask's axes: {}x{} vs {}x{}",
            upright.width,
            upright.height,
            turned.width,
            turned.height
        );
    }
}
