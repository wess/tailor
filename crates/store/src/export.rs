//! Writing generated code to a directory.
//!
//! Every file is written, none are merged: an export is a snapshot of the
//! design, and quietly merging into a file someone has since edited by hand is
//! how a builder eats your work. The report says what was overwritten so the
//! app can say it out loud.

use std::path::{Component, Path, PathBuf};

use tailor_codegen::Generated;
use tailor_model::Project;

#[derive(Debug, Default)]
pub struct ExportReport {
    pub written: Vec<PathBuf>,
    /// Files that already existed and were replaced.
    pub overwritten: Vec<PathBuf>,
    pub failed: Vec<(PathBuf, String)>,
    /// Anything the generator wanted to say about what it produced.
    pub notes: Vec<String>,
}

impl ExportReport {
    pub fn summary(&self) -> String {
        let mut parts = vec![format!("{} files", self.written.len())];
        if !self.overwritten.is_empty() {
            parts.push(format!("{} replaced", self.overwritten.len()));
        }
        if !self.failed.is_empty() {
            parts.push(format!("{} failed", self.failed.len()));
        }
        parts.join(", ")
    }

    pub fn ok(&self) -> bool {
        self.failed.is_empty()
    }
}

/// Write every generated file under `root`.
pub fn export(root: &Path, project: &Project) -> ExportReport {
    write_all(root, tailor_codegen::project_files(project))
}

/// Write an explicit set of files — used by "export this document only".
pub fn write_all(root: &Path, files: Vec<Generated>) -> ExportReport {
    let mut report = ExportReport::default();
    for file in files {
        // Generated paths are built from snake-cased names and literals, so
        // none of them can climb out of the directory. Checked anyway: an
        // export writes files, and "writes files" is worth one `if`.
        if !stays_inside(&file.path) {
            report.failed.push((
                PathBuf::from(&file.path),
                "path escapes the export directory".into(),
            ));
            continue;
        }
        let path = root.join(&file.path);
        report.notes.extend(file.notes);
        if let Some(parent) = path.parent() {
            if let Err(err) = std::fs::create_dir_all(parent) {
                report.failed.push((path.clone(), err.to_string()));
                continue;
            }
        }
        let existed = path.exists();
        match std::fs::write(&path, &file.source) {
            Ok(()) => {
                if existed {
                    report.overwritten.push(path.clone());
                }
                report.written.push(path);
            }
            Err(err) => report.failed.push((path, err.to_string())),
        }
    }
    report.notes.sort();
    report.notes.dedup();
    report
}

/// Whether a generated path is a plain relative path — no root, no `..`.
fn stays_inside(path: &str) -> bool {
    let path = Path::new(path);
    path.is_relative()
        && path
            .components()
            .all(|part| matches!(part, Component::Normal(_) | Component::CurDir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_that_climbs_out_is_refused() {
        assert!(stays_inside("src/ui/main.rs"));
        assert!(stays_inside("Cargo.toml"));
        assert!(!stays_inside("../escaped.rs"));
        assert!(!stays_inside("/etc/passwd"));
        assert!(!stays_inside("src/../../out.rs"));

        let root = std::env::temp_dir().join("tailor-export-escape");
        let _ = std::fs::remove_dir_all(&root);
        let report = write_all(
            &root,
            vec![Generated {
                path: "../escaped.rs".into(),
                source: "// no".into(),
                notes: Vec::new(),
            }],
        );
        assert!(!report.ok());
        assert!(report.written.is_empty());
        assert!(!root.parent().unwrap().join("escaped.rs").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_export_writes_the_tree_and_reports_replacements() {
        let root = std::env::temp_dir().join("tailor-export-test");
        let _ = std::fs::remove_dir_all(&root);
        let project = Project::new("Demo");

        let first = export(&root, &project);
        assert!(first.ok());
        assert!(first.overwritten.is_empty());
        assert!(root.join("src/ui/main_screen.rs").exists());
        assert!(root.join("Cargo.toml").exists());

        let second = export(&root, &project);
        assert_eq!(second.overwritten.len(), second.written.len());
        assert!(second.summary().contains("replaced"));

        let _ = std::fs::remove_dir_all(&root);
    }
}
