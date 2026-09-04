//! guise's catalog: every component Tailor can place, in palette order.
//!
//! Ten tables, one per category, declared with `tailor_model::comp!`. Nothing
//! here imports guise the crate — a catalog *describes* a library, and a
//! description compiles without it. That is also why Tailor can be told about
//! a library it was not built against.
//!
//! Adding a component is an entry in the right table plus an arm in
//! `tailor-guiserender`; `coverage` fails the build until the entry exists.

use tailor_model::catalog::ComponentSpec;

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

/// The catalog, flattened once and kept for the life of the process.
///
/// Category order is palette order: the boxes you build with, then what goes
/// in them, then the things that sit on top.
pub fn registry() -> &'static [&'static ComponentSpec] {
  use std::sync::OnceLock;
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
