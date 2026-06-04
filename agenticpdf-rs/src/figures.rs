// SPDX-License-Identifier: AGPL-3.0-or-later
//! Figure / caption detection and linking.
//!
//! Associates placed image XObjects with their captions ("Figure N", "Chart
//! N", "Table N") by spatial proximity, and surfaces caption-only (vector)
//! figures. Produces structured records with page, bounding box, label, and
//! caption text for citation and RAG.

use crate::engine::{self, PlacedImage};
use crate::{PdfDocument, PdfError, TextBlock};
use serde::{Deserialize, Serialize};

/// A figure (raster image and/or captioned graphic).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Figure {
    pub id: String,
    /// "image" (no caption), "figure", "chart", or "table".
    pub kind: String,
    pub page_number: usize,
    /// Bounding box [left, bottom, right, top] in PDF points.
    pub bbox: [f64; 4],
    /// Caption label such as "Figure 3", if linked.
    pub label: Option<String>,
    /// Full caption text, if linked.
    pub caption: Option<String>,
    /// Image pixel dimensions (0 for caption-only figures).
    pub width: u32,
    pub height: u32,
}

struct Caption {
    label: String,
    kind: String,
    text: String,
    bbox: [f64; 4],
    used: bool,
}

/// Detect figures across the document and link captions.
pub fn extract_figures(data: &[u8], doc: &PdfDocument) -> Result<Vec<Figure>, PdfError> {
    let images = engine::extract_placed_images(data)?;
    let mut figures = Vec::new();
    let mut counter = 0usize;

    for page in &doc.pages {
        let page_no = page.index + 1;
        let mut captions = find_captions(&page.text_content);
        let page_images: Vec<&PlacedImage> =
            images.iter().filter(|i| i.page_number == page_no).collect();

        // Link each image to its nearest caption.
        for img in page_images {
            let best = nearest_caption(img, &captions);
            let (label, caption, kind) = match best {
                Some(idx) => {
                    captions[idx].used = true;
                    let c = &captions[idx];
                    (Some(c.label.clone()), Some(c.text.clone()), c.kind.clone())
                }
                None => (None, None, "image".to_string()),
            };
            figures.push(Figure {
                id: format!("fig_{}", counter),
                kind,
                page_number: page_no,
                bbox: img.bbox,
                label,
                caption,
                width: img.width,
                height: img.height,
            });
            counter += 1;
        }

        // Caption-only (vector) figures and charts that matched no raster image.
        for c in captions.iter().filter(|c| !c.used && c.kind != "table") {
            figures.push(Figure {
                id: format!("fig_{}", counter),
                kind: c.kind.clone(),
                page_number: page_no,
                bbox: c.bbox,
                label: Some(c.label.clone()),
                caption: Some(c.text.clone()),
                width: 0,
                height: 0,
            });
            counter += 1;
        }
    }

    Ok(figures)
}

/// Find the index of the caption nearest an image (x-overlap, small vertical
/// gap, preferring captions below the image).
fn nearest_caption(img: &PlacedImage, captions: &[Caption]) -> Option<usize> {
    let [il, ib, ir, it] = img.bbox;
    let mut best: Option<(usize, f64)> = None;
    for (i, c) in captions.iter().enumerate() {
        if c.used {
            continue;
        }
        let [cl, cb, cr, ct] = c.bbox;
        // Require horizontal overlap.
        if cr < il - 2.0 || cl > ir + 2.0 {
            continue;
        }
        // Vertical gap to the image, with a bias for captions below.
        let gap_below = ib - ct; // caption top below image bottom
        let gap_above = cb - it; // caption bottom above image top
        let score = if gap_below >= -2.0 {
            gap_below.max(0.0)
        } else if gap_above >= -2.0 {
            gap_above.max(0.0) + 12.0 // small penalty for above-figure captions
        } else {
            continue; // overlapping vertically — unlikely a caption
        };
        if score <= 80.0 && best.map(|(_, s)| score < s).unwrap_or(true) {
            best = Some((i, score));
        }
    }
    best.map(|(i, _)| i)
}

/// Cluster page fragments into lines and return those that are captions.
fn find_captions(frags: &[TextBlock]) -> Vec<Caption> {
    let mut frags: Vec<&TextBlock> = frags.iter().filter(|f| !f.text.trim().is_empty()).collect();
    frags.sort_by(|a, b| {
        b.y.partial_cmp(&a.y)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
    });

    // Build lines.
    struct Line {
        text: String,
        x0: f64,
        x1: f64,
        y0: f64,
        y1: f64,
    }
    let mut lines: Vec<Line> = Vec::new();
    for f in frags {
        let tol = f.font_size.max(1.0) * 0.5;
        match lines
            .last_mut()
            .filter(|l| (l.y0 - f.y).abs() <= tol && f.x >= l.x0 - 1.0)
        {
            Some(l) => {
                if f.x - l.x1 > f.font_size.max(1.0) * 0.25 && !l.text.ends_with(' ') {
                    l.text.push(' ');
                }
                l.text.push_str(&f.text);
                l.x1 = l.x1.max(f.x + f.width);
                l.x0 = l.x0.min(f.x);
                l.y1 = l.y1.max(f.y + f.height);
            }
            None => lines.push(Line {
                text: f.text.clone(),
                x0: f.x,
                x1: f.x + f.width,
                y0: f.y,
                y1: f.y + f.height,
            }),
        }
    }

    lines
        .into_iter()
        .filter_map(|l| {
            classify_caption(l.text.trim()).map(|(label, kind)| Caption {
                label,
                kind,
                text: l.text.trim().to_string(),
                bbox: [l.x0, l.y0, l.x1, l.y1],
                used: false,
            })
        })
        .collect()
}

/// If a line begins with a figure/table/chart label, return (label, kind).
fn classify_caption(text: &str) -> Option<(String, String)> {
    let lower = text.to_ascii_lowercase();
    let (kind, prefix_len) = if lower.starts_with("figure ") {
        ("figure", 7)
    } else if lower.starts_with("fig. ") {
        ("figure", 5)
    } else if lower.starts_with("fig ") {
        ("figure", 4)
    } else if lower.starts_with("chart ") {
        ("chart", 6)
    } else if lower.starts_with("table ") {
        ("table", 6)
    } else {
        return None;
    };
    // Next token must start with a digit...
    let rest = text[prefix_len..].trim_start();
    let num: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if num.is_empty() {
        return None;
    }
    // ...followed by a caption delimiter (": ", ". ", ")"), which distinguishes
    // a real caption from an in-text reference like "Figure 2 shows ...".
    let after = rest[num.len()..].trim_start_matches(['.', ':', ')', ']']);
    let delim_ok = after.len() < rest[num.len()..].len(); // something was stripped
    if !delim_ok {
        return None;
    }
    let label_word = match kind {
        "figure" => "Figure",
        "chart" => "Chart",
        _ => "Table",
    };
    Some((format!("{} {}", label_word, num), kind.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify() {
        assert_eq!(
            classify_caption("Figure 3: Architecture"),
            Some(("Figure 3".into(), "figure".into()))
        );
        assert_eq!(
            classify_caption("Table 1. Results"),
            Some(("Table 1".into(), "table".into()))
        );
        assert_eq!(classify_caption("Figurative language"), None);
        assert_eq!(classify_caption("regular text"), None);
        // In-text references are not captions.
        assert_eq!(classify_caption("Figure 2 shows detection outcomes"), None);
    }

    #[test]
    fn links_caption_below_image() {
        let img = PlacedImage {
            page_number: 1,
            name: "Im0".into(),
            bbox: [100.0, 500.0, 300.0, 700.0], // image up high
            width: 200,
            height: 200,
            color_space: "DeviceRGB".into(),
        };
        // Caption just below the image (top at 495, within gap).
        let captions = vec![Caption {
            label: "Figure 1".into(),
            kind: "figure".into(),
            text: "Figure 1: A diagram".into(),
            bbox: [100.0, 482.0, 300.0, 495.0],
            used: false,
        }];
        assert_eq!(nearest_caption(&img, &captions), Some(0));
    }
}
