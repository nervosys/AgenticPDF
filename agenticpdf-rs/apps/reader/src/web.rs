// SPDX-License-Identifier: AGPL-3.0-or-later
//! The browser and mobile-web shell.
//!
//! This is the *only* platform-specific code in the mobile build. Everything
//! else — opening, searching, editing, saving, painting the page — is the same
//! code the desktop binary runs, which is the whole reason the portable core
//! exists.
//!
//! The surface is deliberately narrow. Rather than mirror the desktop's message
//! enum in JavaScript, the host calls [`execute`] with the *same* action names
//! an agent uses. There is one command vocabulary for the desktop UI, the
//! browser UI and any agent, so a capability cannot exist for one and not the
//! others.

use wasm_bindgen::prelude::*;

use crate::actions;
use crate::canvas::{RecordingPainter, Transform};
use crate::session::Session;

/// How many glyph masks the host is assumed to hold before the reckoning is
/// dropped and started again.
const MAX_REMEMBERED_MASKS: usize = 4096;

/// A document open in the browser.
#[wasm_bindgen]
pub struct WebReader {
    session: Option<Session>,
    /// Glyph masks the host has already decoded and cached, so a redraw sends
    /// their keys instead of their pixels.
    sent_images: std::cell::RefCell<std::collections::HashSet<String>>,
}

#[wasm_bindgen]
impl WebReader {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WebReader {
        WebReader {
            session: None,
            sent_images: std::cell::RefCell::new(std::collections::HashSet::new()),
        }
    }

    /// Open a document from bytes the host read — a file input, a fetch, a
    /// share intent. There are no paths here.
    pub fn open(&mut self, bytes: &[u8]) -> String {
        match Session::open(bytes.to_vec()) {
            Ok(session) => {
                let summary = format!(
                    r#"{{"ok":true,"title":{},"format":"{}","pages":{}}}"#,
                    serde_json::to_string(&session.title()).unwrap_or_else(|_| "\"\"".into()),
                    session.format().id(),
                    session.page_count()
                );
                self.session = Some(session);
                summary
            }
            Err(why) => format!(
                r#"{{"ok":false,"error":{}}}"#,
                serde_json::to_string(&why.to_string()).unwrap_or_else(|_| "\"failed\"".into())
            ),
        }
    }

    /// Run an action. Same names, parameters and results as the agent protocol.
    ///
    /// `params` is a JSON object as a string; the result is a JSON string.
    /// Strings rather than `JsValue` so the boundary stays trivially testable
    /// and has one representation instead of two.
    pub fn execute(&mut self, action: &str, params: &str) -> String {
        let params: serde_json::Value =
            serde_json::from_str(params).unwrap_or(serde_json::json!({}));
        let result = actions::execute(&mut self.session, action, &params);
        result.to_string()
    }

    /// Paint the current page and return the recording for the host to replay.
    ///
    /// `width` and `height` are the CSS pixel size of the canvas.
    pub fn render_page(&self, width: f32, height: f32, zoom: f32) -> String {
        let Some(session) = &self.session else {
            return "[]".to_string();
        };
        let Ok(list) = session.display_list() else {
            // No geometry is a real answer, not an error: some formats have
            // none, and the host should say so rather than show a blank page.
            return r#"[{"op":"no_geometry"}]"#.to_string();
        };

        let area = dewey::core::Rect::new(0.0, 0.0, width, height);
        let transform = Transform::fit(&list, area, zoom);
        // The host decodes each glyph mask once and keeps it by key, so a
        // redraw sends keys rather than pixels. Without this every scroll and
        // zoom re-inlines the same few hundred masks.
        let known = self.sent_images.borrow().clone();
        let mut painter = RecordingPainter::with_known_images(known);
        crate::canvas::paint_page(&mut painter, &list, transform, &[], session.fonts());
        let json = painter.to_json();
        let mut sent = painter.image_keys();
        // Bounded: every zoom level produces its own masks, so an afternoon of
        // reading would otherwise accumulate them without limit. Clearing
        // costs one redraw that re-sends pixels, which is what the host does
        // on first paint anyway.
        if sent.len() > MAX_REMEMBERED_MASKS {
            sent.clear();
        }
        *self.sent_images.borrow_mut() = sent;
        json
    }

    /// Tell the reader how many device pixels the host paints per CSS pixel.
    ///
    /// The recording is in CSS pixels and the host scales it up by this ratio,
    /// so a mask rasterised one pixel per CSS pixel is enlarged by the same
    /// factor and the text goes soft. Passing the ratio through is what makes
    /// a phone's page as sharp as its screen allows.
    pub fn set_device_pixel_ratio(&self, ratio: f32) {
        if let Some(session) = &self.session {
            session.fonts().set_raster_scale(ratio as f64);
        }
    }

    /// Forget which masks the host is believed to hold, so the next recording
    /// carries their pixels again.
    ///
    /// The host calls this when it finds itself asked to draw a mask it does
    /// not have. The two caches are meant to stay in step, but a host that
    /// drops one for any reason of its own -- memory pressure, a reload, a
    /// mask it declined to decode -- would otherwise be sent keys forever and
    /// show a page with no text on it. One redraw restores it.
    pub fn forget_images(&self) {
        self.sent_images.borrow_mut().clear();
    }

    /// The current page number.
    pub fn page(&self) -> usize {
        self.session.as_ref().map_or(0, Session::page)
    }

    /// Total pages, or zero when nothing is open.
    pub fn page_count(&self) -> usize {
        self.session.as_ref().map_or(0, Session::page_count)
    }

    /// Whether there are unsaved edits, for a "leaving this page?" prompt.
    pub fn is_dirty(&self) -> bool {
        self.session.as_ref().is_some_and(Session::is_dirty)
    }

    /// Serialise to ADF for the host to download or share.
    pub fn save(&mut self) -> Vec<u8> {
        self.session
            .as_mut()
            .and_then(|session| session.save().ok())
            .unwrap_or_default()
    }

    /// What this build can do, for an agent driving the page.
    pub fn capabilities(&self) -> String {
        crate::ontology::capabilities().to_string()
    }
}

impl Default for WebReader {
    fn default() -> WebReader {
        WebReader::new()
    }
}
