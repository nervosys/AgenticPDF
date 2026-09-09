// SPDX-License-Identifier: AGPL-3.0-or-later
//! NS-ONTO-001 conformance: the declaration must describe the product that
//! actually exists.
//!
//! `tests/agent_surface.rs` already pins the `describe` ontology to the command
//! line. This goes the rest of the way, to the surface an agent actually
//! arrives through: every capability declared must be reachable on each surface
//! it claims, and every entry point the engine serves must be declared or named
//! in `unmapped`.
//!
//! Both directions matter, and they catch opposite defects. A declared
//! invocation nobody serves is a promise the engine does not keep, and a caller
//! learns of it by being refused. A served entry point nobody declared is
//! worse: the capability exists and no amount of reading the description
//! reveals it, so the agents that reach this engine through that door never
//! find it.

use std::collections::BTreeSet;

use nervosys_ontology::spec::{self, Route};
use nervosys_ontology::surface::SurfaceKind;

/// Every subcommand the CLI actually exposes, from its own `--help`.
///
/// Parsed from the running binary rather than listed here: a list written in
/// this file would be one more thing to keep in step, which is the defect the
/// test exists to catch.
///
/// `help` is deliberately kept. It is an entry point the product serves, so
/// NS-ONTO-001 §7.4 requires it to be accounted for — and the manifest accounts
/// for it by naming it in `unmapped`, which is the honest answer: it describes
/// the argument parser, not the engine.
fn cli_entry_points() -> BTreeSet<String> {
    let exe = env!("CARGO_BIN_EXE_irondoc");
    let output = std::process::Command::new(exe)
        .arg("--help")
        .output()
        .expect("run irondoc --help");
    let help = String::from_utf8_lossy(&output.stdout).to_string();

    let mut out = BTreeSet::new();
    let mut in_commands = false;
    for line in help.lines() {
        if line.starts_with("Commands:") {
            in_commands = true;
            continue;
        }
        if !in_commands {
            continue;
        }
        if line.starts_with("Options:") {
            break;
        }
        // A command line is indented exactly two spaces; its description
        // continuation is indented further.
        if !line.starts_with("  ") || line.starts_with("   ") {
            continue;
        }
        if let Some(name) = line.split_whitespace().next()
            && name.chars().all(|c| c.is_ascii_lowercase())
        {
            out.insert(name.to_string());
        }
    }
    assert!(
        out.len() > 20,
        "the help output stopped parsing; got {out:?}"
    );
    out
}

/// Every tool the MCP server actually offers, from the tool table itself.
fn mcp_entry_points() -> BTreeSet<String> {
    irondocuments::mcp::tool_defs()
        .iter()
        .filter_map(|tool| tool.get("name").and_then(|n| n.as_str()))
        .map(str::to_string)
        .collect()
}

fn served_routes() -> Vec<Route> {
    let mut routes = Vec::new();
    for name in cli_entry_points() {
        routes.push(Route::on(SurfaceKind::Cli, "", name));
    }
    for name in mcp_entry_points() {
        routes.push(Route::on(SurfaceKind::Machine, "", name));
    }
    routes
}

#[test]
fn the_declaration_conforms_to_ns_onto_001() {
    let manifest = irondocuments::ontology::manifest();
    let certification = spec::certify_manifest(&manifest, &served_routes());

    assert!(certification.conforms(), "{}", certification.report());
    // §2.1 — a run that checked nothing must not read as a pass.
    assert_eq!(certification.checked, spec::CLAUSES.len());
    assert_eq!(certification.revision, spec::REVISION);
}

/// The paired failure test §2.1 requires. A conformance check that cannot fail
/// is a false assurance, and it reads exactly like a passing one.
#[test]
fn an_entry_point_nobody_declared_is_caught() {
    let manifest = irondocuments::ontology::manifest();
    let mut routes = served_routes();
    routes.push(Route::on(SurfaceKind::Machine, "", "undeclared_tool"));

    let certification = spec::certify_manifest(&manifest, &routes);
    assert!(!certification.conforms());
    assert!(
        certification
            .findings
            .iter()
            .any(|f| f.clause == "served-entry-points-are-declared"),
        "{}",
        certification.report()
    );
}

/// The other direction: a capability that claims a door the engine does not
/// open.
#[test]
fn a_capability_unreachable_where_it_claims_is_caught() {
    let manifest = irondocuments::ontology::manifest();
    let cli_only: Vec<Route> = served_routes()
        .into_iter()
        .filter(|r| r.surface == SurfaceKind::Cli)
        .collect();

    let certification = spec::certify_manifest(&manifest, &cli_only);
    assert!(
        !certification.conforms(),
        "dropping every MCP tool should be caught"
    );
    assert!(
        certification
            .findings
            .iter()
            .any(|f| f.clause == "declared-paths-are-served"),
        "{}",
        certification.report()
    );
}

/// Every capability an agent can reach is one the manifest can speak about:
/// what it does to state, and whether an autonomous caller should.
#[test]
fn every_mcp_tool_has_a_declared_effect_and_agent_safety() {
    let manifest = irondocuments::ontology::manifest();
    for tool in mcp_entry_points() {
        let declared = manifest.capabilities.iter().find(|c| {
            c.all_invocations()
                .into_iter()
                .any(|(surface, _, path)| surface == SurfaceKind::Machine && path == tool)
        });
        assert!(
            declared.is_some(),
            "MCP tool `{tool}` is reachable and undeclared"
        );
    }
}
