// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shadings (PDF 8.7.4.5) reduced to flat-filled bands.
//!
//! A gradient has no representation in the display list, and giving it one
//! would mean teaching three renderers -- the desktop rasteriser, the WebGL
//! renderer and the browser replayer -- to draw it. Slicing it into bands of
//! constant colour needs none of that: the ops already exist, and at sixty-four
//! bands the seams are below the resolution anything is compared at.
//!
//! Axial and radial shadings are covered, which is what documents use. The
//! function-based and mesh types are declined rather than approximated: a mesh
//! guessed at wrongly is worse than a gap, because a gap is visible.

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
    let kind = doc
        .get(sh, "ShadingType")
        .and_then(|o| o.as_int())
        .unwrap_or(0);
    let space = doc.get(sh, "ColorSpace").map(|o| doc.resolve(&o));
    let components = space.as_ref().map(|s| components_of(doc, s)).unwrap_or(3);
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
        // Type 4: a mesh, deliberately not implemented.
        sh.insert("ShadingType".into(), Object::Int(4));
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
}
