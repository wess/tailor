//! The recent-projects list the start screen shows.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const LIMIT: usize = 12;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recent {
    pub path: PathBuf,
    pub name: String,
}

impl Recent {
    /// The path as the start screen shows it: the home directory abbreviated,
    /// because a full path is mostly noise you already know.
    pub fn display_path(&self) -> String {
        let text = self.path.to_string_lossy().to_string();
        match dirs::home_dir() {
            Some(home) => {
                let home = home.to_string_lossy().to_string();
                text.strip_prefix(&home)
                    .map(|rest| format!("~{rest}"))
                    .unwrap_or(text)
            }
            None => text,
        }
    }

    pub fn exists(&self) -> bool {
        self.path.exists()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Recents {
    pub entries: Vec<Recent>,
}

impl Recents {
    pub fn load() -> Self {
        std::fs::read_to_string(crate::paths::recents_file())
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        crate::paths::ensure_config_dir();
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(crate::paths::recents_file(), text);
        }
    }

    /// Move a project to the top, or add it. Re-opening a project should not
    /// grow the list.
    pub fn touch(&mut self, path: &Path, name: &str) {
        self.entries.retain(|entry| entry.path != path);
        self.entries.insert(
            0,
            Recent {
                path: path.to_path_buf(),
                name: name.to_string(),
            },
        );
        self.entries.truncate(LIMIT);
    }

    pub fn remove(&mut self, path: &Path) {
        self.entries.retain(|entry| entry.path != path);
    }

    /// Drop entries whose file has been deleted or moved.
    pub fn prune(&mut self) {
        self.entries.retain(|entry| entry.exists());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touching_moves_to_the_front_without_duplicating() {
        let mut recents = Recents::default();
        recents.touch(Path::new("/a.tailor"), "A");
        recents.touch(Path::new("/b.tailor"), "B");
        recents.touch(Path::new("/a.tailor"), "A");
        assert_eq!(recents.entries.len(), 2);
        assert_eq!(recents.entries[0].name, "A");
    }

    #[test]
    fn the_list_is_capped() {
        let mut recents = Recents::default();
        for n in 0..30 {
            recents.touch(Path::new(&format!("/{n}.tailor")), &n.to_string());
        }
        assert_eq!(recents.entries.len(), LIMIT);
        assert_eq!(recents.entries[0].name, "29");
    }

    #[test]
    fn removing_takes_one_entry() {
        let mut recents = Recents::default();
        recents.touch(Path::new("/a.tailor"), "A");
        recents.remove(Path::new("/a.tailor"));
        assert!(recents.entries.is_empty());
    }
}
