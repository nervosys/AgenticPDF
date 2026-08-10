// SPDX-License-Identifier: AGPL-3.0-or-later
//! Document format identification.
//!
//! Detection is driven by **content**, not by file extension. A `.docx` that
//! someone renamed to `.pdf` is still a `.docx`, and an agent handed bytes over
//! MCP or WASM may have no filename at all. An extension, when available, is
//! accepted only as a tie-breaker for the plain-text family — where the bytes
//! genuinely are ambiguous.
//!
//! The interesting cases are the container formats. Both OOXML and OpenDocument
//! are ZIP archives, so `PK\x03\x04` narrows things to "one of seven formats"
//! and the archive's member list decides which:
//!
//! - OpenDocument stores an uncompressed `mimetype` member first, by spec.
//! - OOXML has no such marker, so the well-known part paths identify it
//!   (`word/document.xml`, `xl/workbook.xml`, `ppt/presentation.xml`).
//!
//! Legacy Office is an OLE2 compound file; all three share the same magic
//! bytes, and only the internal stream names distinguish them.

use crate::PdfError;
use crate::container::zip::ZipArchive;

/// How far into a file we look for a signature that need not be at offset 0.
const SNIFF_WINDOW: usize = 1024;

/// A document format the engine can identify.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    /// Portable Document Format.
    Pdf,
    /// Word (OOXML): `.docx`, `.docm`.
    Docx,
    /// PowerPoint (OOXML): `.pptx`, `.pptm`, `.ppsx`.
    Pptx,
    /// Excel (OOXML): `.xlsx`, `.xlsm`.
    Xlsx,
    /// OpenDocument Text.
    Odt,
    /// OpenDocument Spreadsheet.
    Ods,
    /// OpenDocument Presentation.
    Odp,
    /// Word 97-2003 binary.
    Doc,
    /// Excel 97-2003 binary.
    Xls,
    /// PowerPoint 97-2003 binary.
    Ppt,
    /// EPUB electronic book (ZIP of XHTML content documents).
    Epub,
    /// Rich Text Format.
    Rtf,
    /// HTML or XHTML.
    Html,
    /// Markdown.
    Markdown,
    /// Comma- or tab-separated values.
    Csv,
    /// Plain text.
    Text,
    /// The Agentic Document Format — this engine's own, and the only one it
    /// writes as well as reads.
    Adf,
}

impl Format {
    /// Short lowercase identifier, as used in JSON output and CLI flags.
    pub fn id(self) -> &'static str {
        match self {
            Format::Pdf => "pdf",
            Format::Docx => "docx",
            Format::Pptx => "pptx",
            Format::Xlsx => "xlsx",
            Format::Odt => "odt",
            Format::Ods => "ods",
            Format::Odp => "odp",
            Format::Doc => "doc",
            Format::Xls => "xls",
            Format::Ppt => "ppt",
            Format::Epub => "epub",
            Format::Rtf => "rtf",
            Format::Html => "html",
            Format::Markdown => "markdown",
            Format::Csv => "csv",
            Format::Text => "text",
            Format::Adf => "adf",
        }
    }

    /// Human-readable name for CLI output.
    pub fn label(self) -> &'static str {
        match self {
            Format::Pdf => "PDF",
            Format::Docx => "Word (OOXML)",
            Format::Pptx => "PowerPoint (OOXML)",
            Format::Xlsx => "Excel (OOXML)",
            Format::Odt => "OpenDocument Text",
            Format::Ods => "OpenDocument Spreadsheet",
            Format::Odp => "OpenDocument Presentation",
            Format::Doc => "Word 97-2003",
            Format::Xls => "Excel 97-2003",
            Format::Ppt => "PowerPoint 97-2003",
            Format::Epub => "EPUB",
            Format::Rtf => "Rich Text Format",
            Format::Html => "HTML",
            Format::Markdown => "Markdown",
            Format::Csv => "Delimited text",
            Format::Text => "Plain text",
            Format::Adf => "Agentic Document Format",
        }
    }

    /// IANA media type.
    pub fn media_type(self) -> &'static str {
        match self {
            Format::Pdf => "application/pdf",
            Format::Docx => {
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            }
            Format::Pptx => {
                "application/vnd.openxmlformats-officedocument.presentationml.presentation"
            }
            Format::Xlsx => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            Format::Odt => "application/vnd.oasis.opendocument.text",
            Format::Ods => "application/vnd.oasis.opendocument.spreadsheet",
            Format::Odp => "application/vnd.oasis.opendocument.presentation",
            Format::Doc => "application/msword",
            Format::Xls => "application/vnd.ms-excel",
            Format::Ppt => "application/vnd.ms-powerpoint",
            Format::Epub => "application/epub+zip",
            Format::Rtf => "application/rtf",
            Format::Html => "text/html",
            Format::Markdown => "text/markdown",
            Format::Csv => "text/csv",
            Format::Text => "text/plain",
            Format::Adf => "application/vnd.nervosys.adf",
        }
    }

    /// File extensions conventionally used for this format, without the dot.
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Format::Pdf => &["pdf"],
            Format::Docx => &["docx", "docm"],
            Format::Pptx => &["pptx", "pptm", "ppsx", "ppsm"],
            Format::Xlsx => &["xlsx", "xlsm", "xlsb"],
            Format::Odt => &["odt"],
            Format::Ods => &["ods"],
            Format::Odp => &["odp"],
            Format::Doc => &["doc"],
            Format::Xls => &["xls"],
            Format::Ppt => &["ppt", "pps", "pot"],
            Format::Epub => &["epub"],
            Format::Rtf => &["rtf"],
            Format::Html => &["html", "htm", "xhtml"],
            Format::Markdown => &["md", "markdown"],
            Format::Csv => &["csv", "tsv"],
            Format::Text => &["txt", "text"],
            Format::Adf => &["adf"],
        }
    }

    /// Whether the format is laid out by the author into fixed pages (PDF,
    /// slides, sheets) rather than reflowed by the renderer.
    ///
    /// The typesetter uses this to decide between honouring explicit geometry
    /// and computing its own.
    pub fn is_paginated(self) -> bool {
        matches!(
            self,
            Format::Pdf
                | Format::Pptx
                | Format::Odp
                | Format::Ppt
                | Format::Xlsx
                | Format::Ods
                | Format::Xls
        )
    }

    /// Whether this build can parse the format, as opposed to merely
    /// recognising it.
    ///
    /// Every format the engine detects now has a parser, so this is constant —
    /// but it is kept because the distinction is real: detection and parsing
    /// are separate capabilities, and a format added to one before the other
    /// should be reported by name rather than as a corrupt file.
    pub fn is_supported(self) -> bool {
        true
    }

    /// Every format the engine knows about, for capability discovery.
    pub fn all() -> &'static [Format] {
        &[
            Format::Pdf,
            Format::Docx,
            Format::Pptx,
            Format::Xlsx,
            Format::Odt,
            Format::Ods,
            Format::Odp,
            Format::Doc,
            Format::Xls,
            Format::Ppt,
            Format::Epub,
            Format::Rtf,
            Format::Html,
            Format::Markdown,
            Format::Csv,
            Format::Text,
            Format::Adf,
        ]
    }

    /// Resolve a format from its [`Format::id`] or any of its extensions.
    pub fn from_id(value: &str) -> Option<Format> {
        let needle = value.trim().trim_start_matches('.').to_ascii_lowercase();
        Format::all()
            .iter()
            .copied()
            .find(|f| f.id() == needle || f.extensions().contains(&needle.as_str()))
    }

    /// Guess a format from a file path's extension alone.
    pub fn from_path(path: &str) -> Option<Format> {
        let ext = path.rsplit_once('.').map(|(_, ext)| ext)?;
        Format::from_id(ext)
    }
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.id())
    }
}

/// Identify a document from its bytes.
///
/// `hint` is an optional filename or extension, consulted only where the
/// content is genuinely ambiguous (plain text vs Markdown vs CSV).
pub fn detect(data: &[u8], hint: Option<&str>) -> Result<Format, PdfError> {
    if data.is_empty() {
        return Err(PdfError::Unsupported("empty input".into()));
    }

    // Binary signatures first — these are unambiguous.
    if let Some(format) = detect_binary(data) {
        return Ok(format);
    }

    // Formats we can name but not yet process. Saying which is far more useful
    // than "unsupported": the caller learns the file is fine and the engine is
    // the limitation.
    if let Some(name) = detect_known_unsupported(data) {
        return Err(PdfError::Unsupported(format!(
            "{name} is not supported by this engine"
        )));
    }

    if !looks_textual(data) {
        return Err(PdfError::Unsupported(
            "unrecognised binary format".into(),
        ));
    }

    Ok(detect_textual(data, hint))
}

/// Signature-based detection for the binary and container formats.
fn detect_binary(data: &[u8]) -> Option<Format> {
    // Our own format first: its magic is exact and at offset 0, so this is the
    // cheapest test and can never be confused with anything below.
    if data.starts_with(&crate::adf::MAGIC) {
        return Some(Format::Adf);
    }

    // The PDF spec allows the header to sit anywhere in the first 1 KB, and
    // real files do carry leading junk.
    if window(data).windows(5).any(|w| w == b"%PDF-") {
        return Some(Format::Pdf);
    }

    // Any ZIP local-file, empty-archive, or spanned-archive signature.
    if data.starts_with(b"PK\x03\x04")
        || data.starts_with(b"PK\x05\x06")
        || data.starts_with(b"PK\x07\x08")
    {
        return detect_zip(data);
    }

    if data.starts_with(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]) {
        return detect_ole2(data);
    }

    // RTF is plain ASCII but has a fixed opening group, so it is a signature
    // match rather than a text heuristic.
    if window(data).starts_with(b"{\\rtf") {
        return Some(Format::Rtf);
    }

    None
}

/// Distinguish the seven ZIP-based document formats by their member list.
fn detect_zip(data: &[u8]) -> Option<Format> {
    let archive = ZipArchive::open(data).ok()?;

    // OpenDocument and EPUB declare themselves in a `mimetype` member that the
    // spec requires to be first and uncompressed.
    if let Some(bytes) = archive.read_optional("mimetype") {
        let mime = String::from_utf8_lossy(&bytes);
        let mime = mime.trim();
        match mime {
            "application/epub+zip" => return Some(Format::Epub),
            "application/vnd.oasis.opendocument.text" => return Some(Format::Odt),
            "application/vnd.oasis.opendocument.spreadsheet" => return Some(Format::Ods),
            "application/vnd.oasis.opendocument.presentation" => return Some(Format::Odp),
            _ => {}
        }
        // Templates share the document format's structure.
        if mime.starts_with("application/vnd.oasis.opendocument.text") {
            return Some(Format::Odt);
        }
        if mime.starts_with("application/vnd.oasis.opendocument.spreadsheet") {
            return Some(Format::Ods);
        }
        if mime.starts_with("application/vnd.oasis.opendocument.presentation") {
            return Some(Format::Odp);
        }
    }

    // An EPUB whose `mimetype` member is missing or misspelled — common in
    // hand-assembled files — is still identifiable by its required container
    // descriptor.
    if archive.contains("META-INF/container.xml") {
        return Some(Format::Epub);
    }

    // OOXML: identified by its well-known part paths. Checking the parts rather
    // than `[Content_Types].xml` is both cheaper and more tolerant of producers
    // that write unusual content-type overrides.
    if archive.contains("word/document.xml") {
        return Some(Format::Docx);
    }
    if archive.contains("xl/workbook.xml") || archive.contains("xl/workbook.bin") {
        return Some(Format::Xlsx);
    }
    if archive.contains("ppt/presentation.xml") {
        return Some(Format::Pptx);
    }

    // Fall back to a scan for producers that vary the casing or nest the parts.
    let mut has_word = false;
    let mut has_sheet = false;
    let mut has_slide = false;
    for name in archive.names() {
        let lower = name.to_ascii_lowercase();
        has_word |= lower.starts_with("word/");
        has_sheet |= lower.starts_with("xl/");
        has_slide |= lower.starts_with("ppt/");
    }
    match (has_word, has_sheet, has_slide) {
        (true, _, _) => Some(Format::Docx),
        (_, true, _) => Some(Format::Xlsx),
        (_, _, true) => Some(Format::Pptx),
        _ => None,
    }
}

/// Distinguish the three legacy Office formats inside an OLE2 compound file.
///
/// A full directory walk needs the compound-file reader; until then the stream
/// names are searched directly. Directory entry names are stored as UTF-16LE,
/// so each ASCII character is followed by a zero byte.
fn detect_ole2(data: &[u8]) -> Option<Format> {
    let has = |name: &str| contains_utf16le(data, name);
    if has("WordDocument") {
        return Some(Format::Doc);
    }
    if has("Workbook") || has("Book") {
        return Some(Format::Xls);
    }
    if has("PowerPoint Document") {
        return Some(Format::Ppt);
    }
    None
}

/// Formats with recognisable signatures that this engine does not implement.
fn detect_known_unsupported(data: &[u8]) -> Option<&'static str> {
    let head = window(data);
    if head.starts_with(b"\x89PNG\r\n\x1a\n") || head.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("a bare image file");
    }
    if head.starts_with(b"%!PS") {
        return Some("PostScript");
    }
    None
}

/// Classify text content that carries no binary signature.
fn detect_textual(data: &[u8], hint: Option<&str>) -> Format {
    let text = String::from_utf8_lossy(window(data));

    if looks_like_html(&text) {
        return Format::Html;
    }

    // The extension is authoritative for the text family: `.csv`, `.md` and
    // `.txt` are all valid UTF-8 prose and heuristics alone will misfile them.
    if let Some(format) = hint.and_then(Format::from_path).or_else(|| hint.and_then(Format::from_id))
        && matches!(format, Format::Csv | Format::Markdown | Format::Text | Format::Html)
    {
        return format;
    }

    if looks_like_delimited(&text) {
        return Format::Csv;
    }
    if looks_like_markdown(&text) {
        return Format::Markdown;
    }
    Format::Text
}

/// Recognise both full documents and bare fragments.
///
/// Fragments matter: email bodies, CMS field values and EPUB chapters are
/// routinely markup with no `<html>` wrapper, and refusing to see them as HTML
/// would file them as plain text and lose all their structure.
fn looks_like_html(text: &str) -> bool {
    let head = text.trim_start().to_ascii_lowercase();
    if head.starts_with("<!doctype html") || head.starts_with("<html") {
        return true;
    }
    if head.starts_with("<?xml") && head.contains("<html") {
        return true;
    }
    if !head.starts_with('<') {
        return false;
    }
    // A fragment: require a tag we actually recognise, so arbitrary XML or a
    // stray angle bracket is not mistaken for markup.
    const TAGS: [&str; 22] = [
        "<html", "<head", "<body", "<div", "<p>", "<p ", "<h1", "<h2", "<h3", "<h4", "<h5", "<h6",
        "<table", "<tr", "<td", "<th", "<ul", "<ol", "<li", "<span", "<img", "<br",
    ];
    TAGS.iter().any(|tag| head.contains(tag))
}

/// A delimited file has the *same* number of unquoted delimiters on every one
/// of its first few lines. Prose does not.
fn looks_like_delimited(text: &str) -> bool {
    for delimiter in [',', '\t', ';'] {
        let counts: Vec<usize> = text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .take(8)
            .map(|line| count_unquoted(line, delimiter))
            .collect();
        if counts.len() >= 2 && counts[0] >= 1 && counts.iter().all(|&c| c == counts[0]) {
            return true;
        }
    }
    false
}

/// Count delimiters outside double-quoted fields, per RFC 4180 quoting.
fn count_unquoted(line: &str, delimiter: char) -> usize {
    let mut count = 0;
    let mut in_quotes = false;
    for ch in line.chars() {
        match ch {
            '"' => in_quotes = !in_quotes,
            c if c == delimiter && !in_quotes => count += 1,
            _ => {}
        }
    }
    count
}

fn looks_like_markdown(text: &str) -> bool {
    text.lines().take(40).any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("# ")
            || trimmed.starts_with("## ")
            || trimmed.starts_with("### ")
            || trimmed.starts_with("```")
            || trimmed.starts_with("- ")
            || trimmed.starts_with("* ")
            || trimmed.starts_with("> ")
            || trimmed.starts_with("| ")
            || (trimmed.starts_with('[') && trimmed.contains("]("))
    })
}

/// Whether the bytes plausibly decode as text.
///
/// NUL bytes and a high proportion of control characters mean binary; both UTF-8
/// and legacy single-byte encodings pass.
fn looks_textual(data: &[u8]) -> bool {
    let head = window(data);
    if head.contains(&0) {
        return false;
    }
    let suspicious = head
        .iter()
        .filter(|&&b| b < 0x09 || (0x0E..0x20).contains(&b))
        .count();
    suspicious * 20 < head.len().max(1)
}

fn window(data: &[u8]) -> &[u8] {
    &data[..data.len().min(SNIFF_WINDOW)]
}

/// Search for an ASCII string encoded as UTF-16LE.
fn contains_utf16le(haystack: &[u8], needle: &str) -> bool {
    let encoded: Vec<u8> = needle.bytes().flat_map(|b| [b, 0]).collect();
    haystack
        .windows(encoded.len())
        .any(|window| window == encoded.as_slice())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::build_zip;

    fn ole2(stream: &str) -> Vec<u8> {
        let mut data = vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
        data.extend(std::iter::repeat_n(0u8, 500));
        data.extend(stream.bytes().flat_map(|b| [b, 0]));
        data
    }

    #[test]
    fn detects_pdf_even_with_leading_junk() {
        assert_eq!(detect(b"%PDF-1.7\n%\xE2\xE3", None).unwrap(), Format::Pdf);
        let mut padded = b"garbage bytes ".to_vec();
        padded.extend_from_slice(b"%PDF-1.4\n");
        assert_eq!(detect(&padded, None).unwrap(), Format::Pdf);
    }

    #[test]
    fn detects_ooxml_by_part_paths() {
        let docx = build_zip(&[
            ("[Content_Types].xml", b"<Types/>", true),
            ("word/document.xml", b"<w:document/>", true),
        ]);
        assert_eq!(detect(&docx, None).unwrap(), Format::Docx);

        let xlsx = build_zip(&[("xl/workbook.xml", b"<workbook/>", true)]);
        assert_eq!(detect(&xlsx, None).unwrap(), Format::Xlsx);

        let pptx = build_zip(&[("ppt/presentation.xml", b"<presentation/>", true)]);
        assert_eq!(detect(&pptx, None).unwrap(), Format::Pptx);
    }

    #[test]
    fn detects_opendocument_and_epub_by_mimetype_member() {
        for (mime, expected) in [
            ("application/vnd.oasis.opendocument.text", Format::Odt),
            ("application/vnd.oasis.opendocument.spreadsheet", Format::Ods),
            ("application/vnd.oasis.opendocument.presentation", Format::Odp),
            ("application/epub+zip", Format::Epub),
        ] {
            let zip = build_zip(&[
                ("mimetype", mime.as_bytes(), false),
                ("content.xml", b"<office/>", true),
            ]);
            assert_eq!(detect(&zip, None).unwrap(), expected, "for {mime}");
        }
    }

    #[test]
    fn detects_epub_without_a_mimetype_member() {
        // The container descriptor is required by the spec even when the
        // `mimetype` entry is missing or misspelled.
        let epub = build_zip(&[
            ("META-INF/container.xml", b"<container/>", true),
            ("OEBPS/content.opf", b"<package/>", true),
        ]);
        assert_eq!(detect(&epub, None).unwrap(), Format::Epub);
    }

    #[test]
    fn detects_rtf_by_its_opening_group() {
        assert_eq!(
            detect(br"{\rtf1\ansi\deff0 hello}", None).unwrap(),
            Format::Rtf
        );
    }

    #[test]
    fn opendocument_mimetype_wins_over_part_scan() {
        // A malicious or careless archive containing both markers must resolve
        // by the authoritative `mimetype` member.
        let mixed = build_zip(&[
            (
                "mimetype",
                b"application/vnd.oasis.opendocument.text",
                false,
            ),
            ("word/document.xml", b"<w:document/>", true),
        ]);
        assert_eq!(detect(&mixed, None).unwrap(), Format::Odt);
    }

    #[test]
    fn detects_legacy_office_by_stream_name() {
        assert_eq!(detect(&ole2("WordDocument"), None).unwrap(), Format::Doc);
        assert_eq!(detect(&ole2("Workbook"), None).unwrap(), Format::Xls);
        assert_eq!(
            detect(&ole2("PowerPoint Document"), None).unwrap(),
            Format::Ppt
        );
    }

    #[test]
    fn detects_html() {
        assert_eq!(detect(b"<!DOCTYPE html><html></html>", None).unwrap(), Format::Html);
        assert_eq!(detect(b"<html><body>hi</body></html>", None).unwrap(), Format::Html);
    }

    #[test]
    fn detects_delimited_text_by_consistent_columns() {
        let csv = b"name,age,city\nada,36,london\ngrace,45,new york\n";
        assert_eq!(detect(csv, None).unwrap(), Format::Csv);

        let tsv = b"a\tb\tc\n1\t2\t3\n4\t5\t6\n";
        assert_eq!(detect(tsv, None).unwrap(), Format::Csv);
    }

    #[test]
    fn quoted_commas_do_not_make_prose_look_delimited() {
        let prose = b"Hello, world. This is a sentence.\nAnd another one entirely.\n";
        assert_ne!(detect(prose, None).unwrap(), Format::Csv);
    }

    #[test]
    fn detects_markdown_by_markers() {
        assert_eq!(detect(b"# Title\n\nSome prose.\n", None).unwrap(), Format::Markdown);
        assert_eq!(detect(b"- one\n- two\n", None).unwrap(), Format::Markdown);
    }

    #[test]
    fn falls_back_to_plain_text() {
        assert_eq!(
            detect(b"Just an ordinary paragraph of prose.\n", None).unwrap(),
            Format::Text
        );
    }

    #[test]
    fn extension_hint_disambiguates_the_text_family() {
        // Content alone reads as Markdown (leading "- "); the extension says otherwise.
        let ambiguous = b"- a\n- b\n";
        assert_eq!(detect(ambiguous, None).unwrap(), Format::Markdown);
        assert_eq!(detect(ambiguous, Some("notes.txt")).unwrap(), Format::Text);
    }

    #[test]
    fn extension_hint_never_overrides_a_binary_signature() {
        // A renamed file is still what its bytes say it is.
        assert_eq!(detect(b"%PDF-1.7\n", Some("report.docx")).unwrap(), Format::Pdf);
        let docx = build_zip(&[("word/document.xml", b"<w:document/>", true)]);
        assert_eq!(detect(&docx, Some("report.pdf")).unwrap(), Format::Docx);
    }

    #[test]
    fn names_recognisable_but_unimplemented_formats() {
        // A bare image is recognisable but is not a document.
        let err = detect(b"\x89PNG\r\n\x1a\nrest", None).unwrap_err();
        assert!(matches!(&err, PdfError::Unsupported(m) if m.contains("image")));
    }

    #[test]
    fn rejects_empty_and_unrecognised_binary() {
        assert!(detect(b"", None).is_err());
        assert!(detect(&[0u8, 1, 2, 3, 4, 5, 6, 7, 8], None).is_err());
    }

    #[test]
    fn format_ids_and_extensions_round_trip() {
        for format in Format::all() {
            assert_eq!(Format::from_id(format.id()), Some(*format));
            for ext in format.extensions() {
                assert_eq!(
                    Format::from_path(&format!("doc.{ext}")),
                    Some(*format),
                    "extension .{ext}"
                );
            }
        }
    }
}
