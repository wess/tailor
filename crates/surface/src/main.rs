//! Regenerates `libraries/<name>.surface` — what a target library ships.
//!
//! Tailor used to answer "did guise gain a component the catalog does not
//! offer?" by reading the next crate over in the same workspace. Out on its
//! own that crate is a pinned crates.io dependency, so the question is asked
//! of the *published* source instead: cargo already unpacks it, `cargo
//! metadata` says where, and the answer is written here as a file you can
//! read in a diff.
//!
//! The file is the ratchet's source of truth. `catalog::coverage` and
//! `project::presets_match_guise` read it rather than a sibling checkout, so
//! bumping the library is two deliberate steps: change the version, rerun
//! this. Skip the second and the version line no longer matches the lockfile
//! and the tests say so.
//!
//! Usage:
//!   cargo run -p tailor-surface                       # guise, the pinned crates.io source
//!   cargo run -p tailor-surface -- <path>             # guise, a checkout, for a dry run
//!   cargo run -p tailor-surface -- gpuikit            # gpui-component, downloaded at its pin
//!   cargo run -p tailor-surface -- gpuikit <path>     # gpui-component, a checkout
//!
//! guise is a dependency of this workspace, so cargo already unpacked it.
//! gpui-kit is not, and cannot be: it builds on a different gpui snapshot than
//! Tailor does, and putting it in `Cargo.lock` would drag that snapshot in
//! beside ours. Its source is fetched from crates.io by exact version instead
//! — the pin lives here, next to the file it regenerates.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A library the tool can describe: where its components are, and what to
/// call the file.
struct Target {
  file: &'static str,
  krate: &'static str,
  presets: bool,
}

const GUISE: Target = Target {
  file: "guise.surface",
  krate: "guise-ui",
  presets: true,
};

/// gpui-kit's styled components live in `gpui-component`; `gpui-kit` itself is
/// a facade over it. The version is exact because a surface is a record of one
/// release, and bumping it is a deliberate act — the same two steps as guise.
const GPUIKIT: Target = Target {
  file: "gpuikit.surface",
  krate: "gpui-component",
  presets: false,
};
const GPUIKIT_VERSION: &str = "0.7.0";

fn main() {
  let mut args: Vec<String> = std::env::args().skip(1).collect();
  let target = if args.first().is_some_and(|a| a == "gpuikit") {
    args.remove(0);
    GPUIKIT
  } else {
    GUISE
  };
  let krate = target.krate;
  let arg = args.into_iter().next();
  let (src, version) = match arg {
    Some(path) => {
      let root = PathBuf::from(&path);
      let src = if root.join("src").is_dir() {
        root.join("src")
      } else {
        root
      };
      (src, "(checkout)".to_string())
    }
    None if target.krate == GPUIKIT.krate => fetch(GPUIKIT.krate, GPUIKIT_VERSION),
    None => locate(krate),
  };

  eprintln!("[surface] reading {}", src.display());
  let components = components(&src);
  let presets = if target.presets {
    presets(&src)
  } else {
    Vec::new()
  };
  // guise defines a hundred and some; gpui-component about half that.
  let floor = if target.presets { 100 } else { 40 };
  assert!(
    components.len() > floor,
    "found only {} components — did the layout move?",
    components.len()
  );
  assert!(
    !target.presets || !presets.is_empty(),
    "found no theme presets"
  );

  let how = if target.presets {
    "Regenerate with `cargo run -p tailor-surface`, which reads the version\n\
     # pinned in Cargo.lock. Do that in the same commit as a version bump: the\n\
     # `version` line below is checked against the lockfile, so a stale file\n\
     # fails the tests rather than going unnoticed."
  } else {
    "Regenerate with `cargo run -p tailor-surface -- gpuikit`, which fetches the\n\
     # version pinned in the tool. Do that in the same commit as a version bump:\n\
     # the `version` line below is checked against what the provider asks for,\n\
     # so a stale file fails the tests rather than going unnoticed."
  };
  let mut out = String::new();
  out.push_str(&format!(
    "# {krate} {version} — the component surface Tailor catalogues against.\n\
     #\n\
     # {how}\n\
     #\n\
     # component <Type> <file>            a RenderOnce builder or a Render entity\n\
     # preset    <id> <constructor> <scheme>\n\n"
  ));
  out.push_str(&format!("version {version}\n\n"));
  for (name, where_) in &components {
    out.push_str(&format!("component {name} {where_}\n"));
  }
  out.push('\n');
  for (id, ctor, scheme) in &presets {
    out.push_str(&format!("preset {id} {ctor} {scheme}\n"));
  }

  let dest = Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../libraries")
    .join(target.file);
  std::fs::create_dir_all(dest.parent().unwrap()).expect("create libraries/");
  std::fs::write(&dest, out).expect("write surface");
  eprintln!(
    "[surface] {} components, {} presets -> {}",
    components.len(),
    presets.len(),
    dest.display()
  );
}

/// The unpacked source of the pinned dependency, and the version it is.
///
/// `cargo metadata` is how you ask Cargo where a dependency actually lives
/// without guessing at the registry's directory layout.
fn locate(krate: &str) -> (PathBuf, String) {
  let out = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
    .args(["metadata", "--format-version", "1", "--locked"])
    .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
    .output()
    .expect("run cargo metadata");
  assert!(
    out.status.success(),
    "cargo metadata failed: {}",
    String::from_utf8_lossy(&out.stderr)
  );

  let meta: serde_json::Value = serde_json::from_slice(&out.stdout).expect("parse cargo metadata");
  let package = meta["packages"]
    .as_array()
    .expect("packages")
    .iter()
    .find(|p| p["name"] == krate)
    .unwrap_or_else(|| panic!("{krate} is not in the dependency graph"));

  let manifest = PathBuf::from(package["manifest_path"].as_str().expect("manifest_path"));
  let version = package["version"].as_str().expect("version").to_string();
  (manifest.parent().expect("crate dir").join("src"), version)
}

/// A crate's source at an exact version, straight from the registry's tarball.
///
/// Not `cargo metadata`: that would need the crate in this workspace's
/// lockfile, and its whole dependency tree with it. Cached under the target
/// directory so a rerun is offline.
fn fetch(krate: &str, version: &str) -> (PathBuf, String) {
  let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/surface");
  let dir = root.join(format!("{krate}-{version}"));
  if !dir.join("src").is_dir() {
    std::fs::create_dir_all(&root).expect("create target/surface");
    let url = format!("https://static.crates.io/crates/{krate}/{krate}-{version}.crate");
    eprintln!("[surface] fetching {url}");
    let tarball = root.join(format!("{krate}-{version}.crate"));
    let status = Command::new("curl")
      .args(["-fsSL", "-o"])
      .arg(&tarball)
      .arg(&url)
      .status()
      .expect("run curl");
    assert!(status.success(), "could not download {url}");
    let status = Command::new("tar")
      .arg("-xzf")
      .arg(&tarball)
      .arg("-C")
      .arg(&root)
      .status()
      .expect("run tar");
    assert!(status.success(), "could not unpack {}", tarball.display());
  }
  (dir.join("src"), version.to_string())
}

/// Every component type the library defines, by name and the file it is in.
///
/// The two component patterns and nothing else: a `RenderOnce` builder
/// (`derive(.. IntoElement ..)`) or a stateful entity (`impl Render for`).
/// That is exactly the set a catalog can offer.
fn components(src: &Path) -> BTreeMap<String, String> {
  let mut files = Vec::new();
  rust_files(src, &mut files);

  let mut found = BTreeMap::new();
  for path in files {
    let source = std::fs::read_to_string(&path).expect("read source");
    let where_ = path
      .strip_prefix(src)
      .unwrap_or(&path)
      .display()
      .to_string();
    for name in derived(&source).chain(rendered(&source)) {
      // Test harnesses that live beside the component they exercise.
      if name.ends_with("Harness") || name.ends_with("Test") || name.ends_with("Tests") {
        continue;
      }
      found.insert(name, where_.clone());
    }
  }
  found
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
  let entries = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
  for entry in entries.flatten() {
    let path = entry.path();
    if path.is_dir() {
      rust_files(&path, out);
    } else if path.extension().is_some_and(|ext| ext == "rs") {
      // The entity test harness declares throwaway components of its own.
      if path.file_name().is_some_and(|name| name != "apptests.rs") {
        out.push(path);
      }
    }
  }
}

/// `#[derive(.., IntoElement)] pub struct Button` -> `Button`.
fn derived(source: &str) -> impl Iterator<Item = String> + '_ {
  source.match_indices("#[derive(").filter_map(|(at, _)| {
    let rest = &source[at..];
    let close = rest.find(")]")?;
    if !rest[..close].contains("IntoElement") {
      return None;
    }
    type_name_after(&rest[close + 2..])
  })
}

/// `impl Render for Select` -> `Select`.
fn rendered(source: &str) -> impl Iterator<Item = String> + '_ {
  source
    .match_indices("impl Render for ")
    .filter_map(|(at, keyword)| identifier(&source[at + keyword.len()..]))
}

/// The `struct`/`enum` name that follows, skipping any further attributes.
fn type_name_after(rest: &str) -> Option<String> {
  for line in rest.lines().take(6) {
    let line = line.trim();
    for keyword in ["pub struct ", "pub enum ", "struct ", "enum "] {
      if let Some(tail) = line.strip_prefix(keyword) {
        return identifier(tail);
      }
    }
  }
  None
}

fn identifier(text: &str) -> Option<String> {
  let name: String = text
    .trim_start()
    .chars()
    .take_while(|c| c.is_alphanumeric() || *c == '_')
    .collect();
  name
    .chars()
    .next()
    .is_some_and(|c| c.is_ascii_uppercase())
    .then_some(name)
}

/// The prebuilt themes, as `(id, constructor, scheme)`.
///
/// The id is what a `.tailor` file stores; the constructor is what the
/// generator prints, and the two differ (`solarizedlight` is
/// `Theme::solarized_light()`), so it is read out of the lookup rather than
/// derived. The scheme comes from which `Theme::dark()`/`light()` the
/// constructor builds on.
fn presets(src: &Path) -> Vec<(String, String, String)> {
  let source = std::fs::read_to_string(src.join("theme/presets.rs")).expect("read presets.rs");

  // `pub const PRESET_NAMES: [&str; 6] = [ "catppuccin", .. ];`
  let start = source
    .find("PRESET_NAMES")
    .expect("the library names its presets");
  let list = &source[start..];
  let open = list.find('[').expect("a list");
  let open = open + 1 + list[open + 1..].find('[').expect("the values");
  let close = list[open..].find(']').expect("a closed list") + open;

  list[open + 1..close]
    .split(',')
    .map(|piece| piece.trim().trim_matches('"'))
    .filter(|piece| !piece.is_empty())
    .map(|id| {
      // `"solarizedlight" => Some(Theme::solarized_light()),`
      let arm = source
        .find(&format!("\"{id}\" => Some(Theme::"))
        .unwrap_or_else(|| panic!("{id} is named but not looked up"));
      let ctor = identifier_snake(&source[arm..]).expect("a constructor");

      let at = source
        .find(&format!("pub fn {ctor}() -> Theme {{"))
        .unwrap_or_else(|| panic!("no {ctor}() to read a scheme from"));
      let body = &source[at..(at + 80).min(source.len())];
      let scheme = if body.contains("Theme::light()") {
        "light"
      } else {
        "dark"
      };
      (id.to_string(), ctor, scheme.to_string())
    })
    .collect()
}

/// The function name in `.. => Some(Theme::solarized_light()),`.
fn identifier_snake(text: &str) -> Option<String> {
  let at = text.find("Theme::")? + "Theme::".len();
  let name: String = text[at..]
    .chars()
    .take_while(|c| c.is_alphanumeric() || *c == '_')
    .collect();
  (!name.is_empty()).then_some(name)
}
