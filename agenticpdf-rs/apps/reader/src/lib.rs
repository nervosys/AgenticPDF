// SPDX-License-Identifier: AGPL-3.0-or-later
//! The portable core of the document reader.
//!
//! Everything here compiles for desktop, Android, iOS and the browser. The
//! platform shell — window creation, the event loop, the file picker — does
//! not, and lives in the binary beside this.
//!
//! The split is what makes "desktop and mobile" tractable rather than a rewrite
//! per platform: opening, searching, editing, saving, the agent actions and the
//! page painter are one implementation, and each platform supplies only a
//! window and a [`Painter`](dewey::paint::Painter). Two things forced the
//! boundary and are worth stating, because both are easy to reintroduce by
//! accident:
//!
//! - **No filesystem.** [`session::Session::open`] takes bytes. A browser has
//!   no paths, and Android's content URIs are not paths either. Reading a file
//!   is the shell's job.
//! - **No wall clock by default.** `SystemTime::now` panics on
//!   `wasm32-unknown-unknown`. Timestamps come from [`now_millis`], which is
//!   real on native and zero on wasm — and nothing depends on it for
//!   correctness, because the edit log orders by Lamport clock rather than by
//!   time.

pub mod actions;
pub mod canvas;
pub mod glyphs;
pub mod ontology;
pub mod session;

/// The browser and mobile-web shell.
#[cfg(target_arch = "wasm32")]
pub mod web;

/// The Android shell. Structurally the same as the web one: paint into a
/// recording, let the platform replay it.
#[cfg(target_os = "android")]
pub mod android;

/// The iOS shell, same shape again — a C ABI for Swift rather than JNI.
#[cfg(target_os = "ios")]
pub mod apple;

/// Milliseconds since the Unix epoch.
///
/// Advisory only: it is shown to a person deciding whether an edit is recent,
/// never used to order operations. On wasm it is zero rather than a panic,
/// which is why the log must not — and does not — depend on it.
#[cfg(not(target_arch = "wasm32"))]
pub fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

/// See the native implementation above.
#[cfg(target_arch = "wasm32")]
pub fn now_millis() -> u64 {
    0
}
