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

/// A document open in the browser.
#[wasm_bindgen]
pub struct WebReader {
    session: Option<Session>,
}

#[wasm_bindgen]
impl WebReader {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WebReader {
        WebReader { session: None }
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
        let mut painter = RecordingPainter::new();
        crate::canvas::paint_page(&mut painter, &list, transform, &[], session.fonts());
        painter.to_json()
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
