// SPDX-License-Identifier: AGPL-3.0-or-later
//! The iOS shell's Rust side.
//!
//! Structurally the same as Android and the web: the core opens, searches,
//! edits and *paints into a recording*, and the platform replays it — here onto
//! a `CGContext`. Three shells, one [`paint_page`](crate::canvas::paint_page).
//!
//! Swift calls C directly, so this is a plain `extern "C"` surface rather than
//! anything like JNI. That means ownership has to be stated explicitly: every
//! function returning a string hands over a buffer the caller must return via
//! [`apdf_string_free`], and `Reader.swift` wraps each one so a caller cannot
//! forget. (This is the pattern Android must *not* use — Kotlin's `external
//! fun` needs JNI mangling — which is worth remembering when reading the two
//! modules side by side.)
//!
//! Every entry point is `catch_unwind`-guarded: unwinding across an FFI
//! boundary is undefined behaviour, and a malformed document is exactly the
//! input that could provoke a panic.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Mutex;

use crate::actions;
use crate::canvas::{RecordingPainter, Transform};
use crate::session::Session;

/// The open document. One per process, behind a mutex — UIKit calls in from the
/// main thread and background queues alike.
static SESSION: Mutex<Option<Session>> = Mutex::new(None);

/// Hand a string to the caller, who must free it with [`apdf_string_free`].
fn into_c(value: String) -> *mut c_char {
    CString::new(value)
        .unwrap_or_else(|_| CString::new(r#"{"error":"result contained a NUL byte"}"#).unwrap())
        .into_raw()
}

fn failure(message: &str) -> *mut c_char {
    into_c(serde_json::json!({ "error": message }).to_string())
}

/// Open a document from bytes.
///
/// Bytes, not a path: iOS hands back security-scoped URLs from the document
/// picker, which only Swift can resolve.
///
/// # Safety
/// `data` must point to at least `len` readable bytes, or be null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn apdf_open(data: *const u8, len: usize) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        if data.is_null() {
            return failure("no data supplied");
        }
        let bytes = unsafe { std::slice::from_raw_parts(data, len) }.to_vec();

        match Session::open(bytes) {
            Ok(session) => {
                let summary = serde_json::json!({
                    "ok": true,
                    "title": session.title(),
                    "format": session.format().id(),
                    "pages": session.page_count(),
                })
                .to_string();
                if let Ok(mut guard) = SESSION.lock() {
                    *guard = Some(session);
                }
                into_c(summary)
            }
            Err(why) => {
                into_c(serde_json::json!({ "ok": false, "error": why.to_string() }).to_string())
            }
        }
    }))
    .unwrap_or_else(|_| failure("panicked while opening the document"))
}

/// Run an action — the same names the other shells and any agent use.
///
/// # Safety
/// Both arguments must be null or valid NUL-terminated UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn apdf_execute(action: *const c_char, params: *const c_char) -> *mut c_char {
    catch_unwind(AssertUnwindSafe(|| {
        if action.is_null() {
            return failure("action must not be null");
        }
        let Ok(action) = (unsafe { CStr::from_ptr(action) }).to_str() else {
            return failure("action must be UTF-8");
        };
        let params: serde_json::Value = if params.is_null() {
            serde_json::json!({})
        } else {
            unsafe { CStr::from_ptr(params) }
                .to_str()
                .ok()
                .and_then(|text| serde_json::from_str(text).ok())
                .unwrap_or_else(|| serde_json::json!({}))
        };

        let Ok(mut guard) = SESSION.lock() else {
            return failure("the document lock is poisoned");
        };
        into_c(actions::execute(&mut guard, action, &params).to_string())
    }))
    .unwrap_or_else(|_| failure("panicked while executing the action"))
}

/// Paint the current page into a recording for the host to replay.
#[unsafe(no_mangle)]
pub extern "C" fn apdf_render_page(width: f32, height: f32, zoom: f32) -> *mut c_char {
    catch_unwind(|| {
        let Ok(guard) = SESSION.lock() else {
            return failure("the document lock is poisoned");
        };
        let Some(session) = guard.as_ref() else {
            return into_c("[]".to_string());
        };
        let Ok(list) = session.display_list() else {
            // Not an error: some formats carry no geometry, and the host should
            // say so rather than present a blank page.
            return into_c(r#"[{"op":"no_geometry"}]"#.to_string());
        };

        let area = dewey::core::Rect::new(0.0, 0.0, width, height);
        let mut painter = RecordingPainter::new();
        crate::canvas::paint_page(
            &mut painter,
            &list,
            Transform::fit(&list, area, zoom),
            &[],
            session.fonts(),
        );
        into_c(painter.to_json())
    })
    .unwrap_or_else(|_| failure("panicked while rendering"))
}

/// Serialise to ADF.
///
/// Two-call protocol: pass a null `out` to learn the length, allocate, call
/// again. Returning an owned buffer would need a second free function and a
/// lifetime rule for Swift to get wrong.
///
/// # Safety
/// `out` must be null, or point to at least `capacity` writable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn apdf_save(out: *mut u8, capacity: usize) -> usize {
    catch_unwind(AssertUnwindSafe(|| {
        let Ok(mut guard) = SESSION.lock() else {
            return 0;
        };
        let Some(session) = guard.as_mut() else {
            return 0;
        };
        let Ok(bytes) = session.save() else {
            return 0;
        };

        if !out.is_null() && capacity >= bytes.len() {
            unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, bytes.len()) };
        }
        bytes.len()
    }))
    .unwrap_or(0)
}

/// Whether there are unsaved edits, for a dismissal prompt.
#[unsafe(no_mangle)]
pub extern "C" fn apdf_is_dirty() -> bool {
    SESSION
        .lock()
        .map(|guard| guard.as_ref().is_some_and(Session::is_dirty))
        .unwrap_or(false)
}

/// What this build can do, for an agent driving the app.
#[unsafe(no_mangle)]
pub extern "C" fn apdf_capabilities() -> *mut c_char {
    catch_unwind(|| into_c(crate::ontology::capabilities().to_string()))
        .unwrap_or_else(|_| failure("panicked while describing capabilities"))
}

/// Free a string returned by any function above.
///
/// # Safety
/// `pointer` must have come from this module and must not be used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn apdf_string_free(pointer: *mut c_char) {
    if !pointer.is_null() {
        drop(unsafe { CString::from_raw(pointer) });
    }
}
