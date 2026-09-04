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
