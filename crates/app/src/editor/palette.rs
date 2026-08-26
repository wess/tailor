//! The object library: every component you can place, plus the components you
//! built in this project.
//!
//! Search first, categories second — with a hundred components the list is only
//! useful if you can type at it. Rows are drag sources; clicking one drops it
//! into the selection, which is faster than dragging when you already know
//! where it goes.

use gpui::prelude::*;
use gpui::{div, px, Context, ElementId, MouseButton, MouseDownEvent, SharedString, Window};
use tailor_model::catalog::{self, Category};
use tailor_model::node::DEFAULT_SLOT;
use tailor_model::{ComponentSpec, DocKind};
use tailor_render::chrome::DragGhost;
use tailor_render::{DragPayload, DropSpot};
use tailor_store::Panel;

use super::{icon, Workbench};
use crate::theme;

impl Workbench {
  pub(super) fn render_palette(
    &mut self,
    _window: &mut Window,
    cx: &mut Context<Self>,
  ) -> impl IntoElement {
    let chrome = theme::colors(cx);
    let width = self.settings.size(Panel::Palette);
    let query = self.search.read(cx).text();
    let category = self.category;

    let specs: Vec<&'static ComponentSpec> = if query.trim().is_empty() {
      match category {
        Some(category) => catalog::in_category(category),
        None => catalog::all().to_vec(),
      }
    } else {
      catalog::search(&query)
    };

    let components: Vec<(String, String)> = self
      .project
      .docs
      .iter()
      .filter(|doc| doc.kind == DocKind::Component && doc.id != self.doc_id)
      .map(|doc| (doc.name.clone(), doc.id.clone()))
      .collect();

    div()
      .w(px(width))
      .flex_none()
      .h_full()
      .flex()
      .flex_col()
      .bg(chrome.surface)
      .child(self.panel_header(Panel::Palette, None, cx))
      .child(
        div()
          .flex()
          .flex_col()
          .gap(px(8.))
          .p(px(10.))
          .border_b(px(1.))
          .border_color(chrome.border)
          .child(self.search.clone())
          .child(
            div()
              .flex()
              .flex_wrap()
              .gap(px(4.))
              .child(self.category_chip(None, "All", cx))
              .children(
                Category::ALL
                  .iter()
                  .filter(|c| **c != Category::Project)
                  .map(|category| self.category_chip(Some(*category), category.label(), cx)),
              ),
          ),
      )
      .child(
        div()
          .id("palette-list")
          .flex()
          .flex_col()
          .flex_grow()
          .overflow_y_scroll()
          .p(px(8.))
          .gap(px(2.))
          .when(!components.is_empty(), |d| {
            d.child(section_label("This project", chrome.dimmed))
              .children(
                components
                  .into_iter()
                  .map(|(name, id)| self.component_row(name, id, cx)),
              )
          })
          .children(
            grouped(&specs, category.is_none() && query.trim().is_empty())
              .into_iter()
              .map(|(heading, specs)| {
                div()
                  .flex()
                  .flex_col()
                  .gap(px(2.))
                  .when_some(heading, |d, heading| {
                    d.child(section_label(heading, chrome.dimmed))
                  })
                  .children(specs.into_iter().map(|spec| self.palette_row(spec, cx)))
              }),
          ),
      )
  }

  fn category_chip(
    &self,
    category: Option<Category>,
    label: &'static str,
    cx: &mut Context<Self>,
  ) -> impl IntoElement {
    let chrome = theme::colors(cx);
    let selected = self.category == category;
    div()
      .id(ElementId::Name(SharedString::from(format!("cat-{label}"))))
      .px(px(7.))
      .py(px(3.))
      .rounded(px(999.))
      .text_size(px(11.))
      .when(selected, |d| {
        d.bg(chrome.accent_soft).text_color(chrome.accent)
      })
      .when(!selected, |d| d.text_color(chrome.dimmed).bg(chrome.raised))
      .child(label)
      .on_click(cx.listener(move |this, _, _window, cx| {
        this.category = if this.category == category {
          None
        } else {
          category
        };
        cx.notify();
      }))
  }

  fn palette_row(&self, spec: &'static ComponentSpec, cx: &mut Context<Self>) -> impl IntoElement {
    let chrome = theme::colors(cx);
    let kind = spec.kind.to_string();
    let for_click = kind.clone();
    let for_menu = kind.clone();
    let label = spec.title.to_string();
    div()
      .id(ElementId::Name(SharedString::from(format!(
        "spec-{}",
        spec.kind
      ))))
      .flex()
      .flex_row()
      .items_center()
      .gap(px(9.))
      .px(px(8.))
      .py(px(6.))
      .rounded(px(6.))
      .text_color(chrome.text)
      .hover(move |style| style.bg(chrome.raised))
      .child(div().text_color(chrome.dimmed).child(icon(spec.icon)))
      .child(
        div()
          .flex()
          .flex_col()
          .child(
            div()
              .text_size(px(12.))
              .child(SharedString::from(spec.title)),
          )
          .child(
            div()
              .text_size(px(10.))
              .text_color(chrome.dimmed)
              .child(SharedString::from(spec.blurb)),
          ),
      )
      .on_drag(DragPayload::New(kind), {
        let weak = cx.entity().downgrade();
        move |_, _, _, cx| {
          weak
            .update(cx, |this, cx| {
              this.placing = true;
              cx.notify();
            })
            .ok();
          cx.new(|_| DragGhost {
            label: SharedString::from(label.clone()),
          })
        }
      })
      .on_click(cx.listener(move |this, _, _window, cx| {
        this.place_into_selection(&for_click, cx);
      }))
      .on_mouse_down(
        MouseButton::Right,
        cx.listener(move |this, event: &MouseDownEvent, window, cx| {
          cx.stop_propagation();
          this.open_palette_menu(&for_menu, event.position, window, cx);
        }),
      )
  }

  fn component_row(
    &self,
    name: String,
    doc_id: String,
    cx: &mut Context<Self>,
  ) -> impl IntoElement {
    let chrome = theme::colors(cx);
    let payload = DragPayload::Component(name.clone());
    let label = name.clone();
    let for_click = format!("@{name}");
    let menu_name = name.clone();
    let menu_doc = doc_id.clone();
    div()
      .id(ElementId::Name(SharedString::from(format!(
        "comp-{doc_id}"
      ))))
      .flex()
      .items_center()
      .gap(px(9.))
      .px(px(8.))
      .py(px(6.))
      .rounded(px(6.))
      .hover(move |style| style.bg(chrome.raised))
      .child(div().text_color(chrome.accent).child(icon("package")))
      .child(div().text_size(px(12.)).child(SharedString::from(name)))
      .on_drag(payload, {
        let weak = cx.entity().downgrade();
        move |_, _, _, cx| {
          weak
            .update(cx, |this, cx| {
              this.placing = true;
              cx.notify();
            })
            .ok();
          cx.new(|_| DragGhost {
            label: SharedString::from(label.clone()),
          })
        }
      })
      .on_click(cx.listener(move |this, _, _window, cx| {
        this.place_into_selection(&for_click, cx);
      }))
      .on_mouse_down(
        MouseButton::Right,
        cx.listener(move |this, event: &MouseDownEvent, window, cx| {
          cx.stop_propagation();
          this.open_component_menu(&menu_name, &menu_doc, event.position, window, cx);
        }),
      )
  }

  /// Drop a component into whatever is selected — or into the selection's
  /// parent when the selection cannot hold children.
  pub fn place_into_selection(&mut self, kind: &str, cx: &mut Context<Self>) {
    let Some(doc) = self.doc() else { return };
    let root = doc.root;
    let target = self.selection.first().copied().unwrap_or(root);
    let accepts = doc
      .node(target)
      .and_then(|node| catalog::get(&node.kind))
      .map(|spec| spec.takes_children())
      .unwrap_or(false);

    let spot = if accepts {
      let index = doc
        .node(target)
        .map(|node| node.children().len())
        .unwrap_or(0);
      DropSpot::at(target, DEFAULT_SLOT, index)
    } else {
      match doc.parent_of(target) {
        Some((parent, slot, index)) => DropSpot::at(parent, slot, index + 1),
        None => DropSpot::at(root, DEFAULT_SLOT, usize::MAX),
      }
    };
    self.insert_kind(kind, spot, cx);
  }
}

fn section_label(text: impl Into<SharedString>, color: gpui::Hsla) -> impl IntoElement {
  div()
    .pt(px(10.))
    .pb(px(3.))
    .px(px(8.))
    .text_size(px(10.))
    .text_color(color)
    .child(text.into())
}

/// Group by category when nothing is filtering the list; otherwise leave the
/// search's ranking alone.
fn grouped(
  specs: &[&'static ComponentSpec],
  by_category: bool,
) -> Vec<(Option<&'static str>, Vec<&'static ComponentSpec>)> {
  if !by_category {
    return vec![(None, specs.to_vec())];
  }
  let mut out: Vec<(Option<&'static str>, Vec<&'static ComponentSpec>)> = Vec::new();
  for category in Category::ALL {
    if *category == Category::Project {
      continue;
    }
    let group: Vec<&'static ComponentSpec> = specs
      .iter()
      .copied()
      .filter(|spec| spec.category == *category)
      .collect();
    if !group.is_empty() {
      out.push((Some(category.label()), group));
    }
  }
  out
}
