//! The renderer for a library Tailor can describe but not draw.
//!
//! A live component is a value of the library's own gpui types, and two gpui
//! snapshots in one binary are two unrelated sets of types: an `AnyElement`
//! built by one cannot be handed to the other's window. gpui-kit is on a
//! newer snapshot than the one Tailor is built on, so it can have a catalog
//! and a generator — neither touches gpui — and cannot have a `Renderer` that
//! builds its real components.
//!
//! This is what draws it instead: each node as a labelled card, its
//! containers' slots real drop targets, so laying a screen out works exactly
//! as it does with a live library. What it cannot show is how the component
//! *looks*. Run builds the real thing, against the library's own gpui.

use std::any::Any;
use std::rc::Rc;
use std::sync::Mutex;

use gpui::prelude::*;
use gpui::{div, px, AnyElement, App, Context, Window};
use guise::prelude::*;
use tailor_model::library::Library;
use tailor_model::node::DEFAULT_SLOT;
use tailor_model::props::PropValue;
use tailor_model::{Document, Node};

use crate::nodes::slot_children;
use crate::renderer::Renderer;
use crate::store::PreviewStore;
use crate::RenderCtx;

pub struct Schematic {
  library: &'static dyn Library,
}

static CARDS: Mutex<Vec<&'static Schematic>> = Mutex::new(Vec::new());

/// The schematic renderer for a library — one per library, made on first use
/// and kept, because a `Renderer` is a `&'static` and there are only ever as
/// many as there are libraries.
pub fn for_library(library: &'static dyn Library) -> &'static dyn Renderer {
  let mut cards = CARDS.lock().expect("schematic registry");
  if let Some(found) = cards.iter().find(|card| card.library.id() == library.id()) {
    return *found;
  }
  let card: &'static Schematic = Box::leak(Box::new(Schematic { library }));
  cards.push(card);
  card
}

impl Renderer for Schematic {
  fn library(&self) -> &'static dyn Library {
    self.library
  }

  fn element(&self, ctx: &RenderCtx, node: &Node, window: &mut Window, cx: &mut App) -> AnyElement {
    let spec = self.library.get(&node.kind);
    let title = spec.map(|spec| spec.title).unwrap_or(node.kind.as_str());

    // The first text the node carries, so two buttons are tellable apart.
    let gist = spec.and_then(|spec| {
      spec
        .props
        .iter()
        .find_map(|prop| match node.prop(prop.key) {
          Some(PropValue::Text(text)) if !text.is_empty() => Some(text.clone()),
          _ => None,
        })
    });

    let border = guise::theme::theme(cx).border().alpha(0.9);
    let mut header = div()
      .flex()
      .items_center()
      .gap(px(6.))
      .child(Text::new(title.to_string()).size(Size::Sm));
    if let Some(gist) = gist {
      header = header.child(Text::new(gist).size(Size::Sm).dimmed());
    }

    let mut card = div()
      .flex()
      .flex_col()
      .gap(px(6.))
      .p(px(8.))
      .min_w(px(64.))
      .rounded(px(6.))
      .border(px(1.))
      .border_color(border)
      .child(header);

    for slot in spec.map(|spec| spec.slots).unwrap_or_default() {
      let kids = slot_children(ctx, node, slot.key, window, cx);
      if slot.key == DEFAULT_SLOT {
        card = card.child(div().flex().flex_col().gap(px(4.)).children(kids));
      } else {
        card = card.child(
          div()
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(Text::new(slot.label.to_string()).size(Size::Xs).dimmed())
            .children(kids),
        );
      }
    }
    card.into_any_element()
  }

  fn preview(
    &self,
    _node: &Node,
    _doc: &Document,
    _cx: &mut Context<PreviewStore>,
  ) -> Option<Rc<dyn Any>> {
    None
  }
}
