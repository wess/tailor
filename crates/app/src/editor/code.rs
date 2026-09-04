//! The code pane: every file the project generates, and what the compiler
//! said about them.
//!
//! It regenerates on every edit, which is the point — you can watch a drag turn
//! into a `.gap(px(12.))` and learn the library while you use the builder. But
//! a design is more than one file, and the interesting one is not always the
//! screen you have open: `main.rs` is where the window opens, `theme.rs` is
//! where the palette went, `Cargo.toml` is what a build resolves. So the pane
//! is a file strip over the whole crate, defaulting to the open document's
//! file because that is the one the canvas is about.
//!
//! **Read-only, on purpose.** The design is the source and the code is the
//! output; an editable buffer over a file that is rewritten on the next
//! keystroke would be a promise Tailor cannot keep. What it does instead is
//! the two things an editor is actually *for* here — showing you exactly what
//! you are about to ship, and putting the compiler's complaints on the lines
//! they belong to.
//!
//! Highlighting is tree-sitter, the same parser Zed uses, so what you read is
//! parsed Rust rather than a regex's guess at it.

use gpui::prelude::*;
use gpui::{div, px, Context, MouseButton, MouseDownEvent, SharedString, Window};
use guise::editor::{Diagnostic as EditorDiagnostic, Severity as EditorSeverity};
use guise::prelude::*;
use tailor_model::Flavor;
use tailor_store::Panel;

use super::{icon, Workbench};
use crate::theme;

/// Everything the code pane keeps between frames.
#[derive(Default)]
pub struct CodePane {
  /// Every file the project generates, as `(path, source)`, rebuilt with the
  /// rest of the analysis.
  pub files: Vec<(String, String)>,
  /// The file being shown. `None` follows the open document, which is what
  /// makes the pane feel attached to the canvas rather than beside it.
  pub pinned: Option<String>,
  /// The find bar, while it is up.
  pub find: Option<super::find::Find>,
}

impl CodePane {
  /// The file to show for a document, by the path codegen gives it.
  fn path_for(&self, doc_name: &str) -> Option<String> {
    let wanted = format!("{}.rs", tailor_model::snake_case(doc_name));
    self
      .files
      .iter()
      .map(|(path, _)| path)
      .find(|path| path.ends_with(&wanted))
      .cloned()
  }

  fn source(&self, path: &str) -> Option<&str> {
    self
      .files
      .iter()
      .find(|(candidate, _)| candidate == path)
      .map(|(_, source)| source.as_str())
  }
}

impl Workbench {
  /// Make sure the code pane is on screen.
  ///
  /// It is not a panel you toggle — it is half of Split mode, which is the
  /// canvas showing the design and the code side by side. Anything that wants
  /// to point at a line has to put that half up first.
  pub(super) fn show_code(&mut self, cx: &mut Context<Self>) {
    if self.settings.canvas_mode != tailor_store::CanvasMode::Split {
      self.set_mode(tailor_store::CanvasMode::Split, cx);
    }
  }

  /// Which file the pane is showing right now.
  pub fn showing_file(&self) -> Option<String> {
    self.code.pinned.clone().or_else(|| {
      let name = self.doc().map(|doc| doc.name.clone())?;
      self.code.path_for(&name)
    })
  }

  /// Show `path`, and put the caret on `line` (1-based) if given.
  ///
  /// The in-app half of the editor bridge: a compiler error in Problems knows
  /// its file and its line, and this is what turns that into "look here".
  pub fn reveal_in_code(
    &mut self,
    path: &str,
    line: Option<usize>,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    // Cargo reports paths relative to the crate root, which is what codegen
    // names them — but an absolute one still has to land.
    let path = self
      .code
      .files
      .iter()
      .map(|(candidate, _)| candidate.clone())
      .find(|candidate| candidate == path || path.ends_with(candidate.as_str()));
    let Some(path) = path else { return };

    self.code.pinned = Some(path.clone());
    self.show_code(cx);
    self.sync_code_view(cx);

    if let Some(line) = line {
      let target = line.saturating_sub(1);
      self.code_view.update(cx, |editor, cx| {
        // `edit` keeps the caret visible, which is the whole point — the
        // line has to be scrolled to, not just selected.
        editor.edit(window, cx, |model| model.move_to(target, 0, false));
      });
    }
    cx.notify();
  }

  /// Push the showing file's source and diagnostics into the editor.
  ///
  /// Called after every analysis and every build, because either can change
  /// what is on screen: the first rewrites the source, the second decorates
  /// it.
  pub(super) fn sync_code_view(&mut self, cx: &mut Context<Self>) {
    let Some(path) = self.showing_file() else {
      return;
    };
    let source = self
      .code
      .source(&path)
      .map(str::to_string)
      .unwrap_or_default();
    let diagnostics = self.diagnostics_for(&path);

    self.code_view.update(cx, |editor, cx| {
      if editor.text() != source {
        editor.set_text(&source, cx);
      }
      editor.set_diagnostics(diagnostics, cx);
    });
  }

  /// The last build's complaints about one file, as the editor wants them.
  fn diagnostics_for(&self, path: &str) -> Vec<EditorDiagnostic> {
    self
      .build
      .diagnostics
      .iter()
      .filter(|diagnostic| {
        !diagnostic.file.is_empty() && (diagnostic.file == path || diagnostic.file.ends_with(path))
      })
      .map(|diagnostic| {
        // rustc counts from one and the editor from zero, on both axes.
        let line = diagnostic.line.saturating_sub(1);
        let start = diagnostic.col.saturating_sub(1);
        let end = diagnostic.col_end.saturating_sub(1).max(start);
        EditorDiagnostic::new(
          line,
          start..end,
          match diagnostic.severity {
            tailor_build::Severity::Error => EditorSeverity::Error,
            tailor_build::Severity::Warning => EditorSeverity::Warning,
            tailor_build::Severity::Help => EditorSeverity::Hint,
            tailor_build::Severity::Note => EditorSeverity::Info,
          },
          diagnostic.message.clone(),
        )
      })
      .collect()
  }

  pub(super) fn render_code(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let chrome = theme::colors(cx);
    let flavor = self.project.gen.flavor;
    let showing = self.showing_file();
    let lines = showing
      .as_deref()
      .and_then(|path| self.code.source(path))
      .map(|source| source.lines().count())
      .unwrap_or(0);

    let trailing = div()
      .flex()
      .items_center()
      .gap(px(4.))
      .child(
        div()
          .text_size(px(10.))
          .text_color(chrome.dimmed)
          .child(SharedString::from(format!("{lines} lines"))),
      )
      .children(Flavor::ALL.iter().map(|option| {
        let option = *option;
        let selected = option == flavor;
        div()
          .id(SharedString::from(format!("flavor-{}", option.label())))
          .px(px(6.))
          .py(px(2.))
          .rounded(px(4.))
          .text_size(px(10.))
          .when(selected, |d| {
            d.bg(chrome.accent_soft).text_color(chrome.accent)
          })
          .when(!selected, |d| d.text_color(chrome.dimmed))
          .child(SharedString::from(option.label()))
          .on_click(cx.listener(move |this, _, _window, cx| {
            std::sync::Arc::make_mut(&mut this.project).gen.flavor = option;
            this.dirty = true;
            this.refresh(cx);
          }))
      }))
      .child(
        div()
          .id("copy-code")
          .flex()
          .items_center()
          .justify_center()
          .size(px(20.))
          .rounded(px(4.))
          .text_color(chrome.dimmed)
          .hover(move |style| style.bg(chrome.raised).text_color(chrome.text))
          .child(icon("clipboard-copy"))
          .tooltip(tooltip("Copy this file"))
          .on_click(cx.listener(|this, _, _window, cx| this.copy_code(cx))),
      )
      .into_any_element();

    div()
      .w(px(self.settings.size(Panel::Code)))
      .flex_none()
      .h_full()
      .flex()
      .flex_col()
      .bg(chrome.surface)
      .child(self.panel_header(Panel::Code, Some(trailing), cx))
      .child(self.file_strip(showing, cx))
      .children(self.render_find(cx))
      .child(
        div()
          .id("code-body")
          .flex_grow()
          .overflow_hidden()
          .on_mouse_down(
            MouseButton::Right,
            cx.listener(|this, event: &MouseDownEvent, window, cx| {
              cx.stop_propagation();
              this.open_code_menu(event.position, window, cx);
            }),
          )
          .child(self.code_view.clone()),
      )
  }

  /// The files the project generates, as a row you can pick from.
  ///
  /// The one for the open document is not pinned — it follows the canvas —
  /// so switching screens moves the pane with you until you choose otherwise.
  fn file_strip(&self, showing: Option<String>, cx: &mut Context<Self>) -> gpui::AnyElement {
    let chrome = theme::colors(cx);
    let following = self.code.pinned.is_none();
    let current = self
      .doc()
      .and_then(|doc| self.code.path_for(&doc.name))
      .unwrap_or_default();
    // How many errors each file has, so a broken one is visible without
    // opening it — the Issue navigator's dot, on a tab.
    let broken: Vec<String> = self
      .build
      .diagnostics
      .iter()
      .filter(|d| d.severity == tailor_build::Severity::Error)
      .map(|d| d.file.clone())
      .collect();

    div()
      .id("code-files")
      .flex()
      .items_center()
      .gap(px(2.))
      .px(px(6.))
      .pb(px(4.))
      .overflow_x_scroll()
      .children(self.code.files.iter().map(|(path, _)| {
        let path = path.clone();
        let selected = showing.as_deref() == Some(path.as_str());
        let tracks = following && path == current;
        let has_errors = broken.iter().any(|file| file.ends_with(&path));
        let name = path.rsplit('/').next().unwrap_or(&path).to_string();
        let pin = path.clone();
        div()
          .id(SharedString::from(format!("file-{path}")))
          .flex()
          .flex_none()
          .items_center()
          .gap(px(4.))
          .px(px(7.))
          .py(px(2.))
          .rounded(px(4.))
          .text_size(px(10.))
          .when(selected, |d| {
            d.bg(chrome.accent_soft).text_color(chrome.accent)
          })
          .when(!selected, |d| d.text_color(chrome.dimmed))
          .hover(move |style| style.text_color(chrome.text))
          .when(has_errors, |d| {
            d.child(div().text_color(chrome.danger).child(icon("circle-x")))
          })
          .child(SharedString::from(name))
          // A dot for the file the canvas is on, when the pane is following
          // it rather than pinned somewhere else.
          .when(tracks && selected, |d| {
            d.child(div().text_size(px(8.)).text_color(chrome.accent).child("•"))
          })
          .tooltip(tooltip(path.clone()))
          .on_click(cx.listener(move |this, _, _window, cx| {
            // Clicking the file the canvas is already on unpins, which is how
            // you get back to following it.
            this.code.pinned = if this.code.pinned.as_deref() == Some(pin.as_str()) {
              None
            } else {
              Some(pin.clone())
            };
            this.sync_code_view(cx);
            // The hits were positions in the file that just went away.
            this.refresh_find(cx);
            cx.notify();
          }))
      }))
      .into_any_element()
  }
}
