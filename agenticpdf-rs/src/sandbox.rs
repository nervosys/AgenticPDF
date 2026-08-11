// SPDX-License-Identifier: AGPL-3.0-or-later
//! Path confinement for agent-driven file access.
//!
//! The CLI is run by a person who already has a shell: `apdf text /etc/passwd`
//! grants nothing that `cat` does not, so there is no boundary to enforce
//! there. The MCP server is different. There the *model* chooses the path, and
//! a document can carry text arguing for a particular one. That is a confused
//! deputy: the server holds its operator's privileges and takes instructions
//! from untrusted content.
//!
//! So MCP file access is confined to a set of roots. The default is the
//! process's working directory — the directory the operator chose to serve
//! from. `APDF_MCP_ROOTS` overrides it with a platform-separated list.
//! Setting it to `*` restores unrestricted access for operators who genuinely
//! want it and understand what they are turning off.
//!
//! Confinement is by canonicalization, not string matching. `canonicalize`
//! resolves `..` and follows symlinks before the prefix test, so neither
//! `../../etc/passwd` nor a symlink planted inside a root can escape. A
//! textual check would be fooled by both.

use std::path::{Path, PathBuf};

/// Environment variable naming the roots MCP may touch.
pub const ROOTS_VAR: &str = "APDF_MCP_ROOTS";

/// Value of [`ROOTS_VAR`] that disables confinement entirely.
pub const UNRESTRICTED: &str = "*";

/// Which paths an agent-facing entry point may read and write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sandbox {
    /// Confined to these already-canonical roots.
    Roots(Vec<PathBuf>),
    /// No confinement. Only ever set deliberately by an operator.
    Unrestricted,
}

impl Sandbox {
    /// Read the policy from the environment, defaulting to the working
    /// directory. A root that cannot be canonicalized (typo, deleted) is
    /// dropped: keeping it would be a root that matches nothing, and silently
    /// widening to "everything" because a root was misspelled is the wrong
    /// failure direction.
    pub fn from_env() -> Sandbox {
        match std::env::var(ROOTS_VAR) {
            Ok(raw) if raw.trim() == UNRESTRICTED => Sandbox::Unrestricted,
            Ok(raw) if !raw.trim().is_empty() => {
                let roots: Vec<PathBuf> = std::env::split_paths(&raw)
                    .filter(|p| !p.as_os_str().is_empty())
                    .filter_map(|p| p.canonicalize().ok())
                    .collect();
                Sandbox::Roots(roots)
            }
            _ => Sandbox::Roots(
                std::env::current_dir()
                    .and_then(|d| d.canonicalize())
                    .map(|d| vec![d])
                    .unwrap_or_default(),
            ),
        }
    }

    /// Confine to one root. Mainly for tests and embedders.
    pub fn with_root(root: &Path) -> Sandbox {
        Sandbox::Roots(root.canonicalize().into_iter().collect())
    }

    /// Resolve a path for reading. The file must exist and lie inside a root.
    pub fn resolve_read(&self, path: &str) -> Result<PathBuf, String> {
        let resolved = Path::new(path)
            .canonicalize()
            .map_err(|e| format!("cannot read {path}: {e}"))?;
        self.check(&resolved, path)?;
        Ok(resolved)
    }

    /// Resolve a path for writing. The file itself need not exist, so the
    /// *parent* is canonicalized and the file name appended. A path whose
    /// parent does not exist is rejected rather than created: an agent
    /// conjuring directory trees is not a capability this needs.
    pub fn resolve_write(&self, path: &str) -> Result<PathBuf, String> {
        let candidate = Path::new(path);
        let name = candidate
            .file_name()
            .ok_or_else(|| format!("not a file path: {path}"))?;
        let parent = match candidate.parent() {
            Some(p) if !p.as_os_str().is_empty() => p,
            // A bare file name means the working directory.
            _ => Path::new("."),
        };
        let parent = parent
            .canonicalize()
            .map_err(|e| format!("cannot write {path}: {e}"))?;

        // Check the parent, then join. Canonicalizing after the join would
        // fail for a file that does not exist yet.
        self.check(&parent, path)?;
        let resolved = parent.join(name);

        // A pre-existing target may itself be a symlink pointing outside the
        // root; writing through it would escape. Re-check when it exists.
        if let Ok(existing) = resolved.canonicalize() {
            self.check(&existing, path)?;
        }
        Ok(resolved)
    }

    fn check(&self, resolved: &Path, original: &str) -> Result<(), String> {
        let roots = match self {
            Sandbox::Unrestricted => return Ok(()),
            Sandbox::Roots(roots) => roots,
        };
        if roots.iter().any(|root| resolved.starts_with(root)) {
            return Ok(());
        }
        Err(format!(
            "{original} is outside the permitted roots. The MCP server may \
             only touch {}. Set {ROOTS_VAR} to change this, or to `{UNRESTRICTED}` \
             to disable confinement.",
            describe(roots)
        ))
    }
}

fn describe(roots: &[PathBuf]) -> String {
    if roots.is_empty() {
        return "no directory (no readable root was configured)".to_string();
    }
    roots
        .iter()
        .map(|r| display_path(r))
        .collect::<Vec<_>>()
        .join(", ")
}

/// `canonicalize` on Windows returns a verbatim path (`\\?\C:\...`). Showing
/// that to an operator invites them to disable confinement out of confusion,
/// so strip the prefix for display. Comparison still uses the real path.
fn display_path(path: &Path) -> String {
    let shown = path.display().to_string();
    shown
        .strip_prefix(r"\\?\")
        .map(str::to_string)
        .unwrap_or(shown)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// A temp directory that cleans itself up, so the tests leave no residue.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> TempDir {
            // No wall clock: a counter keyed on the tag keeps parallel tests
            // from colliding without pulling in a dependency.
            let base =
                std::env::temp_dir().join(format!("apdf-sandbox-{tag}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&base);
            fs::create_dir_all(&base).expect("create temp dir");
            TempDir(base.canonicalize().expect("canonicalize temp dir"))
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn reads_inside_the_root_are_allowed() {
        let dir = TempDir::new("read-ok");
        let file = dir.path().join("doc.txt");
        fs::write(&file, b"hello").unwrap();

        let sandbox = Sandbox::with_root(dir.path());
        let resolved = sandbox.resolve_read(file.to_str().unwrap()).unwrap();
        assert_eq!(resolved, file.canonicalize().unwrap());
    }

    #[test]
    fn reads_outside_the_root_are_refused() {
        let inside = TempDir::new("read-inside");
        let outside = TempDir::new("read-outside");
        let secret = outside.path().join("secret.env");
        fs::write(&secret, b"AWS_SECRET_ACCESS_KEY=x").unwrap();

        let sandbox = Sandbox::with_root(inside.path());
        let err = sandbox.resolve_read(secret.to_str().unwrap()).unwrap_err();
        assert!(err.contains("outside the permitted roots"), "{err}");
    }

    #[test]
    fn dot_dot_traversal_cannot_escape() {
        let inside = TempDir::new("traverse-inside");
        let outside = TempDir::new("traverse-outside");
        let secret = outside.path().join("secret.env");
        fs::write(&secret, b"x").unwrap();

        // Spell the outside file as a relative walk up from inside the root.
        // A textual prefix check would see a path starting with the root and
        // wave it through; canonicalization is what makes this fail.
        let escape = inside
            .path()
            .join("..")
            .join(outside.path().file_name().unwrap())
            .join("secret.env");

        let sandbox = Sandbox::with_root(inside.path());
        let err = sandbox
            .resolve_read(escape.to_str().unwrap())
            .expect_err("traversal must be refused");
        assert!(err.contains("outside the permitted roots"), "{err}");
    }

    #[test]
    fn writes_may_create_a_new_file_inside_the_root() {
        let dir = TempDir::new("write-new");
        let target = dir.path().join("out.md");

        let sandbox = Sandbox::with_root(dir.path());
        let resolved = sandbox.resolve_write(target.to_str().unwrap()).unwrap();
        assert_eq!(resolved, target);
    }

    #[test]
    fn writes_outside_the_root_are_refused() {
        let inside = TempDir::new("write-inside");
        let outside = TempDir::new("write-outside");
        let victim = outside.path().join("victim.txt");
        fs::write(&victim, b"ORIGINAL").unwrap();

        let sandbox = Sandbox::with_root(inside.path());
        let err = sandbox.resolve_write(victim.to_str().unwrap()).unwrap_err();
        assert!(err.contains("outside the permitted roots"), "{err}");
        // The refusal must actually leave the file alone.
        assert_eq!(fs::read(&victim).unwrap(), b"ORIGINAL");
    }

    #[test]
    fn writing_into_a_missing_directory_is_refused() {
        let dir = TempDir::new("write-missing");
        let target = dir.path().join("nope").join("out.md");

        let sandbox = Sandbox::with_root(dir.path());
        assert!(sandbox.resolve_write(target.to_str().unwrap()).is_err());
    }

    #[test]
    fn unrestricted_allows_anything() {
        let inside = TempDir::new("unres-inside");
        let outside = TempDir::new("unres-outside");
        let file = outside.path().join("f.txt");
        fs::write(&file, b"x").unwrap();

        let _ = inside;
        let sandbox = Sandbox::Unrestricted;
        assert!(sandbox.resolve_read(file.to_str().unwrap()).is_ok());
    }

    #[test]
    fn a_root_that_does_not_exist_does_not_widen_access() {
        let outside = TempDir::new("bad-root-outside");
        let file = outside.path().join("f.txt");
        fs::write(&file, b"x").unwrap();

        // A misspelled root drops out, leaving no roots. That must deny, not
        // allow: failing open here would turn a typo into full access.
        let sandbox = Sandbox::Roots(Vec::new());
        assert!(sandbox.resolve_read(file.to_str().unwrap()).is_err());
    }

    #[test]
    fn a_symlink_inside_the_root_cannot_point_out_of_it() {
        // Symlink creation needs privilege on Windows, so only assert where
        // the link can actually be made. Skipping beats a test that passes
        // because nothing happened.
        let inside = TempDir::new("link-inside");
        let outside = TempDir::new("link-outside");
        let secret = outside.path().join("secret.env");
        fs::write(&secret, b"x").unwrap();
        let link = inside.path().join("link.env");

        #[cfg(unix)]
        let made = std::os::unix::fs::symlink(&secret, &link).is_ok();
        #[cfg(windows)]
        let made = std::os::windows::fs::symlink_file(&secret, &link).is_ok();
        #[cfg(not(any(unix, windows)))]
        let made = false;

        if !made {
            eprintln!("skipping: symlink creation not permitted here");
            return;
        }

        let sandbox = Sandbox::with_root(inside.path());
        let err = sandbox
            .resolve_read(link.to_str().unwrap())
            .expect_err("a symlink out of the root must be refused");
        assert!(err.contains("outside the permitted roots"), "{err}");
    }
}
