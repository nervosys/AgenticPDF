//! Table reconstruction from ruling lines + positioned text.
//!
//! Deterministic, local, and zero-dependency. Tables are anchored on groups of
//! horizontal rules that share a horizontal extent (the top/mid/bottom rules of
//! a table). Within each region, rows come from text-baseline clustering and
//! columns from vertical rules when present, otherwise from text-coverage gaps
//! (so booktabs-style and borderless tables — the common case in papers — are
//! handled, not just fully-boxed grids). Emits GitHub-flavored Markdown tables
//! plus a structured form with per-table bounding boxes for citation.

use crate::engine::{PageGraphics, Seg};
use crate::{PdfPage, TextBlock};
use serde::{Deserialize, Serialize};

/// A reconstructed table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub page_number: usize,
    pub rows: usize,
    pub cols: usize,
    /// Bounding box [left, bottom, right, top] in PDF points.
    pub bbox: [f64; 4],
    /// Row-major cell text.
    pub cells: Vec<Vec<String>>,
}

#[derive(Clone, Copy)]
struct Region {
    x_left: f64,
    x_right: f64,
    y_top: f64,
    y_bottom: f64,
}

/// Cluster scalar values within `tol`, returning cluster means (sorted).
fn cluster(values: &mut [f64], tol: f64) -> Vec<f64> {
    if values.is_empty() {
        return Vec::new();
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut out: Vec<(f64, usize)> = vec![(values[0], 1)];
    let mut last = values[0];
    for &v in &values[1..] {
        if (v - last).abs() <= tol {
            let c = out.last_mut().unwrap();
            c.0 += v;
            c.1 += 1;
        } else {
            out.push((v, 1));
        }
        last = v;
    }
    out.into_iter().map(|(s, c)| s / c as f64).collect()
}

/// Detect tables across all pages.
pub fn detect_tables(graphics: &[PageGraphics], pages: &[PdfPage]) -> Vec<Table> {
    let mut out = Vec::new();
    for g in graphics {
        if let Some(page) = pages.get(g.page_number.saturating_sub(1)) {
            detect_page_tables(g, page, &mut out);
        }
    }
    out
}

/// Group horizontal rules sharing a horizontal extent into table regions.
fn find_regions(h_lines: &[Seg]) -> Vec<Region> {
    use std::collections::HashMap;
    // Signature = rounded (x0, x1) so the top/mid/bottom rules of one table
    // (which share width) land in the same bucket.
    let mut groups: HashMap<(i64, i64), Vec<Seg>> = HashMap::new();
    for s in h_lines {
        let key = ((s.x0 / 8.0).round() as i64, (s.x1 / 8.0).round() as i64);
        groups.entry(key).or_default().push(*s);
    }
    let mut regions = Vec::new();
    for (_, segs) in groups {
        let mut ys: Vec<f64> = segs.iter().map(|s| s.y0).collect();
        let uy = cluster(&mut ys, 3.0);
        if uy.len() < 2 {
            continue; // need at least two rules to bound a table
        }
        let x_left = segs.iter().map(|s| s.x0).fold(f64::INFINITY, f64::min);
        let x_right = segs.iter().map(|s| s.x1).fold(f64::NEG_INFINITY, f64::max);
        let y_top = *uy.last().unwrap();
        let y_bottom = *uy.first().unwrap();
        if x_right - x_left < 40.0 || y_top - y_bottom < 4.0 {
            continue;
        }
        regions.push(Region {
            x_left,
            x_right,
            y_top,
            y_bottom,
        });
    }
    regions
}

fn detect_page_tables(g: &PageGraphics, page: &PdfPage, out: &mut Vec<Table>) {
    let regions = find_regions(&g.h_lines);
    let debug = std::env::var("APDF_DEBUG").is_ok();
    if debug && !regions.is_empty() {
        eprintln!("DEBUG page {}: {} regions", g.page_number, regions.len());
    }
    for region in regions {
        // Fragments inside the region.
        let frags: Vec<&TextBlock> = page
            .text_content
            .iter()
            .filter(|f| {
                let cx = f.x + f.width / 2.0;
                let cy = f.y + f.height / 2.0;
                cx >= region.x_left - 2.0
                    && cx <= region.x_right + 2.0
                    && cy >= region.y_bottom - 2.0
                    && cy <= region.y_top + 2.0
            })
            .collect();
        if frags.len() < 4 {
            continue;
        }

        let all_rows = cluster_rows(&frags);

        // A caption ("Table N", "Figure N") inside the region usually separates
        // two stacked tables; split there and drop the caption row.
        for group in split_on_captions(all_rows) {
            if group.len() < 2 {
                continue;
            }
            let median_font = median_font_size(&group);
            let data_rows: Vec<Vec<&TextBlock>> =
                group.iter().filter(|r| r.len() >= 2).cloned().collect();
            let cols = match columns_from_vlines(&g.v_lines, &region) {
                Some(ranges) => Columns {
                    ranges,
                    panel_split: None,
                },
                None => infer_columns(&data_rows, &region, median_font),
            };
            if debug {
                eprintln!(
                    "DEBUG  region x[{:.0},{:.0}] group rows={} data={} cols={} panel={:?}",
                    region.x_left,
                    region.x_right,
                    group.len(),
                    data_rows.len(),
                    cols.ranges.len(),
                    cols.panel_split
                );
            }
            if cols.ranges.len() < 2 {
                continue;
            }

            // Side-by-side panels: split at a much-wider-than-typical gap.
            if let Some(sx) = cols.panel_split {
                let left: Vec<(f64, f64)> =
                    cols.ranges.iter().copied().filter(|c| c.1 <= sx).collect();
                let right: Vec<(f64, f64)> =
                    cols.ranges.iter().copied().filter(|c| c.0 >= sx).collect();
                if left.len() >= 2 && right.len() >= 2 {
                    if let Some(t) = assemble(g.page_number, &region, &group, &left) {
                        out.push(t);
                    }
                    if let Some(t) = assemble(g.page_number, &region, &group, &right) {
                        out.push(t);
                    }
                    continue;
                }
            }
            if let Some(t) = assemble(g.page_number, &region, &group, &cols.ranges) {
                out.push(t);
            }
        }
    }
}

/// True if a row's joined text is a table/figure caption.
fn is_caption(row: &[&TextBlock]) -> bool {
    let mut text = String::new();
    for f in row {
        text.push_str(f.text.trim());
        text.push(' ');
    }
    let t = text.trim().to_ascii_lowercase();
    let starts = t.starts_with("table ")
        || t.starts_with("figure ")
        || t.starts_with("fig. ")
        || t.starts_with("fig ");
    starts
        && t.split_whitespace()
            .nth(1)
            .map(|w| w.chars().next().is_some_and(|c| c.is_ascii_digit()))
            .unwrap_or(false)
}

/// Split a region's rows into table groups at caption boundaries.
fn split_on_captions(rows: Vec<Vec<&TextBlock>>) -> Vec<Vec<Vec<&TextBlock>>> {
    let mut groups = Vec::new();
    let mut current: Vec<Vec<&TextBlock>> = Vec::new();
    for row in rows {
        if is_caption(&row) {
            if !current.is_empty() {
                groups.push(std::mem::take(&mut current));
            }
            // caption itself is dropped
        } else {
            current.push(row);
        }
    }
    if !current.is_empty() {
        groups.push(current);
    }
    groups
}

fn median_font_size(rows: &[Vec<&TextBlock>]) -> f64 {
    let mut sizes: Vec<f64> = rows.iter().flatten().map(|f| f.font_size).collect();
    if sizes.is_empty() {
        return 10.0;
    }
    sizes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    sizes[sizes.len() / 2].max(4.0)
}

/// Cluster fragments into rows by baseline; returns rows top-to-bottom.
fn cluster_rows<'a>(frags: &[&'a TextBlock]) -> Vec<Vec<&'a TextBlock>> {
    let mut sizes: Vec<f64> = frags.iter().map(|f| f.font_size).collect();
    sizes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = sizes.get(sizes.len() / 2).copied().unwrap_or(10.0).max(4.0);
    let tol = (median * 0.6).max(3.0);

    let mut sorted: Vec<&TextBlock> = frags.to_vec();
    sorted.sort_by(|a, b| b.y.partial_cmp(&a.y).unwrap_or(std::cmp::Ordering::Equal));

    let mut rows: Vec<Vec<&TextBlock>> = Vec::new();
    let mut cur_y = f64::NAN;
    for f in sorted {
        if rows.is_empty() || (cur_y - f.y).abs() > tol {
            rows.push(vec![f]);
            cur_y = f.y;
        } else {
            rows.last_mut().unwrap().push(f);
        }
    }
    for row in rows.iter_mut() {
        row.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
    }
    rows
}

/// Inferred column ranges plus an optional side-by-side panel split x.
struct Columns {
    ranges: Vec<(f64, f64)>,
    panel_split: Option<f64>,
}

/// Column boundaries from vertical rules inside the region, if enough exist.
fn columns_from_vlines(v_lines: &[Seg], region: &Region) -> Option<Vec<(f64, f64)>> {
    let mut xs: Vec<f64> = v_lines
        .iter()
        .filter(|s| {
            let lo = s.y0.min(s.y1);
            let hi = s.y0.max(s.y1);
            s.x0 >= region.x_left - 2.0
                && s.x0 <= region.x_right + 2.0
                && hi >= region.y_bottom - 2.0
                && lo <= region.y_top + 2.0
        })
        .map(|s| s.x0)
        .collect();
    let bounds = cluster(&mut xs, 3.0);
    if bounds.len() < 3 {
        return None; // need >= 2 columns
    }
    Some(bounds.windows(2).map(|w| (w[0], w[1])).collect())
}

/// Infer column boundaries from vertical whitespace shared across data rows.
///
/// Only data rows (>= 2 fragments) are sampled, and a gap must be *entirely*
/// empty across them — this keeps a wide header/note from closing a real
/// column gap, and avoids spurious splits in dense tables.
fn infer_columns(rows: &[Vec<&TextBlock>], region: &Region, median_font: f64) -> Columns {
    let empty = Columns {
        ranges: Vec::new(),
        panel_split: None,
    };
    if rows.is_empty() {
        return empty;
    }
    let step = 1.0;
    let min_gap = (median_font * 0.4).max(3.0);

    // Sample coverage across the region width.
    let mut x = region.x_left;
    let mut samples: Vec<(f64, usize)> = Vec::new();
    while x <= region.x_right {
        let cov = rows
            .iter()
            .filter(|row| {
                row.iter()
                    .any(|f| x >= f.x - 0.5 && x <= f.x + f.width + 0.5)
            })
            .count();
        samples.push((x, cov));
        x += step;
    }

    // A gap is a run that is entirely empty across data rows.
    let mut gaps: Vec<(f64, f64)> = Vec::new();
    let mut run_start: Option<f64> = None;
    for &(sx, cov) in &samples {
        if cov == 0 {
            run_start.get_or_insert(sx);
        } else if let Some(start) = run_start.take()
            && sx - start >= min_gap
        {
            gaps.push((start, sx));
        }
    }
    if let Some(start) = run_start.take() {
        gaps.push((start, region.x_right));
    }

    // Leading/trailing gaps trim the content bounds; internal gaps split columns.
    let content_left = gaps
        .first()
        .filter(|g| g.0 <= region.x_left + 1.0)
        .map(|g| g.1)
        .unwrap_or(region.x_left);
    let content_right = gaps
        .last()
        .filter(|g| g.1 >= region.x_right - 1.0)
        .map(|g| g.0)
        .unwrap_or(region.x_right);

    let internal_gaps: Vec<(f64, f64)> = gaps
        .iter()
        .copied()
        .filter(|g| g.0 > region.x_left + 1.0 && g.1 < region.x_right - 1.0)
        .collect();

    // Detect a panel separator: an internal gap dramatically wider than the
    // others (side-by-side sub-tables share rows but not columns).
    let panel_split = if internal_gaps.len() >= 2 {
        let mut widths: Vec<f64> = internal_gaps.iter().map(|g| g.1 - g.0).collect();
        let (max_i, &max_w) = widths
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();
        widths.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = widths[widths.len() / 2];
        if max_w >= median * 2.5 && max_w >= 18.0 {
            let g = internal_gaps[max_i];
            Some((g.0 + g.1) / 2.0)
        } else {
            None
        }
    } else {
        None
    };

    let mut cuts = vec![content_left];
    cuts.extend(internal_gaps.iter().map(|g| (g.0 + g.1) / 2.0));
    cuts.push(content_right);
    cuts.dedup_by(|a, b| (*a - *b).abs() < 1.0);
    if cuts.len() < 3 {
        return empty;
    }
    Columns {
        ranges: cuts.windows(2).map(|w| (w[0], w[1])).collect(),
        panel_split,
    }
}

fn assemble(
    page_number: usize,
    region: &Region,
    rows: &[Vec<&TextBlock>],
    cols: &[(f64, f64)],
) -> Option<Table> {
    let n_cols = cols.len();
    let mut cells: Vec<Vec<String>> = Vec::with_capacity(rows.len());
    for row in rows {
        let mut cell_row = vec![String::new(); n_cols];
        for f in row {
            let cx = f.x + f.width / 2.0;
            // Assign to the column whose range contains the center; if none,
            // snap to the nearest column so right/left-aligned text isn't lost.
            let ci = cols
                .iter()
                .position(|c| cx >= c.0 - 2.0 && cx <= c.1 + 2.0)
                .or_else(|| {
                    cols.iter()
                        .enumerate()
                        .min_by(|(_, a), (_, b)| {
                            let da = ((a.0 + a.1) / 2.0 - cx).abs();
                            let db = ((b.0 + b.1) / 2.0 - cx).abs();
                            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                        })
                        .map(|(i, _)| i)
                });
            if let Some(ci) = ci {
                let cell = &mut cell_row[ci];
                if !cell.is_empty() && !cell.ends_with(' ') {
                    cell.push(' ');
                }
                cell.push_str(f.text.trim());
            }
        }
        cells.push(cell_row);
    }

    // Drop fully-empty rows.
    cells.retain(|r| r.iter().any(|c| !c.trim().is_empty()));
    if cells.len() < 2 {
        return None;
    }

    // Drop columns that are empty in every row.
    let keep: Vec<usize> = (0..n_cols)
        .filter(|&c| cells.iter().any(|r| !r[c].trim().is_empty()))
        .collect();
    if keep.len() < 2 {
        return None;
    }
    if keep.len() != n_cols {
        for r in cells.iter_mut() {
            *r = keep.iter().map(|&c| std::mem::take(&mut r[c])).collect();
        }
    }
    let final_cols = keep.len();

    // Must look tabular: at least two rows with two or more populated columns.
    let tabular_rows = cells
        .iter()
        .filter(|r| r.iter().filter(|c| !c.trim().is_empty()).count() >= 2)
        .count();
    if tabular_rows < 2 {
        return None;
    }

    // Bbox from the group's actual fragment extents.
    let mut left = f64::INFINITY;
    let mut right = f64::NEG_INFINITY;
    let mut bottom = f64::INFINITY;
    let mut top = f64::NEG_INFINITY;
    for f in rows.iter().flatten() {
        left = left.min(f.x);
        right = right.max(f.x + f.width);
        bottom = bottom.min(f.y);
        top = top.max(f.y + f.height);
    }
    if !left.is_finite() {
        left = region.x_left;
        right = region.x_right;
        bottom = region.y_bottom;
        top = region.y_top;
    }

    Some(Table {
        page_number,
        rows: cells.len(),
        cols: final_cols,
        bbox: [left, bottom, right, top],
        cells,
    })
}

/// Render tables as GitHub-flavored Markdown.
pub fn to_markdown(tables: &[Table]) -> String {
    let mut out = String::new();
    for (i, t) in tables.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&format!(
            "<!-- table: page {} ({}x{}) -->\n",
            t.page_number, t.rows, t.cols
        ));
        out.push_str(&render_one(t));
        out.push('\n');
    }
    out.trim_end().to_string()
}

fn render_one(t: &Table) -> String {
    if t.cells.is_empty() {
        return String::new();
    }
    let escape = |c: &str| c.replace('|', "\\|").replace('\n', " ");
    let mut s = String::new();
    let header = &t.cells[0];
    s.push_str("| ");
    s.push_str(
        &header
            .iter()
            .map(|c| escape(c))
            .collect::<Vec<_>>()
            .join(" | "),
    );
    s.push_str(" |\n| ");
    s.push_str(&vec!["---"; t.cols].join(" | "));
    s.push_str(" |\n");
    for row in &t.cells[1..] {
        s.push_str("| ");
        s.push_str(
            &row.iter()
                .map(|c| escape(c))
                .collect::<Vec<_>>()
                .join(" | "),
        );
        s.push_str(" |\n");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk(t: &str, x: f64, y: f64) -> TextBlock {
        TextBlock {
            text: t.into(),
            x,
            y,
            width: 10.0,
            height: 8.0,
            font_size: 8.0,
            font_name: "F".into(),
            page_number: 1,
        }
    }

    #[test]
    fn cluster_groups_within_tol() {
        let mut v = vec![10.0, 10.5, 50.0, 50.2, 90.0];
        let c = cluster(&mut v, 3.0);
        assert_eq!(c.len(), 3);
    }

    #[test]
    fn booktabs_table_columns_from_text() {
        // Horizontal rules only (booktabs): top/mid/bottom at y=40,24,0, x 0..120.
        let g = PageGraphics {
            page_number: 1,
            width: 200.0,
            height: 200.0,
            h_lines: vec![
                Seg {
                    x0: 0.0,
                    y0: 0.0,
                    x1: 120.0,
                    y1: 0.0,
                },
                Seg {
                    x0: 0.0,
                    y0: 24.0,
                    x1: 120.0,
                    y1: 24.0,
                },
                Seg {
                    x0: 0.0,
                    y0: 40.0,
                    x1: 120.0,
                    y1: 40.0,
                },
            ],
            v_lines: vec![],
        };
        // Two columns with a clear gap at x~30..70; two body rows + header.
        let page = PdfPage {
            index: 0,
            width: 200.0,
            height: 200.0,
            text_content: vec![
                mk("Name", 5.0, 30.0),
                mk("Score", 80.0, 30.0),
                mk("Alice", 5.0, 14.0),
                mk("90", 80.0, 14.0),
                mk("Bob", 5.0, 4.0),
                mk("75", 80.0, 4.0),
            ],
        };
        let mut out = Vec::new();
        detect_page_tables(&g, &page, &mut out);
        assert_eq!(out.len(), 1, "expected one table");
        let t = &out[0];
        assert_eq!(t.cols, 2);
        assert!(t.rows >= 3, "rows = {}", t.rows);
        assert_eq!(t.cells[0][0], "Name");
        assert_eq!(t.cells[0][1], "Score");
        assert_eq!(t.cells[2][1], "75");
    }

    #[test]
    fn splits_side_by_side_panels() {
        // Two 2-column panels separated by a wide whitespace corridor.
        let g = PageGraphics {
            page_number: 1,
            width: 400.0,
            height: 200.0,
            h_lines: vec![
                Seg {
                    x0: 0.0,
                    y0: 0.0,
                    x1: 360.0,
                    y1: 0.0,
                },
                Seg {
                    x0: 0.0,
                    y0: 20.0,
                    x1: 360.0,
                    y1: 20.0,
                },
                Seg {
                    x0: 0.0,
                    y0: 40.0,
                    x1: 360.0,
                    y1: 40.0,
                },
            ],
            v_lines: vec![],
        };
        // Left panel cols ~ x 10 and 60; right panel cols ~ x 250 and 300.
        let mut frags = Vec::new();
        for (r, y) in [(0usize, 30.0), (1, 14.0), (2, 4.0)] {
            frags.push(mk(&format!("a{r}"), 10.0, y));
            frags.push(mk(&format!("b{r}"), 60.0, y));
            frags.push(mk(&format!("c{r}"), 250.0, y));
            frags.push(mk(&format!("d{r}"), 300.0, y));
        }
        let page = PdfPage {
            index: 0,
            width: 400.0,
            height: 200.0,
            text_content: frags,
        };
        let mut out = Vec::new();
        detect_page_tables(&g, &page, &mut out);
        assert_eq!(out.len(), 2, "expected two side-by-side tables");
        assert!(out.iter().all(|t| t.cols == 2));
    }

    #[test]
    fn fully_bordered_uses_vlines() {
        let g = PageGraphics {
            page_number: 1,
            width: 200.0,
            height: 200.0,
            h_lines: vec![
                Seg {
                    x0: 0.0,
                    y0: 0.0,
                    x1: 100.0,
                    y1: 0.0,
                },
                Seg {
                    x0: 0.0,
                    y0: 20.0,
                    x1: 100.0,
                    y1: 20.0,
                },
                Seg {
                    x0: 0.0,
                    y0: 40.0,
                    x1: 100.0,
                    y1: 40.0,
                },
            ],
            v_lines: vec![
                Seg {
                    x0: 0.0,
                    y0: 0.0,
                    x1: 0.0,
                    y1: 40.0,
                },
                Seg {
                    x0: 50.0,
                    y0: 0.0,
                    x1: 50.0,
                    y1: 40.0,
                },
                Seg {
                    x0: 100.0,
                    y0: 0.0,
                    x1: 100.0,
                    y1: 40.0,
                },
            ],
        };
        let page = PdfPage {
            index: 0,
            width: 200.0,
            height: 200.0,
            text_content: vec![
                mk("A", 10.0, 28.0),
                mk("B", 60.0, 28.0),
                mk("C", 10.0, 8.0),
                mk("D", 60.0, 8.0),
            ],
        };
        let mut out = Vec::new();
        detect_page_tables(&g, &page, &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!((out[0].rows, out[0].cols), (2, 2));
        assert_eq!(out[0].cells[0][0], "A");
        assert_eq!(out[0].cells[1][1], "D");
    }
}
