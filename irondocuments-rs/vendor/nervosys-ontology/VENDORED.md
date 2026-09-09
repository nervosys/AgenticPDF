# nervosys-ontology, vendored

The company capability vocabulary, copied in rather than depended on across
repositories.

| | |
| --- | --- |
| **Origin** | `https://github.com/nervosys/Ontology`, crate root |
| **Standard** | NS-ONTO-001, revision **2026-09-09** |
| **Source commit** | `0bfc51f` on branch `ns-onto-001-multi-surface` |
| **Copied** | `src/`, `Cargo.toml` |
| **Not copied** | `hub/`, `dashboard/`, `docs/`, `fixtures/`, `shapes/`, `spec/` |

## Why a copy

NS-ONTO-001 §7.1 requires a project to be able to evaluate its normative
obligations *without network access*, and describes the delivery mechanism as
re-vendoring: a newer copy arrives carrying new clauses, which show up here as
failing tests in `tests/ontology.rs`. That is the earliest this project can
learn of them without watching the hub, and it is why the copy is the design
rather than a workaround.

The alternative — a path dependency on a sibling checkout — was tried and
removed. This repository's CI is a single `actions/checkout`, so a path
dependency outside it builds on a developer's machine and nowhere else.

`hub/` is deliberately absent. It is the company-side runner that reads what
projects emit; a project never reads the hub, and §7 makes that a visibility
boundary rather than a convenience.

## The source branch caveat

The amendment this copy carries — `Invocation`, `Capability::primary_surface`,
`Route::surface`, and the two §7.4 clauses — is committed and pushed, but on
`ns-onto-001-multi-surface` rather than on `master`. Until that branch merges,
this copy is *ahead* of `master`, and re-vendoring from a `master` checkout
would move it backwards and stop this repository compiling.

Re-vendor from `master` only once the branch has merged.

## Re-vendoring

```bash
ONTOLOGY=../../../../Ontology          # relative to this directory
rm -rf src Cargo.toml
cp -r "$ONTOLOGY/src" src
# strip the [workspace] table; the hub is not vendored
python -c "s=open('$ONTOLOGY/Cargo.toml').read(); open('Cargo.toml','w').write(s[s.index('[package]'):])"
cargo test --test ontology                # new clauses arrive as failures here
```

Then read the failures: each one is a normative obligation this project has not
yet met, citing the section that requires it.

## Currency

NS-ONTO-001 §7.2 has currency checked from both ends, and neither end reads the
company datastore. This copy states the revision it holds, above and in
`spec::REVISION`. The other half — comparing against the published descriptor
at `/spec/ns-onto-001.json` — needs a reachable publisher and is not wired up
here; a copy cannot tell on its own that a newer one exists.
