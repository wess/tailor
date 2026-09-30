//! gpui-kit, described so Tailor can target it.
//!
//! gpui-kit is Longbridge's component library for gpui — a styled set built on
//! an unstyled base, shipped as one facade crate. This crate is everything
//! Tailor needs to know about it that does not need a window: the components
//! and their props ([`catalog`]), the Rust each one generates ([`codegen`]),
//! and the type names a document must not shadow.
//!
//! **There is no renderer here, and that is not an omission.** gpui-kit is
//! built on `gpui-pre`, a snapshot of Zed's gpui that is a different crate from
//! the `gpui 0.2` Tailor draws with. Two gpuis in one binary means two sets of
//! `Element` and `Render` types that do not unify, so a gpui-kit component
//! cannot be handed to Tailor's canvas. The catalog and generator are
//! unaffected — generated code compiles against gpui-kit in the user's own
//! crate — and the canvas draws a placeholder for what it cannot build.
//!
//! Exports depend on `gpui-kit` alone, renamed to `gpui`: the facade
//! re-exports gpui and itself as `gpui`, so `gpui::prelude`, `gpui::div` and
//! `gpui::component::button::Button` all resolve against the one snapshot the
//! library was built with.

pub mod catalog;
pub mod codegen;

use tailor_model::catalog::ComponentSpec;
use tailor_model::library::{Library, TokenPaths};
use tailor_model::project::ThemePreset;

/// The library handle. Zero-sized: everything it answers is `static`.
pub struct Gpuikit;

pub fn library() -> &'static dyn Library {
  &Gpuikit
}

/// Make gpui-kit available to open documents and export them. Idempotent.
pub fn register() {
  tailor_model::library::register(library());
  tailor_codegen::register(&codegen::GpuikitGenerator);
}

impl Library for Gpuikit {
  fn id(&self) -> &'static str {
    "gpuikit"
  }

  fn label(&self) -> &'static str {
    "gpui-kit"
  }

  fn krate(&self) -> &'static str {
    "gpui-kit"
  }

  fn version_req(&self) -> &'static str {
    // `libraries/gpuikit.surface` is generated from the exact version the
    // surface tool pins; `the_surface_matches_the_requirement` holds this to it.
    "0.7"
  }

  fn dependencies(&self) -> Vec<String> {
    // The one dependency, under the name generated code already uses for gpui.
    vec!["gpui = { package = \"gpui-kit\", version = \"0.7\" }".into()]
  }

  fn prelude(&self) -> &'static [&'static str] {
    &[
      "use gpui::prelude::*;",
      // The root exports icons, `h_flex`/`v_flex`, the theme and the sizing
      // traits; each component lives in its own module.
      "use gpui::component::*;",
      "use gpui::component::{alert::*, avatar::*, badge::*, breadcrumb::*, button::*, \
       checkbox::*, empty::*, group_box::*, kbd::*, label::*, link::*, \
       pagination::*, progress::*, radio::*, rating::*, separator::*, skeleton::*, \
       spinner::*, stepper::*, switch::*, tab::*, tag::*};",
      // Named, because gpui-kit has a `Collapsible` trait at the root as well
      // and two globs that both offer a name make it ambiguous.
      "use gpui::component::collapsible::Collapsible;",
      "use gpui::component::input::{Input, InputState, Textarea, TextareaState};",
    ]
  }

  fn components(&self) -> &'static [&'static ComponentSpec] {
    catalog::registry()
  }

  fn presets(&self) -> &'static [ThemePreset] {
    // gpui-kit's themes are JSON sets a registry loads at run time, not
    // constructors a generator can name.
    &[]
  }

  fn token_paths(&self) -> TokenPaths {
    // Sizes are choices here (`Size::Small`), not tokens, so these are only
    // read for props the catalog does not declare — none today.
    TokenPaths {
      size: "Size",
      variant: "ButtonVariant",
      color: "ColorName",
      align: "Align",
      justify: "Justify",
    }
  }

  fn signals(&self) -> bool {
    // `Signal<T>` is guise's. gpui-kit has no equivalent the generator knows
    // how to write, so a document with state fails the lint, not the build.
    false
  }

  fn boxes(&self) -> &'static [&'static str] {
    &["frame", "hstack", "vstack", "spacer"]
  }

  fn reserved(&self) -> &'static [&'static str] {
    // What the two glob imports export besides components.
    &[
      "InputState",
      "TextareaState",
      "Theme",
      "ThemeMode",
      "ActiveTheme",
      "Size",
      "Sizable",
      "Disableable",
      "Selectable",
      "Icon",
      "IconName",
      "Root",
      "Window",
      "App",
      "Axis",
      "ButtonVariant",
      "AlertVariant",
      "TabVariant",
      "TagVariant",
      "GroupBoxVariant",
    ]
  }
}
