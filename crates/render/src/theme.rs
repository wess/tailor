//! The two guise values Tailor's own chrome needs from a document.
//!
//! The canvas is a guise app, so its selection outlines, its drop strips and
//! the entrance it replays are drawn in guise like the rest of Tailor. That is
//! separate from what a *target* library does — building the component under
//! the chrome is [`crate::Renderer`], and a provider that is not guise still
//! gets its nodes wrapped by this one.
//!
//! Both mappings are also what the guise provider reads, rather than keeping a
//! second copy: the canvas and the export have to ease and shade identically.

use gpui::{App, Hsla};
use guise::prelude::*;
use tailor_model::tokens::{ColorSpec, ColorToken};

/// The palette shade a named colour resolves to. Matches `tailor-codegen`, so
/// the canvas and the export agree.
pub const SHADE: usize = 6;

/// An easing token as the guise curve it names. The generator prints the
/// same mapping as a path, so the canvas and the export ease identically.
pub fn easing(token: tailor_model::tokens::EaseToken) -> Easing {
  use tailor_model::tokens::EaseToken as E;
  match token {
    E::Linear => Easing::Linear,
    E::OutQuad => Easing::Out(Curve::Quad),
    E::OutCubic => Easing::Out(Curve::Cubic),
    E::OutQuint => Easing::Out(Curve::Quint),
    E::OutExpo => Easing::Out(Curve::Expo),
    E::OutCirc => Easing::Out(Curve::Circ),
    E::OutBack => Easing::Out(Curve::Back),
    E::OutElastic => Easing::Out(Curve::Elastic),
    E::OutBounce => Easing::Out(Curve::Bounce),
    E::InQuad => Easing::In(Curve::Quad),
    E::InCubic => Easing::In(Curve::Cubic),
    E::InExpo => Easing::In(Curve::Expo),
    E::InOutQuad => Easing::InOut(Curve::Quad),
    E::InOutCubic => Easing::InOut(Curve::Cubic),
    E::InOutSine => Easing::InOut(Curve::Sine),
    E::Spring => Easing::Spring(Spring::default()),
  }
}

/// Resolve a colour spec against the live theme.
pub fn resolve(color: &ColorSpec, cx: &App) -> Hsla {
  match color {
    ColorSpec::Named(token) => theme(cx).color(color_name_of(*token), SHADE).hsla(),
    ColorSpec::Custom(hex) => hex_or(hex, cx),
  }
}

/// A hex string as a colour, or the theme's dimmed text if it does not parse —
/// the inspector lets you type freely and a half-typed hex should not blank the
/// canvas.
pub fn hex_or(hex: &str, cx: &App) -> Hsla {
  css(hex).unwrap_or_else(|_| theme(cx).dimmed().hsla())
}

pub fn color_name_of(token: ColorToken) -> ColorName {
  match token {
    ColorToken::Dark => ColorName::Dark,
    ColorToken::Gray => ColorName::Gray,
    ColorToken::Red => ColorName::Red,
    ColorToken::Pink => ColorName::Pink,
    ColorToken::Grape => ColorName::Grape,
    ColorToken::Violet => ColorName::Violet,
    ColorToken::Indigo => ColorName::Indigo,
    ColorToken::Blue => ColorName::Blue,
    ColorToken::Cyan => ColorName::Cyan,
    ColorToken::Teal => ColorName::Teal,
    ColorToken::Green => ColorName::Green,
    ColorToken::Lime => ColorName::Lime,
    ColorToken::Yellow => ColorName::Yellow,
    ColorToken::Orange => ColorName::Orange,
  }
}
