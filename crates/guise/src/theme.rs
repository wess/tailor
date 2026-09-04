//! guise's prebuilt themes, as a project refers to them.
//!
//! A copy rather than a read of `guise::Theme`: this crate describes guise
//! without linking it. `presets_match_guise` in `surface` holds the copy to
//! `libraries/guise.surface` — the record of what the pinned crate actually
//! ships — so the two cannot drift in silence.

use tailor_model::project::{preset, Scheme, ThemePreset};

pub const PRESETS: &[ThemePreset] = &[
  preset("catppuccin", "Catppuccin", "catppuccin", Scheme::Dark),
  preset("nord", "Nord", "nord", Scheme::Dark),
  preset("tokyonight", "Tokyo Night", "tokyonight", Scheme::Dark),
  preset("gruvbox", "Gruvbox", "gruvbox", Scheme::Dark),
  preset("dracula", "Dracula", "dracula", Scheme::Dark),
  preset(
    "solarizedlight",
    "Solarized Light",
    "solarized_light",
    Scheme::Light,
  ),
];
