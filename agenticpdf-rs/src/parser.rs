//! AgenticPDF PDF Parser — Core parsing implementation.
//!
//! Handles PDF structure: header, xref table, trailer, indirect objects,
//! dictionaries, arrays, streams, and content stream operators.
//! Extracts text with positioning, metadata, annotations, and outlines.

use crate::{
    OutlineItem, PdfAnnotation, PdfDocument, PdfError, PdfMetadata, PdfPage, TextBlock, XRefEntry,
};
use miniz_oxide::inflate::decompress_to_vec_zlib;

/// Maximum recursion depth for object resolution to prevent stack overflow.
const MAX_RECURSION_DEPTH: usize = 64;

/// Maximum number of xref entries to prevent memory exhaustion.
const MAX_XREF_ENTRIES: usize = 1_000_000;

/// Maximum stream length to decompress (256 MB).
const MAX_STREAM_SIZE: usize = 256 * 1024 * 1024;

/// Maximum number of objects to parse.
const MAX_OBJECTS: usize = 500_000;

/// PDF parser with zero-copy byte-level access.
pub struct PdfParser<'a> {
    data: &'a [u8],
    pos: usize,
    xref: Vec<XRefEntry>,
}

impl<'a> PdfParser<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            xref: Vec::new(),
        }
    }

    /// Parse the entire PDF and return a PdfDocument.
    pub fn parse(&mut self) -> Result<PdfDocument, PdfError> {
        let version = self.parse_header()?;
        self.parse_xref()?;
        let metadata = self.parse_metadata(&version)?;
        let pages = self.parse_pages()?;
        let annotations = self.parse_annotations(&pages);
        let outline = self.parse_outline();

        Ok(PdfDocument {
            version,
            pages,
            metadata,
            annotations,
            outline,
            xref: self.xref.clone(),
            data: self.data.to_vec(),
        })
    }

    // ========================================================================
    // Header
    // ========================================================================

    fn parse_header(&mut self) -> Result<String, PdfError> {
        if self.data.len() < 8 {
            return Err(PdfError::InvalidHeader);
        }

        let header = &self.data[..8];
        if !header.starts_with(b"%PDF-") {
            return Err(PdfError::InvalidHeader);
        }

        // Extract version string (e.g., "1.7")
        let version = std::str::from_utf8(&header[5..8])
            .map_err(|_| PdfError::InvalidHeader)?
            .to_string();

        self.pos = 8;
        Ok(version)
    }

    // ========================================================================
    // Cross-reference table
    // ========================================================================

    fn parse_xref(&mut self) -> Result<(), PdfError> {
        let startxref_pos = self
            .find_reverse(b"startxref")
            .ok_or(PdfError::XRefNotFound)?;

        // Read the offset value after "startxref"
        let mut pos = startxref_pos + 9;
        self.skip_whitespace_at(&mut pos);

        let xref_offset = self.parse_number_at(&mut pos)? as usize;

        if xref_offset >= self.data.len() {
            return Err(PdfError::InvalidXRef("offset out of bounds".into()));
        }

        // Check if it's a traditional xref table or xref stream
        if self.data[xref_offset..].starts_with(b"xref") {
            self.parse_xref_table(xref_offset)?;
        } else {
            // Cross-reference stream (PDF 1.5+)
            self.parse_xref_stream(xref_offset)?;
        }

        Ok(())
    }

    fn parse_xref_table(&mut self, offset: usize) -> Result<(), PdfError> {
        let mut pos = offset + 4; // skip "xref"
        self.skip_whitespace_at(&mut pos);

        let mut entry_count = 0usize;

        while pos < self.data.len() && self.data[pos].is_ascii_digit() {
            let start_obj = self.parse_number_at(&mut pos)? as u32;
            self.skip_whitespace_at(&mut pos);
            let count = self.parse_number_at(&mut pos)? as u32;
            self.skip_whitespace_at(&mut pos);

            for i in 0..count {
                if entry_count >= MAX_XREF_ENTRIES {
                    return Err(PdfError::InvalidXRef("too many entries".into()));
                }

                if pos + 20 > self.data.len() {
                    break;
                }

                let entry_offset = self.parse_number_at(&mut pos)? as usize;
                self.skip_whitespace_at(&mut pos);
                let generation = self.parse_number_at(&mut pos)? as u16;
                self.skip_whitespace_at(&mut pos);

                let type_char = self.data[pos];
                pos += 1;
                self.skip_whitespace_at(&mut pos);

                self.xref.push(XRefEntry {
                    obj_num: start_obj + i,
                    offset: entry_offset,
                    generation,
                    in_use: type_char == b'n',
                });

                entry_count += 1;
            }
        }

        Ok(())
    }

    fn parse_xref_stream(&mut self, offset: usize) -> Result<(), PdfError> {
        self.pos = offset;

        // Parse "objNum genNum obj"
        let _obj_num = self.parse_number_at(&mut self.pos.clone())? as u32;
        // Skip to stream dictionary — simplified for now
        // Full xref stream parsing requires content stream decompression
        // which is handled by the miniz_oxide dependency.

        // For the initial scaffold, we parse what we can from the trailer
        let _ = self.find_value_in_dict(offset, b"/Size");

        Ok(())
    }

    // ========================================================================
    // Metadata
    // ========================================================================

    fn parse_metadata(&self, version: &str) -> Result<PdfMetadata, PdfError> {
        let mut metadata = PdfMetadata {
            pdf_version: version.to_string(),
            file_size: self.data.len(),
            ..Default::default()
        };

        // Find trailer dictionary for /Info reference
        if let Some(trailer_pos) = self.find_reverse(b"trailer") {
            // Try to extract metadata from /Info indirect object
            if let Some(info_ref) = self.find_value_in_dict(trailer_pos, b"/Info") {
                // Parse "N G R" indirect reference
                let parts: Vec<&str> = info_ref.trim().split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(obj_num) = parts[0].parse::<u32>() {
                        self.extract_info_dict(&mut metadata, obj_num);
                    }
                }
            }

            // Fallback: try to find metadata directly in trailer region
            if metadata.title.is_none() {
                if let Some(title) = self.extract_string_value(trailer_pos, b"/Title") {
                    metadata.title = Some(title);
                }
            }
            if metadata.author.is_none() {
                if let Some(author) = self.extract_string_value(trailer_pos, b"/Author") {
                    metadata.author = Some(author);
                }
            }
        }

        // Count actual pages by scanning for /Type /Page entries
        metadata.page_count = self.count_pages();

        // Check for encryption
        metadata.encrypted = self.find_reverse(b"/Encrypt").is_some();

        // Feature detection
        metadata.has_forms = self.find_forward_from(b"/AcroForm", 0).is_some();
        metadata.has_outlines = self.find_forward_from(b"/Outlines", 0).is_some();
        metadata.has_annotations = self.find_forward_from(b"/Annots", 0).is_some();

        Ok(metadata)
    }

    /// Extract metadata fields from the /Info dictionary object.
    fn extract_info_dict(&self, metadata: &mut PdfMetadata, obj_num: u32) {
        let entry = self.xref.iter().find(|e| e.obj_num == obj_num && e.in_use);
        let offset = match entry {
            Some(e) if e.offset < self.data.len() => e.offset,
            _ => return,
        };

        if let Some(title) = self.extract_string_value(offset, b"/Title") {
            metadata.title = Some(title);
        }
        if let Some(author) = self.extract_string_value(offset, b"/Author") {
            metadata.author = Some(author);
        }
        if let Some(subject) = self.extract_string_value(offset, b"/Subject") {
            metadata.subject = Some(subject);
        }
        if let Some(creator) = self.extract_string_value(offset, b"/Creator") {
            metadata.creator = Some(creator);
        }
        if let Some(producer) = self.extract_string_value(offset, b"/Producer") {
            metadata.producer = Some(producer);
        }
        if let Some(date) = self.extract_string_value(offset, b"/CreationDate") {
            metadata.creation_date = Some(date);
        }
        if let Some(date) = self.extract_string_value(offset, b"/ModDate") {
            metadata.modification_date = Some(date);
        }
    }

    fn count_pages(&self) -> usize {
        // Count occurrences of "/Type /Page" (not "/Type /Pages")
        let needle = b"/Type /Page";
        let _pages_needle = b"/Type /Pages";
        let mut count = 0usize;
        let mut pos = 0;

        while let Some(found) = self.find_forward_from(needle, pos) {
            // Make sure it's not "/Type /Pages"
            let end = found + needle.len();
            if end < self.data.len() && self.data[end] == b's' {
                // This is "/Type /Pages", skip it
                pos = end + 1;
                continue;
            }
            count += 1;
            pos = end;
        }

        count
    }

    // ========================================================================
    // Page parsing
    // ========================================================================

    fn parse_pages(&self) -> Result<Vec<PdfPage>, PdfError> {
        let mut pages = Vec::new();
        let mut page_index = 0usize;

        // Find all page objects by scanning for /Type /Page
        let needle = b"/Type /Page";
        let mut pos = 0;

        while let Some(found) = self.find_forward_from(needle, pos) {
            let end = found + needle.len();

            // Skip "/Type /Pages"
            if end < self.data.len() && self.data[end] == b's' {
                pos = end + 1;
                continue;
            }

            if page_index >= MAX_OBJECTS {
                break;
            }

            // Find the dictionary start (search backward for "<<")
            if let Some(dict_start) = self.find_dict_start(found) {
                let page = self.parse_page_object(dict_start, page_index)?;
                pages.push(page);
                page_index += 1;
            }

            pos = end;
        }

        Ok(pages)
    }

    fn parse_page_object(&self, dict_start: usize, index: usize) -> Result<PdfPage, PdfError> {
        // Extract MediaBox for page dimensions
        let (width, height) = self.extract_media_box(dict_start).unwrap_or((612.0, 792.0)); // US Letter default

        // Extract text from content stream
        let text_content = self.extract_page_text(dict_start, index + 1);

        Ok(PdfPage {
            index,
            width,
            height,
            text_content,
        })
    }

    fn extract_media_box(&self, dict_start: usize) -> Option<(f64, f64)> {
        let search_region =
            &self.data[dict_start..std::cmp::min(dict_start + 2000, self.data.len())];

        if let Some(mb_pos) = Self::find_in_slice(search_region, b"/MediaBox") {
            let start = mb_pos + 9; // skip "/MediaBox"
            if let Some(bracket) = Self::find_in_slice(&search_region[start..], b"[") {
                let arr_start = start + bracket + 1;
                if let Some(bracket_end) = Self::find_in_slice(&search_region[arr_start..], b"]") {
                    let arr_data = &search_region[arr_start..arr_start + bracket_end];
                    let arr_str = std::str::from_utf8(arr_data).ok()?;
                    let nums: Vec<f64> = arr_str
                        .split_whitespace()
                        .filter_map(|s| s.parse().ok())
                        .collect();

                    if nums.len() >= 4 {
                        return Some((nums[2] - nums[0], nums[3] - nums[1]));
                    }
                }
            }
        }

        None
    }

    fn extract_page_text(&self, dict_start: usize, page_number: usize) -> Vec<TextBlock> {
        let mut blocks = Vec::new();
        let search_end = std::cmp::min(dict_start + 5000, self.data.len());
        let region = &self.data[dict_start..search_end];

        // Find /Contents reference
        if let Some(contents_pos) = Self::find_in_slice(region, b"/Contents") {
            let after = contents_pos + 9;
            // Try to find a stream reference — look for "R" (indirect reference)
            let ref_region = &region[after..std::cmp::min(after + 100, region.len())];
            let ref_str = std::str::from_utf8(ref_region).unwrap_or("");

            // Parse indirect reference "N G R"
            let parts: Vec<&str> = ref_str.split_whitespace().collect();
            if parts.len() >= 3 && parts[2] == "R" {
                if let Ok(obj_num) = parts[0].parse::<u32>() {
                    if let Some(text) = self.extract_text_from_object(obj_num, page_number) {
                        blocks.extend(text);
                    }
                }
            }
        }

        blocks
    }

    fn extract_text_from_object(&self, obj_num: u32, page_number: usize) -> Option<Vec<TextBlock>> {
        // Find object in xref
        let entry = self
            .xref
            .iter()
            .find(|e| e.obj_num == obj_num && e.in_use)?;

        if entry.offset >= self.data.len() {
            return None;
        }

        // Find stream data
        let obj_data = &self.data[entry.offset..];
        let stream_start = Self::find_in_slice(obj_data, b"stream")?;
        let mut data_start = stream_start + 6;

        // Skip \r\n or \n after "stream"
        if data_start < obj_data.len() && obj_data[data_start] == b'\r' {
            data_start += 1;
        }
        if data_start < obj_data.len() && obj_data[data_start] == b'\n' {
            data_start += 1;
        }

        let stream_end = Self::find_in_slice(&obj_data[data_start..], b"endstream")?;
        let stream_bytes = &obj_data[data_start..data_start + stream_end];

        if stream_bytes.len() > MAX_STREAM_SIZE {
            return None;
        }

        // Try to decompress (FlateDecode)
        let content = if Self::find_in_slice(&obj_data[..stream_start], b"/FlateDecode").is_some() {
            decompress_to_vec_zlib(stream_bytes).ok()?
        } else {
            stream_bytes.to_vec()
        };

        // Parse text operators from content stream
        Some(self.parse_text_operators(&content, page_number))
    }

    fn parse_text_operators(&self, content: &[u8], page_number: usize) -> Vec<TextBlock> {
        let mut blocks = Vec::new();
        let text = std::str::from_utf8(content).unwrap_or("");
        let mut x_pos = 0.0f64;
        let mut y_pos = 0.0f64;
        let mut font_size = 12.0f64;
        let mut font_name = String::new();
        let mut current_text = String::new();

        for line in text.lines() {
            let trimmed = line.trim();

            // Font setting: /FontName size Tf
            if trimmed.ends_with("Tf") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 3 {
                    if let Ok(size) = parts[parts.len() - 2].parse::<f64>() {
                        font_size = size;
                    }
                    // Extract font name (strip leading /)
                    let name = parts[parts.len() - 3];
                    font_name = name.strip_prefix('/').unwrap_or(name).to_string();
                }
            }

            // Text positioning: x y Td or x y TD
            if trimmed.ends_with("Td") || trimmed.ends_with("TD") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 3 {
                    if let Ok(x) = parts[parts.len() - 3].parse::<f64>() {
                        x_pos += x;
                    }
                    if let Ok(y) = parts[parts.len() - 2].parse::<f64>() {
                        y_pos += y;
                    }
                }
            }

            // Text matrix: a b c d e f Tm
            if trimmed.ends_with("Tm") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 7 {
                    if let Ok(x) = parts[parts.len() - 3].parse::<f64>() {
                        x_pos = x;
                    }
                    if let Ok(y) = parts[parts.len() - 2].parse::<f64>() {
                        y_pos = y;
                    }
                }
            }

            // Text show: (text) Tj
            if trimmed.ends_with("Tj") {
                if let Some(start) = trimmed.find('(') {
                    if let Some(end) = trimmed.rfind(')') {
                        let extracted = &trimmed[start + 1..end];
                        // Unescape basic PDF string escapes
                        let unescaped = Self::unescape_pdf_string(extracted);
                        current_text.push_str(&unescaped);
                    }
                }
            }

            // TJ array: [(text) kern (text)] TJ
            if trimmed.ends_with("TJ") {
                let mut in_string = false;
                let mut escaped = false;
                let mut chars = trimmed.chars().peekable();
                let mut buffer = String::new();

                while let Some(ch) = chars.next() {
                    if escaped {
                        buffer.push(ch);
                        escaped = false;
                        continue;
                    }

                    match ch {
                        '\\' if in_string => escaped = true,
                        '(' if !in_string => {
                            in_string = true;
                            buffer.clear();
                        }
                        ')' if in_string => {
                            in_string = false;
                            current_text.push_str(&buffer);
                        }
                        _ if in_string => buffer.push(ch),
                        _ => {}
                    }
                }
            }

            // End text object — emit what we have
            if trimmed == "ET" && !current_text.is_empty() {
                blocks.push(TextBlock {
                    text: std::mem::take(&mut current_text),
                    x: x_pos,
                    y: y_pos,
                    width: 0.0,
                    height: font_size,
                    font_size,
                    font_name: font_name.clone(),
                    page_number,
                });
            }
        }

        // Flush remaining text
        if !current_text.is_empty() {
            blocks.push(TextBlock {
                text: current_text,
                x: x_pos,
                y: y_pos,
                width: 0.0,
                height: font_size,
                font_size,
                font_name,
                page_number,
            });
        }

        blocks
    }

    // ========================================================================
    // Annotation parsing
    // ========================================================================

    /// Extract annotations from all pages.
    fn parse_annotations(&self, pages: &[PdfPage]) -> Vec<PdfAnnotation> {
        let mut annotations = Vec::new();

        // Scan for /Annots arrays in page dictionaries
        let needle = b"/Type /Page";
        let mut pos = 0;
        let mut page_num = 0usize;

        while let Some(found) = self.find_forward_from(needle, pos) {
            let end = found + needle.len();

            // Skip "/Type /Pages"
            if end < self.data.len() && self.data[end] == b's' {
                pos = end + 1;
                continue;
            }

            page_num += 1;

            if let Some(dict_start) = self.find_dict_start(found) {
                let search_end = std::cmp::min(dict_start + 2000, self.data.len());
                let region = &self.data[dict_start..search_end];

                if let Some(annots_pos) = Self::find_in_slice(region, b"/Annots") {
                    let after = annots_pos + 7;
                    // Look for [ ... ] array of references
                    if let Some(bracket_start) = Self::find_in_slice(&region[after..], b"[") {
                        let arr_start = after + bracket_start + 1;
                        if let Some(bracket_end) = Self::find_in_slice(&region[arr_start..], b"]") {
                            let arr_data = &region[arr_start..arr_start + bracket_end];
                            let arr_str = std::str::from_utf8(arr_data).unwrap_or("");

                            // Parse indirect references: "N G R N G R ..."
                            let parts: Vec<&str> = arr_str.split_whitespace().collect();
                            let mut i = 0;
                            while i + 2 < parts.len() {
                                if parts[i + 2] == "R" {
                                    if let Ok(obj_num) = parts[i].parse::<u32>() {
                                        if let Some(annot) =
                                            self.parse_annotation_object(obj_num, page_num)
                                        {
                                            annotations.push(annot);
                                        }
                                    }
                                    i += 3;
                                } else {
                                    i += 1;
                                }
                            }
                        }
                    }
                }
            }

            pos = end;
            if page_num >= pages.len() {
                break;
            }
        }

        annotations
    }

    /// Parse a single annotation indirect object.
    fn parse_annotation_object(&self, obj_num: u32, page_number: usize) -> Option<PdfAnnotation> {
        let entry = self
            .xref
            .iter()
            .find(|e| e.obj_num == obj_num && e.in_use)?;
        if entry.offset >= self.data.len() {
            return None;
        }

        let search_end = std::cmp::min(entry.offset + 2000, self.data.len());
        let region = &self.data[entry.offset..search_end];

        // Extract /Subtype
        let subtype = self
            .find_name_value(region, b"/Subtype")
            .unwrap_or_default();

        // Extract /Rect [x1 y1 x2 y2]
        let rect = self.extract_rect(region).unwrap_or([0.0; 4]);

        // Extract /Contents (string)
        let contents = self.extract_string_in_region(region, b"/Contents");

        // Extract /A >> /URI for link annotations
        let uri = self.extract_uri_from_action(region);

        // Extract /Dest
        let dest = self.find_name_value(region, b"/Dest");

        // Extract /T (title/author)
        let title = self.extract_string_in_region(region, b"/T");

        // Extract /C [r g b] color
        let color = self.extract_color_array(region);

        Some(PdfAnnotation {
            subtype,
            page_number,
            rect,
            contents,
            uri,
            dest,
            title,
            color,
        })
    }

    // ========================================================================
    // Outline / bookmarks parsing
    // ========================================================================

    /// Parse the document outline (bookmarks) from /Outlines.
    fn parse_outline(&self) -> Vec<OutlineItem> {
        // Find /Outlines in the catalog
        let outlines_pos = match self.find_forward_from(b"/Outlines", 0) {
            Some(p) => p,
            None => return Vec::new(),
        };

        // Find the indirect reference after /Outlines
        let search_end = std::cmp::min(outlines_pos + 100, self.data.len());
        let region = &self.data[outlines_pos + 9..search_end];
        let ref_str = std::str::from_utf8(region).unwrap_or("");
        let parts: Vec<&str> = ref_str.split_whitespace().collect();

        if parts.len() < 3 || parts[2] != "R" {
            return Vec::new();
        }

        let obj_num = match parts[0].parse::<u32>() {
            Ok(n) => n,
            Err(_) => return Vec::new(),
        };

        // Find the outline root object and its /First child
        let entry = self.xref.iter().find(|e| e.obj_num == obj_num && e.in_use);
        let offset = match entry {
            Some(e) if e.offset < self.data.len() => e.offset,
            _ => return Vec::new(),
        };

        let search_end = std::cmp::min(offset + 2000, self.data.len());
        let region = &self.data[offset..search_end];

        // Get /First child reference
        let first_ref = match self.find_ref_value(region, b"/First") {
            Some(n) => n,
            None => return Vec::new(),
        };

        self.parse_outline_items(first_ref, 0)
    }

    /// Recursively parse outline items following /First and /Next links.
    fn parse_outline_items(&self, obj_num: u32, depth: usize) -> Vec<OutlineItem> {
        if depth >= MAX_RECURSION_DEPTH {
            return Vec::new();
        }

        let mut items = Vec::new();
        let mut current = Some(obj_num);

        while let Some(cur_obj) = current {
            let entry = self.xref.iter().find(|e| e.obj_num == cur_obj && e.in_use);
            let offset = match entry {
                Some(e) if e.offset < self.data.len() => e.offset,
                _ => break,
            };

            let search_end = std::cmp::min(offset + 2000, self.data.len());
            let region = &self.data[offset..search_end];

            // Extract /Title
            let title = self
                .extract_string_in_region(region, b"/Title")
                .unwrap_or_default();

            // Extract destination page (simplified: look for /Dest or /A)
            let dest = self.find_name_value(region, b"/Dest");

            // Extract children via /First
            let children = match self.find_ref_value(region, b"/First") {
                Some(first) => self.parse_outline_items(first, depth + 1),
                None => Vec::new(),
            };

            items.push(OutlineItem {
                title,
                page_number: None, // Would need full dest resolution
                dest,
                children,
            });

            // Follow /Next sibling
            current = self.find_ref_value(region, b"/Next");

            if items.len() >= 10_000 {
                break; // Safety limit
            }
        }

        items
    }

    // ========================================================================
    // Annotation/outline helper methods
    // ========================================================================

    /// Find a /Name value in a byte region (returns the name without /).
    fn find_name_value(&self, region: &[u8], key: &[u8]) -> Option<String> {
        let key_pos = Self::find_in_slice(region, key)?;
        let after = key_pos + key.len();
        let value_region = &region[after..std::cmp::min(after + 100, region.len())];
        let s = std::str::from_utf8(value_region).ok()?;
        let trimmed = s.trim();

        if trimmed.starts_with('/') {
            let end_pos = trimmed[1..]
                .find(|c: char| c.is_whitespace() || c == '/' || c == '>' || c == '[')
                .map(|p| p + 1)
                .unwrap_or(trimmed.len());
            Some(trimmed[1..end_pos].to_string())
        } else {
            None
        }
    }

    /// Extract a [x1 y1 x2 y2] rectangle from a region after /Rect.
    fn extract_rect(&self, region: &[u8]) -> Option<[f64; 4]> {
        let key_pos = Self::find_in_slice(region, b"/Rect")?;
        let after = key_pos + 5;
        let bracket = Self::find_in_slice(&region[after..], b"[")?;
        let arr_start = after + bracket + 1;
        let bracket_end = Self::find_in_slice(&region[arr_start..], b"]")?;
        let arr_data = &region[arr_start..arr_start + bracket_end];
        let arr_str = std::str::from_utf8(arr_data).ok()?;
        let nums: Vec<f64> = arr_str
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        if nums.len() >= 4 {
            Some([nums[0], nums[1], nums[2], nums[3]])
        } else {
            None
        }
    }

    /// Extract a URI from an /A action dictionary.
    fn extract_uri_from_action(&self, region: &[u8]) -> Option<String> {
        // Look for /URI within the region
        let uri_pos = Self::find_in_slice(region, b"/URI")?;
        let after = uri_pos + 4;
        // URI can be (string) or <hex>
        self.extract_string_in_region(&region[after..], b"")
            .or_else(|| {
                // Try finding a parenthesized string right after /URI
                let value_region = &region[after..std::cmp::min(after + 500, region.len())];
                let paren_start = Self::find_in_slice(value_region, b"(")?;
                let str_start = paren_start + 1;
                let paren_end = Self::find_in_slice(&value_region[str_start..], b")")?;
                let bytes = &value_region[str_start..str_start + paren_end];
                std::str::from_utf8(bytes).ok().map(String::from)
            })
    }

    /// Extract a [r g b] color array from /C.
    fn extract_color_array(&self, region: &[u8]) -> Option<[f64; 3]> {
        let key_pos = Self::find_in_slice(region, b"/C")?;
        // Make sure it's actually /C and not /Contents, /Creator, etc.
        let after_c = key_pos + 2;
        if after_c < region.len() && region[after_c].is_ascii_alphabetic() {
            return None;
        }
        let bracket = Self::find_in_slice(&region[after_c..], b"[")?;
        let arr_start = after_c + bracket + 1;
        let bracket_end = Self::find_in_slice(&region[arr_start..], b"]")?;
        let arr_data = &region[arr_start..arr_start + bracket_end];
        let arr_str = std::str::from_utf8(arr_data).ok()?;
        let nums: Vec<f64> = arr_str
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        if nums.len() >= 3 {
            Some([nums[0], nums[1], nums[2]])
        } else {
            None
        }
    }

    /// Extract a string value from a region at a given key.
    fn extract_string_in_region(&self, region: &[u8], key: &[u8]) -> Option<String> {
        let search_region = if key.is_empty() {
            region
        } else {
            let key_pos = Self::find_in_slice(region, key)?;
            &region[key_pos + key.len()..]
        };

        let paren_start = Self::find_in_slice(search_region, b"(")?;
        let str_start = paren_start + 1;

        // Handle nested parentheses and escapes
        let mut depth = 1;
        let mut end = str_start;
        let mut escaped = false;
        while end < search_region.len() && depth > 0 {
            if escaped {
                escaped = false;
                end += 1;
                continue;
            }
            match search_region[end] {
                b'\\' => escaped = true,
                b'(' => depth += 1,
                b')' => depth -= 1,
                _ => {}
            }
            if depth > 0 {
                end += 1;
            }
        }

        let bytes = &search_region[str_start..end];
        std::str::from_utf8(bytes)
            .ok()
            .map(|s| Self::unescape_pdf_string(s))
    }

    /// Find an indirect reference "N G R" after a key and return the object number.
    fn find_ref_value(&self, region: &[u8], key: &[u8]) -> Option<u32> {
        let key_pos = Self::find_in_slice(region, key)?;
        let after = key_pos + key.len();
        let value_region = &region[after..std::cmp::min(after + 50, region.len())];
        let s = std::str::from_utf8(value_region).ok()?;
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() >= 3 && parts[2] == "R" {
            parts[0].parse::<u32>().ok()
        } else {
            None
        }
    }

    // ========================================================================
    // FlateDecode decompression
    // ========================================================================

    /// Decompress a FlateDecode stream, then apply PNG predictor un-filtering
    /// if the dictionary specifies /DecodeParms with /Predictor >= 10.
    pub fn decompress_stream(
        data: &[u8],
        predictor: Option<u8>,
        columns: Option<usize>,
    ) -> Result<Vec<u8>, PdfError> {
        if data.len() > MAX_STREAM_SIZE {
            return Err(PdfError::StreamError("stream too large".into()));
        }

        let decompressed =
            decompress_to_vec_zlib(data).map_err(|e| PdfError::DecompressError(e.to_string()))?;

        match predictor {
            Some(p) if p >= 10 => {
                let cols = columns.unwrap_or(1);
                Self::png_unfilter(&decompressed, cols)
            }
            _ => Ok(decompressed),
        }
    }

    /// Apply PNG un-filtering (predictors 10-14).
    fn png_unfilter(data: &[u8], columns: usize) -> Result<Vec<u8>, PdfError> {
        let row_bytes = columns + 1; // +1 for filter byte
        if data.len() % row_bytes != 0 {
            return Err(PdfError::DecompressError(
                "data length not aligned to row size".into(),
            ));
        }

        let num_rows = data.len() / row_bytes;
        let mut output = Vec::with_capacity(num_rows * columns);
        let mut prev_row = vec![0u8; columns];

        for row_idx in 0..num_rows {
            let row_start = row_idx * row_bytes;
            let filter_type = data[row_start];
            let row_data = &data[row_start + 1..row_start + row_bytes];

            let mut current_row = vec![0u8; columns];

            for i in 0..columns {
                let raw = row_data[i];
                let a = if i > 0 { current_row[i - 1] } else { 0 }; // left
                let b = prev_row[i]; // above
                let c = if i > 0 { prev_row[i - 1] } else { 0 }; // upper-left

                current_row[i] = match filter_type {
                    0 => raw,                                                 // None
                    1 => raw.wrapping_add(a),                                 // Sub
                    2 => raw.wrapping_add(b),                                 // Up
                    3 => raw.wrapping_add(((a as u16 + b as u16) / 2) as u8), // Average
                    4 => raw.wrapping_add(Self::paeth(a, b, c)),              // Paeth
                    _ => raw,
                };
            }

            output.extend_from_slice(&current_row);
            prev_row = current_row;
        }

        Ok(output)
    }

    /// Paeth predictor function.
    fn paeth(a: u8, b: u8, c: u8) -> u8 {
        let p = a as i16 + b as i16 - c as i16;
        let pa = (p - a as i16).unsigned_abs();
        let pb = (p - b as i16).unsigned_abs();
        let pc = (p - c as i16).unsigned_abs();

        if pa <= pb && pa <= pc {
            a
        } else if pb <= pc {
            b
        } else {
            c
        }
    }

    // ========================================================================
    // Utility helpers
    // ========================================================================

    fn unescape_pdf_string(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        let mut chars = s.chars();

        while let Some(ch) = chars.next() {
            if ch == '\\' {
                match chars.next() {
                    Some('n') => result.push('\n'),
                    Some('r') => result.push('\r'),
                    Some('t') => result.push('\t'),
                    Some('b') => result.push('\u{0008}'),
                    Some('f') => result.push('\u{000C}'),
                    Some('(') => result.push('('),
                    Some(')') => result.push(')'),
                    Some('\\') => result.push('\\'),
                    Some(c) => {
                        result.push('\\');
                        result.push(c);
                    }
                    None => result.push('\\'),
                }
            } else {
                result.push(ch);
            }
        }

        result
    }

    fn skip_whitespace_at(&self, pos: &mut usize) {
        while *pos < self.data.len()
            && (self.data[*pos] == b' '
                || self.data[*pos] == b'\n'
                || self.data[*pos] == b'\r'
                || self.data[*pos] == b'\t')
        {
            *pos += 1;
        }
    }

    fn parse_number_at(&self, pos: &mut usize) -> Result<i64, PdfError> {
        self.skip_whitespace_at(pos);

        let start = *pos;
        let negative = if *pos < self.data.len() && self.data[*pos] == b'-' {
            *pos += 1;
            true
        } else {
            false
        };

        while *pos < self.data.len() && self.data[*pos].is_ascii_digit() {
            *pos += 1;
        }

        if *pos == start || (*pos == start + 1 && negative) {
            return Err(PdfError::ObjectParseError("expected number".into()));
        }

        let s = std::str::from_utf8(&self.data[start..*pos])
            .map_err(|_| PdfError::ObjectParseError("invalid UTF-8 in number".into()))?;
        s.parse::<i64>()
            .map_err(|_| PdfError::ObjectParseError(format!("invalid number: {}", s)))
    }

    fn find_reverse(&self, needle: &[u8]) -> Option<usize> {
        if needle.len() > self.data.len() {
            return None;
        }
        for i in (0..=self.data.len() - needle.len()).rev() {
            if self.data[i..i + needle.len()] == *needle {
                return Some(i);
            }
        }
        None
    }

    fn find_forward_from(&self, needle: &[u8], start: usize) -> Option<usize> {
        Self::find_in_slice(&self.data[start..], needle).map(|pos| pos + start)
    }

    fn find_in_slice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        if needle.len() > haystack.len() {
            return None;
        }
        haystack.windows(needle.len()).position(|w| w == needle)
    }

    fn find_dict_start(&self, from: usize) -> Option<usize> {
        // Search backward for "<<"
        if from < 2 {
            return None;
        }
        for i in (0..from - 1).rev() {
            if self.data[i] == b'<' && self.data[i + 1] == b'<' {
                return Some(i);
            }
        }
        None
    }

    fn find_value_in_dict(&self, dict_start: usize, key: &[u8]) -> Option<String> {
        let end = std::cmp::min(dict_start + 2000, self.data.len());
        let region = &self.data[dict_start..end];

        if let Some(key_pos) = Self::find_in_slice(region, key) {
            let value_start = key_pos + key.len();
            let value_region = &region[value_start..std::cmp::min(value_start + 100, region.len())];
            let s = std::str::from_utf8(value_region).ok()?;
            let trimmed = s.trim();

            // Extract until next / or >> or newline
            let end_pos = trimmed
                .find(|c: char| c == '/' || c == '>' || c == '\n' || c == '\r')
                .unwrap_or(trimmed.len());

            Some(trimmed[..end_pos].trim().to_string())
        } else {
            None
        }
    }

    fn extract_string_value(&self, region_start: usize, key: &[u8]) -> Option<String> {
        let end = std::cmp::min(region_start + 5000, self.data.len());
        let region = &self.data[region_start..end];

        // Delegate to extract_string_in_region which handles nested parens and escapes
        self.extract_string_in_region(region, key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_pdf() {
        // Minimal valid PDF structure
        let pdf_bytes = b"%PDF-1.4\n\
            1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
            2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
            3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>\nendobj\n\
            xref\n0 4\n\
            0000000000 65535 f \n\
            0000000009 00000 n \n\
            0000000058 00000 n \n\
            0000000115 00000 n \n\
            trailer\n<< /Size 4 /Root 1 0 R >>\n\
            startxref\n195\n%%EOF";

        let mut parser = PdfParser::new(pdf_bytes);
        let result = parser.parse();
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert_eq!(doc.version, "1.4");
        assert!(!doc.pages.is_empty());
    }

    #[test]
    fn test_invalid_header() {
        let result = PdfParser::new(b"NOT A PDF").parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_decompress_stream() {
        // Test with empty/minimal compressed data
        let empty = miniz_oxide::deflate::compress_to_vec_zlib(&[], 6);
        let result = PdfParser::decompress_stream(&empty, None, None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_png_unfilter_none() {
        // Filter type 0 (None): pass-through
        let data = vec![0, 10, 20, 30, 0, 40, 50, 60]; // 2 rows of 3 columns, filter=0
        let result = PdfParser::png_unfilter(&data, 3).unwrap();
        assert_eq!(result, vec![10, 20, 30, 40, 50, 60]);
    }

    #[test]
    fn test_paeth() {
        assert_eq!(PdfParser::paeth(0, 0, 0), 0);
        assert_eq!(PdfParser::paeth(10, 20, 5), 20); // closest to p=25
    }

    #[test]
    fn test_unescape_pdf_string() {
        assert_eq!(PdfParser::unescape_pdf_string("hello"), "hello");
        assert_eq!(PdfParser::unescape_pdf_string("a\\nb"), "a\nb");
        assert_eq!(PdfParser::unescape_pdf_string("a\\(b\\)"), "a(b)");
    }

    #[test]
    fn test_max_stream_size() {
        let huge = vec![0u8; MAX_STREAM_SIZE + 1];
        let result = PdfParser::decompress_stream(&huge, None, None);
        assert!(result.is_err());
    }
}
