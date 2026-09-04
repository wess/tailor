//! Reading guise's surface file — the ratchet under this whole crate.
//!
//! `libraries/guise.surface` records what the pinned guise ships: every
//! component type, and the theme presets. It is generated from the *published*
//! source of the dependency (`cargo run -p tailor-surface`), which is how the
//! ratchets survived Tailor moving out of guise's workspace — there is no
//! sibling checkout to read any more.
//!
//! [`guise`] also checks the file against `Cargo.lock`, so the two-step
//! "bump the dependency, regenerate the surface" cannot silently become one.
//!
//! A second provider wants the same thing, and gets it the same way: point
//! `tailor-surface` at the crate, check the result in, and read it here.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub struct Surface {
  /// Component type name -> the file in the library that defines it.
  pub components: BTreeMap<String, String>,
  /// `(id, constructor, scheme)` for each prebuilt theme.
  pub presets: Vec<(String, String, String)>,
}

fn workspace_root() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The surface of the guise version this workspace is pinned to.
pub fn guise() -> Surface {
  let path = workspace_root().join("libraries/guise.surface");
  let source = std::fs::read_to_string(&path).unwrap_or_else(|e| {
    panic!(
      "{}: {e}\nGenerate it with `cargo run -p tailor-surface`.",
      path.display()
    )
  });

  let mut version = None;
  let mut components = BTreeMap::new();
  let mut presets = Vec::new();

  for line in source.lines() {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
      continue;
    }
    let mut parts = line.split_whitespace();
    match parts.next() {
      Some("version") => version = parts.next().map(str::to_string),
      Some("component") => {
        let name = parts.next().expect("component name").to_string();
        let where_ = parts.next().unwrap_or("").to_string();
        components.insert(name, where_);
      }
      Some("preset") => presets.push((
        parts.next().expect("preset id").to_string(),
        parts.next().expect("preset constructor").to_string(),
        parts.next().expect("preset scheme").to_string(),
      )),
      other => panic!("unknown surface line: {other:?}"),
    }
  }

  let version = version.expect("the surface file names a version");
  let pinned = locked_version("guise-ui");
  assert_eq!(
    version, pinned,
    "libraries/guise.surface was generated from guise-ui {version} but the \
     lockfile pins {pinned} — rerun `cargo run -p tailor-surface`"
  );
  assert!(
    components.len() > 100,
    "surface lists only {} components — regenerate it",
    components.len()
  );

  Surface {
    components,
    presets,
  }
}

/// The guise version this workspace resolves — what the canvas renders with.
pub fn guise_version() -> String {
  locked_version("guise-ui")
}

/// The version of a package as `Cargo.lock` resolves it. The lockfile is
/// committed and CI builds `--locked`, so this is the version that will
/// actually be compiled against.
fn locked_version(package: &str) -> String {
  let path = workspace_root().join("Cargo.lock");
  let lock = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
  let needle = format!("name = \"{package}\"");
  let at = lock
    .find(&needle)
    .unwrap_or_else(|| panic!("{package} is not in Cargo.lock"));
  lock[at..]
    .lines()
    .find_map(|line| line.trim().strip_prefix("version = "))
    .map(|v| v.trim_matches('"').to_string())
    .unwrap_or_else(|| panic!("no version after {needle}"))
}

#[cfg(test)]
mod tests {
  use crate::theme::PRESETS;
  use tailor_model::project::Scheme;

  /// This crate describes guise without linking it, so [`PRESETS`] is a copy.
  /// This checks it against `libraries/guise.surface` — the record of what the
  /// pinned guise actually ships — and fails when the copy drifts. Same
  /// ratchet as `coverage`, same source.
  #[test]
  fn presets_match_guise() {
    let theirs = super::guise().presets;

    let ours: Vec<&str> = PRESETS.iter().map(|preset| preset.id).collect();
    let names: Vec<&str> = theirs.iter().map(|(id, _, _)| id.as_str()).collect();
    assert_eq!(
      ours, names,
      "PRESETS has drifted from guise's PRESET_NAMES — update the table"
    );

    // The constructor and the scheme, which are the halves a name list cannot
    // carry: the ids run together where the Rust names do not, and a preset is
    // a variation of one scheme or the other.
    for preset in PRESETS {
      let (_, ctor, scheme) = theirs
        .iter()
        .find(|(id, _, _)| id == preset.id)
        .expect("checked above");
      assert_eq!(
        &preset.rust, ctor,
        "{} generates Theme::{}() but guise defines Theme::{ctor}()",
        preset.id, preset.rust
      );
      let expected = match preset.scheme {
        Scheme::Dark => "dark",
        Scheme::Light => "light",
      };
      assert_eq!(
        scheme, expected,
        "{} is {:?} here but not in guise",
        preset.id, preset.scheme
      );
    }
  }
}
