//! Property values and the schema that describes them.
//!
//! One `PropValue` type covers every component prop in the catalog. That is
//! what lets the inspector be generic (a control per [`PropType`]), the
//! renderer read props without knowing which component it is holding, and the
//! generator print them without a second table.

use crate::tokens::{ColorSpec, ColorToken, SizeToken, VariantToken};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", content = "v", rename_all = "lowercase")]
pub enum PropValue {
  Bool(bool),
  Int(i64),
  Float(f64),
  Text(String),
  /// A variant of a component-specific enum, held by its lowercase label.
  Choice(String),
  Color(ColorSpec),
  Size(SizeToken),
  Variant(VariantToken),
  /// A Lucide icon, held by its kebab-case name (`arrow-up`).
  Icon(String),
  /// A list of strings — `data(..)`, `options(..)`, table rows, tab titles.
  Items(Vec<String>),
  /// A numeric series for the chart components.
  Numbers(Vec<f64>),
  /// This prop reads a state variable instead of a literal. The generator
  /// emits `self.<var>.get(cx)` and the canvas reads the variable's value.
  Binding(String),
}

impl PropValue {
  pub fn as_bool(&self) -> Option<bool> {
    match self {
      PropValue::Bool(v) => Some(*v),
      _ => None,
    }
  }

  pub fn as_i64(&self) -> Option<i64> {
    match self {
      PropValue::Int(v) => Some(*v),
      PropValue::Float(v) => Some(*v as i64),
      _ => None,
    }
  }

  pub fn as_f64(&self) -> Option<f64> {
    match self {
      PropValue::Float(v) => Some(*v),
      PropValue::Int(v) => Some(*v as f64),
      _ => None,
    }
  }

  /// The text of a prop, for the props that carry text. `Choice` and `Icon`
  /// answer too — they are text in every place that reads them generically.
  pub fn as_str(&self) -> Option<&str> {
    match self {
      PropValue::Text(v) | PropValue::Choice(v) | PropValue::Icon(v) => Some(v),
      _ => None,
    }
  }

  pub fn as_size(&self) -> Option<SizeToken> {
    match self {
      PropValue::Size(v) => Some(*v),
      _ => None,
    }
  }

  pub fn as_variant(&self) -> Option<VariantToken> {
    match self {
      PropValue::Variant(v) => Some(*v),
      _ => None,
    }
  }

  pub fn as_color(&self) -> Option<&ColorSpec> {
    match self {
      PropValue::Color(v) => Some(v),
      _ => None,
    }
  }

  pub fn as_items(&self) -> Option<&[String]> {
    match self {
      PropValue::Items(v) => Some(v),
      _ => None,
    }
  }

  pub fn as_numbers(&self) -> Option<&[f64]> {
    match self {
      PropValue::Numbers(v) => Some(v),
      _ => None,
    }
  }

  /// The state variable this prop reads, if it is bound to one.
  pub fn as_binding(&self) -> Option<&str> {
    match self {
      PropValue::Binding(v) => Some(v),
      _ => None,
    }
  }

  /// Whether this holds a number JSON cannot write.
  pub fn has_non_finite(&self) -> bool {
    match self {
      PropValue::Float(v) => !v.is_finite(),
      PropValue::Numbers(v) => v.iter().any(|value| !value.is_finite()),
      _ => false,
    }
  }

  /// Whether a value is worth writing out. Defaults are dropped on save so a
  /// document diff shows what was actually changed, and generated code does
  /// not restate what the component already does.
  pub fn is_empty(&self) -> bool {
    match self {
      PropValue::Text(v) | PropValue::Choice(v) | PropValue::Icon(v) => v.is_empty(),
      PropValue::Items(v) => v.is_empty(),
      PropValue::Numbers(v) => v.is_empty(),
      PropValue::Binding(v) => v.is_empty(),
      _ => false,
    }
  }
}

/// The kind of control the inspector shows, and what the generator expects.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PropType {
  Bool,
  Int,
  Float,
  Text,
  /// A longer string; the inspector uses a text area.
  MultilineText,
  /// One of a fixed set of labels, listed by [`PropSpec::choices`].
  Choice,
  Color,
  Size,
  Variant,
  Icon,
  Items,
  Numbers,
}

/// Where a prop lands in generated code.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Emit {
  /// `.method(value)` — the common case. Skipped when the value equals the
  /// spec's default.
  Method(&'static str),
  /// `.method()` when the bool is true, nothing when it is false.
  Flag(&'static str),
  /// Consumed by the renderer and the generator by hand — slot counts, chart
  /// series, anything whose shape is not one chained call.
  Custom,
  /// Editable in the inspector, but never printed: preview-only affordances.
  None,
}

pub struct PropSpec {
  pub key: &'static str,
  pub label: &'static str,
  pub ty: PropType,
  pub emit: Emit,
  /// Labels for [`PropType::Choice`]; empty otherwise.
  pub choices: &'static [&'static str],
  /// One line under the control in the inspector.
  pub hint: &'static str,
  /// The Rust enum a `Choice` or `Color` prop resolves to — `Align`,
  /// `ColorName`, `Color`. The generator needs it to print a path; the
  /// inspector needs it to know a palette-only color picker from a free one.
  pub rust_enum: &'static str,
  default: fn() -> PropValue,
}

impl PropSpec {
  pub fn default_value(&self) -> PropValue {
    (self.default)()
  }
}

impl std::fmt::Debug for PropSpec {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("PropSpec")
      .field("key", &self.key)
      .field("ty", &self.ty)
      .finish()
  }
}

/// A bag of props, keyed by [`PropSpec::key`]. Ordered so files diff cleanly.
pub type Props = BTreeMap<String, PropValue>;

// Constructors used by the catalog tables. They read as the value they build
// (`text("Save")`), which keeps a 90-component table scannable.
pub const fn boolean(
  key: &'static str,
  label: &'static str,
  emit: Emit,
  default: bool,
) -> PropSpec {
  PropSpec {
    key,
    label,
    ty: PropType::Bool,
    emit,
    choices: &[],
    hint: "",
    rust_enum: "",
    default: if default {
      || PropValue::Bool(true)
    } else {
      || PropValue::Bool(false)
    },
  }
}

pub const fn text(key: &'static str, label: &'static str, emit: Emit) -> PropSpec {
  PropSpec {
    key,
    label,
    ty: PropType::Text,
    emit,
    choices: &[],
    hint: "",
    rust_enum: "",
    default: || PropValue::Text(String::new()),
  }
}

pub const fn multiline(key: &'static str, label: &'static str, emit: Emit) -> PropSpec {
  PropSpec {
    key,
    label,
    ty: PropType::MultilineText,
    emit,
    choices: &[],
    hint: "",
    rust_enum: "",
    default: || PropValue::Text(String::new()),
  }
}

pub const fn int(
  key: &'static str,
  label: &'static str,
  emit: Emit,
  default: fn() -> PropValue,
) -> PropSpec {
  PropSpec {
    key,
    label,
    ty: PropType::Int,
    emit,
    choices: &[],
    hint: "",
    rust_enum: "",
    default,
  }
}

pub const fn float(
  key: &'static str,
  label: &'static str,
  emit: Emit,
  default: fn() -> PropValue,
) -> PropSpec {
  PropSpec {
    key,
    label,
    ty: PropType::Float,
    emit,
    choices: &[],
    hint: "",
    rust_enum: "",
    default,
  }
}

pub const fn choice(
  key: &'static str,
  label: &'static str,
  emit: Emit,
  choices: &'static [&'static str],
  default: fn() -> PropValue,
) -> PropSpec {
  PropSpec {
    key,
    label,
    ty: PropType::Choice,
    emit,
    choices,
    hint: "",
    rust_enum: "",
    default,
  }
}

/// The `fn() -> PropValue` for a named-color default. Written out rather than
/// captured because a `const fn` cannot close over its argument.
const fn color_default(default: ColorToken) -> fn() -> PropValue {
  match default {
    ColorToken::Dark => || PropValue::Color(ColorSpec::Named(ColorToken::Dark)),
    ColorToken::Gray => || PropValue::Color(ColorSpec::Named(ColorToken::Gray)),
    ColorToken::Red => || PropValue::Color(ColorSpec::Named(ColorToken::Red)),
    ColorToken::Pink => || PropValue::Color(ColorSpec::Named(ColorToken::Pink)),
    ColorToken::Grape => || PropValue::Color(ColorSpec::Named(ColorToken::Grape)),
    ColorToken::Violet => || PropValue::Color(ColorSpec::Named(ColorToken::Violet)),
    ColorToken::Indigo => || PropValue::Color(ColorSpec::Named(ColorToken::Indigo)),
    ColorToken::Blue => || PropValue::Color(ColorSpec::Named(ColorToken::Blue)),
    ColorToken::Cyan => || PropValue::Color(ColorSpec::Named(ColorToken::Cyan)),
    ColorToken::Teal => || PropValue::Color(ColorSpec::Named(ColorToken::Teal)),
    ColorToken::Green => || PropValue::Color(ColorSpec::Named(ColorToken::Green)),
    ColorToken::Lime => || PropValue::Color(ColorSpec::Named(ColorToken::Lime)),
    ColorToken::Yellow => || PropValue::Color(ColorSpec::Named(ColorToken::Yellow)),
    ColorToken::Orange => || PropValue::Color(ColorSpec::Named(ColorToken::Orange)),
  }
}

/// A color prop whose setter takes `impl Into<ColorValue>` — a palette family
/// or any explicit color. Most components.
pub const fn color(
  key: &'static str,
  label: &'static str,
  emit: Emit,
  default: ColorToken,
) -> PropSpec {
  PropSpec {
    key,
    label,
    ty: PropType::Color,
    emit,
    choices: &[],
    hint: "",
    rust_enum: "ColorValue",
    default: color_default(default),
  }
}

/// A color prop whose setter takes `ColorName` — palette families only, so the
/// inspector hides the custom swatch.
pub const fn color_name(
  key: &'static str,
  label: &'static str,
  emit: Emit,
  default: ColorToken,
) -> PropSpec {
  PropSpec {
    key,
    label,
    ty: PropType::Color,
    emit,
    choices: &[],
    hint: "",
    rust_enum: "ColorName",
    default: color_default(default),
  }
}

/// A color prop whose setter takes guise's `Color` — `Text` and `Title`.
pub const fn color_value(
  key: &'static str,
  label: &'static str,
  emit: Emit,
  default: ColorToken,
) -> PropSpec {
  PropSpec {
    key,
    label,
    ty: PropType::Color,
    emit,
    choices: &[],
    hint: "",
    rust_enum: "Color",
    default: color_default(default),
  }
}

pub const fn size(
  key: &'static str,
  label: &'static str,
  emit: Emit,
  default: SizeToken,
) -> PropSpec {
  PropSpec {
    key,
    label,
    ty: PropType::Size,
    emit,
    choices: &[],
    hint: "",
    rust_enum: "Size",
    default: match default {
      SizeToken::Xs => || PropValue::Size(SizeToken::Xs),
      SizeToken::Sm => || PropValue::Size(SizeToken::Sm),
      SizeToken::Md => || PropValue::Size(SizeToken::Md),
      SizeToken::Lg => || PropValue::Size(SizeToken::Lg),
      SizeToken::Xl => || PropValue::Size(SizeToken::Xl),
    },
  }
}

pub const fn variant(
  key: &'static str,
  label: &'static str,
  emit: Emit,
  default: VariantToken,
) -> PropSpec {
  PropSpec {
    key,
    label,
    ty: PropType::Variant,
    emit,
    choices: &[],
    hint: "",
    rust_enum: "Variant",
    default: match default {
      VariantToken::Filled => || PropValue::Variant(VariantToken::Filled),
      VariantToken::Light => || PropValue::Variant(VariantToken::Light),
      VariantToken::Outline => || PropValue::Variant(VariantToken::Outline),
      VariantToken::Subtle => || PropValue::Variant(VariantToken::Subtle),
      VariantToken::Default => || PropValue::Variant(VariantToken::Default),
      VariantToken::Transparent => || PropValue::Variant(VariantToken::Transparent),
      VariantToken::White => || PropValue::Variant(VariantToken::White),
    },
  }
}

pub const fn icon(key: &'static str, label: &'static str, emit: Emit) -> PropSpec {
  PropSpec {
    key,
    label,
    ty: PropType::Icon,
    emit,
    choices: &[],
    hint: "",
    rust_enum: "",
    default: || PropValue::Icon(String::new()),
  }
}

pub const fn items(
  key: &'static str,
  label: &'static str,
  emit: Emit,
  default: fn() -> PropValue,
) -> PropSpec {
  PropSpec {
    key,
    label,
    ty: PropType::Items,
    emit,
    choices: &[],
    hint: "",
    rust_enum: "",
    default,
  }
}

pub const fn numbers(
  key: &'static str,
  label: &'static str,
  emit: Emit,
  default: fn() -> PropValue,
) -> PropSpec {
  PropSpec {
    key,
    label,
    ty: PropType::Numbers,
    emit,
    choices: &[],
    hint: "",
    rust_enum: "",
    default,
  }
}

/// A choice prop that resolves to a Rust enum: `enums("align", .., "Align",
/// &["start", "center"], ..)` generates `Align::Start`.
pub const fn enums(
  key: &'static str,
  label: &'static str,
  emit: Emit,
  rust_enum: &'static str,
  choices: &'static [&'static str],
  default: fn() -> PropValue,
) -> PropSpec {
  PropSpec {
    key,
    label,
    ty: PropType::Choice,
    emit,
    choices,
    hint: "",
    rust_enum,
    default,
  }
}

/// Attach a hint to a spec built by one of the constructors above.
pub const fn hinted(mut spec: PropSpec, hint: &'static str) -> PropSpec {
  spec.hint = hint;
  spec
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn accessors_answer_for_their_own_variant_only() {
    assert_eq!(PropValue::Bool(true).as_bool(), Some(true));
    assert_eq!(PropValue::Bool(true).as_f64(), None);
    assert_eq!(PropValue::Int(3).as_f64(), Some(3.0));
    assert_eq!(PropValue::Float(2.5).as_i64(), Some(2));
    assert_eq!(PropValue::Text("hi".into()).as_str(), Some("hi"));
    assert_eq!(PropValue::Icon("plus".into()).as_str(), Some("plus"));
  }

  #[test]
  fn empty_is_about_content_not_falsiness() {
    assert!(PropValue::Text(String::new()).is_empty());
    assert!(!PropValue::Bool(false).is_empty());
    assert!(!PropValue::Int(0).is_empty());
    assert!(PropValue::Items(vec![]).is_empty());
  }

  #[test]
  fn spec_constructors_carry_their_default() {
    let spec = size("size", "Size", Emit::Method("size"), SizeToken::Lg);
    assert_eq!(spec.default_value(), PropValue::Size(SizeToken::Lg));
    let spec = boolean("disabled", "Disabled", Emit::Method("disabled"), false);
    assert_eq!(spec.default_value(), PropValue::Bool(false));
  }
}
