//! Best-effort formula → LaTeX extraction.
//!
//! Deterministic and heuristic: detects math content by math-font names
//! (Computer Modern math, Symbol, MSAM/MSBM, STIX, "Math*") and by Unicode
//! math characters, maps symbols to LaTeX commands, and reconstructs super- and
//! sub-scripts from per-fragment baseline shifts and font-size drops. This is
//! symbol-level reconstruction, not full 2-D math layout — fractions, radicals
//! spanning multiple rows, and matrices are approximated, not perfectly parsed.

use crate::engine::PageGraphics;
use crate::{PdfDocument, TextBlock};
use serde::{Deserialize, Serialize};

/// A detected formula.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Formula {
    pub page_number: usize,
    /// Bounding box [left, bottom, right, top] in PDF points.
    pub bbox: [f64; 4],
    /// Best-effort LaTeX reconstruction.
    pub latex: String,
    /// Raw extracted text of the span.
    pub text: String,
}

/// Extract formulas from every page. `graphics` (per-page ruling lines) enables
/// 2-D reconstruction of fractions; pass an empty slice to skip it.
pub fn extract_formulas(doc: &PdfDocument, graphics: &[PageGraphics]) -> Vec<Formula> {
    let mut out = Vec::new();
    for page in &doc.pages {
        let page_no = page.index + 1;
        // Matrices and fractions first (more precise), then inline math lines.
        detect_matrices(&page.text_content, page_no, &mut out);
        if let Some(g) = graphics.iter().find(|g| g.page_number == page_no) {
            detect_fractions(
                &page.text_content,
                &g.h_lines,
                page_no,
                page.width,
                &mut out,
            );
        }
        detect_page_formulas(&page.text_content, page_no, &mut out);
    }
    out
}

/// A candidate fraction bar.
struct Bar {
    x0: f64,
    x1: f64,
    y: f64,
}

/// Detect fractions (including nested `\frac` inside `\frac`): a short
/// horizontal rule (the fraction bar) with text centered above (numerator) and
/// below (denominator); a side that itself contains a bar nests recursively.
fn detect_fractions(
    frags: &[TextBlock],
    h_lines: &[crate::engine::Seg],
    page_number: usize,
    page_width: f64,
    out: &mut Vec<Formula>,
) {
    use std::collections::HashMap;
    use std::collections::HashSet;

    // Table rules repeat their horizontal extent; a fraction bar is unique.
    let mut sig_count: HashMap<(i64, i64), usize> = HashMap::new();
    for s in h_lines {
        let key = (
            (s.x0.min(s.x1) / 6.0).round() as i64,
            (s.x0.max(s.x1) / 6.0).round() as i64,
        );
        *sig_count.entry(key).or_insert(0) += 1;
    }
    let bars: Vec<Bar> = h_lines
        .iter()
        .filter(|s| {
            let key = (
                (s.x0.min(s.x1) / 6.0).round() as i64,
                (s.x0.max(s.x1) / 6.0).round() as i64,
            );
            let len = (s.x0 - s.x1).abs();
            sig_count.get(&key).copied().unwrap_or(0) < 2 && len >= 4.0 && len <= page_width * 0.45
        })
        .map(|s| Bar {
            x0: s.x0.min(s.x1),
            x1: s.x0.max(s.x1),
            y: s.y0,
        })
        .collect();

    let mut consumed: HashSet<usize> = HashSet::new();
    for k in 0..bars.len() {
        if consumed.contains(&k) {
            continue;
        }
        if let Some((latex, bbox, text)) = build_frac(k, &bars, frags, &mut consumed, 0) {
            out.push(Formula {
                page_number,
                bbox,
                latex,
                text,
            });
        }
    }
}

/// Collect fragments centered within a bar's span, split into above/below.
fn frac_sides<'a>(bar: &Bar, frags: &'a [TextBlock]) -> (Vec<&'a TextBlock>, Vec<&'a TextBlock>) {
    let mut above = Vec::new();
    let mut below = Vec::new();
    for f in frags {
        let cx = f.x + f.width / 2.0;
        if cx < bar.x0 - 2.0 || cx > bar.x1 + 2.0 {
            continue;
        }
        let band = f.font_size.max(8.0) * 1.8;
        let dy = f.y - bar.y;
        if dy > 0.5 && dy <= band {
            above.push(f);
        } else if dy < -0.5 && dy >= -band {
            below.push(f);
        }
    }
    above.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
    below.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
    (above, below)
}

/// Build a side's LaTeX: a nested fraction if a sub-bar sits within it,
/// otherwise the mapped text. Returns (latex, nested?).
fn build_side(
    side: &[&TextBlock],
    bar: &Bar,
    above: bool,
    bars: &[Bar],
    frags: &[TextBlock],
    consumed: &mut std::collections::HashSet<usize>,
    depth: usize,
) -> Option<(String, bool)> {
    if side.is_empty() {
        return None;
    }
    // A nested fraction's bar sits in this side, within a generous vertical
    // window above/below the parent bar (a couple of stacked rows).
    let fs = side.iter().map(|f| f.font_size).fold(8.0, f64::max);
    let window = fs * 3.5;
    for (m, sub) in bars.iter().enumerate() {
        if consumed.contains(&m) {
            continue;
        }
        let inside_x = sub.x0 >= bar.x0 - 2.0 && sub.x1 <= bar.x1 + 2.0;
        let inside_y = if above {
            sub.y > bar.y + 2.0 && sub.y <= bar.y + window
        } else {
            sub.y < bar.y - 2.0 && sub.y >= bar.y - window
        };
        if inside_x
            && inside_y
            && let Some((latex, _, _)) = build_frac(m, bars, frags, consumed, depth + 1)
        {
            consumed.insert(m);
            return Some((latex, true));
        }
    }
    let text = join_mapped(side);
    if text.trim().is_empty() {
        return None;
    }
    Some((text.trim().to_string(), false))
}

/// Recursively build a fraction rooted at bar index `k`.
fn build_frac(
    k: usize,
    bars: &[Bar],
    frags: &[TextBlock],
    consumed: &mut std::collections::HashSet<usize>,
    depth: usize,
) -> Option<(String, [f64; 4], String)> {
    if depth > 2 {
        return None;
    }
    let bar = &bars[k];
    let (above, below) = frac_sides(bar, frags);
    if above.is_empty() || below.is_empty() {
        return None;
    }

    let (num, num_nested) = build_side(&above, bar, true, bars, frags, consumed, depth)?;
    let (den, den_nested) = build_side(&below, bar, false, bars, frags, consumed, depth)?;

    // Precision guards apply only to plain (non-nested) sides, so that
    // legitimate sub-fractions aren't rejected for length.
    if !num_nested && (num.chars().count() > 24 || num.split_whitespace().count() > 5) {
        return None;
    }
    if !den_nested && (den.chars().count() > 24 || den.split_whitespace().count() > 5) {
        return None;
    }

    // Bar width should roughly match its content (rejects stray wide rules).
    let span = |v: &[&TextBlock]| {
        if v.is_empty() {
            return 0.0;
        }
        let l = v.iter().map(|f| f.x).fold(f64::INFINITY, f64::min);
        let r = v
            .iter()
            .map(|f| f.x + f.width)
            .fold(f64::NEG_INFINITY, f64::max);
        (r - l).max(0.0)
    };
    let content_span = span(&above).max(span(&below));
    if (bar.x1 - bar.x0) > content_span * 2.0 + 8.0 {
        return None;
    }

    let latex = wrap_radicals(&format!("\\frac{{{}}}{{{}}}", num, den));
    let text: String = above
        .iter()
        .chain(below.iter())
        .map(|f| f.text.trim())
        .collect::<Vec<_>>()
        .join(" ");
    let top = above.iter().map(|f| f.y + f.height).fold(bar.y, f64::max);
    let bottom = below.iter().map(|f| f.y).fold(bar.y, f64::min);
    Some((latex, [bar.x0, bottom, bar.x1, top], text))
}

/// Detect bracket-delimited matrices: an open bracket `[` or `(` and a matching
/// close bracket on the same baseline, enclosing a grid of >=2 rows x >=2 cols.
/// Conservative (requires explicit bracket glyphs) for high precision.
fn detect_matrices(frags: &[TextBlock], page_number: usize, out: &mut Vec<Formula>) {
    let is_open = |t: &str| t == "[" || t == "(";
    let close_for = |t: &str| match t {
        "[" => "]",
        "(" => ")",
        _ => "",
    };
    let opens: Vec<&TextBlock> = frags.iter().filter(|f| is_open(f.text.trim())).collect();
    if opens.is_empty() {
        return;
    }

    for l in opens {
        let lt = l.text.trim();
        let want = close_for(lt);
        // Nearest matching close bracket to the right on ~same baseline.
        let r = frags
            .iter()
            .filter(|f| {
                f.text.trim() == want
                    && f.x > l.x + l.width
                    && (f.y - l.y).abs() <= l.font_size.max(6.0)
            })
            .min_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
        let r = match r {
            Some(r) => r,
            None => continue,
        };
        if r.x - l.x > 400.0 {
            continue;
        }

        // Entries strictly between the brackets, within a few lines vertically.
        let band = l.font_size.max(8.0) * 3.5;
        let entries: Vec<&TextBlock> = frags
            .iter()
            .filter(|f| {
                let cx = f.x + f.width / 2.0;
                cx > l.x + l.width - 1.0
                    && cx < r.x + 1.0
                    && (f.y - l.y).abs() <= band
                    && !f.text.trim().is_empty()
            })
            .collect();
        if entries.len() < 4 {
            continue;
        }

        // Cluster into rows (y) and columns (x).
        let fs = l.font_size.max(8.0);
        let rows = cluster_axis(entries.iter().map(|f| f.y).collect(), fs * 0.7);
        let cols = cluster_axis(
            entries.iter().map(|f| f.x + f.width / 2.0).collect(),
            fs * 1.2,
        );
        if rows.len() < 2 || cols.len() < 2 {
            continue;
        }

        // rows are sorted ascending y; iterate top (largest y) down.
        let mut grid: Vec<Vec<String>> = vec![vec![String::new(); cols.len()]; rows.len()];
        for f in &entries {
            let cy = f.y;
            let cx = f.x + f.width / 2.0;
            let ri = nearest(&rows, cy);
            let ci = nearest(&cols, cx);
            let cell = &mut grid[ri][ci];
            let m = map_text(&f.text);
            if !cell.is_empty() && !cell.ends_with(' ') {
                cell.push(' ');
            }
            cell.push_str(m.trim());
        }
        // Reject sparse grids (likely not a real matrix).
        let filled = grid
            .iter()
            .flatten()
            .filter(|c| !c.trim().is_empty())
            .count();
        if filled < rows.len() * cols.len() {
            continue;
        }

        let env = if lt == "[" { "bmatrix" } else { "pmatrix" };
        let body = grid
            .iter()
            .rev() // top row first
            .map(|row| row.iter().map(|c| c.trim()).collect::<Vec<_>>().join(" & "))
            .collect::<Vec<_>>()
            .join(" \\\\ ");
        let latex = format!("\\begin{{{}}} {} \\end{{{}}}", env, body, env);
        let top = entries.iter().map(|f| f.y + f.height).fold(l.y, f64::max);
        let bottom = entries.iter().map(|f| f.y).fold(l.y, f64::min);
        out.push(Formula {
            page_number,
            bbox: [l.x, bottom, r.x + r.width, top],
            latex,
            text: format!("matrix {}x{}", rows.len(), cols.len()),
        });
    }
}

/// Cluster sorted scalar positions within `tol`, returning cluster means (asc).
fn cluster_axis(mut vals: Vec<f64>, tol: f64) -> Vec<f64> {
    if vals.is_empty() {
        return Vec::new();
    }
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut out: Vec<(f64, usize)> = vec![(vals[0], 1)];
    for &v in &vals[1..] {
        let last = out.last_mut().unwrap();
        if (v - last.0 / last.1 as f64).abs() <= tol {
            last.0 += v;
            last.1 += 1;
        } else {
            out.push((v, 1));
        }
    }
    out.into_iter().map(|(s, c)| s / c as f64).collect()
}

fn nearest(centers: &[f64], v: f64) -> usize {
    centers
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            (**a - v)
                .abs()
                .partial_cmp(&(**b - v).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
        .unwrap_or(0)
}

fn join_mapped(frags: &[&TextBlock]) -> String {
    let mut s = String::new();
    for f in frags {
        let m = map_text(&f.text);
        if !s.is_empty() && !s.ends_with(' ') && !m.starts_with(' ') {
            s.push(' ');
        }
        s.push_str(&m);
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

struct MathLine<'a> {
    frags: Vec<&'a TextBlock>,
}

fn detect_page_formulas(frags: &[TextBlock], page_number: usize, out: &mut Vec<Formula>) {
    let mut frags: Vec<&TextBlock> = frags.iter().filter(|f| !f.text.trim().is_empty()).collect();
    if frags.is_empty() {
        return;
    }
    frags.sort_by(|a, b| {
        b.y.partial_cmp(&a.y)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
    });

    // Cluster into lines loosely so super/subscripts stay with their base line.
    let mut lines: Vec<MathLine> = Vec::new();
    for f in frags {
        let tol = f.font_size.max(1.0) * 0.9;
        match lines.last_mut().filter(|l| {
            let ly = l.frags[0].y;
            (ly - f.y).abs() <= tol
        }) {
            Some(l) => l.frags.push(f),
            None => lines.push(MathLine { frags: vec![f] }),
        }
    }

    for line in &mut lines {
        line.frags
            .sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
        if !is_mathy(line) {
            continue;
        }
        if let Some(f) = build_formula(line, page_number) {
            out.push(f);
        }
    }
}

/// Decide whether a line carries enough math to be a formula.
fn is_mathy(line: &MathLine) -> bool {
    let mut math_chars = 0usize;
    let mut total_chars = 0usize;
    let mut has_math_font = false;
    for f in &line.frags {
        if is_math_font(&f.font_name) {
            has_math_font = true;
        }
        for c in f.text.chars() {
            if !c.is_whitespace() {
                total_chars += 1;
            }
            if to_latex(c).is_some() {
                math_chars += 1;
            }
        }
    }
    if total_chars == 0 {
        return false;
    }
    // Two or more math symbols, or a math font with at least one symbol.
    math_chars >= 2 || (has_math_font && math_chars >= 1)
}

fn build_formula(line: &MathLine, page_number: usize) -> Option<Formula> {
    // Dominant (base) font size and baseline.
    let main_size = line
        .frags
        .iter()
        .map(|f| f.font_size)
        .fold(0.0f64, f64::max);
    if main_size <= 0.0 {
        return None;
    }
    let base_ys: Vec<f64> = line
        .frags
        .iter()
        .filter(|f| f.font_size >= main_size * 0.85)
        .map(|f| f.y)
        .collect();
    let base_y = if base_ys.is_empty() {
        line.frags[0].y
    } else {
        base_ys.iter().sum::<f64>() / base_ys.len() as f64
    };

    let mut latex = String::new();
    let mut text = String::new();
    let mut left = f64::INFINITY;
    let mut right = f64::NEG_INFINITY;
    let mut bottom = f64::INFINITY;
    let mut top = f64::NEG_INFINITY;

    for f in &line.frags {
        text.push_str(&f.text);
        left = left.min(f.x);
        right = right.max(f.x + f.width);
        bottom = bottom.min(f.y);
        top = top.max(f.y + f.height);

        let mapped = map_text(&f.text);
        let rel = f.y - base_y;
        let small = f.font_size < main_size * 0.85;
        if small && rel > main_size * 0.12 {
            latex.push_str(&format!("^{{{}}}", mapped.trim()));
        } else if small && rel < -main_size * 0.12 {
            latex.push_str(&format!("_{{{}}}", mapped.trim()));
        } else {
            if !latex.is_empty() && !latex.ends_with(' ') && !mapped.starts_with(' ') {
                latex.push(' ');
            }
            latex.push_str(&mapped);
        }
    }

    let latex = attach_limits(&wrap_radicals(
        &latex.split_whitespace().collect::<Vec<_>>().join(" "),
    ));
    if latex.is_empty() {
        return None;
    }
    Some(Formula {
        page_number,
        bbox: [left, bottom, right, top],
        latex,
        text: text.trim().to_string(),
    })
}

/// Map every character of a fragment, replacing math symbols with LaTeX.
fn map_text(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match to_latex(c) {
            Some(cmd) => {
                if !out.is_empty() && !out.ends_with(' ') {
                    out.push(' ');
                }
                out.push_str(cmd);
                out.push(' ');
            }
            None => out.push(c),
        }
    }
    out
}

/// Wrap the radicand following `\sqrt` in braces, grouping the sub-expression
/// up to the next additive/relational operator: `\sqrt a b + 1` → `\sqrt{a b} + 1`.
/// Multiplicative terms stay inside; nested `\frac`/`\sqrt` are preserved.
fn wrap_radicals(s: &str) -> String {
    let tokens: Vec<&str> = s.split_whitespace().collect();
    let mut out: Vec<String> = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        if tokens[i] == "\\sqrt" && i + 1 < tokens.len() {
            let next = tokens[i + 1];
            // Already-grouped radicand: take it as one unit.
            if next.starts_with('{') || next.starts_with('(') || next.starts_with("\\frac") {
                out.push(format!("\\sqrt{{{}}}", next));
                i += 2;
                continue;
            }
            // Otherwise consume tokens until a stop operator (bounded).
            let mut rad: Vec<&str> = Vec::new();
            let mut j = i + 1;
            while j < tokens.len() && !is_radicand_stop(tokens[j]) && rad.len() < 6 {
                rad.push(tokens[j]);
                j += 1;
            }
            if rad.is_empty() {
                out.push("\\sqrt".to_string());
                i += 1;
            } else {
                out.push(format!("\\sqrt{{{}}}", rad.join(" ")));
                i = j;
            }
        } else {
            out.push(tokens[i].to_string());
            i += 1;
        }
    }
    out.join(" ")
}

/// Attach sub/superscripts immediately following a large operator as limits:
/// `\sum _{i} ^{n}` → `\sum_{i}^{n}`.
fn attach_limits(s: &str) -> String {
    let tokens: Vec<&str> = s.split_whitespace().collect();
    let mut out: Vec<String> = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        if is_big_operator(tokens[i]) {
            let mut sub: Option<&str> = None;
            let mut sup: Option<&str> = None;
            let mut j = i + 1;
            for _ in 0..2 {
                if j < tokens.len() && tokens[j].starts_with("_{") && sub.is_none() {
                    sub = Some(tokens[j]);
                    j += 1;
                } else if j < tokens.len() && tokens[j].starts_with("^{") && sup.is_none() {
                    sup = Some(tokens[j]);
                    j += 1;
                } else {
                    break;
                }
            }
            let mut merged = tokens[i].to_string();
            if let Some(s) = sub {
                merged.push_str(s);
            }
            if let Some(s) = sup {
                merged.push_str(s);
            }
            out.push(merged);
            i = j;
        } else {
            out.push(tokens[i].to_string());
            i += 1;
        }
    }
    out.join(" ")
}

fn is_big_operator(tok: &str) -> bool {
    matches!(
        tok,
        "\\sum"
            | "\\prod"
            | "\\int"
            | "\\oint"
            | "\\coprod"
            | "\\bigcup"
            | "\\bigcap"
            | "\\bigoplus"
            | "\\bigotimes"
            | "\\lim"
    )
}

/// Tokens that end a radicand group (additive / relational operators).
fn is_radicand_stop(tok: &str) -> bool {
    matches!(tok, "+" | "-" | "=" | "<" | ">" | "±")
        || matches!(
            tok,
            "\\leq"
                | "\\geq"
                | "\\neq"
                | "\\approx"
                | "\\equiv"
                | "\\sim"
                | "\\to"
                | "\\in"
                | "\\Rightarrow"
                | "\\Leftarrow"
                | "\\leftrightarrow"
                | "\\propto"
                | "\\pm"
                | "\\mp"
        )
}

fn is_math_font(name: &str) -> bool {
    let n = name.to_ascii_uppercase();
    n.contains("CMMI")
        || n.contains("CMSY")
        || n.contains("CMEX")
        || n.contains("MSAM")
        || n.contains("MSBM")
        || n.contains("STIX")
        || n.contains("SYMBOL")
        || n.contains("MATHJAX")
        || n.contains("MATH")
        || n.contains("EUSM")
}

/// Map a Unicode math character to a LaTeX command (without trailing space).
fn to_latex(c: char) -> Option<&'static str> {
    Some(match c {
        // Greek lowercase
        'α' => "\\alpha",
        'β' => "\\beta",
        'γ' => "\\gamma",
        'δ' => "\\delta",
        'ε' => "\\epsilon",
        'ζ' => "\\zeta",
        'η' => "\\eta",
        'θ' => "\\theta",
        'ι' => "\\iota",
        'κ' => "\\kappa",
        'λ' => "\\lambda",
        'μ' => "\\mu",
        'ν' => "\\nu",
        'ξ' => "\\xi",
        'π' => "\\pi",
        'ρ' => "\\rho",
        'σ' => "\\sigma",
        'τ' => "\\tau",
        'υ' => "\\upsilon",
        'φ' => "\\phi",
        'χ' => "\\chi",
        'ψ' => "\\psi",
        'ω' => "\\omega",
        'ϕ' => "\\varphi",
        // Greek uppercase
        'Γ' => "\\Gamma",
        'Δ' => "\\Delta",
        'Θ' => "\\Theta",
        'Λ' => "\\Lambda",
        'Ξ' => "\\Xi",
        'Π' => "\\Pi",
        'Σ' => "\\Sigma",
        'Φ' => "\\Phi",
        'Ψ' => "\\Psi",
        'Ω' => "\\Omega",
        '\u{2206}' => "\\Delta", // INCREMENT
        // Operators
        '∑' => "\\sum",
        '∏' => "\\prod",
        '∫' => "\\int",
        '√' => "\\sqrt",
        '∞' => "\\infty",
        '∂' => "\\partial",
        '∇' => "\\nabla",
        '±' => "\\pm",
        '∓' => "\\mp",
        '×' => "\\times",
        '÷' => "\\div",
        '⋅' => "\\cdot",
        '∗' => "\\ast",
        '∘' => "\\circ",
        '⊕' => "\\oplus",
        '⊗' => "\\otimes",
        '∝' => "\\propto",
        '∅' => "\\emptyset",
        // Relations
        '≤' => "\\leq",
        '≥' => "\\geq",
        '≠' => "\\neq",
        '≈' => "\\approx",
        '≡' => "\\equiv",
        '∼' => "\\sim",
        '≃' => "\\simeq",
        '≅' => "\\cong",
        '≪' => "\\ll",
        '≫' => "\\gg",
        '⟨' => "\\langle",
        '⟩' => "\\rangle",
        // Set theory & logic
        '∈' => "\\in",
        '∉' => "\\notin",
        '⊂' => "\\subset",
        '⊆' => "\\subseteq",
        '⊃' => "\\supset",
        '⊇' => "\\supseteq",
        '∪' => "\\cup",
        '∩' => "\\cap",
        '∀' => "\\forall",
        '∃' => "\\exists",
        '¬' => "\\neg",
        '∧' => "\\wedge",
        '∨' => "\\vee",
        // Arrows
        '→' => "\\to",
        '←' => "\\leftarrow",
        '↔' => "\\leftrightarrow",
        '⇒' => "\\Rightarrow",
        '⇐' => "\\Leftarrow",
        '⇔' => "\\Leftrightarrow",
        '↦' => "\\mapsto",
        // Misc
        '…' => "\\ldots",
        '⋯' => "\\cdots",
        '′' => "'",
        '″' => "''",
        '∠' => "\\angle",
        '⊥' => "\\perp",
        '∥' => "\\parallel",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frag(text: &str, x: f64, y: f64, size: f64, font: &str) -> TextBlock {
        TextBlock {
            text: text.into(),
            x,
            y,
            width: text.chars().count() as f64 * size * 0.5,
            height: size,
            font_size: size,
            font_name: font.into(),
            page_number: 1,
        }
    }

    #[test]
    fn maps_symbols() {
        assert_eq!(to_latex('α'), Some("\\alpha"));
        assert_eq!(to_latex('≤'), Some("\\leq"));
        assert_eq!(to_latex('∑'), Some("\\sum"));
        assert_eq!(to_latex('a'), None);
    }

    #[test]
    fn reconstructs_superscript() {
        // "x" base, "2" superscript (smaller, higher), "+ α ≤ β"
        let frags = vec![
            frag("x", 100.0, 500.0, 12.0, "CMMI10"),
            frag("2", 106.0, 505.0, 7.0, "CMR7"), // superscript
            frag(" + α ≤ β", 112.0, 500.0, 12.0, "CMMI10"),
        ];
        let mut out = Vec::new();
        detect_page_formulas(&frags, 1, &mut out);
        assert_eq!(out.len(), 1);
        let latex = &out[0].latex;
        assert!(latex.contains("x"), "{latex}");
        assert!(latex.contains("^{2}"), "{latex}");
        assert!(latex.contains("\\alpha"), "{latex}");
        assert!(latex.contains("\\leq"), "{latex}");
        assert!(latex.contains("\\beta"), "{latex}");
    }

    #[test]
    fn reconstructs_fraction() {
        use crate::engine::Seg;
        // Fraction bar at y=500, x 100..140; numerator "x + 1" above, "2" below.
        let bar = Seg {
            x0: 100.0,
            y0: 500.0,
            x1: 140.0,
            y1: 500.0,
        };
        let frags = vec![
            frag("x + 1", 102.0, 506.0, 10.0, "CMMI10"),
            frag("2", 116.0, 489.0, 10.0, "CMR10"),
        ];
        let mut out = Vec::new();
        detect_fractions(&frags, &[bar], 1, 600.0, &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].latex, "\\frac{x + 1}{2}");
    }

    #[test]
    fn wraps_sqrt() {
        assert_eq!(wrap_radicals("\\sqrt x + 1"), "\\sqrt{x} + 1");
        assert_eq!(wrap_radicals("a \\sqrt b"), "a \\sqrt{b}");
    }

    #[test]
    fn wraps_multi_token_radicand() {
        // Multiplicative terms stay inside; stop at the relational operator.
        assert_eq!(wrap_radicals("\\sqrt a b \\leq c"), "\\sqrt{a b} \\leq c");
        assert_eq!(wrap_radicals("\\sqrt x y z"), "\\sqrt{x y z}");
    }

    #[test]
    fn attaches_operator_limits() {
        assert_eq!(attach_limits("\\sum _{i} ^{n} x"), "\\sum_{i}^{n} x");
        assert_eq!(
            attach_limits("\\int _{0} ^{\\infty} f"),
            "\\int_{0}^{\\infty} f"
        );
        // Non-operator scripts are untouched.
        assert_eq!(attach_limits("x ^{2}"), "x ^{2}");
    }

    #[test]
    fn nested_fraction() {
        use crate::engine::Seg;
        // Outer bar at y=500 (x 100..150); numerator is itself a fraction with a
        // sub-bar at y=512 (a/b); denominator "c".
        let outer = Seg {
            x0: 116.0,
            y0: 500.0,
            x1: 132.0,
            y1: 500.0,
        };
        let inner = Seg {
            x0: 118.0,
            y0: 512.0,
            x1: 128.0,
            y1: 512.0,
        };
        let frags = vec![
            frag("a", 120.0, 520.0, 10.0, "CMR10"), // inner numerator (above inner)
            frag("b", 120.0, 505.0, 10.0, "CMR10"), // inner denominator (above outer)
            frag("c", 120.0, 488.0, 10.0, "CMR10"), // outer denominator (below outer)
        ];
        let mut out = Vec::new();
        detect_fractions(&frags, &[outer, inner], 1, 600.0, &mut out);
        // One top-level fraction whose numerator is the nested fraction.
        assert_eq!(
            out.len(),
            1,
            "{:?}",
            out.iter().map(|f| &f.latex).collect::<Vec<_>>()
        );
        assert_eq!(out[0].latex, "\\frac{\\frac{a}{b}}{c}");
    }

    #[test]
    fn detects_2x2_matrix() {
        let frags = vec![
            frag("[", 100.0, 500.0, 10.0, "CMEX10"),
            frag("]", 180.0, 500.0, 10.0, "CMEX10"),
            frag("1", 110.0, 506.0, 10.0, "CMR10"),
            frag("2", 150.0, 506.0, 10.0, "CMR10"),
            frag("3", 110.0, 494.0, 10.0, "CMR10"),
            frag("4", 150.0, 494.0, 10.0, "CMR10"),
        ];
        let mut out = Vec::new();
        detect_matrices(&frags, 1, &mut out);
        assert_eq!(out.len(), 1, "expected one matrix");
        assert_eq!(
            out[0].latex,
            "\\begin{bmatrix} 1 & 2 \\\\ 3 & 4 \\end{bmatrix}"
        );
    }

    #[test]
    fn ignores_prose() {
        let frags = vec![frag("The quick brown fox", 50.0, 400.0, 12.0, "Times")];
        let mut out = Vec::new();
        detect_page_formulas(&frags, 1, &mut out);
        assert!(out.is_empty());
    }
}
