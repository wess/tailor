//! The bottom pane: Problems, and the console beside it.
//!
//! **Problems** is what will not generate and what probably was not meant —
//! the lint pass — plus whatever the last build's compiler said. Two sources,
//! one list, because they are the same question asked twice: the lint catches
//! what cannot compile before you compile it, and the compiler catches the
//! rest. Both rows point at a component, and clicking either selects it.
//!
//! **Console** is what the build and the app printed. Tabs rather than two
//! panes: they are read one at a time, and a builder has enough panels.

use gpui::prelude::*;
use gpui::{div, px, Context, ElementId, MouseButton, MouseDownEvent, SharedString};
use tailor_model::lint::Severity;
use tailor_store::Panel;

use super::run::Bottom;
use super::{icon, Workbench};
use crate::theme;

/// One problem, flattened out of the lint result so the row closures do not
/// borrow the workbench they are about to mutate.
struct Row {
  index: usize,
  severity: Severity,
  message: String,
  fix: String,
  doc_id: String,
  node: Option<tailor_model::NodeId>,
}

/// The error and warning tallies, beside the panel's name.
fn count_badge(problems: &[Row], cx: &mut Context<Workbench>) -> gpui::AnyElement {
  let chrome = theme::colors(cx);
  let tally = |severity: Severity| {
    problems
      .iter()
      .filter(|row| row.severity == severity)
      .count()
  };
  let (errors, warnings) = (tally(Severity::Error), tally(Severity::Warning));
  div()
    .flex()
    .items_center()
    .gap(px(8.))
    .text_size(px(10.))
    .when(errors > 0, |d| {
      d.child(
        div()
          .flex()
          .items_center()
          .gap(px(3.))
          .text_color(chrome.danger)
          .child(icon("circle-x"))
          .child(SharedString::from(errors.to_string())),
      )
    })
    .when(warnings > 0, |d| {
      d.child(
        div()
          .flex()
          .items_center()
          .gap(px(3.))
          .text_color(chrome.warning)
          .child(icon("triangle-alert"))
          .child(SharedString::from(warnings.to_string())),
      )
    })
    .when(errors == 0 && warnings == 0, |d| {
      d.text_color(chrome.dimmed).child("clear")
    })
    .into_any_element()
}

impl Workbench {
  /// Every problem, from both sources. The compiler's come first: they are the
  /// ones that just stopped a build.
  fn rows(&self) -> Vec<Row> {
    self
      .build
      .problems
      .iter()
      .chain(self.problems.iter())
      .enumerate()
      .map(|(index, problem)| Row {
        index,
        severity: problem.severity,
        message: problem.message.clone(),
        fix: problem.fix.clone(),
        doc_id: problem.doc_id.clone(),
        node: problem.node,
      })
      .collect()
  }

  pub(super) fn render_problems(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let chrome = theme::colors(cx);
    let problems = self.rows();
    let showing = self.bottom;

    let badge = match showing {
      Bottom::Problems => count_badge(&problems, cx),
      Bottom::Console => self.build_summary(cx),
    };
    let tabs = self.bottom_tabs(problems.len(), cx);

    div()
      .h(px(self.settings.size(Panel::Problems)))
      .flex_none()
      .flex()
      .flex_col()
      .bg(chrome.surface)
      .child(self.panel_header(Panel::Problems, Some(badge), cx))
      .child(tabs)
      .child(match showing {
        Bottom::Console => self.render_console(cx),
        Bottom::Problems => self.render_problem_list(problems, cx),
      })
  }

  /// The tab strip: which half of the pane is showing.
  fn bottom_tabs(&self, problems: usize, cx: &mut Context<Self>) -> gpui::AnyElement {
    let chrome = theme::colors(cx);
    let showing = self.bottom;
    let tab =
      |bottom: Bottom, label: &'static str, count: Option<usize>, cx: &mut Context<Self>| {
        let selected = bottom == showing;
        div()
          .id(SharedString::from(format!("bottom-{label}")))
          .flex()
          .items_center()
          .gap(px(5.))
          .px(px(9.))
          .py(px(3.))
          .rounded(px(4.))
          .text_size(px(11.))
          .when(selected, |d| {
            d.bg(chrome.accent_soft).text_color(chrome.accent)
          })
          .when(!selected, |d| d.text_color(chrome.dimmed))
          .child(label)
          .when_some(count.filter(|n| *n > 0), |d, n| {
            d.child(
              div()
                .text_size(px(10.))
                .text_color(chrome.dimmed)
                .child(SharedString::from(n.to_string())),
            )
          })
          .on_click(cx.listener(move |this, _, _window, cx| {
            this.bottom = bottom;
            cx.notify();
          }))
          .into_any_element()
      };
    div()
      .flex()
      .items_center()
      .gap(px(2.))
      .px(px(8.))
      .pb(px(4.))
      .child(tab(Bottom::Problems, "Problems", Some(problems), cx))
      .child(tab(Bottom::Console, "Console", None, cx))
      .into_any_element()
  }

  fn render_problem_list(
    &mut self,
    problems: Vec<Row>,
    cx: &mut Context<Self>,
  ) -> gpui::AnyElement {
    let chrome = theme::colors(cx);
    if problems.is_empty() {
      return div()
        .flex()
        .items_center()
        .justify_center()
        .flex_grow()
        .text_size(px(12.))
        .text_color(chrome.dimmed)
        .child("Nothing to report.")
        .into_any_element();
    }
    {
      div()
        .id("problems-list")
        .flex()
        .flex_col()
        .flex_grow()
        .overflow_y_scroll()
        .children(problems.into_iter().map(|row| {
          let color = match row.severity {
            Severity::Error => chrome.danger,
            Severity::Warning => chrome.warning,
            Severity::Info => chrome.dimmed,
          };
          let (doc_id, node) = (row.doc_id, row.node);
          let menu_doc = doc_id.clone();
          let menu_message = row.message.clone();
          div()
            .id(ElementId::Integer(row.index as u64))
            .flex()
            .items_start()
            .gap(px(8.))
            .px(px(10.))
            .py(px(6.))
            .hover(move |style| style.bg(chrome.raised))
            .child(div().text_color(color).child(icon(row.severity.icon())))
            .child(
              div()
                .flex()
                .flex_col()
                .gap(px(1.))
                .child(
                  div()
                    .text_size(px(12.))
                    .child(SharedString::from(row.message)),
                )
                .child(
                  div()
                    .text_size(px(11.))
                    .text_color(chrome.dimmed)
                    .child(SharedString::from(row.fix)),
                ),
            )
            .on_click(cx.listener(move |this, _, _window, cx| {
              this.open_document(&doc_id, cx);
              if let Some(node) = node {
                this.select_only(node, cx);
              }
            }))
            .on_mouse_down(
              MouseButton::Right,
              cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                this.open_problem_menu(&menu_doc, node, &menu_message, event.position, window, cx);
              }),
            )
        }))
        .into_any_element()
    }
  }
}
