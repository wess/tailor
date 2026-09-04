//! What a catalog knows about one component.
//!
//! This is the vocabulary a component library is described *in*, not a
//! description of any particular one — guise's tables live in `tailor-guise`,
//! behind [`crate::library::Library`], exactly where a second library's would.
//!
//! Three consumers read a spec and nothing else: the palette and inspector
//! (what you can place and what you can set), the renderer (what to build), and
//! the generator (what to print). Keeping the description declarative is what
//! stops those three from drifting — adding a prop is one line in a provider,
//! not three edits in three crates.

use crate::node::{EventSpec, Node, DEFAULT_SLOT};
use crate::props::{PropSpec, PropValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
  Layout,
  Typography,
  Controls,
  Inputs,
  Data,
  Feedback,
  Navigation,
  Charts,
  Media,
  /// The `ai/` module: a transcript and the parts of one.
  Ai,
  /// Components you built in this project.
  Project,
}

impl Category {
  pub const ALL: &'static [Category] = &[
    Category::Layout,
    Category::Typography,
    Category::Controls,
    Category::Inputs,
    Category::Data,
    Category::Feedback,
    Category::Navigation,
    Category::Charts,
    Category::Media,
    Category::Ai,
    Category::Project,
  ];

  pub fn label(self) -> &'static str {
    match self {
      Category::Layout => "Layout",
      Category::Typography => "Typography",
      Category::Controls => "Controls",
      Category::Inputs => "Inputs",
      Category::Data => "Data",
      Category::Feedback => "Feedback",
      Category::Navigation => "Navigation",
      Category::Charts => "Charts",
      Category::Media => "Media",
      Category::Ai => "AI",
      Category::Project => "Project",
    }
  }

  /// A Lucide name for the section header in the palette.
  pub fn icon(self) -> &'static str {
    match self {
      Category::Layout => "layout-dashboard",
      Category::Typography => "type",
      Category::Controls => "mouse-pointer-click",
      Category::Inputs => "text-cursor-input",
      Category::Data => "table",
      Category::Feedback => "bell",
      Category::Navigation => "compass",
      Category::Charts => "chart-line",
      Category::Media => "image",
      Category::Ai => "sparkles",
      Category::Project => "package",
    }
  }
}

/// How the component's constructor is called.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ctor {
  /// `Type::new()`
  Unit,
  /// `Type::new("node-3")` — components that need a stable element id.
  Id,
  /// `Type::new("node-3", <prop>)`
  IdAnd(&'static str),
  /// `Type::new(<prop>)`
  Arg(&'static str),
  /// `Type::new(<prop>, <prop>, ..)` — components whose constructor takes the
  /// two or three facts that make them mean anything (`AIMessage::new(role,
  /// body)`), rather than one plus a chain of setters.
  Args(&'static [&'static str]),
  /// A gpui entity: `cx.new(Type::new)`. The host gets a field for it, which
  /// is exactly how these components are used in a hand-written app.
  Entity,
  /// `cx.new(|cx| Type::new(cx, <prop>))`
  EntityArg(&'static str),
  /// An entity whose constructor does not take a context:
  /// `cx.new(|_| Type::new(<prop>))`.
  EntityValue(&'static str),
  /// Not one call — the renderer and the generator special-case it by kind.
  Special,
}

impl Ctor {
  pub fn is_entity(self) -> bool {
    matches!(
      self,
      Ctor::Entity | Ctor::EntityArg(_) | Ctor::EntityValue(_)
    )
  }
}

/// A region that holds children.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotSpec {
  pub key: &'static str,
  pub label: &'static str,
  /// Holds at most one child — a `Panel`'s icon, an `Indicator`'s target.
  pub single: bool,
  /// Printed as `.method(child)`. The default slot uses `.child(..)`.
  pub method: &'static str,
}

pub const CHILDREN: SlotSpec = SlotSpec {
  key: DEFAULT_SLOT,
  label: "Children",
  single: false,
  method: "child",
};

pub const fn slot(key: &'static str, label: &'static str, method: &'static str) -> SlotSpec {
  SlotSpec {
    key,
    label,
    single: true,
    method,
  }
}

/// Slots that come from a prop's item list — a tab per tab title.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicSlots {
  /// The `Items` prop whose length decides how many slots there are.
  pub from_prop: &'static str,
  /// Slot keys are `<prefix>:<index>`.
  pub prefix: &'static str,
  /// The builder call each slot becomes — `.tab(label, ..)`, `.item(..)`.
  /// Empty when the component takes its regions some other way and the
  /// generator special-cases it (`SettingsView`, whose pages are declared
  /// apart from the one closure that fills them).
  pub method: &'static str,
}

/// A slot resolved against a specific node — dynamic slots have real labels
/// only once you know the node's items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotRef {
  pub key: String,
  pub label: String,
  pub single: bool,
}

pub struct ComponentSpec {
  /// The catalog key, as it appears in the file format.
  pub kind: &'static str,
  /// The name shown in the palette and the layers tree.
  pub title: &'static str,
  /// The library type the generator prints. Empty for `Ctor::Special` kinds
  /// that are not a single type (a frame is a `div`).
  pub rust: &'static str,
  pub category: Category,
  /// A Lucide name for the palette row.
  pub icon: &'static str,
  /// One line, shown under the name in the palette.
  pub blurb: &'static str,
  pub ctor: Ctor,
  pub props: &'static [PropSpec],
  pub slots: &'static [SlotSpec],
  pub dynamic: Option<DynamicSlots>,
  pub events: &'static [EventSpec],
  /// Applied when the component is first placed — the sensible starting size
  /// or padding that makes a fresh drop look like something.
  pub on_place: Option<fn(&mut Node)>,
  /// Extra `use` lines the generated file needs. The library's prelude covers
  /// almost everything; guise's `flex/` is the exception, because its names
  /// deliberately overlap `layout/` and it is not glob-exported.
  pub imports: &'static [&'static str],
  /// Props the component is pointless without, and the fix to suggest. A
  /// `Button` with no label is a rectangle; the linter says so.
  ///
  /// Declared here rather than as a list of kinds in the linter, because which
  /// components those are is a fact about the library.
  pub required: &'static [(&'static str, &'static str)],

  // --- What an agent needs that a person reading the palette does not ---
  //
  // A person picks a component from a row of icons and a one-line blurb, and
  // finds out the rest by dropping one on the canvas and looking. An agent
  // gets one shot from a text description, so the difference between the
  // right component and a plausible wrong one has to be written down.
  /// When to reach for this rather than something adjacent, and what it is
  /// *not* for. Empty means the blurb is the whole story.
  pub docs: &'static str,
  /// One idiomatic snippet, as the generator would print it.
  pub example: &'static str,
  /// Other names for this thing. A person scanning a palette recognises a
  /// "Combobox" on sight; an agent asked for a "dropdown" or a "typeahead"
  /// has to be able to find it.
  pub aliases: &'static [&'static str],
}

impl ComponentSpec {
  pub fn prop(&self, key: &str) -> Option<&PropSpec> {
    self.props.iter().find(|p| p.key == key)
  }

  /// The default for a prop, whether or not the node has set it.
  pub fn default_prop(&self, key: &str) -> Option<PropValue> {
    self.prop(key).map(|p| p.default_value())
  }

  pub fn is_container(&self) -> bool {
    !self.slots.is_empty() || self.dynamic.is_some()
  }

  /// Whether the default `children` slot exists — what a canvas drop asks.
  pub fn takes_children(&self) -> bool {
    self.slots.iter().any(|s| s.key == DEFAULT_SLOT)
  }

  pub fn slot_spec(&self, key: &str) -> Option<&SlotSpec> {
    self.slots.iter().find(|s| s.key == key)
  }

  /// Every slot this node has right now, static and dynamic together.
  pub fn slots_of(&self, node: &Node) -> Vec<SlotRef> {
    let mut out: Vec<SlotRef> = self
      .slots
      .iter()
      .map(|s| SlotRef {
        key: s.key.into(),
        label: s.label.into(),
        single: s.single,
      })
      .collect();
    if let Some(dynamic) = self.dynamic {
      let labels = node
        .prop(dynamic.from_prop)
        .and_then(|v| v.as_items().map(|i| i.to_vec()))
        .or_else(|| {
          self
            .default_prop(dynamic.from_prop)
            .and_then(|v| v.as_items().map(|i| i.to_vec()))
        })
        .unwrap_or_default();
      for (index, label) in labels.iter().enumerate() {
        out.push(SlotRef {
          key: format!("{}:{index}", dynamic.prefix),
          label: label.clone(),
          single: false,
        });
      }
    }
    out
  }

  /// A node of this kind, with the catalog's placement defaults applied.
  pub fn build(&self, id: crate::id::NodeId) -> Node {
    let mut node = Node::new(id, self.kind);
    if let Some(on_place) = self.on_place {
      on_place(&mut node);
    }
    node
  }
}

impl std::fmt::Debug for ComponentSpec {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("ComponentSpec")
      .field("kind", &self.kind)
      .finish()
  }
}

/// A spec with every optional field empty. [`comp!`] fills in the rest.
///
/// `const`, so a provider's tables are `static` data with no start-up cost.
pub const fn base(
  kind: &'static str,
  title: &'static str,
  rust: &'static str,
  category: Category,
  icon: &'static str,
  blurb: &'static str,
  ctor: Ctor,
) -> ComponentSpec {
  ComponentSpec {
    kind,
    title,
    rust,
    category,
    icon,
    blurb,
    ctor,
    props: &[],
    slots: &[],
    dynamic: None,
    events: &[],
    on_place: None,
    imports: &[],
    required: &[],
    docs: "",
    example: "",
    aliases: &[],
  }
}

/// Declare a component: the seven positional facts, then any field that
/// differs from [`base`].
///
/// Exported because providers live in their own crates — this macro is most of
/// what writing one looks like, and a ninety-row table stays scannable only if
/// the interesting part of each row is what it *sets*.
#[macro_export]
macro_rules! comp {
    (
        $kind:literal, $title:literal, $rust:literal, $cat:ident, $icon:literal, $blurb:literal,
        $ctor:expr $(, $field:ident: $value:expr )* $(,)?
    ) => {{
        #[allow(unused_mut)]
        let mut spec = $crate::catalog::base(
            $kind, $title, $rust, $crate::catalog::Category::$cat, $icon, $blurb, $ctor,
        );
        $( spec.$field = $value; )*
        spec
    }};
}
