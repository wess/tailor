//! The start screen: what Tailor shows before a project is open.
//!
//! A render method on `Root` rather than a view of its own — it does exactly
//! two things, and both of them are "replace me with a workbench", which is
//! `Root`'s to do.

use gpui::prelude::*;
use gpui::{div, px, Context, ElementId, MouseButton, MouseDownEvent, SharedString};
use guise::prelude::*;

use crate::root::Root;
use crate::templates::TEMPLATES;
use crate::theme;

impl Root {
  pub(crate) fn render_start(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
    let chrome = theme::colors(cx);
    let recents = self.recents.entries.clone();

    div()
      .size_full()
      .flex()
      .flex_row()
      .child(
        // Left: what you can make.
        div()
          .flex()
          .flex_col()
          .gap(px(28.))
          .flex_grow()
          .px(px(56.))
          .py(px(56.))
          .child(
            div()
              .flex()
              .flex_col()
              .gap(px(6.))
              .child(Title::new("Tailor").order(1))
              .child(Text::new("Lay out a gpui interface, and take the Rust with you.").dimmed()),
          )
          .child(
            div()
              .flex()
              .flex_col()
              .gap(px(10.))
              .children(TEMPLATES.iter().enumerate().map(|(index, template)| {
                let build = template.build;
                div()
                  .id(ElementId::Integer(index as u64))
                  .flex()
                  .flex_row()
                  .items_center()
                  .gap(px(14.))
                  .p(px(14.))
                  .rounded(px(8.))
                  .border(px(1.))
                  .border_color(chrome.border)
                  .bg(chrome.surface)
                  .hover(move |style| style.border_color(chrome.accent))
                  .child(
                    ThemeIcon::new(crate::editor::icon(template.icon))
                      .variant(Variant::Light)
                      .size(Size::Lg),
                  )
                  .child(
                    div()
                      .flex()
                      .flex_col()
                      .gap(px(2.))
                      .child(Text::new(template.name).medium())
                      .child(Text::new(template.blurb).size(Size::Sm).dimmed()),
                  )
                  .on_click(cx.listener(move |this, _, _window, cx| {
                    this.start_project(build(), cx);
                  }))
              })),
          )
          .child(
            div().flex().gap(px(10.)).child(
              Button::new("open", "Open a project…")
                .variant(Variant::Default)
                .left_section(Icon::new(IconName::FolderOpen))
                .on_click(cx.listener(|this, _, _window, cx| {
                  this.start_browse(cx);
                })),
            ),
          ),
      )
      .child(
        // Right: what you had open.
        div()
          .w(px(360.))
          .h_full()
          .flex()
          .flex_col()
          .gap(px(8.))
          .px(px(24.))
          .py(px(56.))
          .bg(chrome.surface)
          .border_l(px(1.))
          .border_color(chrome.border)
          .child(Text::new("Recent").size(Size::Sm).dimmed())
          .child(if recents.is_empty() {
            div()
              .pt(px(8.))
              .child(Text::new("Nothing yet.").size(Size::Sm).dimmed())
              .into_any_element()
          } else {
            div()
              .flex()
              .flex_col()
              .gap(px(2.))
              .children(recents.into_iter().enumerate().map(|(index, entry)| {
                let path = entry.path.clone();
                let menu_path = entry.path.clone();
                div()
                  .id(ElementId::Name(SharedString::from(format!(
                    "recent-{index}"
                  ))))
                  .flex()
                  .flex_col()
                  .gap(px(1.))
                  .px(px(10.))
                  .py(px(8.))
                  .rounded(px(6.))
                  .hover(move |style| style.bg(chrome.raised))
                  .child(Text::new(entry.name.clone()).size(Size::Sm))
                  .child(Text::new(entry.display_path()).size(Size::Xs).dimmed())
                  .on_click(cx.listener(move |this, _, _window, cx| {
                    this.start_open(path.clone(), cx);
                  }))
                  .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                      cx.stop_propagation();
                      this.open_recent_menu(menu_path.clone(), event.position, window, cx);
                    }),
                  )
              }))
              .into_any_element()
          }),
      )
  }
}
