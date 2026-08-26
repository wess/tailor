//! The document tab strip: the screens and components in this project.

use gpui::prelude::*;
use gpui::{div, px, Context, ElementId, MouseButton, MouseDownEvent, SharedString};
use guise::prelude::*;
use tailor_model::DocKind;

use super::{icon, Workbench};
use crate::theme;

impl Workbench {
  pub(super) fn render_doc_tabs(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let chrome = theme::colors(cx);
    let active = self.doc_id.clone();
    let docs: Vec<(String, String, DocKind)> = self
      .project
      .docs
      .iter()
      .map(|doc| (doc.id.clone(), doc.name.clone(), doc.kind))
      .collect();
    let closable = docs.len() > 1;

    div()
      .flex()
      .flex_row()
      .items_center()
      .gap(px(2.))
      .h(px(34.))
      .px(px(8.))
      .bg(chrome.surface)
      .border_b(px(1.))
      .border_color(chrome.border)
      .children(docs.into_iter().map(|(id, name, kind)| {
        let selected = id == active;
        let open_id = id.clone();
        let close_id = id.clone();
        let menu_id = id.clone();
        div()
          .id(ElementId::Name(SharedString::from(format!("doc-{id}"))))
          .flex()
          .flex_row()
          .items_center()
          .gap(px(6.))
          .px(px(10.))
          .py(px(5.))
          .rounded(px(5.))
          .when(selected, |d| d.bg(chrome.raised))
          .text_color(if selected { chrome.text } else { chrome.dimmed })
          .hover(move |style| style.text_color(chrome.text))
          .child(
            Icon::new(match kind {
              DocKind::Screen => IconName::AppWindow,
              DocKind::Component => IconName::Package,
            })
            .size(Size::Xs),
          )
          .child(div().text_size(px(12.)).child(SharedString::from(name)))
          .when(selected && closable, |d| {
            d.child(
              div()
                .id(ElementId::Name(SharedString::from(format!(
                  "close-{close_id}"
                ))))
                .child(Icon::new(IconName::X).size(Size::Xs))
                .on_click(cx.listener(move |this, _, _window, cx| {
                  this.close_document(&close_id, cx);
                })),
            )
          })
          .on_click(cx.listener(move |this, _, _window, cx| {
            this.open_document(&open_id, cx);
          }))
          // Right-click acts on the tab under the pointer, not on the
          // open document, so it selects first the way the canvas does.
          .on_mouse_down(
            MouseButton::Right,
            cx.listener(move |this, event: &MouseDownEvent, window, cx| {
              cx.stop_propagation();
              this.open_document(&menu_id, cx);
              this.open_doc_menu(&menu_id, event.position, window, cx);
            }),
          )
      }))
      .child(
        div()
          .id("new-screen")
          .ml(px(4.))
          .px(px(6.))
          .py(px(4.))
          .rounded(px(5.))
          .text_color(chrome.dimmed)
          .hover(move |style| style.bg(chrome.raised).text_color(chrome.text))
          .child(icon("plus"))
          .tooltip(tooltip("New screen"))
          .on_click(cx.listener(|this, _, window, cx| this.new_screen(window, cx))),
      )
      .child(
        div()
          .id("new-component")
          .px(px(6.))
          .py(px(4.))
          .rounded(px(5.))
          .text_color(chrome.dimmed)
          .hover(move |style| style.bg(chrome.raised).text_color(chrome.text))
          .child(icon("package-plus"))
          .tooltip(tooltip("New component"))
          .on_click(cx.listener(|this, _, window, cx| this.new_component(window, cx))),
      )
  }
}
