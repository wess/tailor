//! The generated-code panel.
//!
//! It regenerates on every edit, which is the point: you can watch a drag turn
//! into a `.gap(px(12.))` and learn the library while you use the builder.

use gpui::prelude::*;
use gpui::{div, px, Context, MouseButton, MouseDownEvent, SharedString};
use guise::prelude::*;
use tailor_model::Flavor;
use tailor_store::Panel;

use super::{icon, Workbench};
use crate::theme;

impl Workbench {
  pub(super) fn render_code(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let chrome = theme::colors(cx);
    let flavor = self.project.gen.flavor;
    let lines = self.generated.lines().count();

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
          .tooltip(tooltip("Copy this component"))
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
}
