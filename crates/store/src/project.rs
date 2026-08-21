//! Reading and writing `.tailor` files.

use std::path::Path;

use tailor_model::Project;

#[derive(Debug)]
pub enum SaveError {
    Io(std::io::Error),
    /// The project holds something JSON cannot write — an infinity or a NaN
    /// that reached a numeric prop.
    Encode(String),
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::Io(err) => write!(f, "{err}"),
            SaveError::Encode(err) => write!(f, "{err}"),
        }
    }
}

#[derive(Debug)]
pub enum OpenError {
    Io(std::io::Error),
    Parse(tailor_model::LoadError),
}

impl std::fmt::Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpenError::Io(err) => write!(f, "{err}"),
            OpenError::Parse(err) => write!(f, "{err}"),
        }
    }
}

pub fn open(path: &Path) -> Result<Project, OpenError> {
    let text = std::fs::read_to_string(path).map_err(OpenError::Io)?;
    Project::from_json(&text).map_err(OpenError::Parse)
}

/// Write the project. The parent directory is created if the user typed a path
/// into a folder that does not exist yet.
pub fn save(path: &Path, project: &Project) -> Result<(), SaveError> {
    // Encode before touching the filesystem, so a project that cannot be
    // written never truncates the file that already holds the good one.
    let text = project.to_json().map_err(SaveError::Encode)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(SaveError::Io)?;
    }
    std::fs::write(path, text).map_err(SaveError::Io)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_project_survives_a_trip_through_a_file() {
        let dir = std::env::temp_dir().join("tailor-store-test");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("nested").join("demo.tailor");

        let project = Project::new("Demo");
        save(&path, &project).unwrap();
        let loaded = open(&path).unwrap();
        assert_eq!(loaded, project);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_file_reports_io() {
        let err = open(Path::new("/definitely/not/here.tailor")).unwrap_err();
        assert!(matches!(err, OpenError::Io(_)));
    }
}
