// SPDX-License-Identifier: AGPL-3.0-or-later
//! Painting a document page through Dewey's [`Painter`].
//!
//! This is the whole bridge between the two halves of the system, and it is
//! deliberately small: the engine already flattens curves, resolves clips and
//! places glyphs, so a page arrives as a flat list of fills, strokes, text runs
//! and images. Nothing here knows what a PDF is.
//!
//! Two coordinate systems meet here. Documents are y-up with the origin at the
//! bottom left; screens are y-down from the top left. Every op is flipped
//! exactly once, in [`Transform::point`], rather than at each use — getting
//! this wrong in one arm and not another produces a page that is subtly,
//! maddeningly upside down in places.

use agenticpdf::engine::{DisplayList, RenderOp};
use dewey::core::{Color, Position, Rect};
use dewey::paint::{ImageData, Painter};

/// Maps document points onto screen pixels.
#[derive(Debug, Clone, Copy)]
pub struct Transform {
    /// Top-left of the page on screen.
    pub origin: Position,
    /// Pixels per document point.
    pub scale: f32,
    /// Page height in document points, for the y flip.
    pub page_height: f32,
}

impl Transform {
    /// Fit a page of `list`'s size inside `area`, centred, preserving aspect.
    pub fn fit(list: &DisplayList, area: Rect, zoom: f32) -> Transform {
        let (width, height) = (list.width.max(1.0) as f32, list.height.max(1.0) as f32);
        let scale = (area.width / width).min(area.height / height).max(0.01) * zoom;

        Transform {
            origin: Position::new(
                area.x + (area.width - width * scale) / 2.0,
                area.y + (area.height - height * scale) / 2.0,
            ),
            scale,
            page_height: height,
        }
    }

    /// Document point to screen position, flipping the y axis.
    pub fn point(&self, x: f64, y: f64) -> Position {
        Position::new(
            self.origin.x + x as f32 * self.scale,
            self.origin.y + (self.page_height - y as f32) * self.scale,
        )
    }

    /// The page's own rectangle on screen.
    pub fn page_rect(&self, list: &DisplayList) -> Rect {
        Rect::new(
            self.origin.x,
            self.origin.y,
            list.width as f32 * self.scale,
            list.height as f32 * self.scale,
        )
    }
}

/// The letters a ligature stands for, for a face that lacks the ligature
/// itself. Only the Latin f-ligatures, which are the ones documents actually
/// use and the ones system faces commonly omit.
fn decompose(code: u32) -> Vec<u32> {
    match char::from_u32(code) {
        Some('\u{FB00}') => vec!['f' as u32, 'f' as u32],
        Some('\u{FB01}') => vec!['f' as u32, 'i' as u32],
        Some('\u{FB02}') => vec!['f' as u32, 'l' as u32],
        Some('\u{FB03}') => vec!['f' as u32, 'f' as u32, 'i' as u32],
        Some('\u{FB04}') => vec!['f' as u32, 'f' as u32, 'l' as u32],
        _ => vec![code],
    }
}

/// Fill the document's own glyphs for one text run.
///
/// Returns false when the run cannot be drawn this way -- no embedded font
/// under that name, or codes that do not line up with the text -- so the
/// caller can fall back rather than leave a blank line.
///
/// Placement is the document's, not the font's: each glyph sits at the
/// cumulative advance the PDF recorded, so a run occupies exactly the space it
/// reserved and cannot run into the next one. That is the whole difference
/// between this and laying text out with a substitute face.
#[allow(clippy::too_many_arguments)]
fn draw_embedded_glyphs(
    painter: &mut dyn Painter,
    fonts: &crate::glyphs::FontSet,
    transform: &Transform,
    text: &str,
    x: f64,
    y: f64,
    size: f64,
    advances: &[f64],
    codes: &[u32],
    rot: f64,
    style: &dewey::core::TextStyle,
    font: &str,
) -> bool {
    // A font we have no glyphs for is not one we can draw this way.
    if fonts.font_matrix(font).is_none() {
        return false;
    }
    let chars: Vec<char> = text.chars().collect();
    // Codes must line up with the text, because a glyph is chosen by code.
    if advances.len() != chars.len() || codes.len() != chars.len() {
        return false;
    }

    // A stand-in face was not drawn for this document's advances. Placing its
    // glyphs at them scatters letters inside words -- Times set to Computer
    // Modern's metrics reads as "Communi cat i on". So a stand-in is spaced by
    // its own advances, and the whole run is then scaled to the width the
    // document reserved, which keeps runs in their slots without disturbing
    // the spacing within a word.
    let substituted = fonts.is_substituted(font);

    // A stand-in is addressed by the decoded character rather than the
    // document's code. The two agree for ordinary text and part company
    // exactly where it matters: a Computer Modern document puts its "fi"
    // ligature at code 2, which Times has nothing at, so the ligature
    // vanished and "intensified" rendered as "intensied". The engine has
    // already decoded the character; use it.
    let lookup: Vec<u32> = match substituted {
        true => chars.iter().map(|glyph| *glyph as u32).collect(),
        false => codes.to_vec(),
    };

    // A stand-in may not have every character. Times New Roman, for one, has
    // no "fi" ligature, so a document that uses one lost it and "intensified"
    // rendered as "intensied". Drawing the letters the ligature stands for is
    // closer to the document than dropping them.
    let expanded: Vec<Vec<u32>> = match substituted {
        false => lookup.iter().map(|code| vec![*code]).collect(),
        true => lookup
            .iter()
            .map(|code| match fonts.own_advance(font, *code).is_some() {
                true => vec![*code],
                false => decompose(*code),
            })
            .collect(),
    };

    let mut own: Vec<f64> = Vec::new();
    let mut fit = 1.0f64;
    if substituted {
        own = expanded
            .iter()
            .map(|codes| {
                codes
                    .iter()
                    .map(|code| fonts.own_advance(font, *code).unwrap_or(0.0) * size)
                    .sum()
            })
            .collect();
        let natural: f64 = own.iter().sum();
        let target: f64 = advances.iter().sum();
        if natural > 0.0 && target > 0.0 {
            // Bounded: a run whose recorded width bears no relation to this
            // face should be left legible rather than squeezed to nothing.
            fit = (target / natural).clamp(0.5, 2.0);
        }
    }

    // The size the run is *drawn* at. `size` is in page units, and every
    // position here goes through the transform on its way to the surface, so a
    // mask rasterised at the unscaled size is too big for its own advances
    // whenever the page is not shown at 1:1 -- letters overlap on a page
    // zoomed out, and stand apart on one zoomed in.
    let device_size = size * transform.scale as f64;

    let (cos, sin) = (rot.cos(), rot.sin());
    let ink = [
        (style.color.r * 255.0).clamp(0.0, 255.0) as u8,
        (style.color.g * 255.0).clamp(0.0, 255.0) as u8,
        (style.color.b * 255.0).clamp(0.0, 255.0) as u8,
    ];
    let mut pen = 0.0f64;
    let mut drew = false;
    // Set where a character is deliberately not drawn rather than failed on.
    let mut handled = false;

    for (index, glyph_char) in chars.iter().enumerate() {
        // A cluster's continuation characters carry no advance and no glyph of
        // their own: the code that opened the cluster drew it.
        let is_continuation = index > 0 && advances[index] == 0.0;
        // A placeholder marks a code the document never named. The face the
        // document embedded still has the glyph, and the code finds it; a
        // stand-in face does not, and drawing its idea of U+FFFD would put a
        // black diamond where a ligature belongs.
        let unnamed = *glyph_char == char::REPLACEMENT_CHARACTER;
        if unnamed && substituted {
            // Nothing to draw, but nothing missing either: the run is handled,
            // and handing it to the painter's own font would set a run of
            // black diamonds where the document has letters we cannot name.
            handled = true;
        } else if !glyph_char.is_whitespace() && !is_continuation {
            // The document's code, not the Unicode it decodes to. A subset
            // font remaps freely: drawing by Unicode picks a neighbouring
            // glyph, which is how "LLMs" came out as "hhMs".
            let mut sub_pen = pen;
            for code in &expanded[index] {
                if let Some(raster) =
                    fonts.raster(font, *code, Some(*glyph_char), device_size, rot, ink)
                {
                    // The mask is placed by its own offset from the glyph origin,
                    // which is where the outline actually sits -- a letter with a
                    // descender starts above the baseline and drops below it.
                    let origin_x = x + sub_pen * cos;
                    let origin_y = y + sub_pen * sin;
                    let at = transform.point(origin_x, origin_y);
                    // The mask may hold several pixels per painter unit; it is
                    // still placed at the size the text is, so the extra
                    // pixels become detail rather than a bigger letter.
                    let rs = fonts.raster_scale() as f32;
                    let rect = Rect::new(
                        at.x + raster.left / rs,
                        at.y + raster.top / rs,
                        raster.width as f32 / rs,
                        raster.height as f32 / rs,
                    );
                    painter.draw_image(
                        rect,
                        &ImageData::new(raster.width, raster.height, &raster.pixels),
                    );
                    drew = true;
                }
                // Within an expansion the pen moves by each component's own
                // advance, so "fi" occupies the space f and i do.
                sub_pen += match substituted {
                    true => fonts.own_advance(font, *code).unwrap_or(0.0) * size * fit,
                    false => 0.0,
                };
            }
        }
        pen += match substituted {
            true => own.get(index).copied().unwrap_or(0.0) * fit,
            false => advances[index],
        };
    }
    drew || handled
}

/// A decoded image the canvas can draw, keyed by the resource name an
/// [`RenderOp::Image`] refers to.
pub struct Texture {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Paint one page.
///
/// `textures` resolves `RenderOp::Image` names. An image with no matching
/// texture is drawn as an outline rather than skipped: a missing figure should
/// leave a visible hole, because silently omitting content from a document a
/// person is reading is the worst available outcome.
pub fn paint_page(
    painter: &mut dyn Painter,
    list: &DisplayList,
    transform: Transform,
    textures: &[Texture],
    fonts: &crate::glyphs::FontSet,
) {
    let page = transform.page_rect(list);
    painter.fill_rect(page, Color::WHITE, 0.0);
    painter.push_clip(page);

    // The engine emits Save/Restore/Clip around nested state. A clip belongs to
    // the graphics state that narrowed it: `q` remembers the clip in force, `W
    // n` narrows it, and `Q` puts back what `q` remembered -- however many
    // clips were added in between, including none.
    //
    // Popping one clip per Restore instead, as this did, goes wrong in both
    // directions at once. A Restore with no clip of its own releases an
    // enclosing one early, and artwork the document clipped away is drawn on
    // top of the page; a state that clipped twice releases only one, and
    // artwork that should be visible is cut. One assembly manual showed both
    // in the same picture: safety labels stacked down the middle of a parts
    // diagram, each missing its first letter.
    //
    // The saved levels are a stack rather than a count so an unbalanced
    // Restore -- which a damaged file can produce -- still cannot pop the page
    // clip and let drawing escape onto the rest of the UI.
    let mut clip_depth = 0usize;
    let mut saved_clips: Vec<usize> = Vec::new();

    for op in &list.ops {
        match op {
            RenderOp::Fill {
                subpaths, color, ..
            } => {
                let paint = to_color(*color);
                for subpath in subpaths {
                    let points = to_points(subpath, &transform);
                    if points.len() >= 3 {
                        painter.fill_path(&points, paint);
                    }
                }
            }
            RenderOp::Stroke {
                subpaths,
                color,
                width,
            } => {
                let paint = to_color(*color);
                // A hairline is width 0 in the document and must stay visible
                // however far the page is zoomed out.
                let stroke = (*width as f32 * transform.scale).max(0.75);
                for subpath in subpaths {
                    let points = to_points(subpath, &transform);
                    if points.len() >= 2 {
                        painter.stroke_path(&points, paint, stroke);
                    }
                }
            }
            RenderOp::Text {
                text,
                x,
                y,
                size,
                width,
                advances,
                codes,
                measured,
                rot,
                color,
                font,
            } => {
                if text.trim().is_empty() {
                    continue;
                }
                let mut style = dewey::core::TextStyle {
                    font_size: (*size as f32 * transform.scale).max(1.0),
                    color: to_color(*color),
                    ..Default::default()
                };

                // Draw the document's own glyphs where they exist. Filling
                // the outlines the file carries is what PDF.js and Okular do,
                // and it is the only way the page matches: a substituted font
                // has different advances, so runs either collide or have to be
                // squeezed to fit.
                if draw_embedded_glyphs(
                    painter, fonts, &transform, text, *x, *y, *size, advances, codes, *rot, &style,
                    font,
                ) {
                    continue;
                }

                // No embedded font -- a non-PDF format laid out by the
                // typesetter, or a PDF that references fonts without embedding
                // them. Lay the run out with the painter's own font and fit it
                // to the width the document reserved, so at least runs keep
                // their slots and do not overlap.
                let target = *width as f32 * transform.scale;
                let natural = painter.measure_text(text, &style).width;
                if *rot == 0.0 {
                    if target > 0.0 && natural > target {
                        // Floored: a run whose slot is very narrow for this
                        // font should be cramped rather than unreadable.
                        let fit = (target / natural).max(0.6);
                        style.font_size = (style.font_size * fit).max(1.0);
                    }
                    let at = transform.point(*x, *y);
                    painter.text(Position::new(at.x, at.y - style.font_size), text, &style);
                    continue;
                }

                // Rotated text with no glyphs to fill: walk the baseline so an
                // arXiv stamp meant for the margin is not drawn across the body.
                let chars: Vec<char> = text.chars().collect();
                let from_pdf = !*measured && advances.len() == chars.len();
                let (dx, dy) = (rot.cos(), rot.sin());
                let mut pen = 0.0f64;
                let mut buf = [0u8; 4];
                for (index, glyph) in chars.iter().enumerate() {
                    let encoded = glyph.encode_utf8(&mut buf);
                    if !glyph.is_whitespace() {
                        let at = transform.point(x + pen * dx, y + pen * dy);
                        painter.text(Position::new(at.x, at.y - style.font_size), encoded, &style);
                    }
                    pen += match from_pdf {
                        true => advances[index],
                        false => {
                            let measured = painter.measure_text(encoded, &style).width as f64;
                            measured / transform.scale.max(f32::EPSILON) as f64
                        }
                    };
                }
            }
            RenderOp::Image { x, y, w, h, name } => {
                // y + h because the op's origin is the image's bottom-left and
                // the screen rect is anchored at its top-left.
                let top_left = transform.point(*x, *y + *h);
                let rect = Rect::new(
                    top_left.x,
                    top_left.y,
                    *w as f32 * transform.scale,
                    *h as f32 * transform.scale,
                );
                match textures.iter().find(|texture| texture.name == *name) {
                    Some(texture) => painter.draw_image(
                        rect,
                        &ImageData::new(texture.width, texture.height, &texture.rgba),
                    ),
                    None => painter.stroke_rect(rect, Color::GRAY, 1.0, 0.0),
                }
            }
            RenderOp::Save => saved_clips.push(clip_depth),
            RenderOp::Restore => {
                // Back to the clip this state was entered with. An unmatched
                // Restore goes back to the page clip and no further.
                // Back to the clip this state was entered with. An unmatched
                // Restore goes back to the page clip and no further.
                let level = saved_clips.pop().unwrap_or(0);
                while clip_depth > level {
                    clip_depth -= 1;
                    painter.pop_clip();
                }
            }
            RenderOp::Clip { rect, .. } => {
                let top_left = transform.point(rect[0], rect[3]);
                let bottom_right = transform.point(rect[2], rect[1]);
                painter.push_clip(Rect::new(
                    top_left.x,
                    top_left.y,
                    (bottom_right.x - top_left.x).abs(),
                    (bottom_right.y - top_left.y).abs(),
                ));
                clip_depth += 1;
            }
        }
    }

    // Balance whatever the list left open, then the page clip itself.
    for _ in 0..clip_depth {
        painter.pop_clip();
    }
    painter.pop_clip();
}

/// A [`Painter`] that records what it was asked to draw instead of drawing it.
///
/// The browser has no `Painter` of ours to hand pixels to — the drawing surface
/// lives in JavaScript. So the wasm build paints through this, ships the
/// recording across as JSON, and a few lines of canvas code replay it. The
/// point is that [`paint_page`] is unchanged: mobile and desktop run the *same*
/// page-painting code, and only the last step differs.
///
/// It is deliberately native-compatible so it can be tested without a browser.
#[derive(Debug)]
pub struct RecordingPainter {
    ops: Vec<String>,
    clip_depth: usize,
    /// Device pixels per painter unit on the host, so an image is sampled
    /// down to the resolution the screen can actually show.
    raster_scale: f32,
    /// Content keys already sent in this recording, so a repeated glyph costs
    /// a key rather than its pixels again.
    images: std::collections::HashSet<String>,
}

impl Default for RecordingPainter {
    fn default() -> RecordingPainter {
        RecordingPainter {
            ops: Vec::new(),
            clip_depth: 0,
            // One device pixel per painter unit until the host says otherwise.
            raster_scale: 1.0,
            images: std::collections::HashSet::new(),
        }
    }
}

/// The largest image sent inline, after sampling to its drawn size.
///
/// Generous, because the sampling above bounds an image by where it is drawn
/// and the page is bounded by the window: a full-page picture on a large
/// screen is a few megabytes once, and a key thereafter. Anything past this is
/// left to the host, which draws its frame.
const MAX_INLINE_IMAGE_BYTES: usize = 6 * 1024 * 1024;

/// Sample an image down to the size it is drawn at, if that is meaningfully
/// smaller than the image itself.
///
/// A box filter over the source pixels covered by each destination pixel:
/// slower than picking one of them and far better looking, which matters
/// because the alternative -- letting the host scale it -- is what we are
/// avoiding by not sending it at full size in the first place.
pub fn sample_rgba(
    src: &[u8],
    src_w: u32,
    src_h: u32,
    want_w: u32,
    want_h: u32,
) -> Option<Vec<u8>> {
    if want_w == 0 || want_h == 0 || src_w == 0 || src_h == 0 {
        return None;
    }
    if src.len() < src_w as usize * src_h as usize * 4 {
        return None;
    }
    let mut out = vec![0u8; want_w as usize * want_h as usize * 4];
    for y in 0..want_h {
        let y0 = (y as usize * src_h as usize) / want_h as usize;
        let y1 = (((y + 1) as usize * src_h as usize) / want_h as usize).max(y0 + 1);
        for x in 0..want_w {
            let x0 = (x as usize * src_w as usize) / want_w as usize;
            let x1 = (((x + 1) as usize * src_w as usize) / want_w as usize).max(x0 + 1);
            let mut sum = [0u32; 4];
            let mut n = 0u32;
            for sy in y0..y1.min(src_h as usize) {
                let row = sy * src_w as usize;
                for sx in x0..x1.min(src_w as usize) {
                    let at = (row + sx) * 4;
                    for (channel, total) in sum.iter_mut().enumerate() {
                        *total += src[at + channel] as u32;
                    }
                    n += 1;
                }
            }
            if n == 0 {
                continue;
            }
            let at = (y as usize * want_w as usize + x as usize) * 4;
            for (channel, total) in sum.iter().enumerate() {
                out[at + channel] = (total / n) as u8;
            }
        }
    }
    Some(out)
}

fn fit_to_draw(
    image: &ImageData<'_>,
    rect: Rect,
    raster_scale: f32,
) -> Option<(u32, u32, Vec<u8>)> {
    let want_w = (rect.width.abs() * raster_scale).ceil().max(1.0) as u32;
    let want_h = (rect.height.abs() * raster_scale).ceil().max(1.0) as u32;
    let (src_w, src_h) = (image.width, image.height);
    // Never an enlargement, and not for a saving too small to pay for the
    // work. The threshold was four times the pixels, which sounds harmless
    // and is not: a full-page scan drawn at nine hundred pixels wide is only
    // 3.6 times its drawn size, so it was sent whole, blew the inline bound,
    // and the host drew an empty frame where the page should be. Anything
    // past a half again is worth sampling -- it is the difference between
    // fifteen megabytes and four.
    if src_w <= want_w
        || src_h <= want_h
        || (src_w as u64 * src_h as u64) * 2 <= (want_w as u64 * want_h as u64) * 3
    {
        return None;
    }
    let out = sample_rgba(image.pixels, src_w, src_h, want_w, want_h)?;
    Some((want_w, want_h, out))
}

/// A short content hash, so identical pixels are sent once.
/// A short key standing for these pixels, so the host decodes them once.
///
/// A glyph mask is a few hundred bytes and is hashed whole. A page image is
/// megabytes, and hashing it whole costs about twenty milliseconds -- on every
/// frame, since a recording is rebuilt each time and the key is how the
/// painter knows the host already has the picture. That is a fifth of a
/// scroll's budget spent proving something we already knew.
///
/// So a large buffer is hashed by its length together with its head, its tail
/// and a stride through the middle. This is cache identity, not integrity:
/// the cost of a collision is a stale picture, not a wrong document, and the
/// same length with matching head, tail and sample is not something two
/// different renderings of a page produce by accident.
fn content_key(pixels: &[u8]) -> String {
    const WHOLE: usize = 64 * 1024;
    if pixels.len() <= WHOLE {
        let digest = agenticpdf::adf::sha256::sha256(pixels);
        return agenticpdf::adf::sha256::hex(&digest[..8]);
    }
    let mut probe: Vec<u8> = Vec::with_capacity(48 * 1024);
    probe.extend_from_slice(&(pixels.len() as u64).to_le_bytes());
    probe.extend_from_slice(&pixels[..8 * 1024]);
    probe.extend_from_slice(&pixels[pixels.len() - 8 * 1024..]);
    // An odd stride so a run of identical rows cannot align with it.
    let stride = (pixels.len() / 16_384).max(1) | 1;
    let mut at = 0usize;
    while at < pixels.len() {
        probe.push(pixels[at]);
        at += stride;
    }
    let digest = agenticpdf::adf::sha256::sha256(&probe);
    agenticpdf::adf::sha256::hex(&digest[..8])
}

impl RecordingPainter {
    pub fn new() -> RecordingPainter {
        RecordingPainter::default()
    }

    /// A recording that assumes the host already holds these masks.
    ///
    /// A fresh recording is built for every frame, so without this the same
    /// few hundred masks are inlined again on every scroll and zoom -- a third
    /// of a megabyte per redraw for a page the host has already decoded and
    /// cached by key.
    pub fn with_known_images(known: std::collections::HashSet<String>) -> RecordingPainter {
        RecordingPainter {
            images: known,
            ..RecordingPainter::default()
        }
    }

    /// How many device pixels the host paints per painter unit.
    ///
    /// Images are sampled to that resolution rather than sent whole; a host
    /// that never says defaults to one, which is the safe assumption.
    pub fn with_raster_scale(mut self, scale: f32) -> RecordingPainter {
        self.raster_scale = scale.clamp(0.25, 4.0);
        self
    }

    /// The masks the host holds after this recording.
    pub fn image_keys(self) -> std::collections::HashSet<String> {
        self.images
    }

    /// The recording, as a JSON array.
    pub fn to_json(&self) -> String {
        format!("[{}]", self.ops.join(","))
    }

    pub fn len(&self) -> usize {
        self.ops.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// Clip pushes still outstanding — a balance check for the replayer.
    pub fn open_clips(&self) -> usize {
        self.clip_depth
    }

    fn push(&mut self, op: String) {
        self.ops.push(op);
    }
}

fn css_color(color: Color) -> String {
    format!(
        "rgba({},{},{},{})",
        (color.r * 255.0) as u8,
        (color.g * 255.0) as u8,
        (color.b * 255.0) as u8,
        color.a
    )
}

fn points_json(points: &[Position]) -> String {
    let pairs: Vec<String> = points
        .iter()
        .map(|point| format!("[{:.2},{:.2}]", point.x, point.y))
        .collect();
    format!("[{}]", pairs.join(","))
}

/// JSON-escape a string. Only the characters JSON actually forbids.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // Control characters must be escaped or the JSON is invalid, and a
            // document is exactly the kind of input that contains them.
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

impl Painter for RecordingPainter {
    fn fill_rect(&mut self, rect: Rect, color: Color, corner_radius: f32) {
        self.push(format!(
            r#"{{"op":"fill_rect","x":{:.2},"y":{:.2},"w":{:.2},"h":{:.2},"color":"{}","r":{:.2}}}"#,
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            css_color(color),
            corner_radius
        ));
    }

    fn stroke_rect(&mut self, rect: Rect, color: Color, width: f32, _corner_radius: f32) {
        self.push(format!(
            r#"{{"op":"stroke_rect","x":{:.2},"y":{:.2},"w":{:.2},"h":{:.2},"color":"{}","width":{:.2}}}"#,
            rect.x, rect.y, rect.width, rect.height, css_color(color), width
        ));
    }

    fn fill_circle(&mut self, center: Position, radius: f32, color: Color) {
        self.push(format!(
            r#"{{"op":"fill_circle","x":{:.2},"y":{:.2},"r":{:.2},"color":"{}"}}"#,
            center.x,
            center.y,
            radius,
            css_color(color)
        ));
    }

    fn stroke_circle(&mut self, center: Position, radius: f32, color: Color, width: f32) {
        self.push(format!(
            r#"{{"op":"stroke_circle","x":{:.2},"y":{:.2},"r":{:.2},"color":"{}","width":{:.2}}}"#,
            center.x,
            center.y,
            radius,
            css_color(color),
            width
        ));
    }

    fn line(&mut self, from: Position, to: Position, color: Color, width: f32) {
        self.push(format!(
            r#"{{"op":"line","x1":{:.2},"y1":{:.2},"x2":{:.2},"y2":{:.2},"color":"{}","width":{:.2}}}"#,
            from.x, from.y, to.x, to.y, css_color(color), width
        ));
    }

    fn text(&mut self, pos: Position, text: &str, style: &dewey::core::TextStyle) {
        self.push(format!(
            r#"{{"op":"text","x":{:.2},"y":{:.2},"size":{:.2},"color":"{}","text":"{}"}}"#,
            pos.x,
            pos.y,
            style.font_size,
            css_color(style.color),
            escape(text)
        ));
    }

    fn measure_text(&self, text: &str, style: &dewey::core::TextStyle) -> dewey::core::Size {
        // An estimate, because the real measurement lives in the browser.
        // `paint_page` fits each run to the width the document reserved, so
        // this feeds that decision: a recording made here will differ slightly
        // from what the browser lays out, which is the price of deciding the
        // fit on this side rather than shipping both widths.
        dewey::core::Size::new(
            text.chars().count() as f32 * style.font_size * 0.5,
            style.font_size,
        )
    }

    fn push_clip(&mut self, rect: Rect) {
        self.clip_depth += 1;
        self.push(format!(
            r#"{{"op":"push_clip","x":{:.2},"y":{:.2},"w":{:.2},"h":{:.2}}}"#,
            rect.x, rect.y, rect.width, rect.height
        ));
    }

    fn pop_clip(&mut self) {
        self.clip_depth = self.clip_depth.saturating_sub(1);
        self.push(r#"{"op":"pop_clip"}"#.to_string());
    }

    fn fill_path(&mut self, points: &[Position], color: Color) {
        self.push(format!(
            r#"{{"op":"fill_path","points":{},"color":"{}"}}"#,
            points_json(points),
            css_color(color)
        ));
    }

    fn stroke_path(&mut self, points: &[Position], color: Color, width: f32) {
        self.push(format!(
            r#"{{"op":"stroke_path","points":{},"color":"{}","width":{:.2}}}"#,
            points_json(points),
            css_color(color),
            width
        ));
    }

    fn draw_image(&mut self, rect: Rect, image: &ImageData<'_>) {
        // Glyph masks are inlined; scanned pages are not.
        //
        // Every glyph on the page arrives here, so a host that is only given
        // geometry draws text as a field of empty rectangles -- which is what
        // the browser did once glyphs became masks. They are small and they
        // repeat: one page uses a hundred or so distinct glyph-and-size
        // combinations across thousands of instances, so each is sent once
        // under a content key and referenced after that.
        //
        // A large image is still left to the host. A scanned page inlined per
        // frame would be megabytes of JSON, and the host already resolves
        // those by index.
        // A photograph is sent at the size it is drawn, not the size it was
        // scanned. A 2700-pixel-wide picture placed in a 400-pixel column
        // costs forty times its useful weight as base64, and the host would
        // throw the difference away on the first draw. Sampling down here is
        // what makes page images affordable over this transport at all.
        let scaled = fit_to_draw(image, rect, self.raster_scale);
        let (width, height, pixels_ref) = match &scaled {
            Some((w, h, bytes)) => (*w, *h, bytes.as_slice()),
            None => (image.width, image.height, image.pixels),
        };

        let inline = pixels_ref.len() <= MAX_INLINE_IMAGE_BYTES;
        let key = match inline {
            true => Some(content_key(pixels_ref)),
            false => None,
        };
        let pixels = match &key {
            // Sent once. A repeat carries the key alone.
            Some(key) if self.images.insert(key.clone()) => {
                format!(r#","pixels":"{}""#, agenticpdf::engine::b64e(pixels_ref))
            }
            _ => String::new(),
        };
        let key = match &key {
            Some(key) => format!(r#","key":"{key}""#),
            None => String::new(),
        };
        self.push(format!(
            r#"{{"op":"image","x":{:.2},"y":{:.2},"w":{:.2},"h":{:.2},"iw":{},"ih":{}{key}{pixels}}}"#,
            rect.x, rect.y, rect.width, rect.height, width, height
        ));
    }
}

fn to_points(subpath: &[[f64; 2]], transform: &Transform) -> Vec<Position> {
    subpath
        .iter()
        .map(|point| transform.point(point[0], point[1]))
        .collect()
}

fn to_color(color: [f64; 4]) -> Color {
    Color::rgba(
        color[0] as f32,
        color[1] as f32,
        color[2] as f32,
        color[3] as f32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use dewey::backend::image_buffer::ImagePainter;

    fn list(ops: Vec<RenderOp>) -> DisplayList {
        DisplayList {
            page_number: 1,
            width: 100.0,
            height: 100.0,
            ops,
        }
    }

    fn area() -> Rect {
        Rect::new(0.0, 0.0, 100.0, 100.0)
    }

    #[test]
    fn the_y_axis_is_flipped_exactly_once() {
        let transform = Transform::fit(&list(Vec::new()), area(), 1.0);
        // A point at the document's top (y = height) lands at the screen's top.
        assert!(transform.point(0.0, 100.0).y < 1.0);
        // A point at the document's bottom lands at the screen's bottom.
        assert!(transform.point(0.0, 0.0).y > 99.0);
    }

    /// The painter must actually clip. Scoping the clips correctly means
    /// nothing if a path can still be drawn outside one.
    #[test]
    fn a_path_outside_the_clip_is_not_drawn() {
        let mut painter = ImagePainter::new(100, 100);
        painter.push_clip(Rect::new(0.0, 0.0, 100.0, 20.0));
        // A square well below the clip band.
        painter.fill_path(
            &[
                Position::new(10.0, 50.0),
                Position::new(90.0, 50.0),
                Position::new(90.0, 90.0),
                Position::new(10.0, 90.0),
            ],
            Color::rgba(0.0, 0.0, 0.0, 1.0),
        );
        painter.pop_clip();
        let pixels = painter.pixels();
        let ink = pixels
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|px| px[3] > 8 && px[0] < 128)
            .count();
        assert_eq!(ink, 0, "a path outside the clip must not reach the surface");
    }

    /// A clip belongs to the `q`/`Q` pair that narrowed it.
    ///
    /// A `Q` that saved no clip of its own must not release the one around
    /// it. Popping one clip per Restore let clipped-away artwork escape onto
    /// the page -- and, where a state clipped twice, cut away artwork that
    /// belonged there.
    #[test]
    fn a_clip_outlives_a_save_and_restore_that_did_not_touch_it() {
        let ops = vec![
            RenderOp::Clip {
                rect: [10.0, 10.0, 40.0, 40.0],
                subpaths: Vec::new(),
            },
            // A state that clips nothing: its Restore is not about that clip.
            RenderOp::Save,
            RenderOp::Restore,
            RenderOp::Fill {
                subpaths: vec![vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0]]],
                color: [0.0, 0.0, 0.0, 1.0],
                even_odd: false,
            },
        ];
        let mut painter = RecordingPainter::new();
        paint_page(
            &mut painter,
            &list(ops),
            Transform::fit(&list(Vec::new()), area(), 1.0),
            &[],
            &crate::glyphs::FontSet::without_substitution(b""),
        );
        let json = painter.to_json();
        let fill_at = json.find(r#""op":"fill_path""#).expect("the fill");
        let before = &json[..fill_at];
        // One clip for the page and one for the document's own; neither
        // released before the fill that they contain.
        assert_eq!(
            before.matches(r#""op":"push_clip""#).count(),
            2,
            "page clip and document clip: {before:.200}"
        );
        assert_eq!(
            before.matches(r#""op":"pop_clip""#).count(),
            0,
            "nothing should have been released yet"
        );
    }

    /// Whatever the list leaves open is closed, and never more than that.
    #[test]
    fn clips_are_balanced_however_the_list_ends() {
        for ops in [
            // Clipped and never restored.
            vec![RenderOp::Clip {
                rect: [1.0, 1.0, 9.0, 9.0],
                subpaths: Vec::new(),
            }],
            // Restored more often than saved, as a damaged file can be.
            vec![RenderOp::Restore, RenderOp::Restore, RenderOp::Restore],
            // Two clips in one state, released together.
            vec![
                RenderOp::Save,
                RenderOp::Clip {
                    rect: [1.0, 1.0, 9.0, 9.0],
                    subpaths: Vec::new(),
                },
                RenderOp::Clip {
                    rect: [2.0, 2.0, 8.0, 8.0],
                    subpaths: Vec::new(),
                },
                RenderOp::Restore,
            ],
        ] {
            let mut painter = RecordingPainter::new();
            paint_page(
                &mut painter,
                &list(ops),
                Transform::fit(&list(Vec::new()), area(), 1.0),
                &[],
                &crate::glyphs::FontSet::without_substitution(b""),
            );
            let json = painter.to_json();
            assert_eq!(
                json.matches(r#""op":"push_clip""#).count(),
                json.matches(r#""op":"pop_clip""#).count(),
                "every clip is released exactly once: {json:.300}"
            );
        }
    }

    #[test]
    fn a_fill_lands_where_the_document_put_it() {
        let mut painter = ImagePainter::new(100, 100);
        // A square in the document's bottom-left quadrant.
        let ops = vec![RenderOp::Fill {
            subpaths: vec![vec![[10.0, 10.0], [40.0, 10.0], [40.0, 40.0], [10.0, 40.0]]],
            color: [1.0, 0.0, 0.0, 1.0],
            even_odd: false,
        }];
        paint_page(
            &mut painter,
            &list(ops),
            Transform::fit(&list(Vec::new()), area(), 1.0),
            &[],
            &crate::glyphs::FontSet::without_substitution(b""),
        );

        // The page is filled white first, and white also has r > 0.9 — so red
        // is identified by the *absence* of green, not the presence of red.
        let filled = painter.get_pixel(25, 75);
        assert!(
            filled.r > 0.9 && filled.g < 0.5,
            "fill is in the wrong place"
        );

        // Bottom-left in document space is high y on screen, so the top must
        // still be the untouched white page.
        let empty = painter.get_pixel(25, 25);
        assert!(empty.g > 0.9, "fill leaked to the top of the page");
    }

    #[test]
    fn an_image_is_drawn_when_its_texture_resolves() {
        let mut painter = ImagePainter::new(100, 100);
        let ops = vec![RenderOp::Image {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
            name: "Im1".into(),
        }];
        let texture = Texture {
            name: "Im1".into(),
            width: 1,
            height: 1,
            rgba: vec![0, 255, 0, 255],
        };
        paint_page(
            &mut painter,
            &list(ops),
            Transform::fit(&list(Vec::new()), area(), 1.0),
            std::slice::from_ref(&texture),
            &crate::glyphs::FontSet::without_substitution(b""),
        );
        // Green, not the white page: check red is gone rather than green present.
        let drawn = painter.get_pixel(50, 50);
        assert!(drawn.g > 0.9 && drawn.r < 0.5, "the texture did not draw");
    }

    #[test]
    fn a_missing_texture_leaves_a_visible_hole_rather_than_nothing() {
        let mut painter = ImagePainter::new(100, 100);
        let ops = vec![RenderOp::Image {
            x: 10.0,
            y: 10.0,
            w: 80.0,
            h: 80.0,
            name: "absent".into(),
        }];
        paint_page(
            &mut painter,
            &list(ops),
            Transform::fit(&list(Vec::new()), area(), 1.0),
            &[],
            &crate::glyphs::FontSet::without_substitution(b""),
        );

        // The frame is gray on a white page, so look for a pixel that is
        // darker than the page — checking alpha would match the page itself.
        let frame_drawn = (0..100)
            .flat_map(|x| (0..100).map(move |y| (x, y)))
            .any(|(x, y)| painter.get_pixel(x, y).r < 0.9);
        assert!(frame_drawn, "a missing image should still show its frame");
    }

    #[test]
    fn an_unbalanced_restore_cannot_escape_the_page_clip() {
        let mut painter = ImagePainter::new(100, 100);
        // More Restores than Clips, as a damaged file can produce.
        let ops = vec![
            RenderOp::Restore,
            RenderOp::Restore,
            RenderOp::Restore,
            RenderOp::Fill {
                subpaths: vec![vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]]],
                color: [1.0, 0.0, 0.0, 1.0],
                even_odd: false,
            },
        ];
        // The test is that this returns at all with the clip stack balanced;
        // an over-pop would corrupt the surrounding UI's clipping.
        paint_page(
            &mut painter,
            &list(ops),
            Transform::fit(&list(Vec::new()), area(), 1.0),
            &[],
            &crate::glyphs::FontSet::without_substitution(b""),
        );
        let filled = painter.get_pixel(50, 50);
        assert!(filled.r > 0.9 && filled.g < 0.5, "the fill did not survive");
    }

    #[test]
    fn the_recording_painter_emits_valid_json_with_balanced_clips() {
        let mut painter = RecordingPainter::new();
        let ops = vec![
            RenderOp::Fill {
                subpaths: vec![vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]]],
                color: [1.0, 0.0, 0.0, 1.0],
                even_odd: false,
            },
            RenderOp::Text {
                // Quotes, a backslash and a newline: exactly what breaks naive
                // string concatenation, and exactly what documents contain.
                text: "he said \"hi\"\\\n".into(),
                x: 5.0,
                y: 5.0,
                size: 12.0,
                width: 40.0,
                advances: vec![],
                codes: vec![],
                measured: false,
                rot: 0.0,
                color: [0.0, 0.0, 0.0, 1.0],
                font: "Helvetica".into(),
            },
        ];
        paint_page(
            &mut painter,
            &list(ops),
            Transform::fit(&list(Vec::new()), area(), 1.0),
            &[],
            &crate::glyphs::FontSet::without_substitution(b""),
        );

        assert_eq!(
            painter.open_clips(),
            0,
            "the replayer would be left clipped"
        );

        // The whole point is that JavaScript can parse this.
        let json: serde_json::Value =
            serde_json::from_str(&painter.to_json()).expect("the recording is not valid JSON");
        let recorded = json.as_array().unwrap();
        assert!(recorded.iter().any(|op| op["op"] == "fill_path"));

        let text = recorded.iter().find(|op| op["op"] == "text").unwrap();
        assert_eq!(
            text["text"], "he said \"hi\"\\\n",
            "escaping corrupted the text"
        );
    }

    #[test]
    fn the_recording_inlines_masks_but_not_photographs() {
        // An image travels at the size it is drawn. A 512-pixel picture in a
        // ten-point slot is ten points of detail; sending the rest costs the
        // host a megabyte to throw away.
        let mut painter = RecordingPainter::new();
        let page_sized = vec![255u8; 512 * 512 * 4];
        painter.draw_image(
            Rect::new(0.0, 0.0, 10.0, 10.0),
            &ImageData::new(512, 512, &page_sized),
        );
        let json = painter.to_json();
        assert!(
            json.contains("\"pixels\":"),
            "sampled down, it is small enough to send"
        );
        assert!(
            json.contains("\"iw\":10"),
            "and it is sent at the size it is drawn: {json:.120}"
        );
        assert!(
            json.len() < 4096,
            "a quarter-megapixel image in a ten-point slot should cost              hundreds of bytes, not hundreds of thousands: {} bytes",
            json.len()
        );

        let mut painter = RecordingPainter::new();
        let mask = vec![255u8; 12 * 14 * 4];
        painter.draw_image(
            Rect::new(0.0, 0.0, 12.0, 14.0),
            &ImageData::new(12, 14, &mask),
        );
        assert!(
            painter.to_json().contains("\"pixels\":"),
            "a glyph mask has to travel, or text does not render at all"
        );
    }

    #[test]
    fn zoom_scales_the_page_without_moving_its_origin_off_centre() {
        let page = list(Vec::new());
        let small = Transform::fit(&page, area(), 1.0);
        let large = Transform::fit(&page, area(), 2.0);
        assert!(large.scale > small.scale);
        assert_eq!(large.page_height, small.page_height);
    }

    fn text_op(text: &str, width: f64, rot: f64) -> RenderOp {
        RenderOp::Text {
            text: text.into(),
            x: 10.0,
            y: 50.0,
            size: 12.0,
            width,
            // One advance per character, summing to `width`, as a real PDF's do.
            advances: vec![width / text.chars().count() as f64; text.chars().count()],
            codes: text.chars().map(|c| c as u32).collect(),
            measured: false,
            rot,
            color: [0.0, 0.0, 0.0, 1.0],
            font: "Times".into(),
        }
    }

    /// A run drawn at its natural width overruns the slot the document gave it
    /// and collides with the next run -- the overlapping sentences this used to
    /// render. The run must be fitted to the width the document reserved.
    #[test]
    fn a_run_is_fitted_to_the_width_the_document_reserved() {
        let mut painter = RecordingPainter::new();
        // A slot far narrower than this text measures at size 12.
        let ops = vec![text_op("a sentence that is much too wide", 20.0, 0.0)];
        paint_page(
            &mut painter,
            &list(ops),
            Transform::fit(&list(Vec::new()), area(), 1.0),
            &[],
            &crate::glyphs::FontSet::without_substitution(b""),
        );
        let json = painter.to_json();
        let drawn: Vec<&str> = json.matches(r#""op":"text""#).collect();
        assert_eq!(drawn.len(), 1, "the run should be laid out in one call");
        // Size reduced from 12 to fit, but floored rather than made unreadable.
        let size = json
            .split(r#""size":"#)
            .nth(1)
            .and_then(|rest| rest.split(',').next())
            .and_then(|n| n.parse::<f32>().ok())
            .expect("a size in the recording");
        assert!(size < 12.0, "expected the run to be fitted, got {size}");
        // A floor, not an exact value: the comparison is in f32, where
        // 12.0 * 0.6 is not exactly 7.2.
        assert!(size >= 12.0 * 0.6 - 0.01, "expected a floor, got {size}");
    }

    /// A run that already fits must not be resized: shrinking text that was
    /// never going to collide would make every page subtly too small.
    #[test]
    fn a_run_that_fits_is_left_alone() {
        let mut painter = RecordingPainter::new();
        let ops = vec![text_op("hi", 400.0, 0.0)];
        paint_page(
            &mut painter,
            &list(ops),
            Transform::fit(&list(Vec::new()), area(), 1.0),
            &[],
            &crate::glyphs::FontSet::without_substitution(b""),
        );
        assert!(
            painter.to_json().contains(r#""size":12.00"#),
            "a fitting run should keep its size: {}",
            painter.to_json()
        );
    }

    /// Rotated text has no layout call that can place it, so it is walked glyph
    /// by glyph along its baseline. Without this an arXiv stamp meant for the
    /// margin is drawn straight across the body text.
    #[test]
    fn rotated_text_runs_along_its_baseline() {
        let mut painter = RecordingPainter::new();
        let quarter_turn = std::f64::consts::FRAC_PI_2;
        let ops = vec![text_op("abc", 30.0, quarter_turn)];
        paint_page(
            &mut painter,
            &list(ops),
            Transform::fit(&list(Vec::new()), area(), 1.0),
            &[],
            &crate::glyphs::FontSet::without_substitution(b""),
        );
        let json = painter.to_json();
        assert_eq!(
            json.matches(r#""op":"text""#).count(),
            3,
            "each glyph should be placed: {json}"
        );
        // Quarter turn: the glyphs advance up the page, so x stays put and y
        // changes. Getting this wrong is what drew the stamp horizontally.
        // Parse the text ops only: the page fill and the clip carry
        // coordinates too, and reading those instead is how this test first
        // "failed" against correct output.
        let glyphs: Vec<(f32, f32)> = json
            .split(r#"{"op":"text","#)
            .skip(1)
            .map(|entry| {
                let field = |name: &str| {
                    entry
                        .split(&format!(r#""{name}":"#))
                        .nth(1)
                        .and_then(|rest| rest.split(',').next())
                        .and_then(|n| n.trim_matches('"').parse::<f32>().ok())
                        .expect("a coordinate")
                };
                (field("x"), field("y"))
            })
            .collect();
        assert_eq!(glyphs.len(), 3, "each glyph should be placed: {json}");
        assert_eq!(glyphs[0].0, glyphs[1].0, "x must not advance: {json}");
        assert!(
            glyphs[0].1 > glyphs[1].1,
            "y must advance up the page: {json}"
        );
    }
}

#[cfg(test)]
mod substitution_render {
    use super::*;

    /// A document that embeds no fonts at all must still draw glyphs, by
    /// standing in a system face. Without that it can only be laid out with
    /// the UI font, which is the approximation this work exists to remove.
    /// A glyph is drawn at the size the page is shown at, not the size the
    /// page is written in.
    ///
    /// Every position in a recording goes through the transform; the masks
    /// have to as well. When they did not, a page shown at less than 1:1 --
    /// which is every page that fits a window, and every page on a phone --
    /// drew full-size letters against shrunken advances, and the words closed
    /// up into a solid bar of ink. The two agree here or the text is wrong at
    /// every zoom but one.
    #[test]
    fn glyph_masks_scale_with_the_page() {
        let Some(pdf) = sample_document() else {
            eprintln!("skipping: no sample document present");
            return;
        };
        let Ok(session) = crate::session::Session::open(pdf) else {
            panic!("the document should open");
        };
        let list = session.display_list().expect("page geometry");

        // The same page at two sizes, one twice the other.
        let widths = |scale: f32| -> Vec<f32> {
            let area = Rect::new(0.0, 0.0, 612.0 * scale, 792.0 * scale);
            let mut painter = RecordingPainter::new();
            paint_page(
                &mut painter,
                &list,
                Transform::fit(&list, area, 1.0),
                &[],
                session.fonts(),
            );
            let json = painter.to_json();
            json.split("{\"op\":\"image\",")
                .skip(1)
                .filter_map(|entry| {
                    entry
                        .split("\"w\":")
                        .nth(1)?
                        .split([',', '}'])
                        .next()?
                        .parse::<f32>()
                        .ok()
                })
                .collect()
        };

        // Measured at readable sizes: every mask carries a couple of pixels of
        // anti-aliased edge whatever its size, so the ratio approaches two
        // from below rather than reaching it. Measuring small letters would
        // measure mostly edge.
        let small = widths(4.0);
        let large = widths(8.0);
        assert!(!small.is_empty(), "the page should draw glyphs");
        assert_eq!(small.len(), large.len(), "the same glyphs either way");

        let total_small: f32 = small.iter().sum();
        let total_large: f32 = large.iter().sum();
        let ratio = total_large / total_small;
        assert!(
            (1.7..2.1).contains(&ratio),
            "doubling the page should very nearly double the glyphs: {ratio}"
        );
    }

    /// A PDF with embedded fonts, wherever the test happens to be run from.
    fn sample_document() -> Option<Vec<u8>> {
        for path in [
            "../../demos/sample.pdf",
            "../../../demos/sample.pdf",
            "demos/sample.pdf",
        ] {
            if let Ok(bytes) = std::fs::read(path) {
                return Some(bytes);
            }
        }
        None
    }

    #[test]
    fn a_document_with_no_embedded_fonts_still_draws_glyphs() {
        let Ok(pdf) = std::fs::read("../../../website/public/shannon1948.pdf")
            .or_else(|_| std::fs::read("../../website/public/shannon1948.pdf"))
            .or_else(|_| std::fs::read("../website/public/shannon1948.pdf"))
            .or_else(|_| std::fs::read("website/public/shannon1948.pdf"))
        else {
            eprintln!("skipping: shannon1948.pdf not present");
            return;
        };
        let Ok(session) = crate::session::Session::open(pdf) else {
            panic!("the document should open");
        };
        assert!(
            session.fonts().is_empty(),
            "this document embeds nothing; that is the point of the test"
        );

        let list = session.display_list().expect("page geometry");
        let area = Rect::new(0.0, 0.0, 612.0, 792.0);
        let mut painter = RecordingPainter::new();
        paint_page(
            &mut painter,
            &list,
            Transform::fit(&list, area, 1.0),
            &[],
            session.fonts(),
        );
        let json = painter.to_json();

        // Glyphs are drawn as image masks; text laid out by the painter's own
        // font would appear as `text` operations instead.
        let glyphs = json.matches(r#""op":"image""#).count();
        let laid_out = json.matches(r#""op":"text""#).count();
        if glyphs == 0 {
            eprintln!("skipping: no system face available to substitute");
            return;
        }
        assert!(
            glyphs > 200,
            "a page of prose should be hundreds of glyphs, got {glyphs}"
        );
        assert_eq!(
            laid_out, 0,
            "nothing should fall back once a face is available"
        );
    }
}

#[cfg(test)]
mod render_report {
    use super::*;

    /// Render a page of any document and report what it took to draw it.
    ///
    /// Ignored by default because it needs a path: run with
    /// `APDF_RENDER_PDF=<file> cargo test -- --ignored render_any`.
    /// The point is to put documents from the wild through the same path the
    /// app uses, where fixtures built here cannot reach.
    #[test]
    #[ignore = "needs a document; run deliberately"]
    fn render_any() {
        let Ok(path) = std::env::var("APDF_RENDER_PDF") else {
            eprintln!("set APDF_RENDER_PDF");
            return;
        };
        let page: usize = std::env::var("APDF_RENDER_PAGE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);
        let bytes = std::fs::read(&path).expect("read the document");
        let mut session = crate::session::Session::open(bytes).expect("open");
        for _ in 1..page {
            session.next_page();
        }

        let fonts = session.fonts();
        eprintln!("embedded fonts: {}", fonts.len());

        let list = session.display_list().expect("geometry");
        let area = Rect::new(0.0, 0.0, 900.0, 1200.0);
        let mut painter = RecordingPainter::new();
        paint_page(
            &mut painter,
            &list,
            Transform::fit(&list, area, 1.0),
            &[],
            fonts,
        );
        let json = painter.to_json();
        let glyphs = json.matches(r#""op":"image""#).count();
        let fallback = json.matches(r#""op":"text""#).count();
        let fills = json.matches(r#""op":"fill_path""#).count();
        eprintln!(
            "page {page}: {glyphs} glyph masks, {fallback} fallback runs, {fills} vector fills"
        );

        // A page whose recording is empty drew nothing at all. A page of
        // images legitimately has no glyphs, so the check is on the recording
        // rather than on text.
        let ops = json.matches(r#""op":"#).count();
        eprintln!("total recorded ops: {ops}");
        assert!(ops > 1, "the page produced no drawing at all");
    }
}

#[cfg(test)]
mod render_sweep {
    use super::*;

    /// Render page one of every document in a directory and report what each
    /// took to draw. Finds the cases fixtures cannot: producers this code has
    /// never seen, damaged files, fonts that resolve to nothing.
    ///
    /// Reports counts only, never text: a sweep is usually run over documents
    /// that are nobody's business but their owner's.
    ///
    /// `APDF_SWEEP_DIR=<dir> cargo test -- --ignored render_sweep`
    #[test]
    #[ignore = "needs a directory; run deliberately"]
    fn render_sweep() {
        let Ok(dir) = std::env::var("APDF_SWEEP_DIR") else {
            eprintln!("set APDF_SWEEP_DIR");
            return;
        };
        let Ok(entries) = std::fs::read_dir(&dir) else {
            eprintln!("cannot read {dir}");
            return;
        };

        let mut files: Vec<std::path::PathBuf> = entries
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|path| {
                path.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.eq_ignore_ascii_case("pdf"))
                    .unwrap_or(false)
            })
            .collect();
        files.sort();

        let limit: usize = std::env::var("APDF_SWEEP_LIMIT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(40);

        let mut opened = 0;
        let mut failed = 0;
        let mut with_fallback = 0;
        let mut blank = 0;
        let mut misplaced = 0;

        for path in files.iter().take(limit) {
            let name: String = path
                .file_name()
                .map(|n| n.to_string_lossy().chars().take(38).collect())
                .unwrap_or_default();
            let Ok(bytes) = std::fs::read(path) else {
                continue;
            };
            let mut session = match crate::session::Session::open(bytes) {
                Ok(session) => session,
                Err(why) => {
                    eprintln!("{name:40} OPEN FAILED: {why}");
                    failed += 1;
                    continue;
                }
            };
            opened += 1;

            // Several pages, not only the first. A title page is the least
            // representative page a document has: the tables, figures, maths
            // and footnotes that stress a renderer are all further in.
            let pages = session.page_count();
            let sample: Vec<usize> = [1usize, 2, 5, 11, 23]
                .into_iter()
                .filter(|page| *page <= pages)
                .collect();

            let mut runs_seen = 0usize;
            let mut glyphs_seen = 0usize;
            let mut fell_back = 0usize;
            let mut stray_seen = 0usize;
            let mut measured_seen = 0usize;
            let mut worst_seen = 0.0f32;
            let mut at = 1usize;

            for page in &sample {
                while at < *page {
                    session.next_page();
                    at += 1;
                }
                let Ok(list) = session.display_list() else {
                    continue;
                };
                let area = Rect::new(0.0, 0.0, 900.0, 1200.0);
                let transform = Transform::fit(&list, area, 1.0);
                let mut painter = RecordingPainter::new();
                paint_page(&mut painter, &list, transform, &[], session.fonts());
                let json = painter.to_json();

                runs_seen += list
                    .ops
                    .iter()
                    .filter(|op| matches!(op, RenderOp::Text { .. }))
                    .count();
                glyphs_seen += json.matches(r#""op":"image""#).count();
                fell_back += json.matches(r#""op":"text""#).count();

                let (measured, stray, worst) = super::measure_placement(&session, &list, transform);
                measured_seen += measured;
                stray_seen += stray;
                worst_seen = worst_seen.max(worst);
            }

            if fell_back > 0 {
                with_fallback += 1;
            }
            if runs_seen > 0 && glyphs_seen == 0 {
                blank += 1;
            }
            if stray_seen > 0 {
                misplaced += 1;
            }
            eprintln!(
                "{name:36} pages={:<2} runs={runs_seen:<5} glyphs={glyphs_seen:<6} fallback={fell_back:<3} stray={stray_seen}/{measured_seen} worst={worst_seen:.1}px",
                sample.len()
            );
        }

        eprintln!(
            "\nopened {opened}, failed {failed}, with fallback {with_fallback}, \
             text but no glyphs {blank}, misplaced {misplaced}"
        );
        assert_eq!(failed, 0, "every document should open");
    }
}

#[cfg(test)]
mod fallback_probe {
    use super::*;

    /// Report which runs could not be drawn with glyphs, and why.
    ///
    /// `APDF_RENDER_PDF=<file> cargo test -- --ignored why_fallback`
    #[test]
    #[ignore = "needs a document; run deliberately"]
    fn why_fallback() {
        let Ok(path) = std::env::var("APDF_RENDER_PDF") else {
            return;
        };
        let bytes = std::fs::read(&path).expect("read");
        let session = crate::session::Session::open(bytes).expect("open");
        let fonts = session.fonts();
        let list = session.display_list().expect("geometry");

        for op in &list.ops {
            let RenderOp::Text {
                text,
                advances,
                codes,
                font,
                size,
                rot,
                ..
            } = op
            else {
                continue;
            };
            let chars: Vec<char> = text.chars().collect();
            let have_face = fonts.font_matrix(font).is_some();
            let aligned = advances.len() == chars.len() && codes.len() == chars.len();

            // How many of the run's non-space characters resolve to a glyph.
            let substituted = fonts.is_substituted(font);
            let mut resolved = 0;
            let mut wanted = 0;
            for (index, ch) in chars.iter().enumerate() {
                if ch.is_whitespace() {
                    continue;
                }
                wanted += 1;
                let code = match substituted {
                    true => *ch as u32,
                    false => codes.get(index).copied().unwrap_or(0),
                };
                if fonts
                    .raster(font, code, Some(*ch), *size, *rot, [0, 0, 0])
                    .is_some()
                {
                    resolved += 1;
                } else {
                    eprintln!("  miss: {font} code={code} U+{:04X}", *ch as u32);
                }
            }
            if have_face && aligned && resolved == wanted && wanted > 0 {
                continue;
            }
            eprintln!(
                "font={font:<32} face={have_face:<5} sub={substituted:<5} chars={:<4} resolved={resolved}/{wanted}",
                chars.len()
            );
        }
    }
}

#[cfg(test)]
mod page_image {
    use super::*;
    use dewey::backend::image_buffer::ImagePainter;

    /// Render a page to an image file, so the output can be looked at and
    /// compared against another renderer rather than counted.
    ///
    /// A count says a glyph was drawn; only a picture says it was drawn in the
    /// right place. Writes a PPM, which needs no encoder.
    ///
    /// `APDF_RENDER_PDF=<file> APDF_IMAGE_OUT=<file.ppm> [APDF_RENDER_PAGE=n]
    /// [APDF_IMAGE_WIDTH=1200] cargo test -- --ignored write_page_image`
    #[test]
    #[ignore = "writes a file; run deliberately"]
    fn write_page_image() {
        let (Ok(path), Ok(out)) = (
            std::env::var("APDF_RENDER_PDF"),
            std::env::var("APDF_IMAGE_OUT"),
        ) else {
            eprintln!("set APDF_RENDER_PDF and APDF_IMAGE_OUT");
            return;
        };
        let page: usize = std::env::var("APDF_RENDER_PAGE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);
        let width: u32 = std::env::var("APDF_IMAGE_WIDTH")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1200);

        let bytes = std::fs::read(&path).expect("read the document");
        let mut session = crate::session::Session::open(bytes).expect("open");
        for _ in 1..page {
            session.next_page();
        }
        let list = session.display_list().expect("geometry");

        // Fit the page to the requested width, keeping its aspect.
        let scale = width as f32 / list.width.max(1.0) as f32;
        let height = (list.height as f32 * scale).round().max(1.0) as u32;
        let area = Rect::new(0.0, 0.0, width as f32, height as f32);

        let mut painter = ImagePainter::new(width, height);
        let textures = session.textures(width as usize * height as usize);
        paint_page(
            &mut painter,
            &list,
            Transform::fit(&list, area, 1.0),
            &textures,
            session.fonts(),
        );

        // PPM: a header and raw RGB, which every image tool reads.
        let pixels = painter.pixels();
        let mut file = format!("P6\n{width} {height}\n255\n").into_bytes();
        for chunk in pixels.as_chunks::<4>().0 {
            file.extend_from_slice(&chunk[..3]);
        }
        std::fs::write(&out, &file).expect("write the image");
        eprintln!("wrote {out} ({width}x{height})");
    }

    /// Measure every page of a corpus against a reference renderer.
    ///
    /// A corpus directory holds one subdirectory per document, each with a
    /// `source.txt` naming the file and a `pN.ref.ppm` per page rendered by
    /// the reference. This renders the same pages and reports how far apart
    /// they are, so a change to the engine can be judged against every
    /// document at once instead of the one that prompted it.
    ///
    /// The two rasters are different sizes, so they are reduced to a grid of
    /// ink coverage and compared as distributions: total variation distance,
    /// where 0.03 to 0.05 is antialiasing alone and 0.12 is the line between
    /// a match and a difference worth looking at.
    ///
    /// `APDF_CORPUS=<dir> cargo test --lib -- --ignored compare_corpus`
    ///
    /// A corpus of any size is one long single-threaded walk, so
    /// `APDF_SHARD=k/n` takes every n-th document and the shards can be run
    /// side by side. Sharding by document rather than by page keeps a
    /// document's pages together, which is how the rows read.
    #[test]
    #[ignore = "needs a rendered corpus; run deliberately"]
    fn compare_corpus() {
        let Ok(root) = std::env::var("APDF_CORPUS") else {
            eprintln!("set APDF_CORPUS to a directory of rendered references");
            return;
        };
        let (shard, shards) = match std::env::var("APDF_SHARD") {
            Ok(spec) => {
                let mut parts = spec.split('/');
                let k: usize = parts.next().and_then(|s| s.parse().ok()).unwrap_or(1);
                let n: usize = parts.next().and_then(|s| s.parse().ok()).unwrap_or(1);
                (k.saturating_sub(1), n.max(1))
            }
            Err(_) => (0, 1),
        };
        let mut entries: Vec<_> = std::fs::read_dir(&root)
            .expect("read the corpus")
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        entries.sort();
        let total = entries.len();
        let entries: Vec<_> = entries
            .into_iter()
            .enumerate()
            .filter(|(i, _)| i % shards == shard)
            .map(|(_, p)| p)
            .collect();
        eprintln!(
            "shard {}/{}: {} of {total} documents",
            shard + 1,
            shards,
            entries.len()
        );

        let mut rows: Vec<(f64, f64, String)> = Vec::new();
        let (mut compared, mut matched, mut skipped) = (0usize, 0usize, 0usize);
        for dir in entries {
            let Ok(source) = std::fs::read_to_string(dir.join("source.txt")) else {
                continue;
            };
            let source = source.trim().to_string();
            let Ok(bytes) = std::fs::read(&source) else {
                continue;
            };
            let name = dir
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            eprintln!("... {name}");
            for page in 1..=8usize {
                let refpath = dir.join(format!("p{page}.ref.ppm"));
                let Some((rw, rh, rpx)) = read_ppm(&refpath) else {
                    continue;
                };
                let Some((ow, oh, opx)) = render_to_rgb(&bytes, page, rw as u32) else {
                    eprintln!("  {name} p{page}: could not render");
                    skipped += 1;
                    continue;
                };
                // A page we render at a wildly different shape is not a
                // rendering difference but a different page box; say so
                // rather than folding it into the score.
                let aspect = (ow as f64 / oh as f64) / (rw as f64 / rh as f64);
                if !(0.97..1.03).contains(&aspect) {
                    eprintln!("  {name} p{page}: aspect {aspect:.3}, not comparable");
                    skipped += 1;
                    continue;
                }
                let ours = ink_grid(ow, oh, &opx);
                let theirs = ink_grid(rw, rh, &rpx);
                let (sa, sb): (f64, f64) = (ours.iter().sum(), theirs.iter().sum());
                if sa <= 1e-6 && sb <= 1e-6 {
                    skipped += 1;
                    continue;
                }
                let tv = if sa <= 1e-6 || sb <= 1e-6 {
                    1.0
                } else {
                    0.5 * ours
                        .iter()
                        .zip(&theirs)
                        .map(|(a, b)| (a / sa - b / sb).abs())
                        .sum::<f64>()
                };
                let mae = ours
                    .iter()
                    .zip(&theirs)
                    .map(|(a, b)| (a - b).abs())
                    .sum::<f64>()
                    / ours.len() as f64;
                compared += 1;
                if tv <= 0.12 {
                    matched += 1;
                }
                rows.push((tv, mae, format!("{name} p{page}")));
            }
        }
        rows.sort_by(|a, b| b.0.total_cmp(&a.0));
        for (tv, mae, what) in rows.iter().take(30) {
            eprintln!("{tv:.3}  mae {mae:.4}  {what}");
        }
        eprintln!("compared {compared}, within 0.12: {matched}, not comparable {skipped}");
    }

    /// Render one page of a document to RGB at the given width.
    fn render_to_rgb(bytes: &[u8], page: usize, width: u32) -> Option<(usize, usize, Vec<u8>)> {
        let mut session = crate::session::Session::open(bytes.to_vec()).ok()?;
        for _ in 1..page {
            session.next_page();
        }
        let list = session.display_list().ok()?;
        let scale = width as f32 / list.width.max(1.0) as f32;
        let height = (list.height as f32 * scale).round().max(1.0) as u32;
        let mut painter = ImagePainter::new(width, height);
        let textures = session.textures(width as usize * height as usize);
        paint_page(
            &mut painter,
            &list,
            Transform::fit(&list, Rect::new(0.0, 0.0, width as f32, height as f32), 1.0),
            &textures,
            session.fonts(),
        );
        let mut rgb = Vec::with_capacity(width as usize * height as usize * 3);
        for chunk in painter.pixels().as_chunks::<4>().0 {
            rgb.extend_from_slice(&chunk[..3]);
        }
        Some((width as usize, height as usize, rgb))
    }

    fn read_ppm(path: &std::path::Path) -> Option<(usize, usize, Vec<u8>)> {
        let bytes = std::fs::read(path).ok()?;
        // "P6", width, height, maxval, then one whitespace byte and the data.
        let mut fields = Vec::new();
        let mut i = 0usize;
        while fields.len() < 4 {
            while bytes.get(i)?.is_ascii_whitespace() {
                i += 1;
            }
            if bytes[i] == b'#' {
                while *bytes.get(i)? != b'\n' {
                    i += 1;
                }
                continue;
            }
            let start = i;
            while !bytes.get(i)?.is_ascii_whitespace() {
                i += 1;
            }
            fields.push(String::from_utf8_lossy(&bytes[start..i]).to_string());
        }
        i += 1;
        let w: usize = fields[1].parse().ok()?;
        let h: usize = fields[2].parse().ok()?;
        Some((w, h, bytes.get(i..i + w * h * 3)?.to_vec()))
    }

    const GRID_W: usize = 60;
    const GRID_H: usize = 40;

    /// Mean ink -- one minus luminance -- per cell of a fixed grid.
    fn ink_grid(w: usize, h: usize, rgb: &[u8]) -> Vec<f64> {
        let mut sums = vec![0.0f64; GRID_W * GRID_H];
        let mut counts = vec![0u32; GRID_W * GRID_H];
        for y in 0..h {
            let gy = y * GRID_H / h;
            for x in 0..w {
                let o = (y * w + x) * 3;
                let lum = (rgb[o] as u32 * 299 + rgb[o + 1] as u32 * 587 + rgb[o + 2] as u32 * 114)
                    / 1000;
                let cell = gy * GRID_W + x * GRID_W / w;
                sums[cell] += (255 - lum) as f64;
                counts[cell] += 1;
            }
        }
        sums.iter()
            .zip(&counts)
            .map(|(s, &n)| if n == 0 { 0.0 } else { s / n as f64 / 255.0 })
            .collect()
    }
}

#[cfg(test)]
/// How many glyph masks a page drew, and how many landed outside the run that
/// asked for them. See `placement::glyphs_land_in_their_run` for why.
fn measure_placement(
    session: &crate::session::Session,
    list: &DisplayList,
    transform: Transform,
) -> (usize, usize, f32) {
    let mut boxes: Vec<(f32, f32, f32, f32)> = Vec::new();
    for op in &list.ops {
        let RenderOp::Text {
            x,
            y,
            size,
            width,
            rot,
            ..
        } = op
        else {
            continue;
        };
        let origin = transform.point(*x, *y);
        let end = transform.point(x + width * rot.cos(), y + width * rot.sin());
        let pad = (*size as f32 * transform.scale).max(1.0) * 1.6;
        boxes.push((
            origin.x.min(end.x) - pad,
            origin.y.min(end.y) - pad,
            origin.x.max(end.x) + pad,
            origin.y.max(end.y) + pad,
        ));
    }

    let mut painter = RecordingPainter::new();
    paint_page(&mut painter, list, transform, &[], session.fonts());
    let json = painter.to_json();

    let mut total = 0usize;
    let mut stray = 0usize;
    let mut worst = 0.0f32;
    for entry in json.split("{\"op\":\"image\",").skip(1) {
        let field = |name: &str| -> Option<f32> {
            entry
                .split(&format!("\"{name}\":"))
                .nth(1)?
                .split([',', '}'])
                .next()?
                .parse()
                .ok()
        };
        let (Some(x), Some(y), Some(w), Some(h)) = (field("x"), field("y"), field("w"), field("h"))
        else {
            continue;
        };
        total += 1;
        let escape = boxes
            .iter()
            .map(|(left, top, right, bottom)| {
                let dx = (left - x).max(x + w - right).max(0.0);
                let dy = (top - y).max(y + h - bottom).max(0.0);
                dx.max(dy)
            })
            .fold(f32::MAX, f32::min);
        if escape > 0.0 {
            stray += 1;
            worst = worst.max(escape);
        }
    }
    (total, stray, worst)
}

#[cfg(test)]
mod placement {
    use super::*;

    /// Every glyph must land inside the run that asked for it.
    ///
    /// Counting glyphs proves they were drawn; it says nothing about where.
    /// A wrong advance, a wrong baseline or a wrong transform all draw the
    /// right number of glyphs in the wrong place. The document states each
    /// run's origin and width, so the ink can be checked against it.
    ///
    /// `APDF_RENDER_PDF=<file> cargo test -- --ignored glyphs_land_in_their_run`
    #[test]
    #[ignore = "needs a document; run deliberately"]
    fn glyphs_land_in_their_run() {
        let Ok(path) = std::env::var("APDF_RENDER_PDF") else {
            eprintln!("set APDF_RENDER_PDF");
            return;
        };
        let bytes = std::fs::read(&path).expect("read");
        let session = crate::session::Session::open(bytes).expect("open");
        let list = session.display_list().expect("geometry");
        let area = Rect::new(0.0, 0.0, 1000.0, 1300.0);
        let transform = Transform::fit(&list, area, 1.0);

        // Each run's box in screen space, generous vertically: ascenders and
        // descenders reach well past the size, and a mask is padded a pixel.
        let mut boxes: Vec<(f32, f32, f32, f32)> = Vec::new();
        for op in &list.ops {
            let RenderOp::Text {
                x,
                y,
                size,
                width,
                rot,
                ..
            } = op
            else {
                continue;
            };
            // The run runs along its baseline direction, so its extent is
            // taken from both ends. Skipping rotated runs would leave their
            // glyphs belonging to no box at all, and they would then read as
            // misplaced -- which is what this test first reported.
            let origin = transform.point(*x, *y);
            let end = transform.point(x + width * rot.cos(), y + width * rot.sin());
            let height = (*size as f32 * transform.scale).max(1.0);
            // Generous: ascenders and descenders reach past the size, a mask
            // is padded, and a rotated run needs the margin on both axes.
            let pad_x = height * 1.6;
            let pad_y = height * 1.6;
            boxes.push((
                origin.x.min(end.x) - pad_x,
                origin.y.min(end.y) - pad_y,
                origin.x.max(end.x) + pad_x,
                origin.y.max(end.y) + pad_y,
            ));
        }
        if boxes.is_empty() {
            eprintln!("skipping: no horizontal text on this page");
            return;
        }

        let mut painter = RecordingPainter::new();
        paint_page(&mut painter, &list, transform, &[], session.fonts());
        let json = painter.to_json();

        // Every glyph mask is an image op; the page itself draws none.
        let mut total = 0usize;
        let mut stray = 0usize;
        let mut gross = 0usize;
        let mut worst = 0.0f32;
        for entry in json.split("{\"op\":\"image\",").skip(1) {
            let field = |name: &str| -> Option<f32> {
                entry
                    .split(&format!("\"{name}\":"))
                    .nth(1)?
                    .split([',', '}'])
                    .next()?
                    .parse()
                    .ok()
            };
            let (Some(x), Some(y), Some(w), Some(h)) =
                (field("x"), field("y"), field("w"), field("h"))
            else {
                continue;
            };
            total += 1;
            // How far outside the nearest run this glyph sits. A tall bracket
            // overshooting a heuristic box by a few pixels is not the same
            // defect as a glyph on the wrong side of the page, so the distance
            // is what gets judged.
            let escape = boxes
                .iter()
                .map(|(left, top, right, bottom)| {
                    let dx = (left - x).max(x + w - right).max(0.0);
                    let dy = (top - y).max(y + h - bottom).max(0.0);
                    dx.max(dy)
                })
                .fold(f32::MAX, f32::min);
            if escape > 0.0 {
                stray += 1;
                worst = worst.max(escape);
            }
            if escape > 24.0 {
                gross += 1;
            }
        }

        eprintln!(
            "{total} glyphs, {stray} outside their run, {gross} grossly so, worst {worst:.1}px"
        );
        assert!(total > 0, "no glyphs were drawn");
        // Overshooting a heuristic box by a few pixels is a tall glyph, not a
        // misplacement. Landing a line away from every run is the real defect,
        // and there should be none of it.
        assert_eq!(
            gross, 0,
            "{gross} of {total} glyphs landed far outside any run (worst {worst:.1}px)"
        );
    }
}

#[cfg(test)]
mod recording_glyphs {
    use super::*;

    /// The recording has to carry the glyph masks themselves.
    ///
    /// Every glyph on a page is an image op, and a host that receives only
    /// geometry draws text as a field of empty rectangles -- which is exactly
    /// what the browser did once glyphs became masks, while the desktop, which
    /// paints directly, looked fine. The two surfaces share `paint_page`, so
    /// the recording is the only place that difference can live.
    #[test]
    fn a_recording_carries_its_glyph_masks() {
        let Ok(pdf) = std::fs::read("../../demos/sample.pdf")
            .or_else(|_| std::fs::read("../demos/sample.pdf"))
        else {
            eprintln!("skipping: demos/sample.pdf not present");
            return;
        };
        let session = crate::session::Session::open(pdf).expect("open");
        let list = session.display_list().expect("geometry");
        let area = Rect::new(0.0, 0.0, 900.0, 1200.0);
        let mut painter = RecordingPainter::new();
        paint_page(
            &mut painter,
            &list,
            Transform::fit(&list, area, 1.0),
            &[],
            session.fonts(),
        );
        let json = painter.to_json();

        let images = json.matches(r#""op":"image""#).count();
        let keyed = json.matches(r#""key":""#).count();
        let inlined = json.matches(r#""pixels":""#).count();
        assert!(images > 100, "the page should draw glyphs: {images}");
        assert_eq!(keyed, images, "every glyph should carry a content key");
        assert!(inlined > 0, "the masks themselves must be sent");

        // The point of the key: a page reuses the same letters constantly, so
        // pixels are sent once and referenced after that. Inlining every
        // instance would multiply the recording several times over.
        assert!(
            inlined * 3 < images,
            "{inlined} masks for {images} glyphs -- repeats should reference, not resend"
        );
    }

    /// Sampling averages the pixels it drops rather than picking one.
    ///
    /// Nearest-neighbour on a photograph reduced tenfold is a field of
    /// speckle: every tenth pixel survives and the other ninety-nine
    /// hundredths of the picture is gone. Averaging keeps the colour.
    #[test]
    fn sampling_down_averages_rather_than_picks() {
        // A checkerboard of black and white pixels: any single sample is one
        // or the other, the average is grey.
        let (w, h) = (64u32, 64u32);
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        for y in 0..h as usize {
            for x in 0..w as usize {
                let at = (y * w as usize + x) * 4;
                let v = if (x + y) % 2 == 0 { 0 } else { 255 };
                pixels[at] = v;
                pixels[at + 1] = v;
                pixels[at + 2] = v;
                pixels[at + 3] = 255;
            }
        }
        let image = ImageData::new(w, h, &pixels);
        let (out_w, out_h, out) =
            fit_to_draw(&image, Rect::new(0.0, 0.0, 8.0, 8.0), 1.0).expect("should sample down");
        assert_eq!((out_w, out_h), (8, 8));
        for pixel in out.as_chunks::<4>().0 {
            assert!(
                (100..=155).contains(&pixel[0]),
                "a black and white checkerboard averages to grey, got {}",
                pixel[0]
            );
            assert_eq!(pixel[3], 255, "alpha survives");
        }
    }

    /// An image drawn at or near its own size is left alone: sampling it
    /// would only lose detail the host can use.
    #[test]
    fn an_image_drawn_at_its_own_size_is_not_sampled() {
        let pixels = vec![9u8; 32 * 32 * 4];
        let image = ImageData::new(32, 32, &pixels);
        assert!(fit_to_draw(&image, Rect::new(0.0, 0.0, 32.0, 32.0), 1.0).is_none());
        // Nor is it enlarged to fill a bigger slot.
        assert!(fit_to_draw(&image, Rect::new(0.0, 0.0, 90.0, 90.0), 1.0).is_none());
        // A host with more pixels per unit gets more of the image.
        assert!(fit_to_draw(&image, Rect::new(0.0, 0.0, 32.0, 32.0), 0.25).is_some());
    }

    /// An image that is still enormous after sampling is left to the host,
    /// which draws its frame: a picture drawn at wall size has nothing to
    /// sample away, and inlining it per frame would be megabytes of JSON.
    #[test]
    fn an_image_too_large_even_when_sampled_is_left_to_the_host() {
        // Drawn as large as it is stored, so there is nothing to sample away
        // and the whole thing would have to travel.
        let side = 1400u32; // 1400^2 * 4 bytes is past the inline bound
        let mut painter = RecordingPainter::new();
        let pixels = vec![0u8; (side * side * 4) as usize];
        painter.draw_image(
            Rect::new(0.0, 0.0, side as f32, side as f32),
            &ImageData::new(side, side, &pixels),
        );
        let json = painter.to_json();
        assert!(json.contains(r#""op":"image""#), "its geometry still goes");
        assert!(
            !json.contains(r#""pixels":""#),
            "past the bound the host draws the frame instead"
        );
    }

    /// The same mask twice costs one copy of the pixels.
    #[test]
    fn a_repeated_mask_is_sent_once() {
        let mut painter = RecordingPainter::new();
        let pixels = vec![7u8; 64];
        for _ in 0..5 {
            painter.draw_image(
                Rect::new(0.0, 0.0, 4.0, 4.0),
                &ImageData::new(4, 4, &pixels),
            );
        }
        let json = painter.to_json();
        assert_eq!(json.matches(r#""op":"image""#).count(), 5);
        assert_eq!(
            json.matches(r#""pixels":""#).count(),
            1,
            "the mask should be sent once and referenced after that"
        );
    }
}

#[cfg(test)]
mod recording_size {
    use super::*;

    #[test]
    #[ignore = "diagnostic"]
    fn report_recording_size() {
        let Ok(path) = std::env::var("APDF_RENDER_PDF") else {
            return;
        };
        let bytes = std::fs::read(&path).expect("read");
        let session = crate::session::Session::open(bytes).expect("open");
        let list = session.display_list().expect("geometry");
        let area = Rect::new(0.0, 0.0, 900.0, 1200.0);
        let mut painter = RecordingPainter::new();
        paint_page(
            &mut painter,
            &list,
            Transform::fit(&list, area, 1.0),
            &[],
            session.fonts(),
        );
        let json = painter.to_json();
        eprintln!(
            "recording {} KiB, {} glyphs, {} masks inlined",
            json.len() / 1024,
            json.matches(r#""op":"image""#).count(),
            json.matches(r#""pixels":""#).count()
        );
    }
}

#[cfg(test)]
mod ligature_probe {
    use super::*;

    #[test]
    #[ignore]
    fn unresolved_codes() {
        let path = std::env::var("APDF_RENDER_PDF").unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let session = crate::session::Session::open(bytes).unwrap();
        let list = session.display_list().unwrap();
        let fonts = session.fonts();
        let mut missing: std::collections::BTreeMap<(String, u32, char), usize> =
            Default::default();
        for op in &list.ops {
            let RenderOp::Text {
                text, codes, font, ..
            } = op
            else {
                continue;
            };
            for (ch, code) in text.chars().zip(codes.iter()) {
                if ch.is_whitespace() {
                    continue;
                }
                if fonts.resolve(font, *code, Some(ch)).is_none() {
                    *missing.entry((font.clone(), *code, ch)).or_default() += 1;
                }
            }
        }
        for ((font, code, ch), n) in &missing {
            eprintln!("{font}: code {code} ({ch:?}) x{n}");
        }
        eprintln!("{} distinct unresolved", missing.len());
        for op in &list.ops {
            let RenderOp::Text {
                text,
                codes,
                advances,
                ..
            } = op
            else {
                continue;
            };
            if text.contains("cient") || text.contains("erent") {
                eprintln!("text: {text:?}");
                eprintln!("codes: {:?}", &codes[..codes.len().min(30)]);
                eprintln!("advs: {:?}", &advances[..advances.len().min(30)]);
                break;
            }
        }
    }
}
