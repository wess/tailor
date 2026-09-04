//! Coverage: every guise component is either in the catalog or excluded on
//! purpose, and the exclusions are checked back against the library.
//!
//! "The catalog is the single source of truth" used to be enforced by nobody,
//! so every guise release widened the gap in silence — the `ai/` module, all of
//! `settings/` and every open-state overlay went missing that way. This turns
//! that drift into a failing test: adding a component costs either a catalog
//! entry or one line here saying why not.
//!
//! What it reads is `libraries/guise.surface`, generated from the published
//! crate by `cargo run -p tailor-surface`. That used to be a walk of the next
//! crate over in the same workspace; out on its own, Tailor depends on guise
//! through crates.io like anyone else, and the surface file is how a pinned
//! dependency still gets to fail the build when it grows a component.
use std::collections::BTreeMap;

use crate::surface;
use tailor_model::catalog::ComponentSpec;

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
fn defined() -> BTreeMap<String, String> {
  surface::guise().components
}

/// Every component Tailor catalogues.
fn catalogued() -> &'static [&'static ComponentSpec] {
  crate::library().components()
}

#[test]
fn every_component_is_catalogued_or_excused() {
  let defined = defined();
  let catalogued: Vec<&str> = catalogued().iter().map(|spec| spec.rust).collect();

  let missing: Vec<String> = defined
    .iter()
    .filter(|(name, _)| !catalogued.contains(&name.as_str()))
    .filter(|(name, _)| !EXCLUDED.iter().any(|(excluded, _)| excluded == name))
    .map(|(name, where_)| format!("  {name} ({where_})"))
    .collect();

  assert!(
    missing.is_empty(),
    "{} guise component(s) are neither in the catalog nor excluded.\n\
     Add a `comp!` entry plus an arm in tailor-guiserender, or add a line \
     to \
     EXCLUDED saying why not:\n{}",
    missing.len(),
    missing.join("\n")
  );
}

#[test]
fn the_exclusion_list_has_not_gone_stale() {
  let defined = defined();
  let catalogued: Vec<&str> = catalogued().iter().map(|spec| spec.rust).collect();

  for (name, reason) in EXCLUDED {
    assert!(
      defined.contains_key(*name),
      "EXCLUDED lists {name}, which guise no longer defines — drop the line"
    );
    assert!(
      !catalogued.contains(name),
      "{name} is excluded ({reason}) but the catalog offers it — drop the line"
    );
  }
}
