//! What a component library adds to the generator.
//!
//! Most of generating a document is library-agnostic: a constructor, a chained
//! call per prop the catalog says was set, a `.child(..)` per slot child, a
//! handler per bound event. All of that comes off
//! [`tailor_model::library::Library`], and a provider gets it for free.
//!
//! This trait is the rest — the handful of places where the shape a library
//! wants is not one chained call. guise has about thirty: a chart that pairs
//! its numbers, a shell whose regions take closures, a settings view that
//! declares its pages and fills them from one `match`. A library with none of
//! those implements two methods and stops.
//!
//! A generator is looked up by library id, so `tailor-mcp` can export a project
//! without linking a renderer — the reason this is separate from
//! `tailor_render::Renderer` rather than one trait with everything on it.

use std::sync::RwLock;

use tailor_model::library::Library;
use tailor_model::motion::Resolved as ResolvedMotion;
use tailor_model::{Flavor, Node, Project};

use crate::file::Generated;
use crate::node::Emitter;
use crate::style::Placement;

pub trait Generator: Send + Sync + 'static {
  /// The library this generates for. Its [`Library::id`] is the key the
  /// registry stores it under.
  fn library(&self) -> &'static dyn Library;

  // -- Component shapes -----------------------------------------------------

  /// The expression for a component whose shape is not `Type::new(..)`.
  /// `None` means the generic path handles it, which is the common case.
  ///
  /// Called for every node, so it is also the hook for the primitives a
  /// document is built from — a frame is a bare `div`, and only the provider
  /// knows what a bare box is called.
  fn special(&self, em: &mut Emitter, node: &Node) -> Option<Vec<String>>;

  /// Calls for the props the catalog marks [`tailor_model::Emit::Custom`] —
  /// the lists that become one call per item rather than one call with a list.
  fn custom_props(&self, em: &mut Emitter, node: &Node) -> Vec<String> {
    let _ = (em, node);
    Vec::new()
  }

  /// Slot keys whose content is a `'static` closure rather than an element.
  fn closure_slots(&self, kind: &str) -> &'static [&'static str] {
    let _ = kind;
    &[]
  }

  /// The prop that sizes a closure region, for the regions that take their
  /// size before their content.
  fn slot_size_prop(&self, kind: &str, slot: &str) -> Option<&'static str> {
    let _ = (kind, slot);
    None
  }

  /// A component that takes its regions some other way entirely, replacing the
  /// generic slot walk rather than adding to it.
  fn slots(&self, em: &mut Emitter, node: &Node, placement: Placement) -> Option<Vec<String>> {
    let _ = (em, node, placement);
    None
  }

  /// The prop a two-way `X::bind(&entity, &signal, cx)` drives on an entity
  /// component. Binding any other prop is a one-shot read at construction.
  fn entity_bind(&self, kind: &str) -> Option<&'static str> {
    let _ = kind;
    None
  }

  /// The prop a controlled builder's `.bind(signal.binding())` drives. These
  /// have no `new` to bind after, so the binding goes in the builder chain and
  /// replaces the setter it would have driven.
  fn controlled_bind(&self, kind: &str) -> Option<&'static str> {
    let _ = kind;
    None
  }

  /// The calls that play a node's entrance. The whole animation vocabulary is
  /// the library's — curves, transition kinds, whether offsets become margins
  /// — so the generic path stops at "this node animates".
  fn motion(
    &self,
    element_id: &str,
    motion: ResolvedMotion,
    pinned: bool,
    flavor: Flavor,
  ) -> Vec<String> {
    let _ = (element_id, motion, pinned, flavor);
    Vec::new()
  }

  // -- The app around them --------------------------------------------------

  /// `theme.rs` — the theme the design was laid out against, rebuilt in the
  /// library's own vocabulary.
  fn theme_rs(&self, project: &Project) -> Generated;

  /// The line `main` calls to install that theme before opening a window.
  fn theme_init(&self) -> &'static str {
    "theme::build().init(cx);"
  }
}

// ---------------------------------------------------------------------------

static GENERATORS: RwLock<Vec<&'static dyn Generator>> = RwLock::new(Vec::new());

/// Make a generator available. Keyed by its library's id; registering the same
/// id twice keeps the first.
pub fn register(generator: &'static dyn Generator) {
  let mut generators = GENERATORS.write().expect("generator registry");
  let id = generator.library().id();
  if generators.iter().any(|g| g.library().id() == id) {
    return;
  }
  generators.push(generator);
}

pub fn get(library_id: &str) -> Option<&'static dyn Generator> {
  GENERATORS
    .read()
    .expect("generator registry")
    .iter()
    .copied()
    .find(|g| g.library().id() == library_id)
}

/// The generator for a project, falling back to the first registered when the
/// project names a library this build does not ship — the same forgiving
/// resolution the canvas uses, so a file always opens and always exports.
pub fn for_project(project: &Project) -> &'static dyn Generator {
  get(project.library().id())
    .or_else(|| {
      GENERATORS
        .read()
        .expect("generator registry")
        .first()
        .copied()
    })
    .expect("no generator is registered — call tailor_guise::register() from main")
}
