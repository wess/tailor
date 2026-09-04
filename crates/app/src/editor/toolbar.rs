//! The toolbar and the status bar.
//!
//! The toolbar carries what you reach for without thinking — undo, the canvas
//! mode, the device — and the status bar carries what you want to glance at:
//! what is selected, how many problems there are, and whether the file is
//! saved.

use gpui::prelude::*;
use gpui::{div, px, Context, ElementId, SharedString};
use guise::prelude::*;
use tailor_model::PRESETS;
use tailor_store::{CanvasMode, Panel};

use super::{icon, Workbench};
use crate::theme;

impl Workbench {
  pub(super) fn render_toolbar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let chrome = theme::colors(cx);
    let mode = self.settings.canvas_mode;
    let preset = self
      .doc()
      .map(|doc| doc.canvas.preset.clone())
      .unwrap_or_default();
    let (width, height) = self
      .doc()
      .map(|doc| (doc.canvas.width, doc.canvas.height))
      .unwrap_or((0.0, 0.0));
    let can_undo = self.history.can_undo();
    let can_redo = self.history.can_redo();
    let live_open = self.live.is_some();
    let settings = self.settings.clone();

    div()
      .flex()
      .flex_row()
      .items_center()
      .justify_between()
      // Roomy on purpose: the toolbar is the one row that is always there,
      // and a cramped one makes the whole window feel cramped. 52 is the
      // height a 28px control sits in with air above and below it.
      .h(px(52.))
      .px(px(16.))
      .gap(px(14.))
      .bg(chrome.surface)
      .border_b(px(1.))
      .border_color(chrome.border)
      // Left: the file and the history.
      .child(
        div()
          .flex()
          .flex_none()
          .items_center()
          .gap(px(3.))
          .child(
            self.tool_button("save", "save", "Save", true, cx, |this, window, cx| {
              this.save(window, cx)
            }),
          )
          .child(self.tool_button(
            "undo",
            "undo-2",
            "Undo",
            can_undo,
            cx,
            |this, window, cx| this.undo(window, cx),
          ))
          .child(self.tool_button(
            "redo",
            "redo-2",
            "Redo",
            can_redo,
            cx,
            |this, window, cx| this.redo(window, cx),
          ))
          .child(divider(chrome.border))
          .child(self.tool_button(
            "embed",
            "group",
            "Embed in frame",
            !self.selection.is_empty(),
            cx,
            |this, window, cx| this.embed_frame(window, cx),
          ))
          .child(self.tool_button(
            "unwrap",
            "ungroup",
            "Unwrap",
            !self.selection.is_empty(),
            cx,
            |this, window, cx| this.unwrap_selection(window, cx),
          )),
      )
      // Middle: what the canvas is showing. The one group allowed to shrink —
      // on a narrow window the device presets truncate, rather than pushing
      // the panel toggles off the right edge where nobody can reach them.
      .child(
        div()
          .flex()
          .min_w(px(0.))
          .overflow_hidden()
          .items_center()
          .gap(px(10.))
          .child(
            div()
              .flex()
              .flex_none()
              .gap(px(1.))
              .p(px(2.))
              .rounded(px(7.))
              .bg(chrome.raised)
              .children(CanvasMode::ALL.iter().map(|option| {
                let option = *option;
                let selected = option == mode;
                div()
                  .id(ElementId::Name(SharedString::from(format!(
                    "mode-{}",
                    option.label()
                  ))))
                  .flex()
                  .items_center()
                  .gap(px(6.))
                  .px(px(11.))
                  .py(px(5.))
                  .rounded(px(6.))
                  .when(selected, |d| d.bg(chrome.surface))
                  .text_size(px(12.))
                  .text_color(if selected { chrome.text } else { chrome.dimmed })
                  .child(icon(option.icon()))
                  .child(SharedString::from(option.label()))
                  .on_click(cx.listener(move |this, _, _window, cx| {
                    this.set_mode(option, cx);
                  }))
              })),
          )
          .child(divider(chrome.border))
          .child(
            div()
              .flex()
              .min_w(px(0.))
              .overflow_hidden()
              .gap(px(2.))
              .children(PRESETS.iter().map(|(name, _, _)| {
                let name = *name;
                let selected = name == preset;
                div()
                  .id(ElementId::Name(SharedString::from(format!(
                    "preset-{name}"
                  ))))
                  .px(px(10.))
                  .py(px(5.))
                  .rounded(px(6.))
                  .text_size(px(12.))
                  .when(selected, |d| d.bg(chrome.raised))
                  .text_color(if selected { chrome.text } else { chrome.dimmed })
                  .child(SharedString::from(name))
                  .on_click(cx.listener(move |this, _, _window, cx| {
                    this.set_preset(name, cx);
                  }))
              })),
          )
          .child(div().flex_none().child(self.tool_button(
            "rotate",
            "rotate-cw",
            "Rotate device",
            true,
            cx,
            |this, window, cx| this.toggle_orientation(window, cx),
          )))
          .child(
            div()
              .flex_none()
              .text_size(px(11.))
              .text_color(chrome.dimmed)
              .child(SharedString::from(format!("{width:.0} × {height:.0}"))),
          ),
      )
      // Right: run, the live window, the export, and the panels. Fixed —
      // these are the controls you must always be able to hit.
      .child(
        div()
          .flex()
          .flex_none()
          .items_center()
          .gap(px(6.))
          .child(self.chip(
            "live",
            "monitor-play",
            "Live",
            live_open,
            "Open a live window that follows every edit",
            cx,
            |this, window, cx| this.open_live_window(window, cx),
          ))
          .child(self.run_button(cx))
          .child(self.chip(
            "export",
            "file-code-2",
            "Export",
            false,
            "Write the project out as a crate",
            cx,
            |this, window, cx| this.export_code(window, cx),
          ))
          .child(divider(chrome.border))
          .child(self.panel_toggle(
            "p-library",
            "panel-left",
            "Library",
            settings.palette_open,
            cx,
            |this, window, cx| this.toggle_palette(window, cx),
          ))
          .child(self.panel_toggle(
            "p-outline",
            "list-tree",
            "Outline",
            settings.outline_open,
            cx,
            |this, window, cx| this.toggle_outline(window, cx),
          ))
          .child(self.panel_toggle(
            "p-problems",
            "triangle-alert",
            "Problems",
            settings.problems_open,
            cx,
            |this, window, cx| this.toggle_problems(window, cx),
          ))
          .child(self.panel_toggle(
            "p-inspector",
            "panel-right",
            "Inspector",
            settings.inspector_open,
            cx,
            |this, window, cx| this.toggle_inspector(window, cx),
          )),
      )
  }

  /// A square icon button in the toolbar.
  fn tool_button(
    &self,
    id: &'static str,
    glyph: &'static str,
    label: &'static str,
    enabled: bool,
    cx: &mut Context<Self>,
    action: fn(&mut Workbench, &mut gpui::Window, &mut Context<Workbench>),
  ) -> impl IntoElement {
    let chrome = theme::colors(cx);
    div()
      .id(id)
      .flex()
      .items_center()
      .justify_center()
      .size(px(30.))
      .rounded(px(6.))
      .text_color(if enabled {
        chrome.dimmed
      } else {
        chrome.border
      })
      .when(enabled, |d| {
        d.hover(move |style| style.bg(chrome.raised).text_color(chrome.text))
      })
      .child(icon(glyph))
      .tooltip(tooltip(label))
      .when(enabled, |d| {
        d.on_click(cx.listener(move |this, _, window, cx| action(this, window, cx)))
      })
  }

  /// A toolbar toggle that shows whether its panel is open.
  fn panel_toggle(
    &self,
    id: &'static str,
    glyph: &'static str,
    label: &'static str,
    on: bool,
    cx: &mut Context<Self>,
    action: fn(&mut Workbench, &mut gpui::Window, &mut Context<Workbench>),
  ) -> impl IntoElement {
    let chrome = theme::colors(cx);
    div()
      .id(id)
      .flex()
      .items_center()
      .justify_center()
      .size(px(30.))
      .rounded(px(6.))
      .when(on, |d| d.bg(chrome.raised).text_color(chrome.text))
      .when(!on, |d| d.text_color(chrome.dimmed))
      .child(icon(glyph))
      .tooltip(tooltip(label))
      .on_click(cx.listener(move |this, _, window, cx| action(this, window, cx)))
  }

  pub(super) fn render_status(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let chrome = theme::colors(cx);
    let (errors, warnings, _) = tailor_model::lint::counts(&self.problems);
    let selection = match self.selection.len() {
      0 => "Nothing selected".to_string(),
      1 => self
        .doc()
        .and_then(|doc| doc.node(self.selection[0]))
        .map(|node| tailor_render::nodes::label_of(self.library(), node))
        .unwrap_or_else(|| "1 node".into()),
      n => format!("{n} nodes"),
    };
    let title = self.title();
    let path = self
      .path
      .as_ref()
      .map(|path| path.display().to_string())
      .unwrap_or_else(|| "Not saved yet".into());
    let nodes = self.doc().map(|doc| doc.nodes.len()).unwrap_or(0);

    div()
      .flex()
      .flex_row()
      .items_center()
      .justify_between()
      .h(px(28.))
      .px(px(16.))
      .gap(px(12.))
      .bg(chrome.surface)
      .border_t(px(1.))
      .border_color(chrome.border)
      .text_size(px(11.))
      .text_color(chrome.dimmed)
      .child(
        div()
          .flex()
          .gap(px(12.))
          .child(SharedString::from(selection))
          .child(SharedString::from(format!("{nodes} nodes"))),
      )
      .child(
        div()
          .flex()
          .items_center()
          .gap(px(12.))
          .when(errors > 0, |d| {
            d.child(
              div()
                .id("status-errors")
                .flex()
                .items_center()
                .gap(px(4.))
                .text_color(chrome.danger)
                .child(icon("circle-x"))
                .child(SharedString::from(errors.to_string()))
                .on_click(cx.listener(|this, _, _window, cx| {
                  this.settings.set_open(Panel::Problems, true);
                  this.save_settings();
                  cx.notify();
                })),
            )
          })
          .when(warnings > 0, |d| {
            d.child(
              div()
                .flex()
                .items_center()
                .gap(px(4.))
                .text_color(chrome.warning)
                .child(icon("triangle-alert"))
                .child(SharedString::from(warnings.to_string())),
            )
          })
          .child(
            div()
              .flex()
              .gap(px(8.))
              .child(SharedString::from(title))
              .child(SharedString::from(path)),
          ),
      )
  }
}

impl Workbench {
  /// The toolbar's one button shape: an icon, a word, and a soft background
  /// when it is on. Everything on the right-hand side is one of these, which
  /// is what stops the bar reading as three different toolbars.
  #[allow(clippy::too_many_arguments)]
  fn chip(
    &self,
    id: &'static str,
    glyph: &'static str,
    label: &'static str,
    on: bool,
    hint: &'static str,
    cx: &mut Context<Self>,
    action: fn(&mut Workbench, &mut gpui::Window, &mut Context<Workbench>),
  ) -> impl IntoElement {
    let chrome = theme::colors(cx);
    div()
      .id(id)
      .flex()
      .items_center()
      .gap(px(6.))
      .px(px(10.))
      .py(px(6.))
      .rounded(px(6.))
      .text_size(px(12.))
      .when(on, |d| d.bg(chrome.accent_soft).text_color(chrome.accent))
      .when(!on, |d| d.text_color(chrome.dimmed))
      .hover(move |style| style.bg(chrome.raised).text_color(chrome.text))
      .child(icon(glyph))
      .child(label)
      .tooltip(tooltip(hint))
      .on_click(cx.listener(move |this, _, window, cx| action(this, window, cx)))
  }
}

fn divider(color: gpui::Hsla) -> impl IntoElement {
  div().w(px(1.)).h(px(20.)).mx(px(8.)).bg(color)
}

impl Workbench {
  /// Run, or Stop while something is running.
  ///
  /// One button rather than two, because at any moment exactly one of them is
  /// the thing you want — and a Stop that is greyed out most of the time is a
  /// worse answer than a button that changes.
  fn run_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let chrome = theme::colors(cx);
    let busy = self.build.status.busy();
    let idle = self.build.status == crate::editor::run::Status::Idle;
    let status = self.build.status.labelled(self.build.profile);

    let profile = self.build.profile;

    // No fill — a filled button in a bar of flat chips reads as pasted on.
    let mut danger_wash = chrome.danger;
    danger_wash.a = 0.14;

    div()
      .flex()
      .flex_none()
      .items_center()
      .gap(px(4.))
      .child(
        div()
          .id("run")
          .flex()
          .items_center()
          .gap(px(6.))
          .px(px(10.))
          .py(px(6.))
          .rounded(px(6.))
          .text_size(px(12.))
          .bg(chrome.accent_soft)
          .text_color(chrome.accent)
          .hover(move |style| style.bg(chrome.raised))
          .child(Icon::new(IconName::Play).size(Size::Xs))
          .child(if busy { "Restart" } else { "Run" })
          .tooltip(tooltip(if profile == tailor_build::Profile::Release {
            "Build in release and run it"
          } else {
            "Build this project and run it"
          }))
          .on_click(cx.listener(|this, _, window, cx| this.run_project(window, cx))),
      )
      // Stop stands beside Run rather than replacing it: restarting is the
      // commonest thing you want while something is running, and it should
      // not cost two clicks.
      .when(busy, |d| {
        d.child(
          div()
            .id("stop")
            .flex()
            .items_center()
            .justify_center()
            .size(px(30.))
            .rounded(px(6.))
            .bg(danger_wash)
            .text_color(chrome.danger)
            .hover(move |style| style.bg(chrome.raised))
            .child(Icon::new(IconName::Square).size(Size::Xs))
            .tooltip(tooltip("Stop the running app"))
            .on_click(cx.listener(|this, _, window, cx| this.stop_project(window, cx))),
        )
      })
      .when(!idle, |d| {
        d.child(
          div()
            .id("build-status")
            .flex_none()
            .max_w(px(150.))
            .overflow_hidden()
            .text_size(px(10.))
            .text_color(match self.build.status {
              crate::editor::run::Status::Failed(_) => chrome.danger,
              crate::editor::run::Status::Running => chrome.accent,
              _ => chrome.dimmed,
            })
            .child(SharedString::from(status))
            .tooltip(tooltip("Show the console"))
            .on_click(cx.listener(|this, _, _window, cx| {
              this.bottom = crate::editor::run::Bottom::Console;
              cx.notify();
            })),
        )
      })
  }
}
