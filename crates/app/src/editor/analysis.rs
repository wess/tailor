//! The work an edit causes, moved off the main thread.
//!
//! Regenerating the Rust and running the lint pass are pure functions over the
//! project, and on a large document they cost more than a frame — so an edit
//! that ran them inline dropped frames while you typed. They run on the
//! background executor instead, against the `Arc` the workbench already holds,
//! and their results are applied on the main thread only if nothing newer has
//! landed in the meantime.
//!
//! Two things make that safe. Every refresh bumps a revision, and a result
//! carrying an old one is discarded. And the `Task` handle lives in the
//! workbench: replacing it drops the previous task, which cancels work that is
//! no longer wanted rather than letting it finish into a void.
//!
//! Both are debounced, so holding a key down does not queue a hundred of them.
//! The delays are fields rather than constants so tests can set them to zero.

use std::sync::Arc;
use std::time::Duration;

use gpui::Context;
use tailor_model::lint::Problem;

/// What one background pass produces: the open document's source, every file
/// the project generates, and the lint result.
pub type Analysis = (String, Vec<(String, String)>, Vec<Problem>);

use super::Workbench;

/// How long an edit has to settle before the code and the lint are recomputed.
/// Short enough that the code panel still reads as live.
pub const ANALYSIS_DELAY: Duration = Duration::from_millis(120);
/// Autosave waits longer: it is a file write, and nobody is watching for it.
pub const AUTOSAVE_DELAY: Duration = Duration::from_millis(600);

impl Workbench {
  /// Recompute the generated code and the problem list in the background.
  pub(super) fn analyse(&mut self, cx: &mut Context<Self>) {
    let revision = self.revision;
    let project = Arc::clone(&self.project);
    let doc_id = self.doc_id.clone();
    let delay = self.analysis_delay;
    // Where the project builds, so a module you own can be read from disk
    // rather than shown as the scaffold Tailor stopped writing.
    let path = self.path.clone();

    self.analysis = Some(cx.spawn(async move |this, cx| {
      if !delay.is_zero() {
        cx.background_executor().timer(delay).await;
      }
      let outcome = cx
        .background_executor()
        .spawn(async move {
          let generated = project
            .doc(&doc_id)
            .map(|doc| tailor_codegen::preview(&project, doc).source)
            .unwrap_or_default();
          // The whole crate, not just the open document: the code pane is a
          // file strip over what a build would compile, and `main.rs` is
          // where a reader looks first.
          let workspace = tailor_build::Workspace::resolve(path.as_deref(), &project);
          let files = tailor_codegen::project_files(&project)
            .into_iter()
            .map(|file| {
              // A scaffold is only Tailor's until it exists. After that the
              // file on disk is the truth, and showing the scaffold instead
              // would be showing something nobody has.
              if file.scaffold {
                let on_disk = workspace.root.join(&file.path);
                if let Ok(source) = std::fs::read_to_string(&on_disk) {
                  return (file.path, source);
                }
              }
              (file.path, file.source)
            })
            .collect();
          let problems = tailor_model::lint::check(&project);
          (generated, files, problems)
        })
        .await;
      this
        .update(cx, |this, cx| this.apply_analysis(revision, outcome, cx))
        .ok();
    }));
  }

  pub(crate) fn apply_analysis(
    &mut self,
    revision: u64,
    (generated, files, problems): Analysis,
    cx: &mut Context<Self>,
  ) {
    // Something newer landed while this was running.
    if self.revision != revision {
      return;
    }
    self.problems = problems;
    let changed = generated != self.generated || files != self.code.files;
    self.generated = generated;
    self.code.files = files;
    // A pin on a file that no longer exists — a screen was renamed or
    // deleted — goes back to following the canvas rather than showing
    // nothing.
    if let Some(pinned) = self.code.pinned.clone() {
      if !self.code.files.iter().any(|(path, _)| *path == pinned) {
        self.code.pinned = None;
      }
    }
    if changed {
      self.sync_code_view(cx);
    }
    cx.notify();
  }

  /// Write the project out, once the edits stop. Replacing the task cancels
  /// the previous one, so a burst of edits costs one write rather than one
  /// per keystroke.
  pub(super) fn schedule_autosave(&mut self, cx: &mut Context<Self>) {
    if !self.settings.autosave || !self.dirty {
      self.autosave = None;
      return;
    }
    let Some(path) = self.path.clone() else {
      self.autosave = None;
      return;
    };
    let project = Arc::clone(&self.project);
    let revision = self.revision;
    let delay = self.autosave_delay;

    self.autosave = Some(cx.spawn(async move |this, cx| {
      if !delay.is_zero() {
        cx.background_executor().timer(delay).await;
      }
      let written = cx
        .background_executor()
        .spawn(async move { tailor_store::save(&path, &project).map_err(|e| e.to_string()) })
        .await;
      this
        .update(cx, |this, cx| {
          // A newer edit is already on its way to disk; leave the flag.
          if this.revision != revision {
            return;
          }
          match written {
            Ok(()) => {
              this.dirty = false;
              this.mark_file_seen();
              cx.notify();
            }
            Err(err) => this.toasts.failed(format!("Autosave failed: {err}"), cx),
          }
        })
        .ok();
    }));
  }
}
