// SPDX-License-Identifier: AGPL-3.0-or-later
//! `idoc` — the short name for the `irondoc` CLI.
//!
//! Cargo needs a distinct entry point per binary, so this one delegates to the
//! same `main` rather than duplicating the command surface.

#[path = "main.rs"]
mod cli;

fn main() {
    cli::main();
}
