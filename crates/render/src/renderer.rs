//! What a component library adds to the canvas.
//!
//! [`tailor_model::library::Library`] describes a library without needing a
//! window; this is the half that cannot. Building a `Button` means calling
//! `Button::new`, and there is no way to do that from a table — so a provider
//! that wants to be drawn implements one big `match` over its own catalog, and
//! that match is this trait.
//!
//! It is deliberately separate from [`tailor_codegen::Generator`]: `tailor-mcp`
//! lists and exports components with no window anywhere near it, and would
//! otherwise have to link gpui to do it.

use std::any::Any;
use std::rc::Rc;
use std::sync::RwLock;

use gpui::{AnyElement, App, Context, Window};
use tailor_model::library::Library;
use tailor_model::{Document, Node};

use crate::store::PreviewStore;
use crate::RenderCtx;

pub trait Renderer: Send + Sync + 'static {
  /// The library this draws. Its [`Library::id`] is the registry key.
  fn library(&self) -> &'static dyn Library;

  /// The component for a node — the thing the canvas's chrome wraps.
  ///
  /// Called for every visible node on every frame, so it does the work and
  /// nothing else: the style box, the outline and the drop targets are already
  /// around it by the time this is called.
  fn element(&self, ctx: &RenderCtx, node: &Node, window: &mut Window, cx: &mut App) -> AnyElement;

  /// The live entity for a node that owns state, or `None` for a kind the
  /// canvas draws itself.
  ///
  /// Half of a component library is gpui entities — a text field owns a focus
  /// handle and a buffer — and an entity cannot be created inside `render`.
  /// [`PreviewStore`] keeps one per node and hands it back here; what is in
  /// the box is the provider's own type, which is why it comes back as `Any`.
  fn preview(
    &self,
    node: &Node,
    doc: &Document,
    cx: &mut Context<PreviewStore>,
  ) -> Option<Rc<dyn Any>>;
}

// ---------------------------------------------------------------------------

static RENDERERS: RwLock<Vec<&'static dyn Renderer>> = RwLock::new(Vec::new());

/// Make a renderer available to the canvas. Keyed by its library's id;
/// registering the same id twice keeps the first.
pub fn register(renderer: &'static dyn Renderer) {
  let mut renderers = RENDERERS.write().expect("renderer registry");
  let id = renderer.library().id();
  if renderers.iter().any(|r| r.library().id() == id) {
    return;
  }
  renderers.push(renderer);
}

pub fn get(library_id: &str) -> Option<&'static dyn Renderer> {
  RENDERERS
    .read()
    .expect("renderer registry")
    .iter()
    .copied()
    .find(|r| r.library().id() == library_id)
}

/// The renderer for a project, falling back to the first registered when the
/// project names a library this build does not ship. A file always opens, and
/// the lint pass is where the substitution is reported.
pub fn for_project(project: &tailor_model::Project) -> &'static dyn Renderer {
  get(project.library().id())
    .or_else(|| {
      RENDERERS
        .read()
        .expect("renderer registry")
        .first()
        .copied()
    })
    .expect("no renderer is registered — call tailor_guiserender::register() from main")
}
