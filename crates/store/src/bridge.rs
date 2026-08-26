//! The editor bridge: going from a line of generated Rust back to the node
//! that made it.
//!
//! Tailor already jumps *out* — select a component, land on its line. This is
//! the other half, the one that makes the pair behave like a designer docked to
//! an editor rather than two apps that happen to share a folder.
//!
//! Two small pieces of state, both in Tailor's own config directory rather than
//! in your source tree — generated code stays code, with no dotfiles or
//! absolute local paths committed alongside it.
//!
//! * **The export index** answers "which project generated this file?" It is
//!   written whenever a project is exported.
//! * **The focus request** is how a command-line invocation reaches a window
//!   that is already open. Tailor polls it beside the file it already polls.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::paths;

fn index_file() -> PathBuf {
  paths::config_dir().join("exports.json")
}

fn focus_file() -> PathBuf {
  paths::config_dir().join("focus.json")
}

/// Which project last exported to which directory.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ExportIndex {
  /// Export directory -> the `.tailor` file that wrote it.
  #[serde(default)]
  pub entries: BTreeMap<String, PathBuf>,
}

impl ExportIndex {
  pub fn load() -> Self {
    std::fs::read_to_string(index_file())
      .ok()
      .and_then(|text| serde_json::from_str(&text).ok())
      .unwrap_or_default()
  }

  pub fn save(&self) {
    paths::ensure_config_dir();
    if let Ok(text) = serde_json::to_string_pretty(self) {
      let _ = std::fs::write(index_file(), text);
    }
  }

  /// Note that `project` exported to `directory`.
  pub fn record(directory: &Path, project: &Path) {
    let mut index = ExportIndex::load();
    index.entries.insert(
      directory.to_string_lossy().to_string(),
      project.to_path_buf(),
    );
    index.save();
  }

  /// Which project owns this generated file? The longest export directory
  /// that is a prefix of it wins — two projects exporting into nested
  /// directories should resolve to the inner one.
  pub fn project_for(&self, file: &Path) -> Option<PathBuf> {
    let file = file.to_string_lossy().to_string();
    self
      .entries
      .iter()
      .filter(|(directory, _)| file.starts_with(directory.as_str()))
      .max_by_key(|(directory, _)| directory.len())
      .map(|(_, project)| project.clone())
  }
}

/// A request for an open window to select something.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Focus {
  /// The `.tailor` file this is about; a window with a different one open
  /// leaves it alone.
  pub project: PathBuf,
  pub document: String,
  pub node: u32,
  /// Set on write, so a repeat of the same request still reads as new.
  pub stamp: u64,
}

impl Focus {
  pub fn write(project: &Path, document: &str, node: u32) -> Result<(), String> {
    paths::ensure_config_dir();
    let stamp = SystemTime::now()
      .duration_since(SystemTime::UNIX_EPOCH)
      .map(|since| since.as_millis() as u64)
      .unwrap_or_default();
    let focus = Focus {
      project: project.to_path_buf(),
      document: document.to_string(),
      node,
      stamp,
    };
    let text = serde_json::to_string(&focus).map_err(|err| err.to_string())?;
    std::fs::write(focus_file(), text).map_err(|err| err.to_string())
  }

  pub fn read() -> Option<Focus> {
    let text = std::fs::read_to_string(focus_file()).ok()?;
    serde_json::from_str(&text).ok()
  }

  /// Taken: the request is cleared so it fires once and not on every poll.
  pub fn take() -> Option<Focus> {
    let focus = Focus::read()?;
    let _ = std::fs::remove_file(focus_file());
    Some(focus)
  }
}

/// The task an editor runs to reveal a component, and where it goes.
///
/// Only Zed for now: it is the one whose task format takes the cursor as
/// variables, which is what makes the binding a paste rather than a script.
pub mod task {
  use std::path::PathBuf;

  pub const LABEL: &str = "Reveal in Tailor";

  pub fn zed_tasks_file() -> PathBuf {
    dirs::home_dir()
      .unwrap_or_default()
      .join(".config/zed/tasks.json")
  }

  /// The task, as Zed's `tasks.json` wants it.
  pub fn zed_task(binary: &str) -> serde_json::Value {
    serde_json::json!({
        "label": LABEL,
        "command": binary,
        "args": ["--reveal", "$ZED_FILE:$ZED_ROW"],
        // Never: the jump's whole point is that Tailor comes forward, and
        // a terminal panel opening behind it on every keystroke is noise.
        // Zed takes `always`, `no_focus` or `never` here and nothing else.
        "reveal": "never",
        "hide": "on_success",
        "shell": "system",
    })
  }

  /// What the outcome of installing was, so the caller can say something
  /// true rather than "done".
  #[derive(Debug, PartialEq)]
  pub enum Installed {
    Added,
    AlreadyThere,
  }

  /// Add the task to Zed's global task file, leaving everything else in it
  /// alone.
  ///
  /// Never clobbers: a file that will not parse is an error rather than
  /// something to overwrite, and a task with this label already in it is left
  /// as it is. Somebody may have edited theirs.
  pub fn install_zed(binary: &str) -> Result<Installed, String> {
    let path = zed_tasks_file();
    let existing = std::fs::read_to_string(&path).unwrap_or_default();

    let mut tasks: Vec<serde_json::Value> = if existing.trim().is_empty() {
      Vec::new()
    } else {
      serde_json::from_str(&strip_comments(&existing)).map_err(|err| {
        format!(
          "{} does not parse as JSON ({err}). Add the task by hand \
                     rather than have this overwrite it.",
          path.display()
        )
      })?
    };

    if tasks
      .iter()
      .any(|task| task.get("label").and_then(|l| l.as_str()) == Some(LABEL))
    {
      return Ok(Installed::AlreadyThere);
    }

    tasks.push(zed_task(binary));
    let text = serde_json::to_string_pretty(&tasks).map_err(|err| err.to_string())?;
    if let Some(parent) = path.parent() {
      std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    std::fs::write(&path, text + "\n").map_err(|err| err.to_string())?;
    Ok(Installed::Added)
  }

  /// Zed's config files allow `//` comments; `serde_json` does not. Only
  /// whole-line comments are stripped, so a `//` inside a string survives.
  pub(crate) fn strip_comments(text: &str) -> String {
    text
      .lines()
      .filter(|line| !line.trim_start().starts_with("//"))
      .collect::<Vec<_>>()
      .join("\n")
  }

  /// The keybinding to paste. Not written for you: claiming a key in
  /// somebody's keymap is not a thing to do quietly.
  pub const KEYBINDING: &str = r#"{
  "context": "Editor",
  "bindings": {
    "alt-cmd-r": ["task::Spawn", { "task_name": "Reveal in Tailor" }]
  }
}"#;
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_innermost_export_wins() {
    let mut index = ExportIndex::default();
    index
      .entries
      .insert("/work/app".into(), PathBuf::from("/work/outer.tailor"));
    index
      .entries
      .insert("/work/app/ui".into(), PathBuf::from("/work/inner.tailor"));

    // A file under both resolves to the one that owns it more closely.
    assert_eq!(
      index.project_for(Path::new("/work/app/ui/src/ui/people.rs")),
      Some(PathBuf::from("/work/inner.tailor"))
    );
    assert_eq!(
      index.project_for(Path::new("/work/app/src/ui/other.rs")),
      Some(PathBuf::from("/work/outer.tailor"))
    );
    assert_eq!(index.project_for(Path::new("/elsewhere/x.rs")), None);
  }
}

#[cfg(test)]
mod task_tests {
  use super::task::*;

  #[test]
  fn an_existing_task_file_keeps_its_other_tasks() {
    // The shape Zed writes, comments and all.
    let existing = r#"// my tasks
[
  { "label": "Build", "command": "cargo", "args": ["build"] }
]"#;
    let stripped: Vec<serde_json::Value> =
      serde_json::from_str(&super::task::strip_comments(existing)).unwrap();
    assert_eq!(stripped.len(), 1);
    assert_eq!(stripped[0]["label"], "Build");
  }

  /// Zed rejects the whole task file over one bad enum, and says so only in
  /// its log — so the values it accepts are worth pinning.
  #[test]
  fn the_task_uses_values_zed_accepts() {
    let task = zed_task("tailordev");
    assert!(
      ["always", "no_focus", "never"].contains(&task["reveal"].as_str().unwrap_or_default()),
      "reveal was {:?}",
      task["reveal"]
    );
    assert!(
      ["always", "never", "on_success"].contains(&task["hide"].as_str().unwrap_or_default()),
      "hide was {:?}",
      task["hide"]
    );
  }

  #[test]
  fn the_task_carries_the_cursor() {
    let task = zed_task("/Applications/Tailor.app/Contents/MacOS/tailor");
    assert_eq!(task["label"], LABEL);
    assert_eq!(task["args"][0], "--reveal");
    assert_eq!(task["args"][1], "$ZED_FILE:$ZED_ROW");
    assert!(KEYBINDING.contains(LABEL));
  }
}
