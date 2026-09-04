//! guise on the canvas.
//!
//! `tailor-guise` says what guise *is*; this says what one of its components
//! looks like when it is on screen. The split is the gpui dependency: listing
//! components, searching them and generating code all happen with no window in
//! sight, and `tailor-mcp` does exactly that.
//!
//! Two things live here and nowhere else. [`nodes::element`] is one arm per
//! catalog kind, building the real component from the node's props — except
//! for the containers guise takes as `'static` closures
//! ([`Library::drawn`](tailor_model::library::Library::drawn)), which are drawn
//! from the theme so their regions are real drop targets. And [`preview`] is
//! the stateful half: components that own a focus handle or a buffer cannot be
//! built inside `render`, so the canvas caches an entity per node.

pub mod nodes;
pub mod preview;
pub mod read;

use std::any::Any;
use std::rc::Rc;

use gpui::{AnyElement, App, Context, Window};
use tailor_model::library::Library;
use tailor_model::{Document, Node};
use tailor_render::{PreviewStore, RenderCtx, Renderer};

/// The renderer handle. Zero-sized: it dispatches on the node's kind and holds
/// nothing.
pub struct GuiseRenderer;

/// Make guise drawable. Call it from a binary that opens a window, after
/// [`tailor_guise::register`].
pub fn register() {
  tailor_guise::register();
  tailor_render::register(&GuiseRenderer);
}

/// The library this draws, borrowed from the description crate rather than
/// declared twice.
pub fn library() -> &'static dyn Library {
  tailor_guise::library()
}

impl Renderer for GuiseRenderer {
  fn library(&self) -> &'static dyn Library {
    library()
  }

  fn element(&self, ctx: &RenderCtx, node: &Node, window: &mut Window, cx: &mut App) -> AnyElement {
    nodes::element(ctx, node, window, cx)
  }

  fn preview(
    &self,
    node: &Node,
    doc: &Document,
    cx: &mut Context<PreviewStore>,
  ) -> Option<Rc<dyn Any>> {
    preview::build(node, doc, cx)
  }
}
