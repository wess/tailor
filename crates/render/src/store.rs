//! The canvas's entity cache.
//!
//! Half of a component library is gpui entities — a text field owns a focus
//! handle and a buffer, a picker owns its open state — and an entity cannot be
//! created inside `render`. So the canvas keeps one per node, rebuilt when the
//! node's props change and dropped when the node goes away.
//!
//! What is *in* one is the provider's business: this stores an `Rc<dyn Any>`
//! that [`Renderer::preview`] made and [`Renderer::element`] downcasts back.
//! The cache policy is not the provider's, which is why it lives here.
//!
//! Rebuilding on a props hash rather than diffing field by field is a deliberate
//! trade: it costs a fresh entity on every keystroke in the inspector, and it
//! means a component can never show a stale prop. On a document of a few
//! hundred nodes that is not a cost worth optimising away.

use std::any::Any;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use gpui::{Bounds, Context, Pixels};
use tailor_model::library::Library;
use tailor_model::props::PropValue;
use tailor_model::{Document, NodeId};

use crate::Renderer;

/// A live component the canvas is holding on behalf of a node. The provider
/// put it in and the provider takes it out; nothing here looks inside.
pub type Preview = Rc<dyn Any>;

#[derive(Default)]
pub struct PreviewStore {
  entities: HashMap<NodeId, Preview>,
  signatures: HashMap<NodeId, u64>,
  /// Controlled components in preview mode — the parent owns the value, and
  /// on the canvas the parent is us.
  values: HashMap<NodeId, PropValue>,
  /// Which page of a tabbed or sectioned container is showing.
  pages: HashMap<NodeId, usize>,
  /// Where each node ended up last frame, in window coordinates. Recorded
  /// during paint, which is the only time anything knows: gpui hands an
  /// element no bounds until then. Resize handles, snapping guides, and the
  /// size readout all read from here.
  bounds: HashMap<NodeId, Bounds<Pixels>>,
}

impl PreviewStore {
  pub fn new(_cx: &mut Context<Self>) -> Self {
    PreviewStore::default()
  }

  pub fn get(&self, id: NodeId) -> Option<&Preview> {
    self.entities.get(&id)
  }

  pub fn page(&self, id: NodeId) -> usize {
    self.pages.get(&id).copied().unwrap_or(0)
  }

  pub fn set_page(&mut self, id: NodeId, page: usize, cx: &mut Context<Self>) {
    self.pages.insert(id, page);
    cx.notify();
  }

  /// The live value of a controlled component, falling back to its prop.
  pub fn value(&self, id: NodeId) -> Option<&PropValue> {
    self.values.get(&id)
  }

  pub fn set_value(&mut self, id: NodeId, value: PropValue, cx: &mut Context<Self>) {
    self.values.insert(id, value);
    cx.notify();
  }

  pub fn bounds(&self, id: NodeId) -> Option<Bounds<Pixels>> {
    self.bounds.get(&id).copied()
  }

  /// Record a node's painted bounds. Deliberately does not notify: this runs
  /// inside paint, and asking for another frame from there is a loop.
  pub fn set_bounds(&mut self, id: NodeId, bounds: Bounds<Pixels>) {
    self.bounds.insert(id, bounds);
  }

  /// Every sibling's bounds under one parent, for alignment guides.
  pub fn sibling_bounds(
    &self,
    doc: &Document,
    parent: NodeId,
    except: NodeId,
  ) -> Vec<Bounds<Pixels>> {
    doc
      .children_of(parent)
      .iter()
      .filter(|id| **id != except)
      .filter_map(|id| self.bounds.get(id).copied())
      .collect()
  }

  /// Forget everything about a document — called when the open tab changes.
  pub fn clear(&mut self) {
    self.entities.clear();
    self.signatures.clear();
    self.values.clear();
    self.pages.clear();
    self.bounds.clear();
  }

  /// Bring the cache in line with the document: build what is new, rebuild
  /// what changed, drop what is gone.
  pub fn sync(
    &mut self,
    renderer: &dyn Renderer,
    library: &dyn Library,
    doc: &Document,
    cx: &mut Context<Self>,
  ) {
    let mut live: HashSet<NodeId> = HashSet::new();
    for id in std::iter::once(doc.root).chain(doc.descendants(doc.root)) {
      let Some(node) = doc.node(id) else { continue };
      let Some(spec) = library.get(&node.kind) else {
        continue;
      };
      // Only the stateful half has an entity at all. The provider still gets
      // the last word — it returns `None` for the ones it draws itself.
      if !spec.ctor.is_entity() {
        continue;
      }
      let signature = signature(node);
      if self.signatures.get(&id) == Some(&signature) {
        live.insert(id);
        continue;
      }
      if let Some(preview) = renderer.preview(node, doc, cx) {
        self.entities.insert(id, preview);
        self.signatures.insert(id, signature);
        live.insert(id);
      }
    }
    self.entities.retain(|id, _| live.contains(id));
    self.signatures.retain(|id, _| live.contains(id));
    self.values.retain(|id, _| doc.node(*id).is_some());
    self.pages.retain(|id, _| doc.node(*id).is_some());
    self.bounds.retain(|id, _| doc.node(*id).is_some());
  }
}

/// A hash of everything that would change how the entity is built. `Debug` is
/// the canonical form here on purpose: `PropValue` derives it, it covers every
/// variant, and it costs a string per node on an edit rather than per frame.
fn signature(node: &tailor_model::Node) -> u64 {
  let mut hasher = DefaultHasher::new();
  node.kind.hash(&mut hasher);
  format!("{:?}", node.props).hash(&mut hasher);
  hasher.finish()
}

pub fn controlled_bool(store: &PreviewStore, id: NodeId, fallback: bool) -> bool {
  store
    .value(id)
    .and_then(|value| value.as_bool())
    .unwrap_or(fallback)
}

/// gpui needs a `Render` impl to hold an entity, and the store is never drawn.
impl gpui::Render for PreviewStore {
  fn render(
    &mut self,
    _window: &mut gpui::Window,
    _cx: &mut Context<Self>,
  ) -> impl gpui::IntoElement {
    gpui::Empty
  }
}
