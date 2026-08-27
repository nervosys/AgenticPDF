// SPDX-License-Identifier: AGPL-3.0-or-later
//! The Android shell.
//!
//! Structurally identical to the web shell: the Rust core opens, searches,
//! edits and *paints into a recording*, and a thin platform layer replays that
//! recording onto the native canvas. Both shells share
//! [`RecordingPainter`](crate::canvas::RecordingPainter) and the same action
//! vocabulary, so Android costs a JNI boundary rather than a second
//! application.
//!
//! Deliberately no `NativeActivity` and no Rust-side event loop. Android's own
//! `Canvas`, text layout, IME and accessibility are better than anything this
//! crate would reimplement, and going through them means the app gets system
//! text selection and screen-reader support for free. Rust owns the document;
//! Kotlin owns the window.
//!
//! Names follow JNI's mangling (`Java_<package>_<class>_<method>`, with `_`
//! escaped as `_1`), so Kotlin's `external fun` binds them with no registration
//! step. Every entry point is `catch_unwind`-guarded, because a panic unwinding
//! across the JNI boundary is undefined behaviour and a malformed document is
//! exactly the input that might cause one.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Mutex;

use jni::JNIEnv;
use jni::objects::{JByteArray, JClass, JString};
use jni::sys::{jboolean, jbyteArray, jfloat, jstring};

use crate::actions;
use crate::canvas::{RecordingPainter, Transform};
use crate::session::Session;

/// The open document.
///
/// One per process, behind a mutex: Android calls in from the UI thread and
/// from background threads, and a document being edited from two at once must
/// serialise rather than race.
static SESSION: Mutex<Option<Session>> = Mutex::new(None);

/// Build a Java string, falling back to a JSON error that the caller can parse.
///
/// Returning a null `jstring` would surface in Kotlin as a confusing NPE rather
/// than something the UI can display.
fn to_jstring(env: &JNIEnv, value: String) -> jstring {
    match env.new_string(value) {
        Ok(string) => string.into_raw(),
        Err(_) => env
            .new_string(r#"{"error":"could not allocate the result string"}"#)
            .map(|string| string.into_raw())
            .unwrap_or(std::ptr::null_mut()),
    }
}

fn error_json(env: &JNIEnv, message: &str) -> jstring {
    to_jstring(env, serde_json::json!({ "error": message }).to_string())
}

/// Open a document from bytes the host read.
///
/// Bytes rather than a path: Android hands out content URIs, which are not
/// paths and which only the Java side can resolve.
#[unsafe(no_mangle)]
pub extern "system" fn Java_ai_nervosys_apdf_Reader_nativeOpen(
    env: JNIEnv,
    _class: JClass,
    data: JByteArray,
) -> jstring {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let Ok(bytes) = env.convert_byte_array(&data) else {
            return error_json(&env, "could not read the supplied bytes");
        };

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
                to_jstring(&env, summary)
            }
            Err(why) => to_jstring(
                &env,
                serde_json::json!({ "ok": false, "error": why.to_string() }).to_string(),
            ),
        }
    }));
    result.unwrap_or(std::ptr::null_mut())
}

/// Run an action — the same names and parameters the web shell and any agent
/// use. One vocabulary across every platform.
#[unsafe(no_mangle)]
pub extern "system" fn Java_ai_nervosys_apdf_Reader_nativeExecute(
    mut env: JNIEnv,
    _class: JClass,
    action: JString,
    params: JString,
) -> jstring {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let Ok(action) = env.get_string(&action) else {
            return error_json(&env, "action must be a string");
        };
        let action: String = action.into();

        let params: serde_json::Value = env
            .get_string(&params)
            .ok()
            .and_then(|text| serde_json::from_str(&String::from(text)).ok())
            .unwrap_or_else(|| serde_json::json!({}));

        let Ok(mut guard) = SESSION.lock() else {
            return error_json(&env, "the document lock is poisoned");
        };
        to_jstring(
            &env,
            actions::execute(&mut guard, &action, &params).to_string(),
        )
    }));
    result.unwrap_or(std::ptr::null_mut())
}

/// Paint the current page and return the recording for the host to replay onto
/// an Android `Canvas`.
#[unsafe(no_mangle)]
pub extern "system" fn Java_ai_nervosys_apdf_Reader_nativeRenderPage(
    env: JNIEnv,
    _class: JClass,
    width: jfloat,
    height: jfloat,
    zoom: jfloat,
) -> jstring {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let Ok(guard) = SESSION.lock() else {
            return error_json(&env, "the document lock is poisoned");
        };
        let Some(session) = guard.as_ref() else {
            return to_jstring(&env, "[]".to_string());
        };
        let Ok(list) = session.display_list() else {
            // Not an error: some formats carry no geometry, and the host should
            // say so rather than show a blank page.
            return to_jstring(&env, r#"[{"op":"no_geometry"}]"#.to_string());
        };

        let area = dewey::core::Rect::new(0.0, 0.0, width, height);
        let mut painter = RecordingPainter::new();
        // The page's own pictures, decoded and sampled to what this surface can
        // show. Passing none, as this did, is not a blank slot: the painter
        // draws a grey frame where a texture is missing, so every photograph in
        // every document came out as an empty box on both mobile shells while
        // the browser -- the one host that passed them -- was fine.
        let scale = session.fonts().raster_scale();
        let budget = (width as f64 * height as f64 * scale * scale).max(1.0) as usize;
        let textures = session.textures(budget);
        crate::canvas::paint_page(
            &mut painter,
            &list,
            Transform::fit(&list, area, zoom),
            &textures,
            session.fonts(),
        );
        to_jstring(&env, painter.to_json())
    }));
    result.unwrap_or(std::ptr::null_mut())
}

/// Serialise to ADF, for the host to write through the Storage Access
/// Framework or hand to a share sheet.
#[unsafe(no_mangle)]
pub extern "system" fn Java_ai_nervosys_apdf_Reader_nativeSave(
    env: JNIEnv,
    _class: JClass,
) -> jbyteArray {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let bytes = SESSION
            .lock()
            .ok()
            .and_then(|mut guard| guard.as_mut().and_then(|session| session.save().ok()))
            .unwrap_or_default();

        env.byte_array_from_slice(&bytes)
            .map(|array| array.into_raw())
            .unwrap_or(std::ptr::null_mut())
    }));
    result.unwrap_or(std::ptr::null_mut())
}

/// Whether there are unsaved edits, for a back-press confirmation.
#[unsafe(no_mangle)]
pub extern "system" fn Java_ai_nervosys_apdf_Reader_nativeIsDirty(
    _env: JNIEnv,
    _class: JClass,
) -> jboolean {
    let dirty = SESSION
        .lock()
        .map(|guard| guard.as_ref().is_some_and(Session::is_dirty))
        .unwrap_or(false);
    jboolean::from(dirty)
}

/// What this build can do, for an agent driving the app.
#[unsafe(no_mangle)]
pub extern "system" fn Java_ai_nervosys_apdf_Reader_nativeCapabilities(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    catch_unwind(AssertUnwindSafe(|| {
        to_jstring(&env, crate::ontology::capabilities().to_string())
    }))
    .unwrap_or(std::ptr::null_mut())
}
