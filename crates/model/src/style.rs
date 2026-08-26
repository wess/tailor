//! The box and layout styling any node can carry, on top of its component props.
//!
//! This is the part of a document that is not "which component" but "how it
//! sits": the padding, the size, the fill, and — for a container — how its
//! children are arranged. It is a deliberate subset of what gpui's `Styled`
//! exposes, chosen so that every field has one unambiguous generated call.
//!
//! Defaults are omitted on save (`skip_serializing_if`), so a node that was
//! only placed and never styled writes no `style` object at all.

use crate::tokens::{AlignToken, ColorSpec, JustifyToken};
use serde::{Deserialize, Serialize};

/// How a container arranges its children. Both modes were asked for and both
/// generate real code: `Flow` is gpui's flexbox, `Absolute` is a `relative()`
/// parent whose children are `absolute()` at an offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LayoutMode {
  #[default]
  Flow,
  Absolute,
}

impl LayoutMode {
  pub const ALL: &'static [LayoutMode] = &[LayoutMode::Flow, LayoutMode::Absolute];

  /// The value as the file format writes it.
  pub fn label(self) -> &'static str {
    match self {
      LayoutMode::Flow => "flow",
      LayoutMode::Absolute => "absolute",
    }
  }

  /// The name the interface uses. Every layout program calls this free form;
  /// "absolute" is what it is, not what anyone calls it.
  pub fn title(self) -> &'static str {
    match self {
      LayoutMode::Flow => "Flow",
      LayoutMode::Absolute => "Free form",
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
  Row,
  #[default]
  Column,
}

impl Direction {
  pub const ALL: &'static [Direction] = &[Direction::Row, Direction::Column];

  pub fn label(self) -> &'static str {
    match self {
      Direction::Row => "row",
      Direction::Column => "column",
    }
  }
}

/// One axis of a node's size.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "lowercase")]
pub enum Dimension {
  /// Sized by content.
  #[default]
  Auto,
  Px(f32),
  /// `w_full()` / `h_full()`.
  Full,
  /// A flex factor: `flex_grow()` with an explicit basis of zero.
  Grow(f32),
}

impl Dimension {
  pub fn label(self) -> &'static str {
    match self {
      Dimension::Auto => "auto",
      Dimension::Px(_) => "px",
      Dimension::Full => "full",
      Dimension::Grow(_) => "grow",
    }
  }

  pub fn is_auto(self) -> bool {
    matches!(self, Dimension::Auto)
  }

  /// The pixel value, for the canvas's resize handles. A non-pixel dimension
  /// has no single number, so the handles fall back to the measured bounds.
  pub fn px(self) -> Option<f32> {
    match self {
      Dimension::Px(v) => Some(v),
      _ => None,
    }
  }
}

/// Four sides in px. Zero on every side is the default and is not written out.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Edges {
  #[serde(default, skip_serializing_if = "is_zero")]
  pub top: f32,
  #[serde(default, skip_serializing_if = "is_zero")]
  pub right: f32,
  #[serde(default, skip_serializing_if = "is_zero")]
  pub bottom: f32,
  #[serde(default, skip_serializing_if = "is_zero")]
  pub left: f32,
}

impl Edges {
  pub fn all(value: f32) -> Self {
    Edges {
      top: value,
      right: value,
      bottom: value,
      left: value,
    }
  }

  pub fn symmetric(horizontal: f32, vertical: f32) -> Self {
    Edges {
      top: vertical,
      right: horizontal,
      bottom: vertical,
      left: horizontal,
    }
  }

  pub fn is_zero(&self) -> bool {
    self.top == 0.0 && self.right == 0.0 && self.bottom == 0.0 && self.left == 0.0
  }

  /// The one value all four sides share, if they do. Lets the generator emit
  /// `.p(px(8.))` instead of four calls, and the inspector show one field.
  pub fn uniform(&self) -> Option<f32> {
    (self.top == self.right && self.right == self.bottom && self.bottom == self.left)
      .then_some(self.top)
  }

  /// The horizontal and vertical pairs, if each axis is symmetric.
  pub fn axes(&self) -> Option<(f32, f32)> {
    (self.left == self.right && self.top == self.bottom).then_some((self.left, self.top))
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShadowToken {
  #[default]
  None,
  Xs,
  Sm,
  Md,
  Lg,
  Xl,
}

impl ShadowToken {
  pub const ALL: &'static [ShadowToken] = &[
    ShadowToken::None,
    ShadowToken::Xs,
    ShadowToken::Sm,
    ShadowToken::Md,
    ShadowToken::Lg,
    ShadowToken::Xl,
  ];

  pub fn label(self) -> &'static str {
    match self {
      ShadowToken::None => "none",
      ShadowToken::Xs => "xs",
      ShadowToken::Sm => "sm",
      ShadowToken::Md => "md",
      ShadowToken::Lg => "lg",
      ShadowToken::Xl => "xl",
    }
  }

  /// The gpui method, or `None` for no shadow.
  pub fn method(self) -> Option<&'static str> {
    match self {
      ShadowToken::None => None,
      ShadowToken::Xs => Some("shadow_xs"),
      ShadowToken::Sm => Some("shadow_sm"),
      ShadowToken::Md => Some("shadow_md"),
      ShadowToken::Lg => Some("shadow_lg"),
      ShadowToken::Xl => Some("shadow_xl"),
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextAlign {
  Left,
  Center,
  Right,
}

impl TextAlign {
  pub const ALL: &'static [TextAlign] = &[TextAlign::Left, TextAlign::Center, TextAlign::Right];

  pub fn label(self) -> &'static str {
    match self {
      TextAlign::Left => "left",
      TextAlign::Center => "center",
      TextAlign::Right => "right",
    }
  }

  pub fn method(self) -> &'static str {
    match self {
      TextAlign::Left => "text_left",
      TextAlign::Center => "text_center",
      TextAlign::Right => "text_right",
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Overflow {
  #[default]
  Visible,
  Hidden,
  ScrollX,
  ScrollY,
}

impl Overflow {
  pub const ALL: &'static [Overflow] = &[
    Overflow::Visible,
    Overflow::Hidden,
    Overflow::ScrollX,
    Overflow::ScrollY,
  ];

  pub fn label(self) -> &'static str {
    match self {
      Overflow::Visible => "visible",
      Overflow::Hidden => "hidden",
      Overflow::ScrollX => "scroll x",
      Overflow::ScrollY => "scroll y",
    }
  }

  pub fn method(self) -> Option<&'static str> {
    match self {
      Overflow::Visible => None,
      Overflow::Hidden => Some("overflow_hidden"),
      Overflow::ScrollX => Some("overflow_x_scroll"),
      Overflow::ScrollY => Some("overflow_y_scroll"),
    }
  }
}

fn is_zero(v: &f32) -> bool {
  *v == 0.0
}

fn is_one(v: &f32) -> bool {
  *v == 1.0
}

fn one() -> f32 {
  1.0
}

fn is_false(v: &bool) -> bool {
  !*v
}

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
  *v == T::default()
}

/// Everything a node carries that is not a component prop.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StyleProps {
  // --- how this node arranges its children ---
  #[serde(skip_serializing_if = "is_default")]
  pub layout: LayoutMode,
  #[serde(skip_serializing_if = "is_default")]
  pub direction: Direction,
  #[serde(skip_serializing_if = "is_false")]
  pub wrap: bool,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub gap: Option<f32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub align: Option<AlignToken>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub justify: Option<JustifyToken>,

  // --- where this node sits, when its parent lays out absolutely ---
  #[serde(skip_serializing_if = "is_zero")]
  pub x: f32,
  #[serde(skip_serializing_if = "is_zero")]
  pub y: f32,

  // --- this node's own box ---
  #[serde(skip_serializing_if = "is_default")]
  pub width: Dimension,
  #[serde(skip_serializing_if = "is_default")]
  pub height: Dimension,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub min_width: Option<f32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub max_width: Option<f32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub min_height: Option<f32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub max_height: Option<f32>,
  #[serde(skip_serializing_if = "Edges::is_zero")]
  pub padding: Edges,
  #[serde(skip_serializing_if = "Edges::is_zero")]
  pub margin: Edges,

  // --- paint ---
  #[serde(skip_serializing_if = "Option::is_none")]
  pub background: Option<ColorSpec>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub text_color: Option<ColorSpec>,
  #[serde(skip_serializing_if = "is_zero")]
  pub border_width: f32,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub border_color: Option<ColorSpec>,
  #[serde(skip_serializing_if = "is_zero")]
  pub radius: f32,
  #[serde(skip_serializing_if = "is_default")]
  pub shadow: ShadowToken,
  #[serde(skip_serializing_if = "is_one")]
  pub opacity: f32,

  // --- text ---
  #[serde(skip_serializing_if = "Option::is_none")]
  pub font_size: Option<f32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub font_weight: Option<u16>,
  #[serde(skip_serializing_if = "is_false")]
  pub italic: bool,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub text_align: Option<TextAlign>,

  #[serde(skip_serializing_if = "is_default")]
  pub overflow: Overflow,
}

impl Default for StyleProps {
  fn default() -> Self {
    StyleProps {
      layout: LayoutMode::default(),
      direction: Direction::default(),
      wrap: false,
      gap: None,
      align: None,
      justify: None,
      x: 0.0,
      y: 0.0,
      width: Dimension::Auto,
      height: Dimension::Auto,
      min_width: None,
      max_width: None,
      min_height: None,
      max_height: None,
      padding: Edges::default(),
      margin: Edges::default(),
      background: None,
      text_color: None,
      border_width: 0.0,
      border_color: None,
      radius: 0.0,
      shadow: ShadowToken::None,
      opacity: one(),
      font_size: None,
      font_weight: None,
      italic: false,
      text_align: None,
      overflow: Overflow::default(),
    }
  }
}

impl StyleProps {
  pub fn is_default(&self) -> bool {
    *self == StyleProps::default()
  }

  /// Whether any number here is an infinity or a NaN. JSON cannot write
  /// those — serde turns them into `null`, which then fails to load — so
  /// they have to be caught before a save, not after.
  pub fn has_non_finite(&self) -> bool {
    let bad = |value: f32| !value.is_finite();
    let bad_opt = |value: Option<f32>| value.map(bad).unwrap_or(false);
    let bad_dim = |dim: Dimension| match dim {
      Dimension::Px(v) | Dimension::Grow(v) => bad(v),
      _ => false,
    };
    let bad_edges =
      |edges: &Edges| bad(edges.top) || bad(edges.right) || bad(edges.bottom) || bad(edges.left);
    bad(self.x)
      || bad(self.y)
      || bad(self.border_width)
      || bad(self.radius)
      || bad(self.opacity)
      || bad_opt(self.gap)
      || bad_opt(self.min_width)
      || bad_opt(self.max_width)
      || bad_opt(self.min_height)
      || bad_opt(self.max_height)
      || bad_opt(self.font_size)
      || bad_dim(self.width)
      || bad_dim(self.height)
      || bad_edges(&self.padding)
      || bad_edges(&self.margin)
  }

  /// Whether anything here needs a wrapping `div`. A component with no box
  /// styling is emitted bare, which is what keeps generated code readable.
  pub fn needs_wrapper(&self) -> bool {
    !self.padding.is_zero()
      || !self.margin.is_zero()
      || self.background.is_some()
      || self.border_width > 0.0
      || self.radius > 0.0
      || self.shadow != ShadowToken::None
      || self.opacity != 1.0
      || !self.width.is_auto()
      || !self.height.is_auto()
      || self.min_width.is_some()
      || self.max_width.is_some()
      || self.min_height.is_some()
      || self.max_height.is_some()
      || self.text_color.is_some()
      || self.font_size.is_some()
      || self.font_weight.is_some()
      || self.italic
      || self.text_align.is_some()
      || self.overflow != Overflow::Visible
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_fresh_style_writes_nothing() {
    let json = serde_json::to_string(&StyleProps::default()).unwrap();
    assert_eq!(json, "{}");
  }

  #[test]
  fn set_fields_survive_the_round_trip() {
    let style = StyleProps {
      padding: Edges::all(12.0),
      width: Dimension::Px(240.0),
      opacity: 0.5,
      layout: LayoutMode::Absolute,
      ..StyleProps::default()
    };
    let json = serde_json::to_string(&style).unwrap();
    assert_eq!(serde_json::from_str::<StyleProps>(&json).unwrap(), style);
    assert!(json.contains("padding"));
    assert!(!json.contains("margin"));
  }

  #[test]
  fn edges_collapse_when_they_can() {
    assert_eq!(Edges::all(4.0).uniform(), Some(4.0));
    assert_eq!(Edges::symmetric(8.0, 4.0).uniform(), None);
    assert_eq!(Edges::symmetric(8.0, 4.0).axes(), Some((8.0, 4.0)));
    assert_eq!(
      Edges {
        top: 1.0,
        ..Edges::all(2.0)
      }
      .axes(),
      None
    );
  }

  #[test]
  fn non_finite_numbers_are_spotted_wherever_they_hide() {
    assert!(!StyleProps::default().has_non_finite());
    for style in [
      StyleProps {
        x: f32::NAN,
        ..StyleProps::default()
      },
      StyleProps {
        gap: Some(f32::INFINITY),
        ..StyleProps::default()
      },
      StyleProps {
        width: Dimension::Px(f32::NAN),
        ..StyleProps::default()
      },
      StyleProps {
        padding: Edges::all(f32::NAN),
        ..StyleProps::default()
      },
    ] {
      assert!(style.has_non_finite());
    }
  }

  #[test]
  fn a_bare_component_needs_no_wrapper() {
    assert!(!StyleProps::default().needs_wrapper());
    let mut style = StyleProps {
      gap: Some(8.0),
      ..StyleProps::default()
    };
    assert!(
      !style.needs_wrapper(),
      "gap belongs to the container's own element"
    );
    style.radius = 4.0;
    assert!(style.needs_wrapper());
  }
}
