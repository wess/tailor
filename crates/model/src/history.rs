//! Undo/redo as whole-project snapshots.
//!
//! Not an inverse-operation log. Every operation — reparent, prop edit, delete
//! a subtree, rename a component every screen references — is correct by
//! construction instead of by a hand-written inverse that has to stay right
//! forever.
//!
//! Snapshots are `Arc<Project>`, so taking one is a refcount bump rather than a
//! deep copy: two hundred entries of a large project cost one project's worth of
//! memory until something is actually edited, and committing on every keystroke
//! costs nothing. The editor clones through `Arc::make_mut`, which pays for the
//! copy once per edit — not once per commit, and not once per frame.

use std::sync::Arc;

use crate::project::Project;

const LIMIT: usize = 200;

#[derive(Debug, Clone)]
struct Snapshot {
    /// What the menu shows: "Undo Move Button".
    label: String,
    project: Arc<Project>,
}

#[derive(Debug, Default)]
pub struct History {
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    /// Set while a drag is in flight, so the hundred intermediate positions
    /// collapse into the one entry the user thinks they performed.
    coalescing: Option<String>,
}

impl History {
    /// Record the state *before* a change. Call it with the project as it
    /// stands, then mutate.
    pub fn commit(&mut self, label: impl Into<String>, before: &Arc<Project>) {
        let label = label.into();
        if let Some(open) = &self.coalescing {
            if *open == label {
                return;
            }
        }
        self.undo.push(Snapshot {
            label,
            project: before.clone(),
        });
        self.redo.clear();
        if self.undo.len() > LIMIT {
            self.undo.remove(0);
        }
    }

    /// Record only if the previous entry was something else. Typing into a
    /// field commits on every keystroke, and thirty undo steps to unwrite a
    /// word is not undo, it is punishment.
    pub fn commit_run(&mut self, label: impl Into<String>, before: &Arc<Project>) {
        let label = label.into();
        if self
            .undo
            .last()
            .map(|entry| entry.label == label)
            .unwrap_or(false)
        {
            self.redo.clear();
            return;
        }
        self.commit(label, before);
    }

    /// Begin a run of changes that should undo as one — a drag, a slider.
    /// The first `commit` under this label records; the rest are dropped.
    pub fn begin(&mut self, label: impl Into<String>, before: &Arc<Project>) {
        let label = label.into();
        self.commit(label.clone(), before);
        self.coalescing = Some(label);
    }

    pub fn end(&mut self) {
        self.coalescing = None;
    }

    /// Undo the last commit and forget it ever happened. For a command that
    /// took a snapshot and then found there was nothing to do: a plain `undo`
    /// would restore the state but leave a redo entry pointing at a change that
    /// never landed.
    pub fn rollback(&mut self, current: &mut Arc<Project>) -> bool {
        match self.undo.pop() {
            Some(snapshot) => {
                *current = snapshot.project;
                true
            }
            None => false,
        }
    }

    pub fn undo(&mut self, current: &mut Arc<Project>) -> Option<String> {
        self.coalescing = None;
        let snapshot = self.undo.pop()?;
        let label = snapshot.label.clone();
        self.redo.push(Snapshot {
            label: snapshot.label,
            project: std::mem::replace(current, snapshot.project),
        });
        Some(label)
    }

    pub fn redo(&mut self, current: &mut Arc<Project>) -> Option<String> {
        self.coalescing = None;
        let snapshot = self.redo.pop()?;
        let label = snapshot.label.clone();
        self.undo.push(Snapshot {
            label: snapshot.label,
            project: std::mem::replace(current, snapshot.project),
        });
        Some(label)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// What the next undo would reverse, for the menu item's label.
    pub fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|s| s.label.as_str())
    }

    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|s| s.label.as_str())
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.coalescing = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(name: &str) -> Arc<Project> {
        Arc::new(Project::new(name))
    }

    #[test]
    fn undo_and_redo_walk_the_stack() {
        let mut history = History::default();
        let mut project = named("one");

        history.commit("Rename", &project);
        Arc::make_mut(&mut project).name = "two".into();
        history.commit("Rename", &project);
        Arc::make_mut(&mut project).name = "three".into();

        assert_eq!(history.undo(&mut project).as_deref(), Some("Rename"));
        assert_eq!(project.name, "two");
        assert_eq!(history.undo(&mut project).as_deref(), Some("Rename"));
        assert_eq!(project.name, "one");
        assert!(history.undo(&mut project).is_none());

        history.redo(&mut project);
        assert_eq!(project.name, "two");
        history.redo(&mut project);
        assert_eq!(project.name, "three");
        assert!(!history.can_redo());
    }

    #[test]
    fn a_new_change_drops_the_redo_stack() {
        let mut history = History::default();
        let mut project = named("one");
        history.commit("Edit", &project);
        Arc::make_mut(&mut project).name = "two".into();
        history.undo(&mut project);
        assert!(history.can_redo());

        history.commit("Edit", &project);
        assert!(!history.can_redo());
    }

    #[test]
    fn a_drag_collapses_into_one_entry() {
        let mut history = History::default();
        let mut project = named("start");

        history.begin("Move", &project);
        for step in 0..10 {
            history.commit("Move", &project);
            Arc::make_mut(&mut project).name = format!("step{step}");
        }
        history.end();

        assert_eq!(history.undo(&mut project).as_deref(), Some("Move"));
        assert_eq!(project.name, "start");
        assert!(!history.can_undo());
    }

    #[test]
    fn a_rollback_leaves_nothing_behind() {
        let mut history = History::default();
        let mut project = named("one");
        history.commit("Edit", &project);
        Arc::make_mut(&mut project).name = "two".into();

        assert!(history.rollback(&mut project));
        assert_eq!(project.name, "one");
        assert!(!history.can_undo());
        assert!(!history.can_redo());
        assert!(!history.rollback(&mut project));
    }

    #[test]
    fn a_run_of_the_same_edit_collapses() {
        let mut history = History::default();
        let mut project = named("start");
        for step in 0..5 {
            history.commit_run("Set label", &project);
            Arc::make_mut(&mut project).name = format!("step{step}");
        }
        history.commit_run("Set width", &project);
        assert_eq!(history.undo(&mut project).as_deref(), Some("Set width"));
        assert_eq!(history.undo(&mut project).as_deref(), Some("Set label"));
        assert_eq!(project.name, "start");
        assert!(!history.can_undo());
    }

    #[test]
    fn a_different_label_breaks_the_run() {
        let mut history = History::default();
        let project = named("start");
        history.begin("Move", &project);
        history.commit("Resize", &project);
        history.end();
        assert_eq!(history.undo_label(), Some("Resize"));
    }
}
