//! Where a project turns into a crate on disk.
//!
//! Running a design means compiling it, and compiling it means the generated
//! code has to be somewhere `cargo` can see. Xcode solved this with DerivedData
//! — a managed directory you never name and rarely look at — and Run should
//! work the same way: press it on a project you have never exported and it
//! builds, with nothing to configure first.
//!
//! So there are two kinds of workspace, and which one you get is decided by
//! whether you have said where the code goes:
//!
//! * **Yours**, when the project has an export directory. Run builds the same
//!   tree *Export* writes, because a build directory that shadowed your export
//!   would let the two drift and you would debug the wrong file.
//! * **Managed**, otherwise: one directory per project under Tailor's own
//!   config dir, named after the project and keyed by its path, so two projects
//!   called `Demo` in different folders do not fight over one `target/`.
//!
//! Keeping the managed one *out* of the user's source tree is deliberate, and
//! the same call the editor bridge made: generated code stays code, with no
//! build artifacts and no absolute local paths appearing next to it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use tailor_codegen::Generated;
use tailor_model::{NodeId, Project};
use tailor_store::ExportReport;

/// A directory a project's generated crate lives in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
  pub root: PathBuf,
  /// True when Tailor chose the directory, so the UI can say *where* a build
  /// went without implying the user picked it.
  pub managed: bool,
}

impl Workspace {
  /// The workspace for a project: its export directory, or a managed one.
  ///
  /// `path` is the `.tailor` file, when it has been saved. An unsaved project
  /// keys on its name alone — the alternative is refusing to run something you
  /// have not named yet, and the first thing anyone does is press Run.
  pub fn resolve(path: Option<&Path>, project: &Project) -> Workspace {
    if let Some(dir) = project.gen.export_dir.as_deref().filter(|d| !d.is_empty()) {
      return Workspace {
        root: PathBuf::from(dir),
        managed: false,
      };
    }
    Workspace {
      root: managed_root().join(slug(path, project)),
      managed: true,
    }
  }

  /// Write the project's crate into the workspace.
  ///
  /// The same `export` the Export command runs, so Run cannot compile
  /// something the user has never been shown.
  pub fn sync(&self, project: &Project) -> ExportReport {
    tailor_store::export(&self.root, project)
  }

  /// The manifest cargo will be pointed at.
  pub fn manifest(&self) -> PathBuf {
    self.root.join("Cargo.toml")
  }

  /// Where `target/` goes. Inside the workspace, which is the ordinary place —
  /// named here so the app can say how much a *Clean* would free.
  pub fn target_dir(&self) -> PathBuf {
    self.root.join("target")
  }

  /// Throw the build away. Returns whether there was one.
  pub fn clean(&self) -> bool {
    let target = self.target_dir();
    target.exists() && std::fs::remove_dir_all(&target).is_ok()
  }
}

/// Every managed workspace lives under here.
pub fn managed_root() -> PathBuf {
  tailor_store::config_dir().join("builds")
}

/// A stable, filesystem-safe directory name for a project.
///
/// The name is for a human reading the path; the hash is what makes it unique.
/// Hashing the *path* rather than the contents means the directory survives
/// every edit, which is the whole point — a rebuild should reuse `target/`.
fn slug(path: Option<&Path>, project: &Project) -> String {
  let name = tailor_model::snake_case(&project.name);
  let name = if name.is_empty() {
    "project".into()
  } else {
    name
  };
  let key = match path {
    Some(path) => path
      .canonicalize()
      .unwrap_or_else(|_| path.to_path_buf())
      .to_string_lossy()
      .to_string(),
    // An unsaved project has no identity beyond its name. Two of them would
    // share a directory; they would also be the same project as far as anyone
    // can tell, and the first save gives it a real key.
    None => format!("unsaved/{}", project.name),
  };
  format!("{name}-{:016x}", fnv1a(&key))
}

/// FNV-1a, 64-bit. Not `DefaultHasher`: that one is explicitly allowed to
/// change between Rust releases, and a toolchain upgrade that silently renamed
/// every build directory would throw away everyone's `target/`.
fn fnv1a(text: &str) -> u64 {
  let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
  for byte in text.as_bytes() {
    hash ^= *byte as u64;
    hash = hash.wrapping_mul(0x100_0000_01b3);
  }
  hash
}

/// Where every node ended up, by file: the map a compiler error is turned back
/// into a selection with.
///
/// Codegen already tags each node's expression with the line it landed on —
/// that is what *Open in Editor* reads. This is the same map, inverted and
/// keyed by path, because a diagnostic arrives as a file and a line.
pub type LineMap = BTreeMap<String, BTreeMap<usize, NodeId>>;

pub fn line_map(project: &Project) -> LineMap {
  let mut out = LineMap::new();
  for file in tailor_codegen::project_files(project) {
    let Generated { path, lines, .. } = file;
    if lines.is_empty() {
      continue;
    }
    let by_line = lines.into_iter().map(|(node, line)| (line, node)).collect();
    out.insert(path, by_line);
  }
  out
}

/// The node a line of generated code belongs to.
///
/// Nearest tag at or above the line, because a node's expression spans several
/// lines and only its first is tagged. An error on the third line of a
/// `Button::new(..)` chain is the button's.
pub fn node_at(map: &LineMap, file: &str, line: usize) -> Option<NodeId> {
  let by_line = map.get(file).or_else(|| {
    // Cargo reports paths relative to the workspace root; a diagnostic that
    // came in absolute still has to match.
    map
      .iter()
      .find(|(path, _)| file.ends_with(path.as_str()))
      .map(|(_, by_line)| by_line)
  })?;
  by_line.range(..=line).next_back().map(|(_, node)| *node)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn project() -> Project {
    tailor_guise::register();
    Project::new("My App")
  }

  #[test]
  fn an_export_directory_is_used_as_it_stands() {
    let mut project = project();
    project.gen.export_dir = Some("/tmp/somewhere".into());
    let workspace = Workspace::resolve(None, &project);
    assert_eq!(workspace.root, PathBuf::from("/tmp/somewhere"));
    assert!(!workspace.managed);
    assert_eq!(
      workspace.manifest(),
      PathBuf::from("/tmp/somewhere/Cargo.toml")
    );
  }

  #[test]
  fn without_one_the_workspace_is_managed_and_named_after_the_project() {
    let workspace = Workspace::resolve(None, &project());
    assert!(workspace.managed);
    assert!(workspace.root.starts_with(managed_root()));
    let name = workspace.root.file_name().unwrap().to_string_lossy();
    assert!(name.starts_with("my_app-"), "{name}");
  }

  #[test]
  fn the_directory_is_keyed_by_path_so_two_demos_do_not_collide() {
    let project = project();
    let one = Workspace::resolve(Some(Path::new("/a/demo.tailor")), &project);
    let two = Workspace::resolve(Some(Path::new("/b/demo.tailor")), &project);
    assert_ne!(one.root, two.root);

    // And it is stable: the same project resolves to the same place every
    // time, which is what lets a rebuild reuse `target/`.
    let again = Workspace::resolve(Some(Path::new("/a/demo.tailor")), &project);
    assert_eq!(one.root, again.root);
  }

  #[test]
  fn an_empty_export_directory_is_treated_as_unset() {
    let mut project = project();
    project.gen.export_dir = Some(String::new());
    assert!(Workspace::resolve(None, &project).managed);
  }

  #[test]
  fn a_line_maps_back_to_the_node_that_generated_it() {
    use tailor_model::node::DEFAULT_SLOT;

    let mut project = project();
    let doc = &mut project.docs[0];
    let root = doc.root;
    let button = doc.create("button");
    let id = doc.insert(root, DEFAULT_SLOT, 0, button);

    let map = line_map(&project);
    let (file, lines) = map
      .iter()
      .find(|(_, lines)| lines.values().any(|node| *node == id))
      .expect("the button is in some file");
    let line = *lines.iter().find(|(_, node)| **node == id).unwrap().0;

    assert_eq!(node_at(&map, file, line), Some(id));
    // A few lines further into the same expression is still the button.
    assert_eq!(node_at(&map, file, line + 1), Some(id));
    // Cargo can report an absolute path; it still has to match.
    let absolute = format!("/build/dir/{file}");
    assert_eq!(node_at(&map, &absolute, line), Some(id));
    // A file nothing generated has nothing to point at.
    assert_eq!(node_at(&map, "src/nowhere.rs", line), None);
  }

  #[test]
  fn the_hash_is_stable_across_runs() {
    // Pinned rather than merely self-consistent: this value is what keeps a
    // build directory attached to its project across a toolchain upgrade.
    assert_eq!(fnv1a(""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(fnv1a("a"), 0xaf63_dc4c_8601_ec8c);
  }
}
