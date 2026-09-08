// SPDX-License-Identifier: AGPL-3.0-or-later
//! Plain-text, delimited-text and Markdown parsers.
//!
//! These three formats share a module because they share a lexical layer:
//! all are UTF-8 line-oriented text, and the only question is how much
//! structure to read out of the lines. Plain text yields paragraphs, delimited
//! text yields one table, and Markdown yields the full block model.
//!
//! The Markdown reader covers the CommonMark constructs that survive a
//! round-trip through [`crate::doc::to_markdown`] — headings, lists (including
//! task lists and nesting), fenced and indented code, block quotes, GFM tables,
//! thematic breaks — plus the inline span types. Reference-style links,
//! HTML blocks and footnote definitions are out of scope; the goal is a
//! faithful document model, not a conforming CommonMark implementation.

use crate::container::zip::decode_utf8_lossy;
use std::collections::HashMap;

use crate::doc::{
    Align, Block, Cell, Inline, List, ListItem, Row, Run, SemanticDoc, Table, TextStyle,
};

/// Parse plain text into paragraphs split on blank lines.
///
/// Single newlines inside a paragraph are treated as soft wraps and joined,
/// which is how plain-text prose is conventionally written.
pub fn parse_text(data: &[u8]) -> SemanticDoc {
    let text = decode_utf8_lossy(data);
    let mut doc = SemanticDoc::new();

    for chunk in text.split("\n\n") {
        let joined = chunk
            .lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join(" ");
        let joined = joined.trim();
        if !joined.is_empty() {
            doc.push(Block::paragraph(joined));
        }
    }

    doc
}

/// Parse delimited text (CSV/TSV) into a single table.
///
/// Quoting follows RFC 4180: fields may be double-quoted, a doubled quote is a
/// literal quote, and a quoted field may span lines. The delimiter is inferred
/// when not given.
pub fn parse_csv(data: &[u8], delimiter: Option<char>) -> SemanticDoc {
    let text = decode_utf8_lossy(data);
    let delimiter = delimiter.unwrap_or_else(|| infer_delimiter(&text));
    let records = split_records(&text, delimiter);

    let mut doc = SemanticDoc::new();
    if records.is_empty() {
        return doc;
    }

    let rows: Vec<Row> = records
        .into_iter()
        .map(|fields| Row {
            cells: fields.into_iter().map(Cell::text).collect(),
        })
        .collect();

    doc.push(Block::Table(Table {
        // The first record is treated as a header: it is right far more often
        // than not, and a GFM table requires one regardless.
        header_rows: 1,
        rows,
        ..Table::default()
    }));
    doc
}

/// Pick the delimiter whose per-line count is most consistent.
fn infer_delimiter(text: &str) -> char {
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(8)
        .collect();

    for candidate in ['\t', ',', ';', '|'] {
        let counts: Vec<usize> = lines
            .iter()
            .map(|line| line.matches(candidate).count())
            .collect();
        if counts.len() >= 2 && counts[0] > 0 && counts.iter().all(|&c| c == counts[0]) {
            return candidate;
        }
    }
    ','
}

/// Split delimited text into records of fields, honouring RFC 4180 quoting.
fn split_records(text: &str, delimiter: char) -> Vec<Vec<String>> {
    let mut records = Vec::new();
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if in_quotes {
            if ch == '"' {
                // A doubled quote inside a quoted field is a literal quote.
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    in_quotes = false;
                }
            } else {
                field.push(ch);
            }
            continue;
        }

        match ch {
            '"' if field.is_empty() => in_quotes = true,
            c if c == delimiter => fields.push(std::mem::take(&mut field)),
            '\r' => {}
            '\n' => {
                fields.push(std::mem::take(&mut field));
                // Skip records that are entirely empty (trailing newline).
                if fields.iter().any(|f| !f.trim().is_empty()) {
                    records.push(std::mem::take(&mut fields));
                } else {
                    fields.clear();
                }
            }
            _ => field.push(ch),
        }
    }

    if !field.is_empty() || !fields.is_empty() {
        fields.push(field);
        if fields.iter().any(|f| !f.trim().is_empty()) {
            records.push(fields);
        }
    }

    records
}

// ============================================================================
// Markdown
// ============================================================================

/// Parse Markdown into the semantic document model.
pub fn parse_markdown(data: &[u8]) -> SemanticDoc {
    let text = decode_utf8_lossy(data);
    let lines: Vec<&str> = text.lines().collect();
    let mut doc = SemanticDoc::new();

    // The definitions are taken out before the body is read, both because they
    // are not body text and because a reference has to know which note it
    // names before the paragraph holding it is built.
    let (lines, notes) = take_footnotes(&lines);
    let blocks = parse_blocks(&lines, 0, &notes);
    doc.body().blocks = blocks;
    doc.footnotes = notes.definitions();

    // A document whose first block is a level-1 heading takes it as the title,
    // matching how the other format parsers populate metadata.
    if let Some(Block::Heading { level: 1, content }) = doc.sections[0].blocks.first() {
        doc.title = Some(crate::doc::inline_text(content));
    }
    doc
}

/// The footnotes a document defines, by the label a reference uses.
///
/// `[^1]: text` is not Markdown proper, but it is what this writer emits and
/// what every Markdown that has footnotes at all spells them as.
#[derive(Debug, Default)]
struct Notes {
    /// Label to position, which is the index a reference resolves to.
    index_of: HashMap<String, usize>,
    /// Each note's text, in the order the labels were first defined.
    bodies: Vec<(String, String)>,
}

impl Notes {
    fn index(&self, label: &str) -> Option<usize> {
        self.index_of.get(label).copied()
    }

    fn definitions(&self) -> Vec<crate::doc::Footnote> {
        self.bodies
            .iter()
            .map(|(label, text)| crate::doc::Footnote {
                label: Some(label.clone()),
                blocks: vec![Block::Paragraph {
                    content: parse_inlines_with(text, &Notes::default()),
                    align: crate::doc::Align::Left,
                    indent: 0.0,
                }],
            })
            .collect()
    }
}

/// Split the footnote definitions out of a document's lines.
///
/// A definition runs to the end of its line; a continuation indented under it
/// belongs to the same note, which is how the writer wraps a long one.
fn take_footnotes<'a>(lines: &[&'a str]) -> (Vec<&'a str>, Notes) {
    /// More labels than this is not a document with footnotes in it.
    const MAX_NOTES: usize = 4096;

    let mut kept = Vec::with_capacity(lines.len());
    let mut notes = Notes::default();
    let mut open: Option<usize> = None;

    for line in lines {
        if let Some((label, text)) = footnote_definition(line)
            && notes.bodies.len() < MAX_NOTES
        {
            open = Some(notes.bodies.len());
            notes
                .index_of
                .entry(label.to_string())
                .or_insert(notes.bodies.len());
            notes.bodies.push((label.to_string(), text.to_string()));
            continue;
        }
        // A blank line ends the note; anything else at the left margin is the
        // document again.
        let continues = open.is_some()
            && line.starts_with(&[' ', '\t'][..])
            && !line.trim().is_empty();
        match continues {
            true => {
                if let Some(at) = open
                    && let Some((_, body)) = notes.bodies.get_mut(at)
                {
                    body.push(' ');
                    body.push_str(line.trim());
                }
            }
            false => {
                open = None;
                kept.push(*line);
            }
        }
    }
    (kept, notes)
}

/// A `[^label]: text` line, as its label and its text.
fn footnote_definition(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix("[^")?;
    let close = rest.find("]:")?;
    let label = &rest[..close];
    if label.is_empty() || label.contains('[') {
        return None;
    }
    Some((label, rest[close + 2..].trim()))
}

/// Maximum nesting of lists and block quotes.
///
/// Both recurse per level, so without a bound `> > > …` repeated a few thousand
/// times overflows the stack — which aborts the process rather than raising an
/// error a caller could handle. Real documents nest a handful of levels; the
/// limit is generous enough that no genuine one reaches it.
const MAX_BLOCK_DEPTH: usize = 64;

/// Parse a run of lines into blocks. Recursion handles nested list content and
/// block quotes.
fn parse_blocks(lines: &[&str], depth: usize, notes: &Notes) -> Vec<Block> {
    let mut blocks = Vec::new();
    if depth > MAX_BLOCK_DEPTH {
        // Past the limit the content is flattened into paragraphs rather than
        // dropped, so its text still reaches the caller.
        for line in lines {
            let trimmed = line.trim().trim_start_matches(['>', '-', '*', '+', ' ']);
            if !trimmed.is_empty() {
                blocks.push(Block::paragraph(trimmed));
            }
        }
        return blocks;
    }
    let mut at = 0usize;

    while at < lines.len() {
        let line = lines[at];
        let trimmed = line.trim();

        if trimmed.is_empty() {
            at += 1;
            continue;
        }

        // Thematic break: three or more of the same marker, nothing else.
        if is_thematic_break(trimmed) {
            blocks.push(Block::Divider);
            at += 1;
            continue;
        }

        // Fenced code.
        if let Some(fence) = code_fence(trimmed) {
            let language = trimmed[fence.len()..].trim();
            let mut body = Vec::new();
            at += 1;
            while at < lines.len() && !lines[at].trim().starts_with(&fence) {
                body.push(lines[at]);
                at += 1;
            }
            at += 1; // consume the closing fence
            blocks.push(Block::Code {
                language: (!language.is_empty()).then(|| language.to_string()),
                text: body.join("\n"),
            });
            continue;
        }

        // ATX heading.
        if let Some((level, content)) = atx_heading(trimmed) {
            blocks.push(Block::Heading {
                level,
                content: parse_inlines_with(content, notes),
            });
            at += 1;
            continue;
        }

        // Setext heading: text underlined by = or -.
        if at + 1 < lines.len()
            && let Some(level) = setext_level(lines[at + 1].trim())
        {
            blocks.push(Block::Heading {
                level,
                content: parse_inlines_with(trimmed, notes),
            });
            at += 2;
            continue;
        }

        // Block quote: consume the contiguous run and recurse on the stripped
        // content so nested constructs inside a quote still parse.
        if trimmed.starts_with('>') {
            let mut inner = Vec::new();
            while at < lines.len() && lines[at].trim_start().starts_with('>') {
                let stripped = lines[at].trim_start();
                let stripped = stripped[1..].strip_prefix(' ').unwrap_or(&stripped[1..]);
                inner.push(stripped);
                at += 1;
            }
            blocks.push(Block::Quote(parse_blocks(&inner, depth + 1, notes)));
            continue;
        }

        // GFM table: a header row followed by a delimiter row of dashes.
        if trimmed.starts_with('|')
            && at + 1 < lines.len()
            && is_table_delimiter(lines[at + 1].trim())
        {
            let (table, consumed) = parse_table(&lines[at..], notes);
            blocks.push(Block::Table(table));
            at += consumed;
            continue;
        }

        // Lists.
        if list_marker(line).is_some() {
            let (list, consumed) = parse_list(&lines[at..], depth + 1, notes);
            blocks.push(Block::List(list));
            at += consumed;
            continue;
        }

        // Indented code: four spaces or a tab, outside a list.
        if line.starts_with("    ") || line.starts_with('\t') {
            let mut body = Vec::new();
            while at < lines.len()
                && (lines[at].starts_with("    ")
                    || lines[at].starts_with('\t')
                    || lines[at].trim().is_empty())
            {
                if lines[at].trim().is_empty() && !body.is_empty() {
                    // A blank line only continues the block if more code follows.
                    let more = lines[at + 1..]
                        .iter()
                        .find(|l| !l.trim().is_empty())
                        .is_some_and(|l| l.starts_with("    ") || l.starts_with('\t'));
                    if !more {
                        break;
                    }
                }
                let stripped = lines[at]
                    .strip_prefix("    ")
                    .or_else(|| lines[at].strip_prefix('\t'))
                    .unwrap_or("");
                body.push(stripped);
                at += 1;
            }
            blocks.push(Block::Code {
                language: None,
                text: body.join("\n").trim_end().to_string(),
            });
            continue;
        }

        // Paragraph: run to the next blank line or block-level construct.
        let mut paragraph = Vec::new();
        while at < lines.len() {
            let candidate = lines[at];
            let candidate_trimmed = candidate.trim();
            if candidate_trimmed.is_empty()
                || is_thematic_break(candidate_trimmed)
                || code_fence(candidate_trimmed).is_some()
                || atx_heading(candidate_trimmed).is_some()
                || candidate_trimmed.starts_with('>')
                || (!paragraph.is_empty() && list_marker(candidate).is_some())
            {
                break;
            }
            if !paragraph.is_empty() && setext_level(candidate_trimmed).is_some() {
                break;
            }
            paragraph.push(candidate_trimmed);
            at += 1;
        }
        if !paragraph.is_empty() {
            blocks.push(Block::Paragraph {
                content: parse_inlines_with(&paragraph.join(" "), notes),
                align: Align::Left,
                indent: 0.0,
            });
        }
    }

    blocks
}

fn is_thematic_break(line: &str) -> bool {
    for marker in ['-', '*', '_'] {
        let stripped: String = line.chars().filter(|c| !c.is_whitespace()).collect();
        if stripped.len() >= 3 && stripped.chars().all(|c| c == marker) {
            return true;
        }
    }
    false
}

fn code_fence(line: &str) -> Option<String> {
    for marker in ['`', '~'] {
        let count = line.chars().take_while(|&c| c == marker).count();
        if count >= 3 {
            return Some(marker.to_string().repeat(count));
        }
    }
    None
}

fn atx_heading(line: &str) -> Option<(u8, &str)> {
    let hashes = line.chars().take_while(|&c| c == '#').count();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    let rest = &line[hashes..];
    // A heading needs a space after the hashes; `#tag` is not a heading.
    if !rest.is_empty() && !rest.starts_with(' ') {
        return None;
    }
    Some((hashes as u8, rest.trim().trim_end_matches('#').trim()))
}

fn setext_level(line: &str) -> Option<u8> {
    if line.len() >= 2 && line.chars().all(|c| c == '=') {
        return Some(1);
    }
    if line.len() >= 2 && line.chars().all(|c| c == '-') {
        return Some(2);
    }
    None
}

fn is_table_delimiter(line: &str) -> bool {
    let body = line.trim().trim_matches('|');
    !body.is_empty()
        && body.split('|').all(|cell| {
            let cell = cell.trim();
            !cell.is_empty() && cell.chars().all(|c| c == '-' || c == ':') && cell.contains('-')
        })
}

/// Parse a GFM pipe table, returning it and the number of lines consumed.
fn parse_table(lines: &[&str], notes: &Notes) -> (Table, usize) {
    let split_row = |line: &str| -> Vec<String> {
        // Split on unescaped pipes, dropping the leading and trailing ones.
        let mut cells = Vec::new();
        let mut current = String::new();
        let mut escaped = false;
        for ch in line.trim().trim_matches('|').chars() {
            if escaped {
                current.push(ch);
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '|' {
                cells.push(std::mem::take(&mut current));
            } else {
                current.push(ch);
            }
        }
        cells.push(current);
        cells.into_iter().map(|c| c.trim().to_string()).collect()
    };

    let mut rows = vec![Row {
        cells: split_row(lines[0])
            .into_iter()
            .map(|text| Cell {
                blocks: vec![Block::Paragraph {
                    content: parse_inlines_with(&text, notes),
                    align: Align::Left,
                    indent: 0.0,
                }],
                ..Cell::default()
            })
            .collect(),
    }];

    let mut consumed = 2; // header + delimiter
    while consumed < lines.len() {
        let line = lines[consumed].trim();
        if !line.starts_with('|') {
            break;
        }
        rows.push(Row {
            cells: split_row(line)
                .into_iter()
                .map(|text| Cell {
                    blocks: vec![Block::Paragraph {
                        content: parse_inlines_with(&text, notes),
                        align: Align::Left,
                        indent: 0.0,
                    }],
                    ..Cell::default()
                })
                .collect(),
        });
        consumed += 1;
    }

    (
        Table {
            header_rows: 1,
            rows,
            ..Table::default()
        },
        consumed,
    )
}

/// A list marker at the start of a line: `(indent columns, ordered, start
/// number, marker width)`.
fn list_marker(line: &str) -> Option<(usize, bool, u64, usize)> {
    let indent = line.len() - line.trim_start().len();
    let trimmed = line.trim_start();

    if let Some(rest) = trimmed.strip_prefix(['-', '*', '+'])
        && rest.starts_with(' ')
    {
        return Some((indent, false, 1, 2));
    }

    let digits: String = trimmed.chars().take_while(char::is_ascii_digit).collect();
    if !digits.is_empty() && digits.len() <= 9 {
        let rest = &trimmed[digits.len()..];
        if (rest.starts_with(". ") || rest.starts_with(") "))
            && let Ok(start) = digits.parse::<u64>()
        {
            return Some((indent, true, start, digits.len() + 2));
        }
    }
    None
}

/// Parse a list and everything nested inside it, returning it and the number of
/// lines consumed.
fn parse_list(lines: &[&str], depth: usize, notes: &Notes) -> (List, usize) {
    let (base_indent, ordered, start, _) = list_marker(lines[0]).expect("caller checked");
    let mut list = List {
        ordered,
        start,
        items: Vec::new(),
    };

    let mut at = 0usize;
    // Lines belonging to the item currently being accumulated.
    let mut item_lines: Vec<String> = Vec::new();
    let mut checked: Option<bool> = None;

    let flush = |item_lines: &mut Vec<String>, checked: &mut Option<bool>, list: &mut List| {
        if item_lines.is_empty() {
            return;
        }
        let owned: Vec<&str> = item_lines.iter().map(String::as_str).collect();
        list.items.push(ListItem {
            blocks: parse_blocks(&owned, depth, notes),
            checked: checked.take(),
        });
        item_lines.clear();
    };

    while at < lines.len() {
        let line = lines[at];

        if line.trim().is_empty() {
            // A blank line ends the list unless the next content is still
            // indented under it.
            let next = lines[at + 1..].iter().find(|l| !l.trim().is_empty());
            match next {
                Some(next)
                    if indent_of(next) > base_indent || is_sibling(next, base_indent, ordered) =>
                {
                    item_lines.push(String::new());
                    at += 1;
                    continue;
                }
                _ => break,
            }
        }

        match list_marker(line) {
            // A marker of the *other* kind at this level starts a new list, it
            // does not continue this one — `1. a` followed by `- b` is two
            // lists, not one.
            Some((indent, other_ordered, _, _))
                if indent == base_indent && other_ordered != ordered =>
            {
                break;
            }
            // A sibling item at this level: close the previous one.
            Some((indent, _, _, width)) if indent == base_indent => {
                flush(&mut item_lines, &mut checked, &mut list);
                let content = &line.trim_start()[width..];
                let (task, content) = task_marker(content);
                checked = task;
                item_lines.push(content.to_string());
                at += 1;
            }
            // A marker indented further belongs to the current item; keep it
            // with its relative indentation so the recursive parse sees a list.
            Some(_) => {
                item_lines.push(dedent(line, base_indent));
                at += 1;
            }
            // A continuation line, indented under the item.
            None if indent_of(line) > base_indent => {
                item_lines.push(dedent(line, base_indent));
                at += 1;
            }
            // Lazy continuation of the item's paragraph.
            None if !item_lines.is_empty() && !item_lines.last().is_some_and(String::is_empty) => {
                item_lines.push(line.trim().to_string());
                at += 1;
            }
            None => break,
        }
    }

    flush(&mut item_lines, &mut checked, &mut list);
    (list, at)
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Whether `line` opens another item of the *same* list.
fn is_sibling(line: &str, base_indent: usize, ordered: bool) -> bool {
    list_marker(line).is_some_and(|(indent, other_ordered, _, _)| {
        indent == base_indent && other_ordered == ordered
    })
}

/// Remove up to `columns` leading spaces, plus the list marker's own width.
fn dedent(line: &str, columns: usize) -> String {
    let strip = line.len() - line.trim_start().len();
    let remove = strip.min(columns + 2);
    line[remove..].to_string()
}

/// Split a leading `[x]` / `[ ]` task marker off an item's content.
fn task_marker(content: &str) -> (Option<bool>, &str) {
    let trimmed = content.trim_start();
    for (prefix, value) in [("[x] ", true), ("[X] ", true), ("[ ] ", false)] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return (Some(value), rest);
        }
    }
    (None, content)
}

// ============================================================================
// Inline parsing
// ============================================================================

/// Parse inline spans: emphasis, code, links, images and hard breaks.
pub fn parse_inlines(text: &str) -> Vec<Inline> {
    parse_inlines_with(text, &Notes::default())
}

fn parse_inlines_with(text: &str, notes: &Notes) -> Vec<Inline> {
    let mut out = Vec::new();
    let mut buffer = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut at = 0usize;

    let flush = |buffer: &mut String, out: &mut Vec<Inline>| {
        if !buffer.is_empty() {
            out.push(Inline::Run(Run::plain(std::mem::take(buffer))));
        }
    };

    while at < chars.len() {
        let ch = chars[at];

        // Backslash escape: the next character is literal.
        if ch == '\\' && at + 1 < chars.len() {
            buffer.push(chars[at + 1]);
            at += 2;
            continue;
        }

        // Inline code spans win over every other construct.
        if ch == '`' {
            let ticks = chars[at..].iter().take_while(|&&c| c == '`').count();
            let fence: String = "`".repeat(ticks);
            if let Some(end) = find_from(&chars, at + ticks, &fence) {
                flush(&mut buffer, &mut out);
                let code: String = chars[at + ticks..end].iter().collect();
                out.push(Inline::Run(Run::styled(
                    code.trim(),
                    TextStyle {
                        code: true,
                        ..TextStyle::default()
                    },
                )));
                at = end + ticks;
                continue;
            }
        }

        // A footnote reference, which must be tested before a link: `[^1]`
        // is bracketed like one and is not one. Written by this writer and,
        // until now, read back by nothing -- so a document that went out as
        // Markdown and came back had `\[^1\]` in place of its notes.
        if ch == '['
            && chars.get(at + 1) == Some(&'^')
            && let Some((label, next)) = footnote_reference(&chars, at)
            && let Some(index) = notes.index(&label)
        {
            flush(&mut buffer, &mut out);
            out.push(Inline::FootnoteRef { index });
            at = next;
            continue;
        }

        // The inline HTML Markdown has always used for what it has no mark
        // for. Only the tags this model has a meaning for: everything else is
        // left as the text it appears as, which is what Markdown says to do
        // with HTML a reader does not handle.
        if ch == '<'
            && let Some((style, inner, next)) = html_span(&chars, at)
        {
            flush(&mut buffer, &mut out);
            for mut run in runs_of(&parse_inlines_with(&inner, notes)) {
                run.style.subscript |= style.subscript;
                run.style.superscript |= style.superscript;
                run.style.strikethrough |= style.strikethrough;
                run.style.underline |= style.underline;
                run.style.bold |= style.bold;
                run.style.italic |= style.italic;
                out.push(Inline::Run(run));
            }
            at = next;
            continue;
        }

        // Image, then link — `![` must be tested before `[`.
        if ch == '!'
            && chars.get(at + 1) == Some(&'[')
            && let Some((alt, target, next)) = link_parts(&chars, at + 1)
        {
            flush(&mut buffer, &mut out);
            out.push(Inline::Image(crate::doc::ImageRef {
                asset_id: target,
                alt: (!alt.is_empty()).then_some(alt),
                ..crate::doc::ImageRef::default()
            }));
            at = next;
            continue;
        }

        if ch == '['
            && let Some((label, target, next)) = link_parts(&chars, at)
        {
            flush(&mut buffer, &mut out);
            out.push(Inline::Link {
                href: target,
                runs: runs_of(&parse_inlines_with(&label, notes)),
            });
            at = next;
            continue;
        }

        // Emphasis, longest marker first so `***` and `~~` are not mis-split.
        if let Some((marker, style)) = emphasis_marker(&chars, at)
            && let Some(end) = find_from(&chars, at + marker.len(), &marker)
        {
            let inner: String = chars[at + marker.len()..end].iter().collect();
            if !inner.trim().is_empty() {
                flush(&mut buffer, &mut out);
                for mut run in runs_of(&parse_inlines_with(&inner, notes)) {
                    run.style.bold |= style.bold;
                    run.style.italic |= style.italic;
                    run.style.strikethrough |= style.strikethrough;
                    out.push(Inline::Run(run));
                }
                at = end + marker.len();
                continue;
            }
        }

        // Two trailing spaces before a newline is a hard break; the line
        // joining upstream has already collapsed the newline itself.
        buffer.push(ch);
        at += 1;
    }

    flush(&mut buffer, &mut out);
    out
}

/// Flatten inlines to runs, for contexts that cannot nest (link labels).
fn runs_of(inlines: &[Inline]) -> Vec<Run> {
    let mut runs = Vec::new();
    for inline in inlines {
        match inline {
            Inline::Run(run) => runs.push(run.clone()),
            Inline::Link { runs: inner, .. } => runs.extend(inner.iter().cloned()),
            _ => {}
        }
    }
    runs
}

/// A `[^label]` reference at `at`, as its label and where it ends.
fn footnote_reference(chars: &[char], at: usize) -> Option<(String, usize)> {
    let close = chars[at + 2..].iter().position(|&c| c == ']')? + at + 2;
    let label: String = chars[at + 2..close].iter().collect();
    match label.is_empty() || label.contains('[') {
        true => None,
        false => Some((label, close + 1)),
    }
}

/// An inline HTML span at `at`, as the style it sets and what it wraps.
///
/// Only the tags the model can hold, and only in their simple form: a tag with
/// attributes is markup this reader has no use for, and passing it through as
/// text is both honest and what Markdown prescribes.
fn html_span(chars: &[char], at: usize) -> Option<(TextStyle, String, usize)> {
    /// A tag name and what it turns on.
    type Tag = (&'static str, fn(&mut TextStyle));

    const TAGS: [Tag; 10] = [
        ("sub", |style| style.subscript = true),
        ("sup", |style| style.superscript = true),
        ("del", |style| style.strikethrough = true),
        ("s", |style| style.strikethrough = true),
        ("u", |style| style.underline = true),
        ("ins", |style| style.underline = true),
        ("b", |style| style.bold = true),
        ("strong", |style| style.bold = true),
        ("i", |style| style.italic = true),
        ("em", |style| style.italic = true),
    ];

    let rest: String = chars[at..].iter().take(16).collect::<String>().to_lowercase();
    for (tag, apply) in TAGS {
        let open = format!("<{tag}>");
        if !rest.starts_with(&open) {
            continue;
        }
        let close = format!("</{tag}>");
        let Some(end) = find_from(chars, at + open.chars().count(), &close) else {
            continue;
        };
        let inner: String = chars[at + open.chars().count()..end].iter().collect();
        let mut style = TextStyle::default();
        apply(&mut style);
        return Some((style, inner, end + close.chars().count()));
    }
    None
}

/// Match an emphasis opener at `at`, returning its marker and the style it sets.
fn emphasis_marker(chars: &[char], at: usize) -> Option<(String, TextStyle)> {
    let bold = TextStyle {
        bold: true,
        ..TextStyle::default()
    };
    let italic = TextStyle {
        italic: true,
        ..TextStyle::default()
    };
    let bold_italic = TextStyle {
        bold: true,
        italic: true,
        ..TextStyle::default()
    };
    let strike = TextStyle {
        strikethrough: true,
        ..TextStyle::default()
    };

    let starts = |marker: &str| chars[at..].starts_with(&marker.chars().collect::<Vec<_>>()[..]);

    for (marker, style) in [
        ("***", bold_italic.clone()),
        ("___", bold_italic),
        ("**", bold.clone()),
        ("__", bold),
        ("~~", strike),
        ("*", italic.clone()),
        ("_", italic),
    ] {
        if starts(marker) {
            return Some((marker.to_string(), style));
        }
    }
    None
}

/// Parse `[label](target)` starting at the `[`, returning the label, the
/// target, and the index just past the closing paren.
fn link_parts(chars: &[char], at: usize) -> Option<(String, String, usize)> {
    if chars.get(at) != Some(&'[') {
        return None;
    }
    // Balance nested brackets so `[a [b] c](url)` parses.
    let mut depth = 0usize;
    let mut close = None;
    for (index, &character) in chars.iter().enumerate().skip(at) {
        match character {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(index);
                    break;
                }
            }
            _ => {}
        }
    }
    let close = close?;
    if chars.get(close + 1) != Some(&'(') {
        return None;
    }
    let mut paren = 0usize;
    let mut end = None;
    for (index, &character) in chars.iter().enumerate().skip(close + 1) {
        match character {
            '(' => paren += 1,
            ')' => {
                paren -= 1;
                if paren == 0 {
                    end = Some(index);
                    break;
                }
            }
            _ => {}
        }
    }
    let end = end?;
    let label: String = chars[at + 1..close].iter().collect();
    let target: String = chars[close + 2..end].iter().collect();
    Some((label, target.trim().to_string(), end + 1))
}

/// Find `needle` in `chars` at or after `from`, respecting backslash escapes.
fn find_from(chars: &[char], from: usize, needle: &str) -> Option<usize> {
    let needle: Vec<char> = needle.chars().collect();
    let mut at = from;
    while at + needle.len() <= chars.len() {
        if chars[at] == '\\' {
            at += 2;
            continue;
        }
        if chars[at..at + needle.len()] == needle[..] {
            return Some(at);
        }
        at += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::to_markdown;

    fn markdown_of(source: &str) -> String {
        to_markdown(&parse_markdown(source.as_bytes()))
    }

    #[test]
    fn deeply_nested_quotes_and_lists_do_not_exhaust_the_stack() {
        // Both constructs recurse per level. Before the depth cap this aborted
        // the process rather than returning an error, which no caller can
        // defend against — so the test is that it returns at all, and that the
        // text still comes through rather than being silently dropped.
        let depth = MAX_BLOCK_DEPTH * 40;

        let quotes = format!("{}deep\n", "> ".repeat(depth));
        let document = parse_markdown(quotes.as_bytes());
        assert!(document.text().contains("deep"));

        let list = format!("{}- deep\n", "  ".repeat(depth));
        let document = parse_markdown(list.as_bytes());
        assert!(document.text().contains("deep"));
    }

    /// A footnote written by this writer is read back as one.
    ///
    /// `[^1]` is bracketed like a link and is not one, so it was read as
    /// literal text and then escaped on the way out: a document that left as
    /// Markdown and came back had `\[^1\]` where its notes had been.
    #[test]
    fn markdown_reads_the_footnotes_it_writes() {
        let doc = parse_markdown(
            b"Body text with[^1] a note.\n\n[^1]: The note itself.\n",
        );
        assert_eq!(doc.footnotes.len(), 1);
        assert_eq!(doc.footnotes[0].label.as_deref(), Some("1"));

        let markdown = to_markdown(&doc);
        assert!(markdown.contains("Body text with[^1] a note."), "{markdown}");
        assert!(markdown.contains("[^1]: The note itself."), "{markdown}");
    }

    /// A reference to a note that was never defined stays as it was written.
    #[test]
    fn an_undefined_footnote_reference_is_left_as_text() {
        let doc = parse_markdown(b"See[^missing] here.\n");
        assert!(doc.footnotes.is_empty());
        assert!(doc.text().contains("[^missing]"), "{}", doc.text());
    }

    /// Markdown has no mark for these, and takes HTML for what it lacks.
    #[test]
    fn markdown_reads_the_inline_html_it_writes() {
        let doc = parse_markdown(b"Water is H<sub>2</sub>O and 5m<sup>2</sup>.\n");
        assert_eq!(
            to_markdown(&doc),
            "Water is H<sub>2</sub>O and 5m<sup>2</sup>.\n"
        );

        // A tag the model has no meaning for is text, which is what Markdown
        // says to do with HTML a reader does not handle.
        let passed = parse_markdown(b"a <span class=x>b</span> c\n");
        assert!(passed.text().contains("<span class=x>"), "{}", passed.text());
    }

    /// Rendering, reading back and rendering again gives the same Markdown.
    ///
    /// The property that found the two gaps above: whatever the writer emits,
    /// the reader has to understand, or a document loses something by passing
    /// through the form this tool most often hands to a caller.
    #[test]
    fn rendering_markdown_is_idempotent() {
        let source = concat!(
            "# Title\n\n",
            "Body with[^1] a note, H<sub>2</sub>O, ~~struck~~ and **bold**.\n\n",
            "> A quotation.\n>\n> Second paragraph of it.\n\n",
            "- one\n  - under one\n- two\n\n",
            "| a | b |\n| --- | --- |\n| 1 | 2 |\n\n",
            "[^1]: The note itself.\n",
        );
        let once = to_markdown(&parse_markdown(source.as_bytes()));
        let twice = to_markdown(&parse_markdown(once.as_bytes()));
        assert_eq!(once, twice, "left is the first rendering");
    }

    #[test]
    fn plain_text_joins_soft_wraps_and_splits_on_blank_lines() {
        let doc = parse_text(b"first para\nstill first\n\nsecond para\n");
        assert_eq!(doc.sections[0].blocks.len(), 2);
        assert_eq!(doc.text(), "first para still first\nsecond para\n");
    }

    #[test]
    fn csv_becomes_a_table_with_a_header_row() {
        let doc = parse_csv(b"name,age\nada,36\ngrace,45\n", None);
        let Block::Table(table) = &doc.sections[0].blocks[0] else {
            panic!("expected a table")
        };
        assert_eq!(table.header_rows, 1);
        assert_eq!(table.rows.len(), 3);
        assert_eq!(
            to_markdown(&doc),
            "| name | age |\n| --- | --- |\n| ada | 36 |\n| grace | 45 |\n"
        );
    }

    #[test]
    fn csv_honours_rfc4180_quoting() {
        let doc = parse_csv(b"a,b\n\"has, comma\",\"say \"\"hi\"\"\"\n", None);
        let Block::Table(table) = &doc.sections[0].blocks[0] else {
            panic!("expected a table")
        };
        let row = &table.rows[1];
        assert_eq!(cell_text(&row.cells[0]), "has, comma");
        assert_eq!(cell_text(&row.cells[1]), r#"say "hi""#);
    }

    #[test]
    fn csv_quoted_field_may_span_lines() {
        let doc = parse_csv(b"a,b\n\"line one\nline two\",z\n", None);
        let Block::Table(table) = &doc.sections[0].blocks[0] else {
            panic!("expected a table")
        };
        assert_eq!(table.rows.len(), 2);
        assert_eq!(cell_text(&table.rows[1].cells[0]), "line one\nline two");
    }

    #[test]
    fn infers_tab_delimiter() {
        let doc = parse_csv(b"a\tb\tc\n1\t2\t3\n", None);
        let Block::Table(table) = &doc.sections[0].blocks[0] else {
            panic!("expected a table")
        };
        assert_eq!(table.rows[0].cells.len(), 3);
    }

    fn cell_text(cell: &Cell) -> String {
        let mut out = String::new();
        for block in &cell.blocks {
            if let Block::Paragraph { content, .. } = block {
                out.push_str(&crate::doc::inline_text(content));
            }
        }
        out
    }

    #[test]
    fn markdown_headings_round_trip() {
        assert_eq!(markdown_of("# One\n\n## Two\n"), "# One\n\n## Two\n");
        // Setext headings normalise to ATX.
        assert_eq!(markdown_of("One\n===\n\nTwo\n---\n"), "# One\n\n## Two\n");
    }

    #[test]
    fn markdown_takes_its_title_from_a_leading_h1() {
        let doc = parse_markdown(b"# The Title\n\nbody\n");
        assert_eq!(doc.title.as_deref(), Some("The Title"));
    }

    #[test]
    fn hash_without_a_space_is_not_a_heading() {
        let doc = parse_markdown(b"#hashtag\n");
        assert!(matches!(doc.sections[0].blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn markdown_inline_styles_round_trip() {
        assert_eq!(
            markdown_of("**bold** and _italic_ and ~~struck~~ and `code`\n"),
            "**bold** and _italic_ and ~~struck~~ and `code`\n"
        );
    }

    #[test]
    fn markdown_bold_italic_combines_both_styles() {
        let doc = parse_markdown(b"***both***\n");
        let Block::Paragraph { content, .. } = &doc.sections[0].blocks[0] else {
            panic!("expected paragraph")
        };
        let Inline::Run(run) = &content[0] else {
            panic!("expected run")
        };
        assert!(run.style.bold && run.style.italic);
    }

    #[test]
    fn markdown_code_spans_are_not_reinterpreted() {
        let doc = parse_markdown(b"`**not bold**`\n");
        let Block::Paragraph { content, .. } = &doc.sections[0].blocks[0] else {
            panic!("expected paragraph")
        };
        let Inline::Run(run) = &content[0] else {
            panic!("expected run")
        };
        assert!(run.style.code);
        assert_eq!(run.text, "**not bold**");
    }

    #[test]
    fn markdown_links_and_images_round_trip() {
        assert_eq!(
            markdown_of("[site](https://example.com) ![alt](img.png)\n"),
            "[site](https://example.com) ![alt](img.png)\n"
        );
    }

    #[test]
    fn markdown_lists_round_trip_including_numbering_and_tasks() {
        assert_eq!(markdown_of("- a\n- b\n"), "- a\n- b\n");
        assert_eq!(markdown_of("3. c\n4. d\n"), "3. c\n4. d\n");
        assert_eq!(
            markdown_of("- [x] done\n- [ ] todo\n"),
            "- [x] done\n- [ ] todo\n"
        );
    }

    #[test]
    fn markdown_nested_lists_round_trip() {
        assert_eq!(
            markdown_of("- parent\n  - child\n"),
            "- parent\n  - child\n"
        );
    }

    #[test]
    fn markdown_fenced_code_round_trips_with_language() {
        assert_eq!(
            markdown_of("```rust\nlet a = 1;\n```\n"),
            "```rust\nlet a = 1;\n```\n"
        );
    }

    #[test]
    fn markdown_indented_code_becomes_a_code_block() {
        let doc = parse_markdown(b"    indented\n    lines\n");
        let Block::Code { text, language } = &doc.sections[0].blocks[0] else {
            panic!("expected code")
        };
        assert!(language.is_none());
        assert_eq!(text, "indented\nlines");
    }

    #[test]
    fn markdown_block_quotes_round_trip_and_nest() {
        assert_eq!(markdown_of("> quoted\n"), "> quoted\n");
        let doc = parse_markdown(b"> # heading in quote\n");
        let Block::Quote(inner) = &doc.sections[0].blocks[0] else {
            panic!("expected quote")
        };
        assert!(matches!(inner[0], Block::Heading { level: 1, .. }));
    }

    #[test]
    fn markdown_tables_round_trip() {
        let source = "| a | b |\n| --- | --- |\n| 1 | 2 |\n";
        assert_eq!(markdown_of(source), source);
    }

    #[test]
    fn markdown_table_cells_keep_escaped_pipes() {
        let doc = parse_markdown(b"| a | b |\n| --- | --- |\n| x\\|y | z |\n");
        let Block::Table(table) = &doc.sections[0].blocks[0] else {
            panic!("expected table")
        };
        assert_eq!(cell_text(&table.rows[1].cells[0]), "x|y");
    }

    #[test]
    fn markdown_thematic_breaks_round_trip() {
        assert_eq!(markdown_of("a\n\n---\n\nb\n"), "a\n\n---\n\nb\n");
    }

    #[test]
    fn markdown_backslash_escapes_are_honoured() {
        let doc = parse_markdown(br"not \*emphasis\*");
        let Block::Paragraph { content, .. } = &doc.sections[0].blocks[0] else {
            panic!("expected paragraph")
        };
        assert_eq!(crate::doc::inline_text(content), "not *emphasis*");
    }

    #[test]
    fn markdown_paragraph_soft_wraps_join() {
        let doc = parse_markdown(b"one line\nsecond line\n\nnew para\n");
        assert_eq!(doc.sections[0].blocks.len(), 2);
        assert_eq!(doc.text(), "one line second line\nnew para\n");
    }

    #[test]
    fn a_full_document_survives_a_markdown_round_trip() {
        // The strongest available check on the reader/writer pair: parse, emit,
        // parse again, and require the emitted forms to agree.
        let source = "# Title\n\nIntro paragraph with **bold**.\n\n## Section\n\n- one\n- two\n  - nested\n\n| h1 | h2 |\n| --- | --- |\n| a | b |\n\n> a quote\n\n```py\nprint(1)\n```\n";
        let once = markdown_of(source);
        let twice = markdown_of(&once);
        assert_eq!(once, twice, "round trip is not stable:\n{once}");
        assert!(once.contains("# Title"));
        assert!(once.contains("- nested"));
        assert!(once.contains("| h1 | h2 |"));
        assert!(once.contains("```py"));
    }
}
