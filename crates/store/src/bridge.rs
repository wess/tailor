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
        self.entries
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
