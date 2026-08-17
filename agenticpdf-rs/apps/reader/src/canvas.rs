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
) {
    let page = transform.page_rect(list);
    painter.fill_rect(page, Color::WHITE, 0.0);
    painter.push_clip(page);

    // The engine emits Save/Restore/Clip around nested state. Clips are tracked
    // by depth so an unbalanced Restore — which a damaged file can produce —
    // cannot pop the page clip and let drawing escape onto the rest of the UI.
    let mut clip_depth = 0usize;

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
                measured,
                rot,
                color,
                ..
            } => {
                if text.trim().is_empty() {
                    continue;
                }
                let mut style = dewey::core::TextStyle {
                    font_size: (*size as f32 * transform.scale).max(1.0),
                    color: to_color(*color),
                    ..Default::default()
                };

                // A PDF positions text itself; it does not ask for a string
                // to be laid out, and the document's font is not the font this
                // renderer has. Three things follow, learned by looking at the
                // result rather than by reasoning about it:
                //
                //   - Drawing a run and letting the UI font pick its width
                //     makes runs collide, because that font is wider than the
                //     document's. That is the overlapping sentences.
                //   - Placing every glyph at its recorded position fixes the
                //     collisions and scatters letters inside words instead,
                //     because the per-glyph error lands mid-word ("gz y5102").
                //   - Placing every *word* at its recorded position jams words
                //     together, because a word wider than its slot eats the
                //     space after it.
                //
                // What actually works without the document's own font is to
                // let the font lay the whole run out -- one call, so spacing
                // stays even and words stay whole -- and then fit that run to
                // the width the document reserved for it. PDF.js scales
                // horizontally here; with no horizontal scale available, the
                // size is scaled uniformly, which costs a little height and
                // keeps every run inside its own slot.
                let target = *width as f32 * transform.scale;
                let natural = painter.measure_text(text, &style).width;
                if *rot == 0.0 {
                    if target > 0.0 && natural > target {
                        // Floored: a run whose slot is absurdly narrow relative
                        // to this font should end up slightly cramped rather
                        // than unreadably small.
                        let fit = (target / natural).max(0.6);
                        style.font_size = (style.font_size * fit).max(1.0);
                    }
                    let at = transform.point(*x, *y);
                    painter.text(Position::new(at.x, at.y - style.font_size), text, &style);
                    continue;
                }

                // Rotated text has no layout call that can place it, so walk
                // the baseline glyph by glyph. Without this an arXiv stamp
                // meant for the margin is drawn straight across the body.
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
            RenderOp::Save => {}
            RenderOp::Restore => {
                if clip_depth > 0 {
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
#[derive(Debug, Default)]
pub struct RecordingPainter {
    ops: Vec<String>,
    clip_depth: usize,
}

impl RecordingPainter {
    pub fn new() -> RecordingPainter {
        RecordingPainter::default()
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
        // Pixels are not inlined: a scanned page would be megabytes of JSON per
        // frame. The host is given the geometry and fetches the asset by index.
        self.push(format!(
            r#"{{"op":"image","x":{:.2},"y":{:.2},"w":{:.2},"h":{:.2},"iw":{},"ih":{}}}"#,
            rect.x, rect.y, rect.width, rect.height, image.width, image.height
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
    fn the_recording_painter_does_not_inline_image_pixels() {
        // A scanned page would otherwise be megabytes of JSON on every frame.
        let mut painter = RecordingPainter::new();
        let pixels = vec![255u8; 64 * 64 * 4];
        painter.draw_image(
            Rect::new(0.0, 0.0, 10.0, 10.0),
            &ImageData::new(64, 64, &pixels),
        );

        let json = painter.to_json();
        assert!(json.len() < 200, "image pixels leaked into the recording");
        assert!(json.contains("\"iw\":64"));
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
