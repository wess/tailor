//! guise, described so Tailor can draw it.
//!
//! Tailor is written in guise, but it does not *target* guise by being written
//! in it — it targets guise because of this crate. Everything Tailor knows
//! about the library goes through [`tailor_model::library::Library`] and
//! [`tailor_codegen::Generator`]: the components, the props each one takes, the
//! theme presets, the Rust the generator prints, the type names a document must
//! not shadow.
//!
//! Nothing here depends on `guise-ui`, or on gpui. A catalog is a description,
//! and a description compiles without the thing it describes. The half that
//! cannot — building a live component for the canvas — is
//! `tailor-guiserender`, which is where the gpui dependency lives so that
//! `tailor-mcp` can list and generate components without one.
//!
//! ## What a second library needs
//!
//! This crate is the worked example, and it is deliberately small: two impls
//! over `static` tables. A provider supplies facts —
//!
//! - a catalog ([`catalog`]), one `comp!` per component;
//! - the theme presets it ships ([`theme`]);
//! - the enum paths it spells sizes and colours with
//!   ([`tailor_model::library::TokenPaths`]);
//! - the `use` lines a generated file opens with, and the names it must not
//!   collide with;
//! - the handful of components whose generated shape is not one chained call
//!   ([`codegen`]).
//!
//! — and inherits the behaviour. See `docs/libraries.md`.

pub mod catalog;
pub mod codegen;
#[cfg(test)]
mod coverage;
#[cfg(test)]
mod surface;
pub mod theme;

use tailor_model::catalog::ComponentSpec;
use tailor_model::library::{Library, TokenPaths};
use tailor_model::project::ThemePreset;

/// The library handle. Zero-sized: everything it answers is `static`.
pub struct Guise;

/// guise as Tailor sees it. `&'static` because a catalog outlives every
/// document that reads it.
pub fn library() -> &'static dyn Library {
  &Guise
}

/// Make guise available to open documents, and make its generator available to
/// export them. Idempotent, so a test may call it defensively.
///
/// Call it from `main` before a project is loaded. The canvas needs one more —
/// `tailor_guiserender::register()` — which is a separate call because the
/// renderer is the half that needs a window.
pub fn register() {
  tailor_model::library::register(library());
  tailor_codegen::register(&codegen::GuiseGenerator);
}

impl Library for Guise {
  fn id(&self) -> &'static str {
    "guise"
  }

  fn label(&self) -> &'static str {
    "guise"
  }

  fn krate(&self) -> &'static str {
    "guise-ui"
  }

  fn version_req(&self) -> &'static str {
    // What Tailor renders with. Generated code asks for the same thing, or an
    // export compiles against a library that is not the one you previewed.
    // `the_generated_dependency_matches_what_tailor_renders_with` holds this
    // to Cargo.lock.
    "1.6"
  }

  fn prelude(&self) -> &'static [&'static str] {
    &["use gpui::prelude::*;", "use guise::prelude::*;"]
  }

  fn components(&self) -> &'static [&'static ComponentSpec] {
    catalog::registry()
  }

  fn presets(&self) -> &'static [ThemePreset] {
    theme::PRESETS
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
    // Four kinds whose whole behaviour is their style. A `frame` and a
    // `canvas` are a bare `div`; a `surface` is one with a fill; a `spacer` is
    // one that grows. Their layout goes straight onto them rather than into a
    // wrapper, which is one fewer flex item between a child and the row it
    // was laid out in.
    &["frame", "canvas", "surface", "spacer"]
  }

  fn drawn(&self) -> &'static [&'static str] {
    // Each of these takes its regions as `'static` closures, which a designer
    // cannot drop into. The canvas draws them from the theme instead — which
    // is also what makes their slots real drop targets — and generated code
    // uses the real component.
    &[
      "tabs",
      "accordion",
      "splitpanel",
      "appshell",
      "carousel",
      "settingsview",
      "virtuallist",
    ]
  }

  fn reserved(&self) -> &'static [&'static str] {
    // The prelude exports more than components: a document named `Theme` or
    // `Size` shadows the token type every generated file uses.
    &[
      "Signal",
      "Binding",
      "Theme",
      "Size",
      "Variant",
      "ColorName",
      "Color",
      "Align",
      "Justify",
      "Glyph",
      "IconName",
      "Window",
      "App",
    ]
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tailor_model::catalog::Ctor;
  use tailor_model::node::DEFAULT_SLOT;

  fn all() -> &'static [&'static ComponentSpec] {
    library().components()
  }

  #[test]
  fn every_kind_is_unique() {
    let mut kinds: Vec<&str> = all().iter().map(|spec| spec.kind).collect();
    let count = kinds.len();
    kinds.sort_unstable();
    kinds.dedup();
    assert_eq!(kinds.len(), count, "two catalog entries share a kind");
  }

  #[test]
  fn every_prop_key_is_unique_within_its_component() {
    for spec in all() {
      let mut keys: Vec<&str> = spec.props.iter().map(|p| p.key).collect();
      let count = keys.len();
      keys.sort_unstable();
      keys.dedup();
      assert_eq!(keys.len(), count, "{} repeats a prop key", spec.kind);
    }
  }

  #[test]
  fn constructors_name_props_that_exist() {
    for spec in all() {
      let referenced: Vec<&str> = match spec.ctor {
        Ctor::IdAnd(key) | Ctor::Arg(key) | Ctor::EntityArg(key) | Ctor::EntityValue(key) => {
          vec![key]
        }
        _ => vec![],
      };
      for key in referenced {
        assert!(
          spec.prop(key).is_some(),
          "{} constructs from unknown prop {key}",
          spec.kind
        );
      }
      if let Some(dynamic) = spec.dynamic {
        assert!(
          spec.prop(dynamic.from_prop).is_some(),
          "{} takes slots from unknown prop {}",
          spec.kind,
          dynamic.from_prop
        );
      }
    }
  }

  #[test]
  fn required_props_exist() {
    for spec in all() {
      for (key, _) in spec.required {
        assert!(
          spec.prop(key).is_some(),
          "{} requires unknown prop {key}",
          spec.kind
        );
      }
    }
  }

  #[test]
  fn containers_declare_the_default_slot_first() {
    for spec in all() {
      if let Some(index) = spec.slots.iter().position(|s| s.key == DEFAULT_SLOT) {
        assert_eq!(
          index, 0,
          "{} lists its children slot out of order",
          spec.kind
        );
      }
    }
  }

  #[test]
  fn every_drawn_container_is_a_component() {
    for kind in library().drawn() {
      assert!(
        library().get(kind).is_some(),
        "{kind} is listed as drawn but is not in the catalog"
      );
    }
  }

  #[test]
  fn an_alias_finds_its_component() {
    // The whole point of aliases: an agent asked for a "dropdown" has to land
    // on the component a person would have recognised from the palette.
    for spec in all() {
      for alias in spec.aliases {
        let hits = library().search(alias);
        assert!(
          hits.iter().any(|hit| hit.kind == spec.kind),
          "{} lists the alias {alias:?}, which finds it nothing",
          spec.kind
        );
      }
    }
  }

  #[test]
  fn no_alias_collides_with_another_components_title() {
    // An alias that is some other component's name would rank that component
    // first and quietly shadow the one claiming the alias.
    let titles: Vec<String> = all().iter().map(|s| s.title.to_lowercase()).collect();
    for spec in all() {
      for alias in spec.aliases {
        assert!(
          !titles.contains(&alias.to_lowercase()) || spec.title.eq_ignore_ascii_case(alias),
          "{} claims the alias {alias:?}, which is another component's name",
          spec.kind
        );
      }
    }
  }

  #[test]
  fn the_generated_dependency_matches_what_tailor_renders_with() {
    // Generated code asks for `version_req`; the canvas draws with whatever
    // Cargo.lock resolved. If those drift, an export compiles against a
    // library that is not the one you previewed.
    let req = library().version_req();
    let pinned = surface::guise_version();
    assert!(
      pinned.starts_with(req),
      "generated code asks for {} {req}, but Tailor renders with {pinned}",
      library().krate()
    );
  }

  #[test]
  fn search_ranks_the_exact_title_first() {
    let hits = library().search("card");
    assert_eq!(hits[0].kind, "card");
    assert!(library().search("zzzznotathing").is_empty());
    assert_eq!(library().search("  ").len(), all().len());
  }

  #[test]
  fn the_catalog_covers_the_library() {
    // A floor, not a target: if a category is accidentally dropped from the
    // registry this catches it, and the number only ever goes up.
    assert!(all().len() >= 85, "catalog has shrunk to {}", all().len());
  }

  #[test]
  fn registering_twice_keeps_one() {
    register();
    register();
    let ids: Vec<&str> = tailor_model::library::all()
      .iter()
      .map(|l| l.id())
      .collect();
    assert_eq!(ids.iter().filter(|id| **id == "guise").count(), 1);
    assert_eq!(tailor_model::library::resolve("nonesuch").id(), "guise");
  }
}
