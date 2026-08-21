//! Watching the open file.
//!
//! Tailor's MCP server edits the same `.tailor` file this window has open, and
//! an agent building a screen is much more useful if you can watch it happen.
//! A poll of the modified time is enough — the file is small, the interval is
//! long, and it needs no dependency.
//!
//! Unsaved work always wins. If the file changed on disk while there are edits
//! here that are not in it, the reload is refused and said out loud rather than
//! quietly picking one of the two.

use std::path::Path;
use std::time::{Duration, SystemTime};

use gpui::Context;

use super::Workbench;

/// How often to look. Slow enough to be free, quick enough to feel live.
const INTERVAL: Duration = Duration::from_millis(700);

pub fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

impl Workbench {
    /// Start the poll. Ends when the workbench goes away.
    pub(super) fn watch_file(&self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(INTERVAL).await;

                // What to look at, read on the main thread — it is two field
                // reads.
                let Ok(Some((path, seen, dirty))) = this.update(cx, |this, _| {
                    this.path
                        .clone()
                        .map(|path| (path, this.file_seen, this.dirty))
                }) else {
                    // The workbench is gone; so is the reason to watch.
                    break;
                };

                // The stat, the read and the parse all happen off the main
                // thread. None of them is slow on a small file, but all three
                // are syscalls or allocation, and doing them on the foreground
                // every 700ms is a stutter waiting to happen on a big project.
                let found = cx
                    .background_executor()
                    .spawn(async move {
                        let stamp = modified(&path)?;
                        if Some(stamp) == seen {
                            return None;
                        }
                        if dirty {
                            // Do not even parse it: the answer is already no.
                            return Some((stamp, None));
                        }
                        // A read that fails is a half-written file; the next
                        // poll finds it whole.
                        Some((stamp, tailor_store::open(&path).ok()))
                    })
                    .await;

                let Some((stamp, loaded)) = found else {
                    continue;
                };
                if this
                    .update(cx, |this, cx| this.file_changed(stamp, loaded, cx))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    /// Note the file's current time as ours, so our own writes never look like
    /// somebody else's.
    pub(super) fn mark_file_seen(&mut self) {
        self.file_seen = self.path.as_ref().and_then(|path| modified(path));
    }

    /// Something else wrote the file. Take it, unless there is unsaved work
    /// here — that always wins.
    fn file_changed(
        &mut self,
        stamp: SystemTime,
        loaded: Option<tailor_model::Project>,
        cx: &mut Context<Self>,
    ) {
        if self.dirty {
            // Say it once, then stop: a stream of toasts about a file you are
            // deliberately not reloading is worse than the conflict.
            if !self.warned_about_file {
                self.warned_about_file = true;
                self.toasts.titled(
                    "Changed on disk",
                    "Something else edited this project. Your unsaved changes are still here; \
                     save to overwrite, or close without saving to take theirs.",
                    guise::theme::ColorName::Yellow,
                    cx,
                );
            }
            return;
        }
        let Some(project) = loaded else { return };
        self.file_seen = Some(stamp);
        self.warned_about_file = false;
        self.adopt(project, cx);
        self.toasts
            .info("Reloaded — the project changed on disk", cx);
    }

    /// Replace the project with one that came from outside, keeping as much of
    /// the session as still makes sense.
    fn adopt(&mut self, project: tailor_model::Project, cx: &mut Context<Self>) {
        self.project = std::sync::Arc::new(project);
        if self.project.doc(&self.doc_id).is_none() {
            self.doc_id = self
                .project
                .docs
                .first()
                .map(|doc| doc.id.clone())
                .unwrap_or_default();
        }
        self.selection.retain(|id| {
            self.project
                .doc(&self.doc_id)
                .map(|doc| doc.node(*id).is_some())
                .unwrap_or(false)
        });
        self.fields.clear();
        self.areas.clear();
        self.menu = None;
        self.grab = None;
        self.guides.clear();
        // The undo stack describes a document nobody has any more.
        self.history.clear();
        self.dirty = false;
        self.store.update(cx, |store, _| store.clear());
        crate::theme::install(&self.project.theme, cx);
        self.refresh(cx);
    }
}
