//! Finding cargo from inside a bundled app.
//!
//! This module exists because of one difference that is easy to miss until it
//! bites: a `.app` launched from Finder does not inherit your shell's
//! environment. Its `PATH` is `/usr/bin:/bin:/usr/sbin:/sbin`, and `cargo` is
//! not in any of them — rustup installs to `~/.cargo/bin`. So Run works from a
//! terminal and fails from the Dock, with "No such file or directory", which
//! tells you nothing.
//!
//! So cargo is looked up rather than assumed, and the directory it was found in
//! is put back on the child's `PATH` — cargo shells out to `rustc` and `cc`,
//! and finding cargo is no use if cargo cannot find the compiler.

use std::path::{Path, PathBuf};

/// Where a toolchain might be, in the order worth trying.
///
/// `$CARGO` first: cargo sets it for anything it runs, so a Tailor started
/// with `cargo run` uses the same toolchain as the workspace around it.
fn candidates() -> Vec<PathBuf> {
  let mut out = Vec::new();
  if let Some(cargo) = std::env::var_os("CARGO") {
    out.push(PathBuf::from(cargo));
  }
  if let Some(home) = std::env::var_os("CARGO_HOME") {
    out.push(PathBuf::from(home).join("bin/cargo"));
  }
  if let Some(home) = dirs::home_dir() {
    out.push(home.join(".cargo/bin/cargo"));
  }
  // Homebrew and a system install, for a machine without rustup.
  out.push(PathBuf::from("/opt/homebrew/bin/cargo"));
  out.push(PathBuf::from("/usr/local/bin/cargo"));
  out.push(PathBuf::from("/usr/bin/cargo"));
  out
}

/// The cargo to run, or `None` when there is no toolchain to find.
///
/// Also searches `PATH`, which is what makes this work when the app *was*
/// launched from a shell.
pub fn cargo() -> Option<PathBuf> {
  candidates()
    .into_iter()
    .find(|path| is_program(path))
    .or_else(|| which("cargo"))
}

/// What to tell the user when there is none. One sentence and the fix.
pub const MISSING: &str =
  "cargo was not found. Install Rust from https://rustup.rs and reopen Tailor.";

/// The `PATH` a build should run with: the toolchain's own directory, then
/// whatever this process already had.
///
/// Prepended rather than appended, so a rustup shim wins over a stale system
/// cargo — which is the same order a shell would have given you.
pub fn path_with_toolchain(cargo: &Path) -> std::ffi::OsString {
  let existing = std::env::var_os("PATH").unwrap_or_default();
  let mut parts: Vec<PathBuf> = Vec::new();
  if let Some(dir) = cargo.parent() {
    parts.push(dir.to_path_buf());
  }
  // A rustup cargo lives beside rustc; a Homebrew one does not, and the
  // compiler it drives is on the ordinary path. Both cases are covered by
  // keeping what we had.
  parts.extend(std::env::split_paths(&existing));
  parts.dedup();
  std::env::join_paths(parts).unwrap_or(existing)
}

/// Whether the path is something we could execute.
fn is_program(path: &Path) -> bool {
  path.is_file() && executable(path)
}

#[cfg(unix)]
fn executable(path: &Path) -> bool {
  use std::os::unix::fs::PermissionsExt;
  std::fs::metadata(path)
    .map(|meta| meta.permissions().mode() & 0o111 != 0)
    .unwrap_or(false)
}

#[cfg(not(unix))]
fn executable(_path: &Path) -> bool {
  true
}

/// A `PATH` lookup, which is the one thing `Command` will not do for us when
/// we need to know the answer before spawning.
fn which(program: &str) -> Option<PathBuf> {
  let path = std::env::var_os("PATH")?;
  std::env::split_paths(&path)
    .map(|dir| dir.join(program))
    .find(|candidate| is_program(candidate))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_toolchain_is_found_on_a_machine_that_has_one() {
    // The test suite is being run by cargo, so there is one by definition.
    let cargo = cargo().expect("cargo runs these tests");
    assert!(cargo.is_file());
    assert_eq!(cargo.file_stem().unwrap(), "cargo");
  }

  #[test]
  fn the_toolchain_directory_leads_the_path() {
    let path = path_with_toolchain(Path::new("/opt/rust/bin/cargo"));
    let first = std::env::split_paths(&path).next().unwrap();
    assert_eq!(first, PathBuf::from("/opt/rust/bin"));
    // And what the process already had is still there — cargo needs `cc`.
    let all: Vec<PathBuf> = std::env::split_paths(&path).collect();
    assert!(all.len() > 1);
  }

  #[test]
  fn a_directory_is_not_a_program() {
    assert!(!is_program(Path::new("/tmp")));
    assert!(!is_program(Path::new("/nonesuch/cargo")));
  }
}
