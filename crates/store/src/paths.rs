//! Where Tailor keeps its own files.
//!
//! One directory under the platform config dir, holding the editor's settings
//! and the recent-projects list. Projects themselves live wherever the user
//! saved them — Tailor is a document app, not a workspace app.

use std::path::PathBuf;

/// `~/Library/Application Support/tailor` on macOS, `~/.config/tailor` on
/// Linux. Falls back to the working directory if the platform has no config
/// dir, which is a strange machine but not a reason to refuse to start.
pub fn config_dir() -> PathBuf {
  dirs::config_dir()
    .map(|dir| dir.join("tailor"))
    .unwrap_or_else(|| PathBuf::from(".tailor"))
}

pub fn settings_file() -> PathBuf {
  config_dir().join("settings.json")
}

pub fn recents_file() -> PathBuf {
  config_dir().join("recents.json")
}

/// The directory a save dialog should open in: where the last project was
/// saved, or the user's documents folder.
pub fn default_project_dir() -> PathBuf {
  dirs::document_dir()
    .or_else(dirs::home_dir)
    .unwrap_or_else(|| PathBuf::from("."))
}

/// Create the config directory if it is missing. Errors are swallowed on
/// purpose: a read-only config dir should cost you your recents list, not the
/// ability to open the app.
pub fn ensure_config_dir() {
  let _ = std::fs::create_dir_all(config_dir());
}

/// The extension a Tailor project is saved under.
pub const EXTENSION: &str = "tailor";

/// Force `path` to end in `.tailor`.
pub fn with_extension(path: PathBuf) -> PathBuf {
  if path
    .extension()
    .map(|ext| ext == EXTENSION)
    .unwrap_or(false)
  {
    path
  } else {
    let mut path = path;
    let name = path
      .file_name()
      .map(|n| n.to_string_lossy().to_string())
      .unwrap_or_default();
    path.set_file_name(format!("{name}.{EXTENSION}"));
    path
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_extension_is_added_but_never_doubled() {
    assert_eq!(
      with_extension(PathBuf::from("/a/b")),
      PathBuf::from("/a/b.tailor")
    );
    assert_eq!(
      with_extension(PathBuf::from("/a/b.tailor")),
      PathBuf::from("/a/b.tailor")
    );
    assert_eq!(
      with_extension(PathBuf::from("/a/b.json")),
      PathBuf::from("/a/b.json.tailor")
    );
  }
}
