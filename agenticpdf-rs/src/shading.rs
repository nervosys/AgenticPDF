// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shadings (PDF 8.7.4.5) reduced to flat-filled bands.
//!
//! A gradient has no representation in the display list, and giving it one
//! would mean teaching three renderers -- the desktop rasteriser, the WebGL
//! renderer and the browser replayer -- to draw it. Slicing it into bands of
//! constant colour needs none of that: the ops already exist, and at sixty-four
//! bands the seams are below the resolution anything is compared at.
//!
//! Axial and radial shadings are covered, and so are all four mesh types --
//! the free-form and lattice triangle meshes, and the Coons and tensor patch
//! meshes -- each sliced into small flat pieces for the same reason a gradient
//! is sliced into bands. Only the function-based type (1) is still declined.

use crate::engine::{Dict, Document, Object, decode_stream};

/// One flat-coloured piece of a gradient, in device space.
pub struct Band {
    pub points: Vec<[f64; 2]>,
    pub color: [f64; 4],
}

/// How finely a gradient is sliced.
///
/// The band edges are the visible artefact, and they scale with the gradient's
/// length rather than its area, so this is a resolution rather than a budget.
const BANDS: usize = 64;

/// How many segments approximate a circle in a radial shading.
const CIRCLE_STEPS: usize = 48;

/// Turn a shading dictionary into bands covering `clip`.
///
/// `matrix` maps the shading's own space to device space; `clip` is the device
/// rectangle the paint is confined to, which for `sh` is the current clip.
pub fn bands(doc: &Document, sh: &Dict, matrix: [f64; 6], clip: [f64; 4]) -> Vec<Band> {
    bands_of(doc, sh, &[], matrix, clip)
}

/// As [`bands`], with the shading stream's own bytes.
///
/// A mesh shading keeps its geometry in the stream rather than in the
/// dictionary, so it cannot be drawn from the dictionary alone. `raw` is empty
/// for the shadings that are wholly described by their dictionary.
pub fn bands_of(
    doc: &Document,
    sh: &Dict,
    raw: &[u8],
    matrix: [f64; 6],
    clip: [f64; 4],
) -> Vec<Band> {
    let kind = doc
        .get(sh, "ShadingType")
        .and_then(|o| o.as_int())
        .unwrap_or(0);
    let space = doc.get(sh, "ColorSpace").map(|o| doc.resolve(&o));
    let components = space.as_ref().map(|s| components_of(doc, s)).unwrap_or(3);
    if matches!(kind, 4 | 5) {
        return triangles(doc, sh, raw, matrix, components, kind == 5);
    }
    if matches!(kind, 6 | 7) {
        return patches(doc, sh, raw, matrix, components, kind == 7);
    }
    let Some(function) = doc.get(sh, "Function") else {
        return Vec::new();
    };
    let coords: Vec<f64> = doc
        .get(sh, "Coords")
        .and_then(|o| {
            o.as_array()
                .map(|a| a.iter().filter_map(|v| v.as_f64()).collect())
        })
        .unwrap_or_default();
    let domain = doc
        .get(sh, "Domain")
        .and_then(|o| {
            o.as_array()
                .map(|a| a.iter().filter_map(|v| v.as_f64()).collect::<Vec<_>>())
        })
        .filter(|d| d.len() == 2)
        .unwrap_or_else(|| vec![0.0, 1.0]);
    let extend = doc
        .get(sh, "Extend")
        .and_then(|o| {
            o.as_array().map(|a| {
                a.iter()
                    .map(|v| matches!(v, Object::Bool(true)))
                    .collect::<Vec<_>>()
            })
        })
        .unwrap_or_else(|| vec![false, false]);
    let (ext0, ext1) = (
        extend.first().copied().unwrap_or(false),
        extend.get(1).copied().unwrap_or(false),
    );

    let at = |s: f64| -> [f64; 4] {
        let t = domain[0] + s.clamp(0.0, 1.0) * (domain[1] - domain[0]);
        let v = eval(doc, &function, t, components).unwrap_or_default();
        to_rgb(&v)
    };

    match kind {
        2 if coords.len() >= 4 => axial(&coords, matrix, clip, ext0, ext1, &at),
        3 if coords.len() >= 6 => radial(&coords, matrix, clip, ext0, ext1, &at),
        _ => Vec::new(),
    }
}

fn axial(
    c: &[f64],
    m: [f64; 6],
    clip: [f64; 4],
    ext0: bool,
    ext1: bool,
    at: &dyn Fn(f64) -> [f64; 4],
) -> Vec<Band> {
    let p0 = point(&m, c[0], c[1]);
    let p1 = point(&m, c[2], c[3]);
    let d = [p1[0] - p0[0], p1[1] - p0[1]];
    let len2 = d[0] * d[0] + d[1] * d[1];
    if len2 <= f64::EPSILON {
        return Vec::new();
    }
    // The axis parameter of a device point, so a band is the region between two
    // values of it. Working in device space keeps the clip rectangle a
    // rectangle; doing it in shading space would need the inverse matrix and a
    // sheared clip.
    let u = move |p: [f64; 2]| ((p[0] - p0[0]) * d[0] + (p[1] - p0[1]) * d[1]) / len2;

    let rect = rect_polygon(clip);
    let mut out = Vec::with_capacity(BANDS);
    for i in 0..BANDS {
        let (a, b) = (i as f64 / BANDS as f64, (i + 1) as f64 / BANDS as f64);
        let lo = if i == 0 && ext0 { f64::NEG_INFINITY } else { a };
        let hi = if i + 1 == BANDS && ext1 {
            f64::INFINITY
        } else {
            b
        };
        let mut poly = clip_half(&rect, &u, lo, true);
        poly = clip_half(&poly, &u, hi, false);
        if poly.len() >= 3 {
            out.push(Band {
                points: poly,
                color: at((a + b) / 2.0),
            });
        }
    }
    out
}

fn radial(
    c: &[f64],
    m: [f64; 6],
    clip: [f64; 4],
    ext0: bool,
    ext1: bool,
    at: &dyn Fn(f64) -> [f64; 4],
) -> Vec<Band> {
    // Concentric discs painted largest first, so each smaller one covers the
    // middle of the last. That is what the specification's own description of
    // the radial type amounts to, and it needs no annulus geometry.
    let scale = ((m[0] * m[3] - m[1] * m[2]).abs()).sqrt();
    let mut out = Vec::new();
    let growing = c[5] >= c[2];
    if (growing && ext1) || (!growing && ext0) {
        // Beyond the larger circle the paint extends over everything.
        out.push(Band {
            points: rect_polygon(clip),
            color: at(if growing { 1.0 } else { 0.0 }),
        });
    }
    for i in 0..=BANDS {
        // Largest first: reverse when the gradient shrinks.
        let s = if growing {
            1.0 - i as f64 / BANDS as f64
        } else {
            i as f64 / BANDS as f64
        };
        let cx = c[0] + s * (c[3] - c[0]);
        let cy = c[1] + s * (c[4] - c[1]);
        let r = (c[2] + s * (c[5] - c[2])) * scale;
        if r <= 0.0 {
            continue;
        }
        let centre = point(&m, cx, cy);
        let mut poly = Vec::with_capacity(CIRCLE_STEPS);
        for k in 0..CIRCLE_STEPS {
            let a = k as f64 * std::f64::consts::TAU / CIRCLE_STEPS as f64;
            poly.push([centre[0] + r * a.cos(), centre[1] + r * a.sin()]);
        }
        let poly = clip_rect(&poly, clip);
        if poly.len() >= 3 {
            out.push(Band {
                points: poly,
                color: at(s),
            });
        }
    }
    out
}

/// Trim a polygon to a rectangle, edge by edge.
///
/// A disc is not otherwise bounded by anything: the clip is what says how much
/// of the gradient the page actually shows, and a disc drawn past it paints
/// over content that should be on top.
fn clip_rect(poly: &[[f64; 2]], r: [f64; 4]) -> Vec<[f64; 2]> {
    let mut out = poly.to_vec();
    for (axis, bound, keep_above) in [
        (0usize, r[0], true),
        (0, r[2], false),
        (1, r[1], true),
        (1, r[3], false),
    ] {
        if !bound.is_finite() || out.is_empty() {
            continue;
        }
        let u = |p: [f64; 2]| p[axis];
        out = clip_half(&out, &u, bound, keep_above);
    }
    out
}

/// Trim `subject` to the inside of a convex polygon.
///
/// Every band a shading produces is convex -- a rectangle cut by half-planes,
/// or a disc -- which is what lets one clip the other with a fixed number of
/// half-plane passes and no general polygon library.
pub fn clip_to_convex(subject: &[[f64; 2]], convex: &[[f64; 2]]) -> Vec<[f64; 2]> {
    if convex.len() < 3 {
        return Vec::new();
    }
    // Which side is "inside" depends on the winding, so take it from the
    // polygon itself rather than assuming one.
    let area: f64 = (0..convex.len())
        .map(|i| {
            let (a, b) = (convex[i], convex[(i + 1) % convex.len()]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum();
    let sign = if area >= 0.0 { 1.0 } else { -1.0 };
    let mut out = subject.to_vec();
    for i in 0..convex.len() {
        if out.is_empty() {
            break;
        }
        let (a, b) = (convex[i], convex[(i + 1) % convex.len()]);
        let (ex, ey) = (b[0] - a[0], b[1] - a[1]);
        // Signed distance to the edge line, positive inside.
        let side = move |p: [f64; 2]| sign * (ex * (p[1] - a[1]) - ey * (p[0] - a[0]));
        let mut next = Vec::with_capacity(out.len() + 2);
        for k in 0..out.len() {
            let (p, q) = (out[k], out[(k + 1) % out.len()]);
            let (sp, sq) = (side(p), side(q));
            if sp >= 0.0 {
                next.push(p);
            }
            if (sp >= 0.0) != (sq >= 0.0) {
                let denom = sp - sq;
                if denom.abs() > f64::EPSILON {
                    let t = sp / denom;
                    next.push([p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])]);
                }
            }
        }
        out = next;
    }
    out
}

/// The coverage a luminosity mask gives, from the colour it painted.
pub fn luminosity(color: [f64; 4]) -> f64 {
    (0.3 * color[0] + 0.59 * color[1] + 0.11 * color[2]).clamp(0.0, 1.0)
}

fn rect_polygon(r: [f64; 4]) -> Vec<[f64; 2]> {
    vec![[r[0], r[1]], [r[2], r[1]], [r[2], r[3]], [r[0], r[3]]]
}

/// Sutherland-Hodgman against one half-plane of the axis parameter.
fn clip_half(
    poly: &[[f64; 2]],
    u: &dyn Fn([f64; 2]) -> f64,
    bound: f64,
    keep_above: bool,
) -> Vec<[f64; 2]> {
    if !bound.is_finite() {
        return poly.to_vec();
    }
    let inside = |p: [f64; 2]| {
        let v = u(p);
        if keep_above { v >= bound } else { v <= bound }
    };
    let mut out = Vec::with_capacity(poly.len() + 2);
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        let (ia, ib) = (inside(a), inside(b));
        if ia {
            out.push(a);
        }
        if ia != ib {
            let (ua, ub) = (u(a), u(b));
            let denom = ub - ua;
            if denom.abs() > f64::EPSILON {
                let t = (bound - ua) / denom;
                out.push([a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]);
            }
        }
    }
    out
}

/// How finely each patch is diced: `N` across and `N` down, so N-squared quads.
const PATCH_STEPS: usize = 6;

/// How finely each triangle is diced: `N` along each edge, so N-squared
/// sub-triangles -- the same count a patch gets, and for the same reason.
const TRIANGLE_STEPS: usize = 6;

/// A ceiling on the quads one mesh may produce, so a document with thousands of
/// patches cannot turn a single `sh` into a display list nothing can draw.
const MAX_PATCH_QUADS: usize = 60_000;

/// A reader for big-endian bit fields, which is how a mesh packs its numbers.
struct Bits<'a> {
    data: &'a [u8],
    at: usize,
}

impl<'a> Bits<'a> {
    fn new(data: &'a [u8]) -> Self {
        Bits { data, at: 0 }
    }

    fn take(&mut self, bits: usize) -> Option<u64> {
        if bits == 0 || bits > 32 || self.at + bits > self.data.len() * 8 {
            return None;
        }
        let mut out: u64 = 0;
        for _ in 0..bits {
            let byte = self.data[self.at / 8];
            let bit = (byte >> (7 - self.at % 8)) & 1;
            out = (out << 1) | bit as u64;
            self.at += 1;
        }
        Some(out)
    }

    /// Advance to the next byte boundary.
    ///
    /// A free-form triangle mesh pads every vertex out to a whole number of
    /// bytes (PDF 8.7.4.5.5); the lattice and patch meshes do not pad at all.
    /// That is the only structural difference between reading a type 4 and a
    /// type 5, and getting it wrong does not fail loudly -- it shears every
    /// vertex after the first by a few bits and draws confetti.
    fn align(&mut self) {
        self.at = self.at.div_ceil(8) * 8;
    }

    fn done(&self) -> bool {
        self.at + 8 > self.data.len() * 8
    }
}

/// The header every mesh shading is read through, and the stream it reads.
///
/// Types 4 through 7 differ in what they do with the numbers, not in how the
/// numbers are packed: the same bit widths, the same `/Decode` ranges, and the
/// same choice between one parametric value put through `/Function` and colour
/// components carried directly.
struct Mesh {
    data: Vec<u8>,
    coord_bits: usize,
    comp_bits: usize,
    flag_bits: usize,
    decode: Vec<f64>,
    function: Option<Object>,
    /// Numbers per vertex or corner: one with a `/Function`, else the colour
    /// space's component count.
    per_corner: usize,
    components: usize,
    coord_max: f64,
    comp_max: f64,
}

impl Mesh {
    /// Read the header, or decline the shading.
    ///
    /// `flagged` is false only for the lattice-form mesh (type 5), the one mesh
    /// with no per-vertex flag and therefore no `/BitsPerFlag`. Requiring the
    /// key there would decline every valid type 5 there is.
    fn read(
        doc: &Document,
        sh: &Dict,
        raw: &[u8],
        components: usize,
        flagged: bool,
    ) -> Option<Mesh> {
        let data = decode_stream(sh, raw).ok()?;
        let int = |key: &str| {
            doc.get(sh, key)
                .and_then(|o| o.as_int())
                .map(|v| v as usize)
        };
        let coord_bits = int("BitsPerCoordinate")?;
        let comp_bits = int("BitsPerComponent")?;
        let flag_bits = match flagged {
            true => int("BitsPerFlag")?,
            false => 0,
        };
        let decode: Vec<f64> = doc
            .get(sh, "Decode")
            .and_then(|o| {
                o.as_array()
                    .map(|a| a.iter().filter_map(|v| doc.resolve(v).as_f64()).collect())
            })
            .unwrap_or_default();
        // A `/Function` maps one parametric value to a colour; without it each
        // vertex carries its colour components directly.
        let function = doc.get(sh, "Function");
        let per_corner = match function.is_some() {
            true => 1,
            false => components,
        };
        if decode.len() < 4 + per_corner * 2 || coord_bits > 32 || comp_bits > 32 {
            return None;
        }
        Some(Mesh {
            data,
            coord_bits,
            comp_bits,
            flag_bits,
            decode,
            function,
            per_corner,
            components,
            coord_max: ((1u64 << coord_bits) - 1) as f64,
            comp_max: ((1u64 << comp_bits) - 1) as f64,
        })
    }

    /// One vertex position, mapped through `/Decode` and then to device space.
    fn point(&self, bits: &mut Bits, matrix: &[f64; 6]) -> Option<[f64; 2]> {
        let (xr, yr) = (bits.take(self.coord_bits)?, bits.take(self.coord_bits)?);
        let x = self.decode[0] + (xr as f64 / self.coord_max) * (self.decode[1] - self.decode[0]);
        let y = self.decode[2] + (yr as f64 / self.coord_max) * (self.decode[3] - self.decode[2]);
        Some(point(matrix, x, y))
    }

    /// One vertex or corner colour, as RGBA.
    fn color(&self, doc: &Document, bits: &mut Bits) -> Option<[f64; 4]> {
        let mut v = Vec::with_capacity(self.per_corner);
        for k in 0..self.per_corner {
            let cr = bits.take(self.comp_bits)?;
            let (lo, hi) = (self.decode[4 + k * 2], self.decode[5 + k * 2]);
            v.push(lo + (cr as f64 / self.comp_max) * (hi - lo));
        }
        Some(match &self.function {
            Some(f) => to_rgb(&eval(doc, f, v[0], self.components).unwrap_or_default()),
            None => to_rgb(&v),
        })
    }
}

/// A patch's twelve boundary control points and its four corner colours: what
/// the next patch in a strip may inherit an edge from.
type Patch = ([[f64; 2]; 12], [[f64; 4]; 4]);

/// One vertex of a triangle mesh: where it is, and what colour it is there.
type Vertex = ([f64; 2], [f64; 4]);

/// A cubic Bezier through four control points.
fn bezier(p: [[f64; 2]; 4], t: f64) -> [f64; 2] {
    let u = 1.0 - t;
    let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    [
        a * p[0][0] + b * p[1][0] + c * p[2][0] + d * p[3][0],
        a * p[0][1] + b * p[1][1] + c * p[2][1] + d * p[3][1],
    ]
}

/// Slice one Gouraud-shaded triangle into flat-coloured sub-triangles.
///
/// The corners carry three colours and the interior is their barycentric blend,
/// which the display list has no way to express -- so the triangle is diced the
/// way a gradient is sliced into bands, and each piece takes the colour at its
/// own centroid. At six steps a triangle becomes thirty-six pieces: twenty-one
/// pointing the same way as the original and fifteen pointing against it, which
/// together tile it exactly, with no seam and no overlap.
fn dice(tri: [Vertex; 3], out: &mut Vec<Band>) {
    let at = |u: f64, v: f64| -> [f64; 2] {
        let w = 1.0 - u - v;
        [
            u * tri[0].0[0] + v * tri[1].0[0] + w * tri[2].0[0],
            u * tri[0].0[1] + v * tri[1].0[1] + w * tri[2].0[1],
        ]
    };
    let shade = |u: f64, v: f64| -> [f64; 4] {
        let w = 1.0 - u - v;
        let mut c = [0.0; 4];
        for (k, channel) in c.iter_mut().enumerate() {
            *channel = u * tri[0].1[k] + v * tri[1].1[k] + w * tri[2].1[k];
        }
        c
    };
    let n = TRIANGLE_STEPS;
    let s = 1.0 / n as f64;
    for i in 0..n {
        for j in 0..(n - i) {
            let (u0, u1) = (i as f64 * s, (i + 1) as f64 * s);
            let (v0, v1) = (j as f64 * s, (j + 1) as f64 * s);
            out.push(Band {
                points: vec![at(u0, v0), at(u1, v0), at(u0, v1)],
                color: shade((2.0 * u0 + u1) / 3.0, (2.0 * v0 + v1) / 3.0),
            });
            if i + j + 2 <= n {
                out.push(Band {
                    points: vec![at(u1, v0), at(u1, v1), at(u0, v1)],
                    color: shade((u0 + 2.0 * u1) / 3.0, (v0 + 2.0 * v1) / 3.0),
                });
            }
        }
    }
}

/// Slice a free-form (type 4) or lattice-form (type 5) triangle mesh.
///
/// Both describe the same surface, written two ways. A free-form mesh gives
/// every vertex a flag saying which two vertices of the triangle before it this
/// one joins -- so a strip costs one vertex per triangle rather than three --
/// and a lattice-form mesh drops the flags entirely, saying instead how many
/// vertices make a row, with the triangles implied between consecutive rows.
///
/// Neither appears anywhere in this project's measurement corpus, so **nothing
/// here is backed by a score**, and no sweep can be cited for it. That is why
/// the tests are synthetic and check decoded geometry directly: a corpus that
/// does not exercise a path cannot be evidence that the path is right, and a
/// flat sweep after this change means only that the corpus was silent.
fn triangles(
    doc: &Document,
    sh: &Dict,
    raw: &[u8],
    matrix: [f64; 6],
    components: usize,
    lattice: bool,
) -> Vec<Band> {
    let Some(mesh) = Mesh::read(doc, sh, raw, components, !lattice) else {
        return Vec::new();
    };
    let mut bits = Bits::new(&mesh.data);
    let mut out: Vec<Band> = Vec::new();

    if lattice {
        // `/VerticesPerRow` is what makes a lattice mesh readable at all: with
        // no flags, the row width is the only thing saying where a row ends.
        // Two is the narrowest row that has a triangle in it.
        let per_row = doc
            .get(sh, "VerticesPerRow")
            .and_then(|o| o.as_int())
            .unwrap_or(0);
        if per_row < 2 {
            return Vec::new();
        }
        let per_row = per_row as usize;
        let mut previous: Option<Vec<Vertex>> = None;
        while !bits.done() && out.len() < MAX_PATCH_QUADS {
            let mut row = Vec::with_capacity(per_row);
            for _ in 0..per_row {
                let (Some(p), Some(c)) =
                    (mesh.point(&mut bits, &matrix), mesh.color(doc, &mut bits))
                else {
                    return out;
                };
                row.push((p, c));
            }
            if let Some(above) = &previous {
                for i in 0..per_row - 1 {
                    dice([above[i], above[i + 1], row[i]], &mut out);
                    dice([above[i + 1], row[i + 1], row[i]], &mut out);
                    if out.len() >= MAX_PATCH_QUADS {
                        return out;
                    }
                }
            }
            previous = Some(row);
        }
        return out;
    }

    // Free-form. `tri` is the triangle last completed; a flag of 1 or 2 keeps
    // two of its vertices and replaces the third, and a flag of 0 abandons it
    // and starts a fresh one from this vertex and the next two.
    let mut tri: Vec<Vertex> = Vec::new();
    while !bits.done() && out.len() < MAX_PATCH_QUADS {
        let Some(flag) = bits.take(mesh.flag_bits) else {
            break;
        };
        let (Some(p), Some(c)) = (mesh.point(&mut bits, &matrix), mesh.color(doc, &mut bits))
        else {
            break;
        };
        bits.align();
        let vertex = (p, c);
        if tri.len() == 3 {
            tri = match flag {
                1 => vec![tri[1], tri[2], vertex],
                2 => vec![tri[0], tri[2], vertex],
                _ => vec![vertex],
            };
        } else {
            // Mid-triangle, where the spec says the flag repeats the one that
            // opened it. A file that says otherwise is read as continuing
            // rather than declined: the vertex is present either way.
            tri.push(vertex);
        }
        if tri.len() == 3 {
            dice([tri[0], tri[1], tri[2]], &mut out);
        }
    }
    out
}

/// Slice a Coons or tensor patch mesh into small flat-coloured quads.
///
/// A patch is bounded by four cubic Beziers with a colour at each corner, and
/// the surface between them is the Coons interpolation of those edges. The four
/// extra control points a tensor patch (type 7) adds bend the interior; they are
/// read past rather than used, which shows only where a patch's interior is
/// pulled far from the surface its own edges describe. That is not what these
/// are used for -- they are used for smooth backgrounds, and a background that
/// is nearly right is worth far more than one that is absent. The ebook that
/// prompted this sets its table of contents in white on such a background, so
/// declining the mesh made the gradient and thirty-four lines of text disappear
/// together, and the page still scored only 0.872 for losing both.
fn patches(
    doc: &Document,
    sh: &Dict,
    raw: &[u8],
    matrix: [f64; 6],
    components: usize,
    tensor: bool,
) -> Vec<Band> {
    let Some(mesh) = Mesh::read(doc, sh, raw, components, true) else {
        return Vec::new();
    };
    let mut bits = Bits::new(&mesh.data);
    let mut out: Vec<Band> = Vec::new();
    // The previous patch's boundary and corner colours. A mesh is written as a
    // strip: every patch after the first names which edge of the one before it
    // it grows from, and carries only the points and colours that adds.
    let mut prev: Option<Patch> = None;

    while !bits.done() && out.len() < MAX_PATCH_QUADS {
        let Some(flag) = bits.take(mesh.flag_bits) else {
            break;
        };
        let shared = flag != 0 && prev.is_some();
        let new_points = match (tensor, shared) {
            (true, false) => 16,
            (true, true) => 12,
            (false, false) => 12,
            (false, true) => 8,
        };
        let new_colors = if shared { 2 } else { 4 };

        let mut read = Vec::with_capacity(new_points);
        for _ in 0..new_points {
            let Some(p) = mesh.point(&mut bits, &matrix) else {
                return out;
            };
            read.push(p);
        }
        let mut colors = Vec::with_capacity(new_colors);
        for _ in 0..new_colors {
            let Some(c) = mesh.color(doc, &mut bits) else {
                return out;
            };
            colors.push(c);
        }

        // The twelve boundary points, clockwise from the first corner, with the
        // four corner colours to go with them.
        let (boundary, corners) = match (shared, prev) {
            (false, _) => {
                let mut b = [[0.0f64; 2]; 12];
                b.copy_from_slice(&read[..12]);
                let mut c = [[0.0f64; 4]; 4];
                c.copy_from_slice(&colors[..4]);
                (b, c)
            }
            (true, Some((pb, pc))) => {
                let (edge, first, second) = match flag {
                    1 => ([pb[3], pb[4], pb[5], pb[6]], pc[1], pc[2]),
                    2 => ([pb[6], pb[7], pb[8], pb[9]], pc[2], pc[3]),
                    _ => ([pb[9], pb[10], pb[11], pb[0]], pc[3], pc[0]),
                };
                let mut b = [[0.0f64; 2]; 12];
                b[..4].copy_from_slice(&edge);
                b[4..12].copy_from_slice(&read[..8]);
                (b, [first, second, colors[0], colors[1]])
            }
            (true, None) => return out,
        };
        prev = Some((boundary, corners));

        let top = [boundary[0], boundary[1], boundary[2], boundary[3]];
        let right = [boundary[3], boundary[4], boundary[5], boundary[6]];
        let bottom = [boundary[9], boundary[8], boundary[7], boundary[6]];
        let left = [boundary[0], boundary[11], boundary[10], boundary[9]];
        let corner = [boundary[0], boundary[3], boundary[6], boundary[9]];
        let at = |u: f64, v: f64| -> [f64; 2] {
            let t = bezier(top, u);
            let b = bezier(bottom, u);
            let l = bezier(left, v);
            let r = bezier(right, v);
            let mut p = [0.0; 2];
            for k in 0..2 {
                let edges = (1.0 - v) * t[k] + v * b[k] + (1.0 - u) * l[k] + u * r[k];
                let bilinear = (1.0 - u) * (1.0 - v) * corner[0][k]
                    + u * (1.0 - v) * corner[1][k]
                    + u * v * corner[2][k]
                    + (1.0 - u) * v * corner[3][k];
                p[k] = edges - bilinear;
            }
            p
        };
        let shade = |u: f64, v: f64| -> [f64; 4] {
            let mut c = [0.0; 4];
            for k in 0..4 {
                c[k] = (1.0 - u) * (1.0 - v) * corners[0][k]
                    + u * (1.0 - v) * corners[1][k]
                    + u * v * corners[2][k]
                    + (1.0 - u) * v * corners[3][k];
            }
            c
        };

        let step = 1.0 / PATCH_STEPS as f64;
        for i in 0..PATCH_STEPS {
            for j in 0..PATCH_STEPS {
                let (u0, u1) = (i as f64 * step, (i + 1) as f64 * step);
                let (v0, v1) = (j as f64 * step, (j + 1) as f64 * step);
                out.push(Band {
                    points: vec![at(u0, v0), at(u1, v0), at(u1, v1), at(u0, v1)],
                    color: shade((u0 + u1) / 2.0, (v0 + v1) / 2.0),
                });
                if out.len() >= MAX_PATCH_QUADS {
                    return out;
                }
            }
        }
    }
    out
}

fn point(m: &[f64; 6], x: f64, y: f64) -> [f64; 2] {
    [m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]]
}

fn components_of(doc: &Document, cs: &Object) -> usize {
    match cs {
        Object::Name(n) => match n.as_str() {
            "DeviceGray" | "CalGray" | "G" => 1,
            "DeviceCMYK" | "CMYK" => 4,
            _ => 3,
        },
        Object::Array(a) => match a.first().and_then(|o| o.as_name()) {
            Some("ICCBased") => a
                .get(1)
                .map(|o| doc.resolve(o))
                .and_then(|s| {
                    s.as_dict()
                        .and_then(|sd| doc.get(sd, "N"))
                        .and_then(|o| o.as_int())
                })
                .unwrap_or(3) as usize,
            Some("CalGray") => 1,
            Some("DeviceN") => a
                .get(1)
                .and_then(|o| o.as_array())
                .map(|n| n.len())
                .unwrap_or(1),
            Some("Separation") => 1,
            _ => 3,
        },
        _ => 3,
    }
}

/// Component values to RGBA, by count, matching how the interpreter reads a
/// colour operand.
fn to_rgb(v: &[f64]) -> [f64; 4] {
    match v.len() {
        0 => [0.0, 0.0, 0.0, 1.0],
        1 => [v[0], v[0], v[0], 1.0],
        3 => [v[0], v[1], v[2], 1.0],
        4 => [
            (1.0 - v[0]) * (1.0 - v[3]),
            (1.0 - v[1]) * (1.0 - v[3]),
            (1.0 - v[2]) * (1.0 - v[3]),
            1.0,
        ],
        // A Separation or DeviceN tint: full tint is full ink, so it darkens.
        _ => {
            let ink = v.iter().cloned().fold(0.0f64, f64::max).clamp(0.0, 1.0);
            [1.0 - ink, 1.0 - ink, 1.0 - ink, 1.0]
        }
    }
}

/// Evaluate a PDF function of one input at `t`.
///
/// Types 0, 2 and 3 are here. Type 4 is a PostScript calculator -- a small
/// stack language -- and is declined; a shading whose function cannot be
/// evaluated produces no bands rather than a wrong colour.
pub fn eval(doc: &Document, f: &Object, t: f64, want: usize) -> Option<Vec<f64>> {
    // An array of n functions, each giving one component.
    if let Some(list) = f.as_array()
        && list
            .first()
            .map(|o| doc.resolve(o).as_dict().is_some())
            .unwrap_or(false)
    {
        let mut out = Vec::with_capacity(list.len());
        for item in list {
            out.extend(eval(doc, item, t, 1)?);
        }
        return Some(out);
    }
    let resolved = doc.resolve(f);
    let dict = resolved.as_dict()?;
    let kind = doc.get(dict, "FunctionType").and_then(|o| o.as_int())?;
    let domain = numbers(doc, dict, "Domain").unwrap_or_else(|| vec![0.0, 1.0]);
    let t = t.clamp(
        domain.first().copied().unwrap_or(0.0),
        domain.get(1).copied().unwrap_or(1.0),
    );
    match kind {
        2 => {
            let c0 = numbers(doc, dict, "C0").unwrap_or_else(|| vec![0.0]);
            let c1 = numbers(doc, dict, "C1").unwrap_or_else(|| vec![1.0]);
            let n = doc.get(dict, "N").and_then(|o| o.as_f64()).unwrap_or(1.0);
            let span = (domain[1] - domain[0]).abs().max(f64::EPSILON);
            let x = ((t - domain[0]) / span).clamp(0.0, 1.0).powf(n);
            Some(
                c0.iter()
                    .zip(c1.iter().chain(std::iter::repeat(&0.0)))
                    .map(|(a, b)| a + x * (b - a))
                    .collect(),
            )
        }
        3 => {
            let functions = doc.get(dict, "Functions")?;
            let list = functions.as_array()?;
            let bounds = numbers(doc, dict, "Bounds").unwrap_or_default();
            let encode = numbers(doc, dict, "Encode").unwrap_or_default();
            let mut k = 0usize;
            while k < bounds.len() && t >= bounds[k] {
                k += 1;
            }
            let lo = if k == 0 { domain[0] } else { bounds[k - 1] };
            let hi = if k == bounds.len() {
                domain[1]
            } else {
                bounds[k]
            };
            let (e0, e1) = (
                encode.get(2 * k).copied().unwrap_or(0.0),
                encode.get(2 * k + 1).copied().unwrap_or(1.0),
            );
            let span = (hi - lo).abs().max(f64::EPSILON);
            let inner = e0 + ((t - lo) / span) * (e1 - e0);
            eval(doc, list.get(k)?, inner, want)
        }
        0 => sampled(doc, &resolved, t, &domain, want),
        _ => None,
    }
}

/// A sampled function, one input, linearly interpolated between neighbours.
fn sampled(doc: &Document, obj: &Object, t: f64, domain: &[f64], want: usize) -> Option<Vec<f64>> {
    let Object::Stream(dict, raw) = obj else {
        return None;
    };
    let data = decode_stream(dict, raw).ok()?;
    let size = numbers(doc, dict, "Size")?;
    let n = *size.first()? as usize;
    if n < 1 {
        return None;
    }
    let bits = doc.get(dict, "BitsPerSample").and_then(|o| o.as_int())? as usize;
    if !matches!(bits, 1 | 2 | 4 | 8 | 16 | 24 | 32) {
        return None;
    }
    let range = numbers(doc, dict, "Range")?;
    let outputs = (range.len() / 2).clamp(1, 32);
    let encode = numbers(doc, dict, "Encode").unwrap_or_else(|| vec![0.0, (n - 1) as f64]);
    let span = (domain[1] - domain[0]).abs().max(f64::EPSILON);
    let e = encode[0] + ((t - domain[0]) / span) * (encode[1] - encode[0]);
    let e = e.clamp(0.0, (n - 1) as f64);
    let (i0, frac) = (e.floor() as usize, e - e.floor());
    let i1 = (i0 + 1).min(n - 1);

    let max = ((1u64 << bits) - 1) as f64;
    let read = |index: usize, out: usize| -> f64 {
        let bit = (index * outputs + out) * bits;
        let mut v = 0u64;
        for k in 0..bits {
            let b = bit + k;
            let byte = match data.get(b / 8) {
                Some(x) => *x,
                None => return 0.0,
            };
            v = (v << 1) | ((byte >> (7 - b % 8)) & 1) as u64;
        }
        v as f64 / max
    };

    let mut out = Vec::with_capacity(outputs);
    for k in 0..outputs {
        let a = read(i0, k);
        let b = read(i1, k);
        let raw = a + frac * (b - a);
        // Decode defaults to Range, so a sample maps onto its output interval.
        let (lo, hi) = (
            range.get(2 * k).copied().unwrap_or(0.0),
            range.get(2 * k + 1).copied().unwrap_or(1.0),
        );
        out.push(lo + raw * (hi - lo));
    }
    if want > 0 && out.len() > want {
        out.truncate(want);
    }
    Some(out)
}

fn numbers(doc: &Document, d: &Dict, key: &str) -> Option<Vec<f64>> {
    let v = doc.get(d, key)?;
    let arr = doc.resolve(&v);
    Some(arr.as_array()?.iter().filter_map(|o| o.as_f64()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A half-plane clip keeps the part of the rectangle it is told to.
    #[test]
    fn a_band_is_the_slice_of_the_clip_between_two_bounds() {
        let rect = rect_polygon([0.0, 0.0, 10.0, 10.0]);
        // Axis parameter is simply x/10.
        let u = |p: [f64; 2]| p[0] / 10.0;
        let left = clip_half(&rect, &u, 0.5, false);
        assert!(left.iter().all(|p| p[0] <= 5.0 + 1e-9), "{left:?}");
        let strip = clip_half(&left, &u, 0.25, true);
        assert!(
            strip
                .iter()
                .all(|p| (2.5 - 1e-9..=5.0 + 1e-9).contains(&p[0])),
            "{strip:?}"
        );
        // A closed strip of a rectangle is still a quadrilateral.
        assert_eq!(strip.len(), 4, "{strip:?}");
    }

    /// Trimming to a convex polygon keeps the overlap and nothing else.
    #[test]
    fn a_polygon_trimmed_to_a_convex_one_keeps_the_overlap() {
        let subject = rect_polygon([0.0, 0.0, 10.0, 10.0]);
        let window = rect_polygon([5.0, 5.0, 20.0, 20.0]);
        let cut = clip_to_convex(&subject, &window);
        assert_eq!(cut.len(), 4, "{cut:?}");
        for p in &cut {
            assert!(p[0] >= 5.0 - 1e-9 && p[1] >= 5.0 - 1e-9, "{cut:?}");
            assert!(p[0] <= 10.0 + 1e-9 && p[1] <= 10.0 + 1e-9, "{cut:?}");
        }
        // No overlap leaves nothing, rather than the whole subject.
        let away = rect_polygon([50.0, 50.0, 60.0, 60.0]);
        assert!(clip_to_convex(&subject, &away).is_empty());
    }

    /// Winding must not decide what survives.
    #[test]
    fn trimming_does_not_depend_on_the_winding() {
        let subject = rect_polygon([0.0, 0.0, 10.0, 10.0]);
        let mut window = rect_polygon([5.0, 5.0, 20.0, 20.0]);
        let a = clip_to_convex(&subject, &window).len();
        window.reverse();
        let b = clip_to_convex(&subject, &window).len();
        assert_eq!(a, b, "reversing the clip polygon changed the result");
        assert_eq!(a, 4);
    }

    /// An infinite bound is the extend case and must not clip at all.
    #[test]
    fn an_extended_end_covers_everything_before_it() {
        let rect = rect_polygon([0.0, 0.0, 10.0, 10.0]);
        let u = |p: [f64; 2]| p[0] / 10.0;
        let all = clip_half(&rect, &u, f64::NEG_INFINITY, true);
        assert_eq!(all.len(), 4);
    }

    /// The exponential function interpolates between its two colours.
    #[test]
    fn an_exponential_function_runs_from_c0_to_c1() {
        let mut d: Dict = Default::default();
        d.insert("FunctionType".into(), Object::Int(2));
        d.insert(
            "C0".into(),
            Object::Array(vec![
                Object::Real(1.0),
                Object::Real(0.0),
                Object::Real(0.0),
            ]),
        );
        d.insert(
            "C1".into(),
            Object::Array(vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(1.0),
            ]),
        );
        d.insert("N".into(), Object::Int(1));
        let doc = Document::empty();
        let f = Object::Dict(d);
        let start = eval(&doc, &f, 0.0, 3).expect("evaluates");
        let end = eval(&doc, &f, 1.0, 3).expect("evaluates");
        let mid = eval(&doc, &f, 0.5, 3).expect("evaluates");
        assert!(
            (start[0] - 1.0).abs() < 1e-9 && start[2] < 1e-9,
            "{start:?}"
        );
        assert!(end[0] < 1e-9 && (end[2] - 1.0).abs() < 1e-9, "{end:?}");
        assert!(
            (mid[0] - 0.5).abs() < 1e-9 && (mid[2] - 0.5).abs() < 1e-9,
            "{mid:?}"
        );
    }

    /// A shading whose function cannot be evaluated paints nothing.
    ///
    /// The alternative is a flat rectangle in whatever colour a failed
    /// evaluation happens to return, which hides a real gap behind a plausible
    /// block of colour.
    #[test]
    fn an_unsupported_shading_paints_nothing() {
        let doc = Document::empty();
        let mut sh: Dict = Default::default();
        // Type 1: the function-based shading, the one type still declined.
        // This used to name type 4, which is now drawn -- a test asserting an
        // absence has to be re-pointed when the absence is filled, or it goes
        // on passing while asserting nothing.
        sh.insert("ShadingType".into(), Object::Int(1));
        sh.insert("Function".into(), Object::Int(0));
        assert!(
            bands(
                &doc,
                &sh,
                [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 10.0, 10.0]
            )
            .is_empty()
        );
    }

    /// Pack bytes for a mesh with 8-bit coordinates and 8-bit components.
    ///
    /// Eight bits everywhere means one vertex is a whole number of bytes
    /// already, so a type 4's padding is invisible here. That is deliberate:
    /// these tests are about the triangle topology, and the padding gets its
    /// own test below where it can actually be wrong.
    fn mesh_dict(kind: i64, decode_hi: f64) -> Dict {
        let mut sh: Dict = Default::default();
        sh.insert("ShadingType".into(), Object::Int(kind));
        sh.insert("ColorSpace".into(), Object::Name("DeviceRGB".into()));
        sh.insert("BitsPerCoordinate".into(), Object::Int(8));
        sh.insert("BitsPerComponent".into(), Object::Int(8));
        sh.insert("BitsPerFlag".into(), Object::Int(8));
        sh.insert(
            "Decode".into(),
            Object::Array(vec![
                Object::Real(0.0),
                Object::Real(decode_hi),
                Object::Real(0.0),
                Object::Real(decode_hi),
                Object::Real(0.0),
                Object::Real(1.0),
                Object::Real(0.0),
                Object::Real(1.0),
                Object::Real(0.0),
                Object::Real(1.0),
            ]),
        );
        sh
    }

    /// The bounding box of every point a set of bands covers.
    fn extent(bands: &[Band]) -> [f64; 4] {
        let mut r = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
        for band in bands {
            for p in &band.points {
                r[0] = r[0].min(p[0]);
                r[1] = r[1].min(p[1]);
                r[2] = r[2].max(p[0]);
                r[3] = r[3].max(p[1]);
            }
        }
        r
    }

    /// A free-form triangle mesh is drawn, and drawn where it says.
    ///
    /// One triangle with red, green and blue corners over (0,0)-(255,255).
    /// The check that matters is the extent: a mis-read of the bit packing
    /// still produces bands, just in the wrong place, so counting them proves
    /// nothing on its own.
    #[test]
    fn a_free_form_triangle_mesh_is_drawn() {
        let doc = Document::empty();
        let sh = mesh_dict(4, 255.0);
        #[rustfmt::skip]
        let data: Vec<u8> = vec![
            0,   0,   0, 255,   0,   0,
            0, 255,   0,   0, 255,   0,
            0, 128, 255,   0,   0, 255,
        ];
        let out = bands_of(
            &doc,
            &sh,
            &data,
            [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 255.0, 255.0],
        );
        assert_eq!(out.len(), TRIANGLE_STEPS * TRIANGLE_STEPS, "{}", out.len());
        let r = extent(&out);
        assert!(r[0].abs() < 1e-6 && r[1].abs() < 1e-6, "{r:?}");
        assert!(
            (r[2] - 255.0).abs() < 1e-6 && (r[3] - 255.0).abs() < 1e-6,
            "{r:?}"
        );
        // The corner colours survive the barycentric blend: some piece is
        // mostly red and some piece is mostly blue.
        assert!(out.iter().any(|b| b.color[0] > 0.7 && b.color[2] < 0.3));
        assert!(out.iter().any(|b| b.color[2] > 0.7 && b.color[0] < 0.3));
    }

    /// A flag of 1 grows the next triangle from the previous one's second edge.
    ///
    /// Two triangles from four vertices rather than six is the whole point of
    /// the free-form encoding, so a reader that ignores the flag and starts a
    /// fresh triangle every three vertices produces one triangle here and looks
    /// like it works.
    #[test]
    fn a_flagged_vertex_continues_the_previous_triangle() {
        let doc = Document::empty();
        let sh = mesh_dict(4, 255.0);
        #[rustfmt::skip]
        let data: Vec<u8> = vec![
            0,   0,   0, 255,   0,   0,
            0, 255,   0,   0, 255,   0,
            0,   0, 255,   0,   0, 255,
            1, 255, 255, 255, 255,   0,
        ];
        let out = bands_of(
            &doc,
            &sh,
            &data,
            [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 255.0, 255.0],
        );
        assert_eq!(
            out.len(),
            2 * TRIANGLE_STEPS * TRIANGLE_STEPS,
            "one vertex should have added a whole second triangle, got {}",
            out.len()
        );
    }

    /// A lattice mesh reads its rows and stitches two triangles per cell.
    ///
    /// Three columns by two rows is one row-pair and two cells, so four
    /// triangles. A reader that mistakes `/VerticesPerRow` for a triangle count
    /// gets a different number and a sheared quad.
    #[test]
    fn a_lattice_triangle_mesh_is_drawn() {
        let doc = Document::empty();
        let mut sh = mesh_dict(5, 255.0);
        sh.remove("BitsPerFlag");
        sh.insert("VerticesPerRow".into(), Object::Int(3));
        #[rustfmt::skip]
        let data: Vec<u8> = vec![
              0,   0, 255,   0,   0,
            128,   0,   0, 255,   0,
            255,   0,   0,   0, 255,
              0, 255, 255, 255,   0,
            128, 255,   0, 255, 255,
            255, 255, 255,   0, 255,
        ];
        let out = bands_of(
            &doc,
            &sh,
            &data,
            [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 255.0, 255.0],
        );
        assert_eq!(
            out.len(),
            4 * TRIANGLE_STEPS * TRIANGLE_STEPS,
            "two cells between two rows is four triangles, got {}",
            out.len()
        );
        let r = extent(&out);
        assert!(r[0].abs() < 1e-6 && r[1].abs() < 1e-6, "{r:?}");
        assert!(
            (r[2] - 255.0).abs() < 1e-6 && (r[3] - 255.0).abs() < 1e-6,
            "{r:?}"
        );
    }

    /// A lattice mesh with no usable `/VerticesPerRow` paints nothing.
    ///
    /// Without it there is no way to know where a row ends, and guessing a
    /// width draws a shape the document never described.
    #[test]
    fn a_lattice_mesh_without_a_row_width_paints_nothing() {
        let doc = Document::empty();
        let mut sh = mesh_dict(5, 255.0);
        sh.remove("BitsPerFlag");
        let data = vec![0u8; 64];
        assert!(
            bands_of(
                &doc,
                &sh,
                &data,
                [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 255.0, 255.0]
            )
            .is_empty()
        );
        sh.insert("VerticesPerRow".into(), Object::Int(1));
        assert!(
            bands_of(
                &doc,
                &sh,
                &data,
                [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 255.0, 255.0]
            )
            .is_empty()
        );
    }

    /// A free-form mesh pads each vertex to a byte; a lattice mesh does not.
    ///
    /// This is the one place the two readers genuinely differ, and it is
    /// invisible whenever the field widths happen to be byte multiples -- which
    /// is why every other test here uses eight bits and this one uses four. A
    /// vertex is a 4-bit flag plus two 4-bit coordinates plus three 4-bit
    /// components: 24 bits, which is already whole bytes for type 4 only
    /// because the flag is counted. Drop the flag, as a lattice mesh does, and
    /// the vertex is 20 bits, so the second vertex of a lattice row starts
    /// mid-byte. A reader that aligned both would read the second row's
    /// coordinates from padding and place the mesh somewhere else entirely.
    #[test]
    fn a_lattice_mesh_does_not_pad_its_vertices() {
        let doc = Document::empty();
        let mut sh = mesh_dict(5, 15.0);
        sh.remove("BitsPerFlag");
        sh.insert("BitsPerCoordinate".into(), Object::Int(4));
        sh.insert("BitsPerComponent".into(), Object::Int(4));
        sh.insert("VerticesPerRow".into(), Object::Int(2));
        // Four vertices, 20 bits each, packed end to end: 80 bits = 10 bytes.
        // (0,0) (15,0) / (0,15) (15,15), every colour full white.
        let mut packed: Vec<u8> = Vec::new();
        let mut acc: u32 = 0;
        let mut held = 0usize;
        let push = |v: u32, w: usize, packed: &mut Vec<u8>, acc: &mut u32, held: &mut usize| {
            *acc = (*acc << w) | v;
            *held += w;
            while *held >= 8 {
                packed.push((*acc >> (*held - 8)) as u8);
                *held -= 8;
                *acc &= (1 << *held) - 1;
            }
        };
        for (x, y) in [(0u32, 0u32), (15, 0), (0, 15), (15, 15)] {
            push(x, 4, &mut packed, &mut acc, &mut held);
            push(y, 4, &mut packed, &mut acc, &mut held);
            for _ in 0..3 {
                push(15, 4, &mut packed, &mut acc, &mut held);
            }
        }
        assert_eq!(held, 0, "the packing should land on a byte boundary");
        assert_eq!(packed.len(), 10);
        let out = bands_of(
            &doc,
            &sh,
            &packed,
            [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 15.0, 15.0],
        );
        assert_eq!(out.len(), 2 * TRIANGLE_STEPS * TRIANGLE_STEPS);
        let r = extent(&out);
        assert!(
            r[0].abs() < 1e-6
                && r[1].abs() < 1e-6
                && (r[2] - 15.0).abs() < 1e-6
                && (r[3] - 15.0).abs() < 1e-6,
            "unpadded rows should span the full square, got {r:?}"
        );
    }

    /// Malformed triangle meshes terminate, and stay inside their budget.
    ///
    /// The cases that hang rather than fail: a zero `/BitsPerFlag`, which read
    /// as "a value of no bits" consumes nothing per iteration, and a lattice
    /// row wide enough that one row exhausts the stream.
    #[test]
    fn malformed_triangle_meshes_terminate_within_bounds() {
        let doc = Document::empty();
        let mut noise = vec![0u8; 40_000];
        let mut x: u32 = 0x12345678;
        for b in noise.iter_mut() {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            *b = x as u8;
        }
        for kind in [4, 5] {
            for flag_bits in [0, 1, 8, 33] {
                let mut sh = mesh_dict(kind, 255.0);
                sh.insert("BitsPerFlag".into(), Object::Int(flag_bits));
                sh.insert("VerticesPerRow".into(), Object::Int(3));
                let out = bands_of(
                    &doc,
                    &sh,
                    &noise,
                    [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 255.0, 255.0],
                );
                assert!(
                    out.len() <= MAX_PATCH_QUADS + 2 * TRIANGLE_STEPS * TRIANGLE_STEPS,
                    "type {kind} with {flag_bits} flag bits produced {}",
                    out.len()
                );
            }
        }
        // A row wider than the stream can fill must not loop forever.
        let mut sh = mesh_dict(5, 255.0);
        sh.remove("BitsPerFlag");
        sh.insert("VerticesPerRow".into(), Object::Int(1_000_000));
        assert!(
            bands_of(
                &doc,
                &sh,
                &noise,
                [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 255.0, 255.0]
            )
            .is_empty()
        );
    }
}
