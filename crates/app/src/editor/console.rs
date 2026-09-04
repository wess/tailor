//! The console: what the compiler said, and what the app printed.
//!
//! Two streams in one list, told apart by a gutter tag rather than by two
//! panes. A build and the run that follows it are one sequence — you read down
//! it — and splitting them would mean deciding which one to look at before
//! knowing which one had the answer.
//!
//! Monospace, because half of what lands here is rustc's caret diagrams, and
//! they only line up in a fixed pitch.

use gpui::prelude::*;
use gpui::{div, px, Context, ElementId, SharedString};
use tailor_build::session::Phase;

use super::Workbench;
use crate::theme;

impl Workbench {
  /// Every line, as text. What you paste into an issue.
  pub fn copy_console(&mut self, cx: &mut Context<Self>) {
    if self.build.console.is_empty() {
      self.toasts.info("Nothing in the console", cx);
      return;
    }
    let text: String = self
      .build
      .console
      .iter()
      .map(|line| line.text.as_str())
      .collect::<Vec<_>>()
      .join("\n");
    cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
    self.toasts.info("Copied the console", cx);
  }

  pub fn clear_console(&mut self, cx: &mut Context<Self>) {
    self.build.console.clear();
    cx.notify();
  }

  pub(super) fn render_console(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
    let chrome = theme::colors(cx);

    if self.build.console.is_empty() {
      return div()
        .id("console-empty")
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(6.))
        .flex_grow()
        .on_mouse_down(
          gpui::MouseButton::Right,
          cx.listener(|this, event: &gpui::MouseDownEvent, window, cx| {
            cx.stop_propagation();
            this.open_console_menu(event.position, window, cx);
          }),
        )
        .text_size(px(12.))
        .text_color(chrome.dimmed)
        .child("Nothing has run yet.")
        .child(
          div()
            .text_size(px(11.))
            .child("Press Run to build this project and launch it."),
        )
        .into_any_element();
    }

    // Rows are built from a snapshot so the closures below do not borrow the
    // workbench they may go on to mutate.
    let rows: Vec<(usize, Phase, SharedString)> = self
      .build
      .console
      .iter()
      .enumerate()
      .map(|(index, line)| (index, line.phase, SharedString::from(line.text.clone())))
      .collect();

    div()
      .id("console-list")
      .flex()
      .flex_col()
      .flex_grow()
      .overflow_y_scroll()
      .on_mouse_down(
        gpui::MouseButton::Right,
        cx.listener(|this, event: &gpui::MouseDownEvent, window, cx| {
          cx.stop_propagation();
          this.open_console_menu(event.position, window, cx);
        }),
      )
      // Anchored to the bottom, the way a terminal is: a build scrolls
      // itself, and the newest line is the one being waited for.
      .child(div().flex_grow())
      .children(rows.into_iter().map(|(index, phase, text)| {
        let (tag, tint) = match phase {
          Phase::Build => ("build", chrome.dimmed),
          Phase::Run => ("app", chrome.accent),
        };
        // rustc's own colouring is off (`--color=never`), so the severity is
        // read back off the text. Cheap, and it is the only signal there is.
        let color = if text.starts_with("error") {
          chrome.danger
        } else if text.starts_with("warning") {
          chrome.warning
        } else {
          chrome.text
        };
        div()
          .id(ElementId::Integer(index as u64))
          .flex()
          .items_start()
          .gap(px(8.))
          .px(px(10.))
          .py(px(1.))
          .child(
            div()
              .w(px(30.))
              .flex_none()
              .text_size(px(9.))
              .text_color(tint)
              .child(tag),
          )
          .child(
            div()
              .flex_grow()
              .font_family("ui-monospace")
              .text_size(px(11.))
              .text_color(color)
              .child(text),
          )
      }))
      .into_any_element()
  }

  /// The status strip beside the bottom pane's tabs: where the build went, and
  /// how it ended.
  pub(super) fn build_summary(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
    let chrome = theme::colors(cx);
    let status = &self.build.status;
    let color = match status {
      super::run::Status::Failed(_) => chrome.danger,
      super::run::Status::Running | super::run::Status::Built => chrome.accent,
      _ => chrome.dimmed,
    };
    div()
      .flex()
      .items_center()
      .gap(px(6.))
      .text_size(px(10.))
      .text_color(color)
      .child(SharedString::from(status.label()))
      .into_any_element()
  }
}
