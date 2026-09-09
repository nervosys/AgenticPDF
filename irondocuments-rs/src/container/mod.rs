// SPDX-License-Identifier: AGPL-3.0-or-later
//! Container formats that document formats are packaged inside.
//!
//! Modern Office (`.docx`/`.pptx`/`.xlsx`), OpenDocument (`.odt`/`.ods`/`.odp`)
//! and EPUB are ZIP archives of XML parts; legacy Office (`.doc`/`.xls`/`.ppt`)
//! is an OLE2 compound file. Neither container knows anything about the
//! document model — they just hand named byte streams to the format parsers in
//! [`crate::formats`].

pub mod ole;
pub mod zip;
