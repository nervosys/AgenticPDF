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

/// Outline cache, keyed by font name and character code.
/// Keyed by the hint as well as the code: the result depends on both, so
/// caching by code alone lets a hintless lookup poison a hinted one.
type OutlineCache = Mutex<HashMap<(String, u32, Option<char>), Option<(Arc<Glyph>, [f64; 6])>>>;
/// Raster cache: font name, code, size in quarter-pixels, degrees, colour.
type RasterCache =
    Mutex<HashMap<(String, u32, Option<char>, u32, i32, [u8; 3]), Option<Arc<RasterGlyph>>>>;
/// Stand-ins resolved by name. `None` is cached so a face the system lacks is
/// looked for once rather than per glyph.
type SubstituteCache = Mutex<HashMap<String, Option<Arc<EmbeddedFont>>>>;

/// How much rasterised glyph the cache may hold before it starts again.
///
/// Enough for a page of text at several sizes; far less than a reading
/// session's worth of zooming, which is unbounded.
const MAX_RASTER_BYTES: usize = 32 << 20;

/// The fonts one document draws with, and their glyphs, cached.
///
/// Shared behind locks rather than cells because the mobile shells keep the
/// session in a `static Mutex`, which requires everything in it to be `Send`.
#[derive(Default)]
pub struct FontSet {
    /// Fonts the document embeds, by `/BaseFont` name.
    ///
    /// Several per name: a producer commonly embeds one subset per chunk of
    /// the document, all under the same name, each carrying only the glyphs
    /// its own chunk needs. Keeping one loses the rest of the alphabet.
    embedded: HashMap<String, Vec<Arc<EmbeddedFont>>>,
    /// Stand-ins for fonts it names but does not embed.
    substitutes: SubstituteCache,
    cache: OutlineCache,
    /// Rasterising is the expensive half, and a page reuses the same letters
    /// at the same size constantly, so this is what keeps a redraw cheap.
    rasters: RasterCache,
    /// Whether a font the document does not embed may be stood in for from
    /// the system. False in the browser, which has no filesystem to read, and
    /// in tests that need the fallback path to be the one taken.
    substitution: bool,
    /// What the rasterised masks currently weigh, so the cache can be kept
    /// to a budget without walking it on every insertion.
    raster_bytes: std::sync::atomic::AtomicUsize,
    /// Mask pixels per painter unit, as hundredths.
    ///
    /// A painter working in logical units on a display with more pixels than
    /// that -- any phone, any HiDPI screen -- has the host scale the result
    /// up. A mask rasterised one pixel per logical unit is then blown up by
    /// the same factor and the text goes soft, which at small sizes is the
    /// difference between a word and a smudge. Rasterising at the host's
    /// ratio and placing at the same size costs memory, not sharpness.
    raster_scale: std::sync::atomic::AtomicU32,
}

impl FontSet {
    /// Read every embedded font from a document.
    pub fn load(data: &[u8]) -> FontSet {
        let mut embedded: HashMap<String, Vec<Arc<EmbeddedFont>>> = HashMap::new();
        for font in font::embedded_fonts(data) {
            embedded
                .entry(font.base_font.clone())
                .or_default()
                .push(Arc::new(font));
        }
        FontSet {
            embedded,
            substitution: true,
            raster_scale: std::sync::atomic::AtomicU32::new(100),
            raster_bytes: std::sync::atomic::AtomicUsize::new(0),
            ..FontSet::default()
        }
    }

    /// A set that will not stand in for a missing font.
    ///
    /// This is the browser's situation -- there is no filesystem to read a
    /// system face from -- so it is a real configuration rather than a test
    /// affordance, and the fit-to-width fallback exists to serve it.
    pub fn without_substitution(data: &[u8]) -> FontSet {
        FontSet {
            substitution: false,
            raster_scale: std::sync::atomic::AtomicU32::new(100),
            raster_bytes: std::sync::atomic::AtomicUsize::new(0),
            ..FontSet::load(data)
        }
    }

    pub fn is_empty(&self) -> bool {
        self.embedded.is_empty()
    }

    pub fn len(&self) -> usize {
        self.embedded.len()
    }

    /// The face to draw a named font with: the document's own where it
    /// embedded one, otherwise whatever the system has that stands in.
    ///
    /// The standard fourteen are the case that matters. A document may name
    /// Times without embedding it, because every reader is expected to have
    /// it; Poppler, and so Okular, substitutes a system face there. The
    /// document still supplies the advances, so only the letterforms differ.
    fn face(&self, base_font: &str) -> Option<Arc<EmbeddedFont>> {
        self.faces(base_font).into_iter().next()
    }

    /// Every face that could serve a name, in the order to try them.
    ///
    /// Embedded first, then a stand-in. The stand-in is appended rather than
    /// used only when nothing is embedded, because a name can be shared: a
    /// document may embed a composite font and also reference a simple,
    /// non-embedded font under the same name, and a run of the second one
    /// would otherwise be handed the first and draw nothing.
    fn faces(&self, base_font: &str) -> Vec<Arc<EmbeddedFont>> {
        let mut out: Vec<Arc<EmbeddedFont>> =
            self.embedded.get(base_font).cloned().unwrap_or_default();
        out.extend(self.substitute(base_font));
        out
    }

    fn substitute(&self, base_font: &str) -> Option<Arc<EmbeddedFont>> {
        if !self.substitution {
            return None;
        }
        if let Ok(cache) = self.substitutes.lock()
            && let Some(hit) = cache.get(base_font)
        {
            return hit.clone();
        }
        let found = font::substitute::load(base_font).map(Arc::new);
        if let Ok(mut cache) = self.substitutes.lock() {
            cache.insert(base_font.to_string(), found.clone());
        }
        found
    }

    /// Whether the face used for a name is a stand-in rather than the
    /// document's own.
    ///
    /// It matters for placement: an embedded face's glyphs were drawn for the
    /// advances the document records, so each glyph belongs exactly where the
    /// document puts it. A stand-in's were not, and placing its glyphs at
    /// another font's advances scatters letters inside words.
    pub fn is_substituted(&self, base_font: &str) -> bool {
        !self.embedded.contains_key(base_font) && self.face(base_font).is_some()
    }

    /// The advance the face itself gives a code, in text-space ems.
    pub fn own_advance(&self, base_font: &str, code: u32) -> Option<f64> {
        let (glyph, matrix) = self.resolve(base_font, code, char::from_u32(code))?;
        Some(glyph.advance as f64 * matrix[0])
    }

    /// The em-space scale for a font's outlines: its `/FontMatrix` reduces
    /// font units to text space, and is very nearly always 1/1000.
    pub fn font_matrix(&self, base_font: &str) -> Option<[f64; 6]> {
        Some(self.face(base_font)?.font_matrix())
    }

    /// The outline a character code selects in a named font.
    pub fn glyph(&self, base_font: &str, code: u32) -> Option<Arc<Glyph>> {
        self.glyph_for(base_font, code, None)
    }

    /// The outline a code selects, given the character the document decoded it
    /// to. A TrueType `cmap` is keyed by Unicode, so the character is what
    /// finds the curly quotes and dashes WinAnsi hides in its control range.
    pub fn glyph_for(
        &self,
        base_font: &str,
        code: u32,
        unicode: Option<char>,
    ) -> Option<Arc<Glyph>> {
        self.resolve(base_font, code, unicode).map(|found| found.0)
    }

    /// The glyph a code selects together with the matrix of the face it came
    /// from.
    ///
    /// The two travel together on purpose: with several subsets under one
    /// name, scaling a glyph by another subset's matrix would draw it at the
    /// wrong size.
    pub fn resolve(
        &self,
        base_font: &str,
        code: u32,
        unicode: Option<char>,
    ) -> Option<(Arc<Glyph>, [f64; 6])> {
        let key = (base_font.to_string(), code, unicode);
        if let Ok(cache) = self.cache.lock()
            && let Some(hit) = cache.get(&key)
        {
            return hit.clone();
        }
        // Try every subset under the name: each carries only what its own
        // chunk of the document needed.
        let found = self.faces(base_font).into_iter().find_map(|face| {
            face.outline_for(code, unicode)
                .filter(|glyph| !glyph.contours.is_empty())
                .map(|glyph| (Arc::new(glyph), face.font_matrix()))
        });
        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(key, found.clone());
        }
        found
    }


    /// How many mask pixels to rasterise per painter unit.
    ///
    /// Set by the host from its device pixel ratio. Glyphs are placed at the
    /// same size either way; only the detail in the mask changes.
    pub fn set_raster_scale(&self, scale: f64) {
        let clamped = scale.clamp(1.0, 4.0);
        self.raster_scale.store(
            (clamped * 100.0).round() as u32,
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    /// The current mask resolution, in pixels per painter unit.
    pub fn raster_scale(&self) -> f64 {
        self.raster_scale.load(std::sync::atomic::Ordering::Relaxed) as f64 / 100.0
    }

    /// A font set with system stand-ins, for tests that need something to
    /// rasterise. `None` where the machine has no usable face.
    #[cfg(test)]
    fn for_tests() -> Option<FontSet> {
        let fonts = FontSet::load(b"");
        fonts.face("Times-Roman")?;
        Some(fonts)
    }

    /// How much the rasterised glyphs currently weigh, in bytes.
    ///
    /// The masks are the only part of this that grows with use rather than
    /// with the document, so it is the number worth watching.
    pub fn raster_bytes(&self) -> usize {
        self.raster_bytes.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// How many rasterised glyphs are held.
    pub fn raster_count(&self) -> usize {
        self.rasters.lock().map(|cache| cache.len()).unwrap_or(0)
    }

    /// A rasterised glyph, cached.
    ///
    /// Size is quantised to a quarter pixel so that a zoom animation reuses
    /// masks instead of rasterising a fresh set every frame.
    #[allow(clippy::too_many_arguments)]
    pub fn raster(
        &self,
        base_font: &str,
        code: u32,
        unicode: Option<char>,
        size: f64,
        rot: f64,
        color: [u8; 3],
    ) -> Option<Arc<RasterGlyph>> {
        // Quantised after scaling, so masks at one ratio never stand in for
        // another's.
        let quantised = (size * self.raster_scale() * 4.0).round().max(0.0) as u32;
        // Angle quantised to a degree: a run shares one angle, so this costs a
        // cache entry per angle actually used rather than per glyph.
        let angle = (rot.to_degrees().round() as i32).rem_euclid(360);
        let key = (
            base_font.to_string(),
            code,
            unicode,
            quantised,
            angle,
            color,
        );
        if let Ok(cache) = self.rasters.lock()
            && let Some(hit) = cache.get(&key)
        {
            return hit.clone();
        }
        let raster = self
            .resolve(base_font, code, unicode)
            .and_then(|(glyph, matrix)| {
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
            let weight = raster.as_ref().map(|g| g.pixels.len()).unwrap_or(0);
            let held = self
                .raster_bytes
                .fetch_add(weight, std::sync::atomic::Ordering::Relaxed)
                + weight;
            // Bounded, because this is the one cache that grows with use
            // rather than with the document: every zoom step asks for a new
            // size of every letter, and a page zoomed through twenty steps
            // holds sixteen megabytes of masks that will never be asked for
            // again. Clearing wholesale suits the way they go stale -- a zoom
            // retires every size at once -- and costs the one frame that
            // rasterises what is on screen now.
            if held > MAX_RASTER_BYTES {
                cache.clear();
                self.raster_bytes
                    .store(weight, std::sync::atomic::Ordering::Relaxed);
            }
            cache.insert(key, raster.clone());
        }
        raster
    }
}

#[cfg(test)]
mod bounded {
    use super::*;

    /// The raster cache is the one that grows with use rather than with the
    /// document: every zoom step asks for a new size of every letter. Left
    /// unbounded it held sixteen megabytes after twenty zoom steps of a single
    /// page, and would have kept going for as long as someone kept reading.
    #[test]
    fn rasterised_glyphs_are_kept_to_a_budget() {
        let Some(fonts) = FontSet::for_tests() else {
            eprintln!("skipping: no face to rasterise with");
            return;
        };
        // Ask for many sizes of a few letters, as zooming does.
        // Big glyphs, so the budget is reached in a second rather than a
        // minute: one letter at 600 points is a third of a megabyte.
        let mut asked = 0usize;
        for step in 0..240 {
            let size = 400.0 + (step % 60) as f64 * 4.0;
            for code in *b"MW" {
                let _ = fonts.raster(
                    "Times-Roman",
                    code as u32,
                    Some(code as char),
                    size,
                    0.0,
                    [0, 0, 0],
                );
                asked += 1;
            }
            if fonts.raster_bytes() > MAX_RASTER_BYTES {
                panic!(
                    "cache reached {} bytes after {asked} glyphs",
                    fonts.raster_bytes()
                );
            }
        }
        assert!(asked > 400, "the sweep should have asked for plenty");
    }
}

impl std::fmt::Debug for FontSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FontSet")
            .field("embedded", &self.embedded.len())
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
    fn a_document_without_fonts_embeds_nothing() {
        let set = FontSet::load(b"not a pdf");
        assert!(set.is_empty(), "nothing was embedded");
    }

    /// Without substitution there is no face at all, which is the browser's
    /// situation and the reason the fit-to-width fallback exists.
    #[test]
    fn substitution_can_be_turned_off() {
        let set = FontSet::without_substitution(b"not a pdf");
        assert!(set.glyph("Times-Roman", b'A' as u32).is_none());
        assert!(set.font_matrix("Times-Roman").is_none());
    }

    /// A font the document names but does not embed is stood in for, which is
    /// what Okular does and what the standard fourteen require.
    #[test]
    fn a_named_but_unembedded_font_is_stood_in_for() {
        let set = FontSet::load(b"not a pdf");
        match set.glyph("Times-Roman", b'A' as u32) {
            Some(glyph) => assert!(!glyph.contours.is_empty()),
            None => eprintln!("skipping: no system face to substitute"),
        }
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
        // A name the document never embedded is stood in for rather than
        // refused, so the miss to check is a code the face has no glyph for.
        assert!(
            FontSet::without_substitution(&pdf)
                .glyph("no such font", 65)
                .is_none()
        );
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
                None,
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
                None,
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
            None,
            12.0,
            0.0,
            [0, 0, 0],
        );
        let second = set.raster(
            "MGMJCW+NimbusRomNo9L-Regu",
            b'e' as u32,
            None,
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
                None,
                40.0,
                0.0,
                [0, 0, 0],
            )
            .expect("L");
        let turned = set
            .raster(
                "MGMJCW+NimbusRomNo9L-Regu",
                b'L' as u32,
                None,
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
