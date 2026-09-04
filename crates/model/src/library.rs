//! What a component library is, from Tailor's side.
//!
//! Tailor was built against guise, and for a long time "the catalog" and "what
//! guise ships" were the same sentence. They are not the same thing: a catalog
//! is a *description* of a library — its components, the Rust it generates, the
//! token vocabulary it spells sizes and colours in — and guise is one library
//! that can be described that way.
//!
//! This trait is that description. Everything gpui-free reads a document
//! through it: the generator, the linter, the MCP server, and the palette's
//! search. The half that needs a window — actually building the component —
//! is [`tailor_render::Renderer`], because it cannot exist without gpui and
//! `tailor-mcp` must not grow a gpui dependency to list components.
//!
//! A library is `&'static dyn Library`. Providers are compiled in rather than
//! loaded: a component is Rust that has to link, so "supporting a library"
//! means a provider crate and a release, not a plugin directory. The trait is
//! what makes that a bounded amount of work instead of a fork.
//!
//! ## Writing a provider
//!
//! Implement this over `const` tables built with the `comp!` macro, the way
//! `tailor-guise` does. Everything below the divider has a default: a provider
//! supplies facts, not behaviour, and every library searches and groups its
//! catalog identically.

use std::sync::RwLock;

use crate::catalog::{Category, ComponentSpec};
use crate::project::ThemePreset;

/// The Rust enum paths a library spells its scales with.
///
/// Tokens are Tailor's own vocabulary — a `SizeToken::Md` is "medium" in the
/// file format regardless of who renders it — but the *path* they generate is
/// the library's. guise writes `Size::Md`; another library might write
/// `Spacing::Medium` or nothing at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenPaths {
  pub size: &'static str,
  pub variant: &'static str,
  pub color: &'static str,
  pub align: &'static str,
  pub justify: &'static str,
}

pub trait Library: Send + Sync + 'static {
  /// The id a `.tailor` file stores to say which library it targets.
  fn id(&self) -> &'static str;

  /// What the UI calls it.
  fn label(&self) -> &'static str;

  /// The crates.io name, and the requirement a generated `Cargo.toml` asks
  /// for. Tailor pins what it renders with; generated code should ask for the
  /// same thing, or the export will not compile against what you previewed.
  fn krate(&self) -> &'static str;
  fn version_req(&self) -> &'static str;

  /// The `use` lines every generated file opens with.
  fn prelude(&self) -> &'static [&'static str];

  /// The catalog, in palette order.
  fn components(&self) -> &'static [&'static ComponentSpec];

  /// The prebuilt themes a project can start from.
  fn presets(&self) -> &'static [ThemePreset];

  fn token_paths(&self) -> TokenPaths;

  /// Type names a generated document must not take. A generated file
  /// glob-imports the library's prelude, so a document called `Button` would
  /// shadow the real one and every `Button::new` in the file would resolve to
  /// the wrong type. The linter refuses it rather than letting the export fail
  /// to compile.
  /// Kinds whose expression *is* a styled box, so a node's own layout and
  /// padding go straight onto it instead of into a wrapper around it.
  ///
  /// A wrapper is a new flex item: a child that was `w_full` would start
  /// measuring against the wrapper rather than the row it was in. Knowing
  /// which components are already boxes is what avoids one.
  fn boxes(&self) -> &'static [&'static str] {
    &[]
  }

  /// Kinds whose regions are `'static` closures, so the canvas draws them
  /// rather than instantiating them, and nothing that owns state can go
  /// inside one.
  ///
  /// A library fact with two readers: the renderer, which has to draw those
  /// regions from the theme to make them drop targets, and the linter, which
  /// warns before the export fails to compile.
  fn drawn(&self) -> &'static [&'static str] {
    &[]
  }

  fn reserved(&self) -> &'static [&'static str] {
    // Every component's own name, which is the whole of the prelude that a
    // document name can collide with.
    // Providers override only if the prelude exports more than components.
    &[]
  }

  // ---------------------------------------------------------------------
  // Derived. Identical for every library — a provider supplies facts above
  // this line and nothing below it.
  // ---------------------------------------------------------------------

  fn get(&self, kind: &str) -> Option<&'static ComponentSpec> {
    self.components().iter().copied().find(|s| s.kind == kind)
  }

  fn in_category(&self, category: Category) -> Vec<&'static ComponentSpec> {
    self
      .components()
      .iter()
      .copied()
      .filter(|s| s.category == category)
      .collect()
  }

  fn shadows(&self, name: &str) -> bool {
    self
      .components()
      .iter()
      .any(|s| !s.rust.is_empty() && s.rust == name)
      || self.reserved().contains(&name)
  }

  /// Palette and agent search: title, then kind, then alias, then blurb and
  /// docs. Ranked, because the caller wants the best match first — a palette
  /// shows the list, an agent takes the top of it.
  fn search(&self, query: &str) -> Vec<&'static ComponentSpec> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
      return self.components().to_vec();
    }
    let mut scored: Vec<(u8, &'static ComponentSpec)> = self
      .components()
      .iter()
      .copied()
      .filter_map(|spec| {
        let title = spec.title.to_lowercase();
        let rank = if title == needle {
          0
        } else if spec.aliases.iter().any(|a| *a == needle) {
          1
        } else if title.starts_with(&needle) {
          2
        } else if title.contains(&needle) || spec.kind.contains(&needle) {
          3
        } else if spec.aliases.iter().any(|a| a.contains(&needle)) {
          4
        } else if spec.blurb.to_lowercase().contains(&needle)
          || spec.docs.to_lowercase().contains(&needle)
        {
          5
        } else {
          return None;
        };
        Some((rank, spec))
      })
      .collect();
    scored.sort_by_key(|(rank, spec)| (*rank, spec.title));
    scored.into_iter().map(|(_, spec)| spec).collect()
  }
}

// ---------------------------------------------------------------------------
// The registry
// ---------------------------------------------------------------------------
//
// Providers are compiled in, but nothing below them may name one: `tailor-guise`
// depends on `tailor-model`, so the arrow cannot also point the other way. So a
// provider registers itself and everything else looks a library up by the id a
// `.tailor` file stores.
//
// Registration happens once, from a binary's `main`, before a project is
// opened — `tailor_guise::register()` and, where there is a window,
// `tailor_guiserender::register()`. That is also the whole list of what a host
// has to change to ship a new library.

static LIBRARIES: RwLock<Vec<&'static dyn Library>> = RwLock::new(Vec::new());

/// Make a library available to open documents. Idempotent: registering the
/// same id twice keeps the first, so a test that registers defensively and a
/// `main` that registers once do not fight.
pub fn register(library: &'static dyn Library) {
  let mut libraries = LIBRARIES.write().expect("library registry");
  if libraries.iter().any(|l| l.id() == library.id()) {
    return;
  }
  libraries.push(library);
}

/// Every registered library, in registration order.
pub fn all() -> Vec<&'static dyn Library> {
  LIBRARIES.read().expect("library registry").clone()
}

pub fn get(id: &str) -> Option<&'static dyn Library> {
  LIBRARIES
    .read()
    .expect("library registry")
    .iter()
    .copied()
    .find(|l| l.id() == id)
}

/// The library a document targets when it does not say, and what a new project
/// starts on: the first one registered.
///
/// Panics when nothing has registered. That is a wiring mistake in a binary's
/// `main`, not a runtime condition — every path that draws or generates needs
/// *some* library, and there is nothing sensible to fall back to.
pub fn default() -> &'static dyn Library {
  LIBRARIES
    .read()
    .expect("library registry")
    .first()
    .copied()
    .expect("no component library is registered — call tailor_guise::register() from main")
}

/// The library for an id, falling back to the default when the id is unknown.
///
/// A `.tailor` file naming a library this build does not ship still has to
/// open: the alternative is a project that cannot be looked at, and the lint
/// pass reports the substitution where a person will see it.
pub fn resolve(id: &str) -> &'static dyn Library {
  if id.is_empty() {
    return default();
  }
  get(id).unwrap_or_else(default)
}

/// A small library to test the model against.
///
/// The obvious thing would be to borrow `tailor-guise`, and that does not
/// work: a dev-dependency back onto this crate compiles `tailor-model` twice,
/// and a `&dyn Library` from one copy is a different type from the other's. It
/// is the better test anyway — these are the model's own rules, and asserting
/// them against a nine-component fixture says so, where asserting them against
/// guise's ninety would quietly be asserting things about guise.
#[cfg(test)]
pub mod fixture {
  use super::*;
  use crate::catalog::{slot, Ctor, CHILDREN};
  use crate::comp;
  use crate::node::{CLICK, TOGGLE};
  use crate::project::{preset, Scheme};
  use crate::props::{boolean, icon, text, Emit};

  pub struct Fixture;

  /// Register it and hand it back. Idempotent, so every test may call it.
  pub fn library() -> &'static dyn Library {
    register(&Fixture);
    &Fixture
  }

  static SPECS: &[ComponentSpec] = &[
    comp!("frame", "Frame", "", Layout, "square", "A plain box.", Ctor::Special,
      slots: &[CHILDREN]),
    comp!("stack", "Stack", "Stack", Layout, "rows", "Children in a column.", Ctor::Unit,
      slots: &[CHILDREN]),
    comp!("text", "Text", "Text", Typography, "type", "A run of text.", Ctor::Arg("content"),
      props: &[text("content", "Content", Emit::None)],
      required: &[("content", "Type into Content, or bind it to a state variable.")]),
    comp!("button", "Button", "Button", Controls, "square", "A button.", Ctor::Arg("label"),
      props: &[text("label", "Label", Emit::None)],
      events: &[CLICK],
      required: &[("label", "Set one in the Attributes inspector.")]),
    comp!("actionicon", "Action Icon", "ActionIcon", Controls, "square", "An icon button.",
    Ctor::Id,
    props: &[icon("icon", "Icon", Emit::Method("icon")), text("label", "Label", Emit::None)],
    required: &[
      ("icon", "Pick one from the icon picker."),
      ("label", "Name the action in the Attributes inspector."),
    ]),
    comp!("checkbox", "Checkbox", "Checkbox", Inputs, "check", "A checkbox.", Ctor::Id,
      props: &[boolean("checked", "Checked", Emit::Flag("checked"), false)],
      events: &[TOGGLE]),
    comp!("textinput", "Text Input", "TextInput", Inputs, "text-cursor-input",
      "A single-line field.", Ctor::Entity,
      props: &[text("value", "Value", Emit::Method("value"))]),
    comp!("tabs", "Tabs", "Tabs", Navigation, "folder", "Panels behind a bar.", Ctor::Entity,
    props: &[crate::props::items("tabs", "Tabs", Emit::Custom, || {
      crate::props::PropValue::Items(vec!["One".into(), "Two".into()])
    })],
    dynamic: Some(crate::catalog::DynamicSlots {
      from_prop: "tabs",
      prefix: "tab",
      method: "tab",
    })),
    comp!("progress", "Progress", "Progress", Feedback, "loader", "A bar.", Ctor::Unit,
    props: &[crate::props::float("value", "Value", Emit::Method("value"), || {
      crate::props::PropValue::Float(0.5)
    })]),
    comp!("panel", "Panel", "Panel", Layout, "panel-left", "A titled box.", Ctor::Id,
      slots: &[CHILDREN, slot("icon", "Icon", "icon")]),
  ];

  static REGISTRY: std::sync::OnceLock<Vec<&'static ComponentSpec>> = std::sync::OnceLock::new();

  static PRESETS: &[ThemePreset] = &[
    preset("dracula", "Dracula", "dracula", Scheme::Dark),
    preset("nord", "Nord", "nord", Scheme::Dark),
    preset("daylight", "Daylight", "daylight", Scheme::Light),
  ];

  impl Library for Fixture {
    fn id(&self) -> &'static str {
      "fixture"
    }
    fn label(&self) -> &'static str {
      "guise"
    }
    fn krate(&self) -> &'static str {
      "guise-ui"
    }
    fn version_req(&self) -> &'static str {
      "1.6"
    }
    fn prelude(&self) -> &'static [&'static str] {
      &["use guise::prelude::*;"]
    }
    fn components(&self) -> &'static [&'static ComponentSpec] {
      REGISTRY.get_or_init(|| SPECS.iter().collect())
    }
    fn presets(&self) -> &'static [ThemePreset] {
      PRESETS
    }
    fn token_paths(&self) -> TokenPaths {
      TokenPaths {
        size: "Size",
        variant: "Variant",
        color: "ColorName",
        align: "Align",
        justify: "Justify",
      }
    }
    fn boxes(&self) -> &'static [&'static str] {
      &["frame"]
    }
    fn drawn(&self) -> &'static [&'static str] {
      &["tabs"]
    }
    fn reserved(&self) -> &'static [&'static str] {
      &["Theme", "Size"]
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_registered_library_resolves_by_id_and_an_unknown_one_falls_back() {
    let library = fixture::library();
    assert_eq!(get("fixture").map(|l| l.id()), Some("fixture"));
    assert_eq!(resolve("fixture").id(), "fixture");
    // A file naming a library this build does not ship still opens.
    assert_eq!(resolve("nonesuch").id(), default().id());
    assert_eq!(resolve("").id(), default().id());

    // Registering twice keeps one.
    register(&fixture::Fixture);
    assert_eq!(all().iter().filter(|l| l.id() == "fixture").count(), 1);
    assert!(library.get("button").is_some());
  }

  #[test]
  fn shadowing_covers_component_names_and_the_rest_of_the_prelude() {
    let library = fixture::library();
    assert!(library.shadows("Button"));
    assert!(library.shadows("Theme"));
    assert!(!library.shadows("ButtonRow"));
    // A kind with no type of its own — a frame is a `div` — must not make the
    // empty name collide with everything.
    assert!(!library.shadows(""));
  }

  #[test]
  fn search_ranks_an_exact_title_over_a_blurb_mention() {
    let library = fixture::library();
    let hits = library.search("text");
    assert_eq!(hits[0].kind, "text");
    assert!(library.search("zzzz").is_empty());
    assert_eq!(library.search("   ").len(), library.components().len());
  }
}
