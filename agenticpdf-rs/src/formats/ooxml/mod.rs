// SPDX-License-Identifier: AGPL-3.0-or-later
//! Office Open XML parsers (`.docx`, `.xlsx`, `.pptx`).
//!
//! This module holds the Open Packaging Conventions layer the three formats
//! share; the format-specific readers live in [`docx`], [`xlsx`] and [`pptx`].
//!
//! OPC is a small, strict indirection that is easy to get wrong by guessing.
//! Nothing in an OOXML package is found by hard-coded path: the root
//! `_rels/.rels` names the main document part, and every reference *from* a
//! part — a hyperlink target, an image, a worksheet, a slide — is a
//! relationship id resolved against that part's own `.rels` sidecar. Producers
//! do vary the paths, so following the relationships is what makes the readers
//! work on files from Word, LibreOffice, Google Docs and the many libraries
//! that write these formats.
//!
//! Relationship types are matched by their *suffix* rather than in full: the
//! namespace prefix differs between the strict and transitional flavours of
//! the spec, while the last path segment (`officeDocument`, `image`,
//! `hyperlink`) is stable across both.

pub mod docx;
pub mod pptx;
pub mod xlsx;

#[cfg(test)]
mod format_tests;

use std::collections::HashMap;

use crate::PdfError;
use crate::container::zip::ZipArchive;
use crate::detect::Format;
use crate::doc::SemanticDoc;
use crate::xml::{self, Event, Reader};

/// English Metric Units per PDF point. OOXML measures geometry in EMU:
/// 914,400 per inch, and 72 points per inch.
pub const EMU_PER_POINT: f64 = 12_700.0;

/// Parse an OOXML package of the given format.
pub fn parse(data: &[u8], format: Format) -> Result<SemanticDoc, PdfError> {
    let archive = ZipArchive::open(data)?;
    let package = Package::open(&archive)?;

    let mut document = match format {
        Format::Docx => docx::parse(&archive, &package)?,
        Format::Xlsx => xlsx::parse(&archive, &package)?,
        Format::Pptx => pptx::parse(&archive, &package)?,
        other => {
            return Err(PdfError::Unsupported(format!(
                "{} is not an OOXML format",
                other.label()
            )));
        }
    };

    package.apply_core_properties(&mut document);
    Ok(document)
}

/// One OPC relationship.
#[derive(Debug, Clone, Default)]
pub struct Relationship {
    pub id: String,
    /// Full relationship type URI.
    pub kind: String,
    /// Target as written — relative to the owning part unless external.
    pub target: String,
    /// Whether the target is outside the package (hyperlinks are).
    pub external: bool,
}

impl Relationship {
    /// Whether this relationship's type ends with `suffix`.
    pub fn is(&self, suffix: &str) -> bool {
        self.kind.rsplit('/').next() == Some(suffix)
    }
}

/// A resolved relationship table for one part.
#[derive(Debug, Clone, Default)]
pub struct Rels {
    by_id: HashMap<String, Relationship>,
}

impl Rels {
    /// Read the `.rels` sidecar for `part`, or an empty table if absent.
    ///
    /// The sidecar lives beside the part in a `_rels` directory, named after
    /// the part with `.rels` appended: `word/document.xml` →
    /// `word/_rels/document.xml.rels`.
    pub fn for_part(archive: &ZipArchive, part: &str) -> Rels {
        let (directory, name) = split_path(part);
        let path = format!("{directory}_rels/{name}.rels");
        let Some(bytes) = archive.read_optional(&path) else {
            return Rels::default();
        };

        let mut by_id = HashMap::new();
        let mut reader = Reader::new(&bytes);
        while let Some(event) = reader.read_event() {
            let Event::Start(element) = event else {
                continue;
            };
            if element.local != "Relationship" {
                continue;
            }
            let id = element.attr_local("Id").unwrap_or_default().to_string();
            if id.is_empty() {
                continue;
            }
            by_id.insert(
                id.clone(),
                Relationship {
                    id,
                    kind: element.attr_local("Type").unwrap_or_default().to_string(),
                    target: element.attr_local("Target").unwrap_or_default().to_string(),
                    external: element
                        .attr_local("TargetMode")
                        .is_some_and(|mode| mode.eq_ignore_ascii_case("External")),
                },
            );
        }
        Rels { by_id }
    }

    /// Look up a relationship by id.
    pub fn get(&self, id: &str) -> Option<&Relationship> {
        self.by_id.get(id)
    }

    /// The first relationship whose type ends with `suffix`.
    pub fn find(&self, suffix: &str) -> Option<&Relationship> {
        self.by_id.values().find(|rel| rel.is(suffix))
    }

    /// Resolve a relationship id to a package path, relative to `base`.
    ///
    /// External targets (hyperlinks) are returned verbatim, since they are URLs
    /// rather than package parts.
    pub fn resolve(&self, base: &str, id: &str) -> Option<String> {
        let rel = self.get(id)?;
        if rel.external {
            return Some(rel.target.clone());
        }
        Some(resolve_path(base, &rel.target))
    }
}

/// A parsed OOXML package: where its main part is, and what it says about
/// itself.
#[derive(Debug, Clone)]
pub struct Package {
    /// Path of the main document part (`word/document.xml` and friends).
    pub main_part: String,
    /// Relationships owned by the main part.
    pub main_rels: Rels,
    core: CoreProperties,
}

#[derive(Debug, Clone, Default)]
struct CoreProperties {
    title: Option<String>,
    creator: Option<String>,
    subject: Option<String>,
    last_modified_by: Option<String>,
    created: Option<String>,
    modified: Option<String>,
}

impl Package {
    /// Locate the main document part by following the package relationships.
    pub fn open(archive: &ZipArchive) -> Result<Package, PdfError> {
        let root = Rels::for_part(archive, "");
        let main_part = root
            .find("officeDocument")
            .map(|rel| resolve_path("", &rel.target))
            // Producers occasionally ship a package with no root relationship;
            // the conventional locations are still worth trying before failing.
            .or_else(|| {
                [
                    "word/document.xml",
                    "xl/workbook.xml",
                    "ppt/presentation.xml",
                ]
                .into_iter()
                .find(|path| archive.contains(path))
                .map(str::to_string)
            })
            .ok_or_else(|| PdfError::MissingPart("OOXML main document part".into()))?;

        if !archive.contains(&main_part) {
            return Err(PdfError::MissingPart(main_part));
        }

        Ok(Package {
            main_rels: Rels::for_part(archive, &main_part),
            main_part,
            core: read_core_properties(archive),
        })
    }

    /// The directory containing the main part, for resolving its references.
    pub fn base(&self) -> String {
        split_path(&self.main_part).0
    }

    /// Copy package metadata onto a parsed document.
    ///
    /// Applied after parsing so a format that finds a better title in the
    /// content — a slide's own title, say — is not overwritten by an empty or
    /// boilerplate core property.
    pub fn apply_core_properties(&self, document: &mut SemanticDoc) {
        let core = &self.core;
        if document.title.is_none() {
            document.title = core.title.clone();
        }
        if document.author.is_none() {
            document.author = core.creator.clone();
        }
        if document.subject.is_none() {
            document.subject = core.subject.clone();
        }
        if document.creator.is_none() {
            document.creator = core.last_modified_by.clone();
        }
        document.created = core.created.clone();
        document.modified = core.modified.clone();
    }
}

/// Read `docProps/core.xml`, which carries Dublin Core metadata.
fn read_core_properties(archive: &ZipArchive) -> CoreProperties {
    let Some(bytes) = archive.read_optional("docProps/core.xml") else {
        return CoreProperties::default();
    };

    let mut core = CoreProperties::default();
    let mut reader = Reader::new(&bytes);
    while let Some(event) = reader.read_event() {
        let Event::Start(element) = event else {
            continue;
        };
        // The namespace matters: `<dc:title>` and `<cp:lastModifiedBy>` live in
        // different vocabularies that share a document.
        let slot = match (element.ns.as_str(), element.local.as_str()) {
            (xml::ns::DC, "title") => &mut core.title,
            (xml::ns::DC, "creator") => &mut core.creator,
            (xml::ns::DC, "subject") | (xml::ns::DC, "description") => &mut core.subject,
            (xml::ns::CP, "lastModifiedBy") => &mut core.last_modified_by,
            (_, "created") => &mut core.created,
            (_, "modified") => &mut core.modified,
            _ => continue,
        };
        let value = xml::text_of(&mut reader, &element.qname).trim().to_string();
        if !value.is_empty() && slot.is_none() {
            *slot = Some(value);
        }
    }
    core
}

// ============================================================================
// Path helpers
// ============================================================================

/// Split a package path into `(directory with trailing slash, file name)`.
pub fn split_path(path: &str) -> (String, String) {
    match path.rfind('/') {
        Some(at) => (path[..=at].to_string(), path[at + 1..].to_string()),
        None => (String::new(), path.to_string()),
    }
}

/// Resolve a relationship target against the owning part's directory.
pub fn resolve_path(base: &str, target: &str) -> String {
    if let Some(absolute) = target.strip_prefix('/') {
        return absolute.to_string();
    }

    let mut parts: Vec<&str> = Vec::new();
    for segment in base.split('/').chain(target.split('/')) {
        match segment {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

/// Convert an EMU measurement to PDF points.
pub fn emu_to_points(emu: i64) -> f64 {
    emu as f64 / EMU_PER_POINT
}

/// Parse an attribute that OOXML writes as an integer.
pub fn attr_i64(element: &xml::Element, local: &str) -> Option<i64> {
    element.attr_local(local)?.trim().parse().ok()
}

/// The relationship id on an element, as distinct from any local `id`.
///
/// This matters on `<p:sldId id="256" r:id="rId2"/>`, where both attributes
/// have the local name `id` but only the prefixed one is a relationship — the
/// other is an internal number. Matching on the local name alone silently picks
/// the wrong attribute and no slide resolves.
pub fn rel_id(element: &xml::Element) -> Option<&str> {
    element
        .attrs
        .iter()
        .find(|(key, _)| key.contains(':') && xml::local_name(key) == "id")
        .map(|(_, value)| value.as_str())
}

/// Whether an OOXML boolean property is on.
///
/// `<w:b/>` means bold; `<w:b w:val="0"/>` means *not* bold. An absent `val` is
/// true, which is the opposite of the usual default and a common source of
/// documents rendering entirely bold.
pub fn is_on(element: &xml::Element) -> bool {
    match element.attr_local("val") {
        None => true,
        Some(value) => !matches!(value.trim(), "0" | "false" | "off"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::build_zip;

    const ROOT_RELS: &[u8] = br#"<?xml version="1.0"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>
</Relationships>"#;

    const CORE: &[u8] = br#"<?xml version="1.0"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
  xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/">
  <dc:title>Annual Review</dc:title>
  <dc:creator>R. Author</dc:creator>
  <dc:subject>Finance</dc:subject>
  <cp:lastModifiedBy>An Editor</cp:lastModifiedBy>
  <dcterms:created>2026-01-02T10:00:00Z</dcterms:created>
  <dcterms:modified>2026-02-03T11:00:00Z</dcterms:modified>
</cp:coreProperties>"#;

    fn package_zip() -> Vec<u8> {
        build_zip(&[
            ("_rels/.rels", ROOT_RELS, true),
            ("docProps/core.xml", CORE, true),
            ("word/document.xml", b"<w:document/>", true),
            (
                "word/_rels/document.xml.rels",
                br#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
                  <Relationship Id="rId7" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image1.png"/>
                  <Relationship Id="rId8" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com" TargetMode="External"/>
                </Relationships>"#,
                true,
            ),
        ])
    }

    #[test]
    fn finds_the_main_part_through_the_root_relationship() {
        let zip = package_zip();
        let archive = ZipArchive::open(&zip).unwrap();
        let package = Package::open(&archive).unwrap();
        assert_eq!(package.main_part, "word/document.xml");
        assert_eq!(package.base(), "word/");
    }

    #[test]
    fn falls_back_to_conventional_paths_without_a_root_relationship() {
        let zip = build_zip(&[("xl/workbook.xml", b"<workbook/>", true)]);
        let archive = ZipArchive::open(&zip).unwrap();
        let package = Package::open(&archive).unwrap();
        assert_eq!(package.main_part, "xl/workbook.xml");
    }

    #[test]
    fn a_package_with_no_main_part_is_an_error() {
        let zip = build_zip(&[("random.xml", b"<x/>", true)]);
        let archive = ZipArchive::open(&zip).unwrap();
        assert!(matches!(
            Package::open(&archive),
            Err(PdfError::MissingPart(_))
        ));
    }

    #[test]
    fn reads_core_properties_by_namespace() {
        let zip = package_zip();
        let archive = ZipArchive::open(&zip).unwrap();
        let package = Package::open(&archive).unwrap();

        let mut document = SemanticDoc::new();
        package.apply_core_properties(&mut document);
        assert_eq!(document.title.as_deref(), Some("Annual Review"));
        assert_eq!(document.author.as_deref(), Some("R. Author"));
        assert_eq!(document.subject.as_deref(), Some("Finance"));
        assert_eq!(document.creator.as_deref(), Some("An Editor"));
        assert_eq!(document.created.as_deref(), Some("2026-01-02T10:00:00Z"));
        assert_eq!(document.modified.as_deref(), Some("2026-02-03T11:00:00Z"));
    }

    #[test]
    fn core_properties_do_not_overwrite_a_title_found_in_content() {
        let zip = package_zip();
        let archive = ZipArchive::open(&zip).unwrap();
        let package = Package::open(&archive).unwrap();

        let mut document = SemanticDoc::new();
        document.title = Some("From the content".into());
        package.apply_core_properties(&mut document);
        assert_eq!(document.title.as_deref(), Some("From the content"));
    }

    #[test]
    fn resolves_part_relationships_relative_to_their_owner() {
        let zip = package_zip();
        let archive = ZipArchive::open(&zip).unwrap();
        let package = Package::open(&archive).unwrap();

        // An image target is relative to `word/`, the main part's directory.
        assert_eq!(
            package.main_rels.resolve("word/", "rId7").as_deref(),
            Some("word/media/image1.png")
        );
    }

    #[test]
    fn external_targets_are_returned_verbatim() {
        let zip = package_zip();
        let archive = ZipArchive::open(&zip).unwrap();
        let package = Package::open(&archive).unwrap();
        // A hyperlink is a URL, not a package path — resolving it against the
        // part directory would corrupt it.
        assert_eq!(
            package.main_rels.resolve("word/", "rId8").as_deref(),
            Some("https://example.com")
        );
    }

    #[test]
    fn relationship_types_match_on_their_suffix() {
        let rel = Relationship {
            kind: "http://purl.oclc.org/ooxml/officeDocument/relationships/image".into(),
            ..Relationship::default()
        };
        // The strict-flavour namespace differs, but the suffix does not.
        assert!(rel.is("image"));
        assert!(!rel.is("hyperlink"));
    }

    #[test]
    fn resolves_relative_and_absolute_paths() {
        assert_eq!(resolve_path("word/", "media/i.png"), "word/media/i.png");
        assert_eq!(
            resolve_path("ppt/slides/", "../media/i.png"),
            "ppt/media/i.png"
        );
        assert_eq!(
            resolve_path("word/", "/docProps/core.xml"),
            "docProps/core.xml"
        );
        assert_eq!(resolve_path("", "word/document.xml"), "word/document.xml");
    }

    #[test]
    fn splits_package_paths() {
        assert_eq!(
            split_path("word/document.xml"),
            ("word/".to_string(), "document.xml".to_string())
        );
        assert_eq!(
            split_path("top.xml"),
            (String::new(), "top.xml".to_string())
        );
    }

    #[test]
    fn an_absent_val_attribute_means_the_property_is_on() {
        let mut reader = Reader::new(br#"<w:b/><w:i w:val="0"/><w:u w:val="true"/>"#);
        let mut states = Vec::new();
        while let Some(event) = reader.read_event() {
            if let Event::Start(element) = event {
                states.push(is_on(&element));
            }
        }
        assert_eq!(states, vec![true, false, true]);
    }

    #[test]
    fn converts_emu_to_points() {
        // 914,400 EMU is one inch, which is 72 points.
        assert_eq!(emu_to_points(914_400), 72.0);
    }
}
