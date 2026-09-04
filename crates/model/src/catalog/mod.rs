//! The component catalog — every guise component Tailor can place.
//!
//! One table, read by four consumers: the palette lists it, the inspector
//! builds a control per prop, the renderer builds the real component, and the
//! generator prints it. Adding a component is an entry here plus an arm in
//! `tailor-render`; everything else follows.
//!
//! Entries are declared with the `comp!` macro below, which starts from
//! [`base`] and overwrites only the fields that differ. That keeps a ninety-row
//! table scannable — the interesting part of each row is what it *sets*.

pub mod spec;

pub use spec::{Category, ComponentSpec, Ctor, DynamicSlots, SlotRef, SlotSpec, CHILDREN};

use std::sync::OnceLock;

/// A spec with every optional field empty. `comp!` fills in the rest.
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
  }
}

/// Declare a component: the seven positional facts, then any field that
/// differs from [`base`].
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

mod ai;
mod charts;
mod controls;
mod data;
mod feedback;
mod inputs;
mod layout;
mod media;
mod nav;
mod typography;

fn registry() -> &'static Vec<&'static ComponentSpec> {
  static REGISTRY: OnceLock<Vec<&'static ComponentSpec>> = OnceLock::new();
  REGISTRY.get_or_init(|| {
    let groups: [&'static [ComponentSpec]; 10] = [
      layout::SPECS,
      typography::SPECS,
      controls::SPECS,
      inputs::SPECS,
      data::SPECS,
      feedback::SPECS,
      nav::SPECS,
      charts::SPECS,
      media::SPECS,
      ai::SPECS,
    ];
    groups.into_iter().flatten().collect()
  })
}

/// Every component, in palette order.
pub fn all() -> &'static [&'static ComponentSpec] {
  registry()
}

pub fn get(kind: &str) -> Option<&'static ComponentSpec> {
  registry().iter().copied().find(|spec| spec.kind == kind)
}

/// The specs in one category.
pub fn in_category(category: Category) -> Vec<&'static ComponentSpec> {
  registry()
    .iter()
    .copied()
    .filter(|spec| spec.category == category)
    .collect()
}

/// Palette search: matches the title, the kind, or the blurb, title first.
pub fn search(query: &str) -> Vec<&'static ComponentSpec> {
  let needle = query.trim().to_lowercase();
  if needle.is_empty() {
    return registry().to_vec();
  }
  let mut scored: Vec<(u8, &'static ComponentSpec)> = registry()
    .iter()
    .copied()
    .filter_map(|spec| {
      let title = spec.title.to_lowercase();
      if title == needle {
        Some((0, spec))
      } else if title.starts_with(&needle) {
        Some((1, spec))
      } else if title.contains(&needle) || spec.kind.contains(&needle) {
        Some((2, spec))
      } else if spec.blurb.to_lowercase().contains(&needle) {
        Some((3, spec))
      } else {
        None
      }
    })
    .collect();
  scored.sort_by_key(|(rank, spec)| (*rank, spec.title));
  scored.into_iter().map(|(_, spec)| spec).collect()
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::node::DEFAULT_SLOT;

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
  fn search_ranks_the_exact_title_first() {
    let hits = search("card");
    assert_eq!(hits[0].kind, "card");
    assert!(search("zzzznotathing").is_empty());
    assert_eq!(search("  ").len(), all().len());
  }

  #[test]
  fn the_catalog_covers_the_library() {
    // A floor, not a target: if a category is accidentally dropped from the
    // registry this catches it, and the number only ever goes up.
    assert!(all().len() >= 85, "catalog has shrunk to {}", all().len());
  }
}

/// Coverage: every guise component is either in the catalog or excluded on
/// purpose, and the exclusions are checked back against the library.
///
/// "The catalog is the single source of truth" used to be enforced by nobody,
/// so every guise release widened the gap in silence — the `ai/` module, all of
/// `settings/` and every open-state overlay went missing that way. This turns
/// that drift into a failing test: adding a component costs either a catalog
/// entry or one line here saying why not.
///
/// What it reads is `libraries/guise.surface`, generated from the published
/// crate by `cargo run -p tailor-surface`. That used to be a walk of the next
/// crate over in the same workspace; out on its own, Tailor depends on guise
/// through crates.io like anyone else, and the surface file is how a pinned
/// dependency still gets to fail the build when it grows a component.
#[cfg(test)]
mod coverage {
  use std::collections::BTreeMap;

  use super::all;
  use crate::surface;

  /// Components guise ships that the catalog deliberately does not offer, and
  /// the reason. Keep the reason honest: it is the only record of the call.
  const EXCLUDED: &[(&str, &str)] = &[
    // `flex/` is the pixel-based Flutter-style set, deliberately not
    // glob-exported because its names collide with `layout/`. The catalog
    // offers the token-based `layout/` equivalents instead, plus the two
    // (`Row`/`Column`/`Expanded`) that have no `layout/` twin.
    ("Align", "flex/ primitive; the catalog uses layout/ tokens"),
    (
      "Padding",
      "flex/ primitive; the catalog uses layout/ tokens",
    ),
    (
      "Flexible",
      "flex/ primitive; the catalog uses layout/ tokens",
    ),
    (
      "SizedBox",
      "flex/ primitive; the catalog uses layout/ tokens",
    ),
    ("Spacer", "flex/ primitive; the catalog uses layout/ tokens"),
    (
      "Positioned",
      "flex/ primitive; the catalog uses layout/ tokens",
    ),
    ("Wrap", "flex/ primitive; the catalog uses layout/ tokens"),
    // Motion is a property of a node (`MotionProps`), not something you drop.
    (
      "Animated",
      "motion is MotionProps on a node, not a component",
    ),
    (
      "Presence",
      "motion is MotionProps on a node, not a component",
    ),
    (
      "Transition",
      "motion is MotionProps on a node, not a component",
    ),
    (
      "Collapse",
      "motion is MotionProps on a node, not a component",
    ),
    // One per window, installed by the host — not canvas content.
    (
      "OverlayHost",
      "window-level plumbing the host installs once",
    ),
    ("ToastStack", "window-level plumbing the host installs once"),
    (
      "DevTools",
      "the inspector itself; Tailor embeds it directly",
    ),
    // Driven by a live `Updater`, so there is nothing to preview.
    ("UpdatePrompt", "needs a live Updater; nothing to preview"),
    ("UpdateNotice", "needs a live Updater; nothing to preview"),
    // Values and internals that are not components.
    ("Glyph", "an icon value, not a component (see Icon)"),
    ("TabDrag", "internal drag payload for PaneGroup"),
    // Two element-producing closures each, and no content of their own: what
    // a designer would lay out is the panel, and the panel is the half the
    // generator cannot synthesise from a trigger. `Tooltip` covers the case
    // that fits in one prop.
    (
      "Popover",
      "trigger and content are both 'static element closures",
    ),
    (
      "HoverCard",
      "trigger and content are both 'static element closures",
    ),
    // Guided flows over element ids that only exist in the running app.
    ("Spotlight", "steps point at live element ids"),
    ("Tour", "steps point at live element ids"),
    // The app's menu bar, wired to actions rather than to a layout.
    ("MenuBar", "app-level menus dispatch actions, not layout"),
    // Window chrome, meaningful only against a real borderless window.
    (
      "ResizeHandles",
      "window chrome; needs a real borderless window",
    ),
    // Both are generic over a payload or a collection the host owns.
    ("Draggable", "generic over a payload type the host owns"),
    ("DropTarget", "generic over a payload type the host owns"),
    ("SortableList", "reorders a list the host owns"),
    // The host owns the items; the component owns only the layout over them.
    ("PaneGroup", "the host owns the items it lays out"),
    // Wants a GpuScene assembled in code, which is the whole component.
    ("GpuView", "draws a GpuScene the host builds"),
  ];

  /// Every component type guise defines, by name.
  fn library() -> BTreeMap<String, String> {
    surface::guise().components
  }

  #[test]
  fn every_component_is_catalogued_or_excused() {
    let library = library();
    let catalogued: Vec<&str> = all().iter().map(|spec| spec.rust).collect();

    let missing: Vec<String> = library
      .iter()
      .filter(|(name, _)| !catalogued.contains(&name.as_str()))
      .filter(|(name, _)| !EXCLUDED.iter().any(|(excluded, _)| excluded == name))
      .map(|(name, where_)| format!("  {name} ({where_})"))
      .collect();

    assert!(
      missing.is_empty(),
      "{} guise component(s) are neither in the catalog nor excluded.\n\
       Add a `comp!` entry plus an arm in tailor-render, or add a line to \
       EXCLUDED saying why not:\n{}",
      missing.len(),
      missing.join("\n")
    );
  }

  #[test]
  fn the_exclusion_list_has_not_gone_stale() {
    let library = library();
    let catalogued: Vec<&str> = all().iter().map(|spec| spec.rust).collect();

    for (name, reason) in EXCLUDED {
      assert!(
        library.contains_key(*name),
        "EXCLUDED lists {name}, which guise no longer defines — drop the line"
      );
      assert!(
        !catalogued.contains(name),
        "{name} is excluded ({reason}) but the catalog offers it — drop the line"
      );
    }
  }
}
