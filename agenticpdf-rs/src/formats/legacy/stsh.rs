// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Derived from anydoc (https://github.com/firecrawl/anydoc), MIT licensed,
// Copyright (c) 2026 Sideguide Technologies Inc. See LICENSE-MIT-anydoc.txt.
//
//! The STSH (style sheet) and its inheritance chains, per [MS-DOC].
//!
//! Word stores styles as STD records that each carry a delta and a pointer to
//! the style they are based on. A "Heading 2" style typically states only what
//! differs from its parent, so resolving one means walking `istdBase` to the
//! root and applying the chain leaf-ward.
//!
//! Two properties of real files shape the implementation. Chains can be long —
//! any fixed depth cutoff loses inheritance in ordinary documents — and they
//! can contain cycles, because corrupt files exist. So resolution is memoised
//! (each style resolved once, reused by every descendant) and cycle-guarded
//! (a repeat visit stops the walk and resolves from the acyclic prefix).

use std::collections::{HashMap, HashSet};

use crate::container::ole::{u16_at, u32_at};

use super::sprm::{CharProps, PapProps, apply_pap, apply_style_chpx};

/// `istdNil`: the chain's terminator.
const ISTD_NIL: u16 = 0x0FFF;

/// A style with its chain fully applied.
#[derive(Debug, Clone, Default)]
pub struct ResolvedStyle {
    /// Effective character formatting.
    pub chp: CharProps,
    /// Effective paragraph properties.
    pub pap: PapProps,
    /// Heading level, for the built-in `heading 1`..`heading 9` styles.
    pub heading: Option<u8>,
    /// Whether the style's name marks preformatted text.
    pub code: bool,
    /// Whether the style's name marks a block quote.
    pub quote: bool,
}

/// The document's resolved style table.
#[derive(Debug, Default)]
pub struct Stylesheet {
    resolved: HashMap<u16, ResolvedStyle>,
    fallback: ResolvedStyle,
}

impl Stylesheet {
    /// The resolved style for an `istd`, or the default for an unknown one.
    pub fn get(&self, istd: u16) -> &ResolvedStyle {
        self.resolved.get(&istd).unwrap_or(&self.fallback)
    }
}

/// One STD record, before its chain is applied.
struct Std {
    /// Built-in style identifier; 1-9 are the heading styles.
    sti: u16,
    istd_base: u16,
    code: bool,
    quote: bool,
    /// Paragraph styles carry a paragraph UPX (prefixed with their own istd)
    /// and a character UPX; character styles carry only the latter.
    upx_papx: Vec<u8>,
    upx_chpx: Vec<u8>,
    is_paragraph: bool,
}

/// Parse the style sheet out of the table stream.
pub fn parse(word_doc: &[u8], table: &[u8]) -> Stylesheet {
    let Some(fc) = u32_at(word_doc, 0xA2).map(|v| v as usize) else {
        return Stylesheet::default();
    };
    let Some(lcb) = u32_at(word_doc, 0xA6).map(|v| v as usize) else {
        return Stylesheet::default();
    };
    let Some(stsh) = table.get(fc..fc.saturating_add(lcb)) else {
        return Stylesheet::default();
    };
    let (Some(cb_stshi), Some(count), Some(cb_std_base)) =
        (u16_at(stsh, 0), u16_at(stsh, 2), u16_at(stsh, 4))
    else {
        return Stylesheet::default();
    };

    let mut styles: HashMap<u16, Std> = HashMap::new();
    let mut pos = 2 + cb_stshi as usize;
    for istd in 0..count {
        let Some(cb_std) = u16_at(stsh, pos) else {
            break;
        };
        pos += 2;
        // A zero-length record is an empty slot, not the end of the array.
        if cb_std == 0 {
            continue;
        }
        let Some(record) = stsh.get(pos..).and_then(|rest| rest.get(..cb_std as usize)) else {
            break;
        };
        pos += cb_std as usize;
        if let Some(std) = parse_std(record, cb_std_base as usize) {
            styles.insert(istd, std);
        }
    }

    let mut sheet = Stylesheet::default();
    let mut memo: HashMap<u16, ResolvedStyle> = HashMap::new();
    for &istd in styles.keys() {
        let resolved = resolve(istd, &styles, &mut memo);
        sheet.resolved.insert(istd, resolved);
    }
    sheet
}

/// Parse one STD record.
fn parse_std(record: &[u8], cb_std_base: usize) -> Option<Std> {
    let first = u16_at(record, 0)?;
    let sti = first & 0x0FFF;
    let second = u16_at(record, 2)?;
    // sgc: 1 = paragraph style, 2 = character style.
    let sgc = second & 0x000F;
    let istd_base = (second >> 4) & 0x0FFF;
    let upx_count = u16_at(record, 4).map(|v| v & 0x000F).unwrap_or(0);

    // The style's name follows the fixed header area, then the UPX payloads,
    // each aligned to an even offset from the record start.
    let name_offset = cb_std_base.max(10);
    let name_len = u16_at(record, name_offset)? as usize;
    let name_bytes = name_len.checked_mul(2)?;
    let name = record
        .get(name_offset..)
        .and_then(|rest| rest.get(2..))
        .and_then(|rest| rest.get(..name_bytes))
        .map(crate::container::ole::decode_utf16le)
        .unwrap_or_default();

    // Past the length prefix, the name, and its terminator.
    let mut pos = name_offset.checked_add(4)?.checked_add(name_bytes)?;
    let mut payloads: Vec<&[u8]> = Vec::new();
    for _ in 0..upx_count {
        if pos % 2 == 1 {
            pos += 1;
        }
        let cb = u16_at(record, pos)? as usize;
        payloads.push(record.get(pos..)?.get(2..)?.get(..cb)?);
        pos += 2 + cb;
    }

    let is_paragraph = sgc == 1;
    let (upx_papx, upx_chpx) = if is_paragraph {
        (
            payloads.first().copied().unwrap_or_default().to_vec(),
            payloads.get(1).copied().unwrap_or_default().to_vec(),
        )
    } else {
        (
            Vec::new(),
            payloads.first().copied().unwrap_or_default().to_vec(),
        )
    };

    let lower = name.to_ascii_lowercase();
    Some(Std {
        sti,
        istd_base,
        code: lower.contains("code") || lower.contains("plain text"),
        quote: lower.contains("quote"),
        upx_papx,
        upx_chpx,
        is_paragraph,
    })
}

/// Resolve one style's chain, memoised across styles.
fn resolve(
    istd: u16,
    styles: &HashMap<u16, Std>,
    memo: &mut HashMap<u16, ResolvedStyle>,
) -> ResolvedStyle {
    if let Some(hit) = memo.get(&istd) {
        return hit.clone();
    }

    // Walk root-ward, collecting the part of the chain not yet resolved.
    let mut chain: Vec<u16> = Vec::new();
    let mut visiting: HashSet<u16> = HashSet::new();
    let mut base = ResolvedStyle::default();
    let mut cursor = Some(istd);

    while let Some(current) = cursor {
        if let Some(hit) = memo.get(&current) {
            base = hit.clone();
            break;
        }
        // A repeat visit means the file's chain has a cycle; resolve from what
        // came before it rather than looping.
        if !visiting.insert(current) {
            break;
        }
        let Some(style) = styles.get(&current) else {
            break;
        };
        chain.push(current);
        cursor =
            (style.istd_base != ISTD_NIL && style.istd_base != current).then_some(style.istd_base);
    }

    // Apply root-to-leaf, memoising each step so siblings reuse the work.
    for &current in chain.iter().rev() {
        let style = &styles[&current];
        if style.is_paragraph && style.upx_papx.len() >= 2 {
            // The paragraph UPX begins with the style's own istd.
            let mut delta = PapProps::default();
            apply_pap(&style.upx_papx[2..], &[], &mut delta);
            base.pap = base.pap.clone().merge(delta);
        }
        base.chp = apply_style_chpx(&style.upx_chpx, base.chp);
        if let sti @ 1..=9 = style.sti {
            base.heading = Some(sti as u8);
        }
        base.code |= style.code;
        base.quote |= style.quote;
        memo.insert(current, base.clone());
    }
    base
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style(sti: u16, istd_base: u16) -> Std {
        Std {
            sti,
            istd_base,
            code: false,
            quote: false,
            upx_papx: Vec::new(),
            upx_chpx: Vec::new(),
            is_paragraph: true,
        }
    }

    #[test]
    fn a_deep_chain_still_inherits_from_its_root() {
        // Real documents have chains far longer than any plausible fixed cap.
        let mut styles = HashMap::new();
        styles.insert(0u16, style(3, ISTD_NIL)); // built-in heading 3
        for index in 1u16..=40 {
            styles.insert(index, style(0x0FFE, index - 1));
        }
        let mut memo = HashMap::new();
        assert_eq!(resolve(40, &styles, &mut memo).heading, Some(3));
    }

    #[test]
    fn a_cycle_resolves_from_its_acyclic_prefix_instead_of_hanging() {
        let mut styles = HashMap::new();
        styles.insert(0u16, style(2, 1)); // based on a member of the cycle
        styles.insert(1u16, style(0x0FFE, 2));
        styles.insert(2u16, style(0x0FFE, 1));
        let mut memo = HashMap::new();
        assert_eq!(resolve(0, &styles, &mut memo).heading, Some(2));
    }

    #[test]
    fn character_formatting_accumulates_along_the_chain() {
        let mut styles = HashMap::new();
        let mut root = style(0x0FFE, ISTD_NIL);
        // Root sets bold.
        root.upx_chpx = vec![0x35, 0x08, 1];
        styles.insert(0u16, root);
        let mut child = style(0x0FFE, 0);
        // Child sets italic and leaves bold alone.
        child.upx_chpx = vec![0x36, 0x08, 1];
        styles.insert(1u16, child);

        let mut memo = HashMap::new();
        let resolved = resolve(1, &styles, &mut memo);
        assert!(resolved.chp.bold, "inherited from the root");
        assert!(resolved.chp.italic, "set by the child");
    }

    #[test]
    fn a_child_can_switch_an_inherited_property_off() {
        let mut styles = HashMap::new();
        let mut root = style(0x0FFE, ISTD_NIL);
        root.upx_chpx = vec![0x35, 0x08, 1];
        styles.insert(0u16, root);
        let mut child = style(0x0FFE, 0);
        child.upx_chpx = vec![0x35, 0x08, 0];
        styles.insert(1u16, child);

        let mut memo = HashMap::new();
        assert!(!resolve(1, &styles, &mut memo).chp.bold);
    }

    #[test]
    fn an_unknown_istd_gets_the_default_style() {
        let sheet = Stylesheet::default();
        let resolved = sheet.get(0x0FFF);
        assert!(resolved.heading.is_none());
        assert!(!resolved.chp.bold);
    }

    #[test]
    fn a_stylesheet_with_no_table_stream_is_empty_not_an_error() {
        let sheet = parse(&[0u8; 0x100], &[]);
        assert!(sheet.resolved.is_empty());
    }
}
