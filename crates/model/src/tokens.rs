//! The theme tokens a document can carry.
//!
//! A size is `md` in the file and in the inspector whoever draws it. What that
//! becomes in Rust is the target library's business: guise spells it
//! `Size::Md`, and another library might spell it something else entirely — so
//! a token knows its [`variant`](SizeToken::variant) name and nothing about the
//! type in front of it. [`crate::library::TokenPaths`] supplies that half.

use serde::{Deserialize, Serialize};

/// Generate a token enum plus its `label` (what the UI and the file format
/// spell it) and its `variant` (the Rust name a generator prefixes).
macro_rules! token {
    (
        $(#[$meta:meta])*
        $name:ident {
            $( $variant:ident => $label:literal ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum $name {
            $( $variant ),*
        }

        impl $name {
            /// Every variant, in declaration order — the order pickers list them.
            pub const ALL: &'static [$name] = &[ $( $name::$variant ),* ];

            /// The lowercase name used in the file format and in the inspector.
            pub fn label(self) -> &'static str {
                match self { $( $name::$variant => $label ),* }
            }

            /// Parse a label back. Unknown labels return `None`.
            pub fn parse(s: &str) -> Option<Self> {
                match s { $( $label => Some($name::$variant), )* _ => None }
            }

            /// The Rust variant name, e.g. `Md`. The *type* it hangs off is
            /// the target library's, not the model's — see
            /// [`crate::library::TokenPaths`], which is what a generator
            /// prefixes this with.
            pub fn variant(self) -> &'static str {
                match self { $( $name::$variant => stringify!($variant) ),* }
            }
        }
    };
}

token! {
    /// The `xs..xl` scale used for spacing, radius, and font size.
    SizeToken {
        Xs => "xs",
        Sm => "sm",
        Md => "md",
        Lg => "lg",
        Xl => "xl",
    }
}

token! {
    /// How a component fills itself against its color.
    VariantToken {
        Filled => "filled",
        Light => "light",
        Outline => "outline",
        Subtle => "subtle",
        Default => "default",
        Transparent => "transparent",
        White => "white",
    }
}

token! {
    /// A named palette family. Explicit colors go through [`ColorSpec::Custom`].
    ColorToken {
        Dark => "dark",
        Gray => "gray",
        Red => "red",
        Pink => "pink",
        Grape => "grape",
        Violet => "violet",
        Indigo => "indigo",
        Blue => "blue",
        Cyan => "cyan",
        Teal => "teal",
        Green => "green",
        Lime => "lime",
        Yellow => "yellow",
        Orange => "orange",
    }
}

token! {
    /// Cross-axis alignment of flex children.
    AlignToken {
        Start => "start",
        Center => "center",
        End => "end",
        Stretch => "stretch",
    }
}

token! {
    /// Main-axis distribution of flex children.
    JustifyToken {
        Start => "start",
        Center => "center",
        End => "end",
        Between => "between",
        Around => "around",
    }
}

/// A color a node can carry: a palette family, or one explicit color.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "lowercase")]
pub enum ColorSpec {
  Named(ColorToken),
  /// `#rrggbb` or `#rrggbbaa`. Kept as text so the file format stays readable
  /// and the round-trip through a color picker is lossless.
  Custom(String),
}

impl Default for ColorSpec {
  fn default() -> Self {
    ColorSpec::Named(ColorToken::Blue)
  }
}

impl ColorSpec {
  /// Split `#rrggbb`/`#rrggbbaa` into 0..1 channel floats. Invalid text is
  /// mid-gray rather than an error — the inspector lets you type freely, and
  /// a half-finished hex should not blank the canvas.
  pub fn rgba(&self, palette: impl Fn(ColorToken) -> [f32; 4]) -> [f32; 4] {
    match self {
      ColorSpec::Named(name) => palette(*name),
      ColorSpec::Custom(hex) => parse_hex(hex).unwrap_or([0.5, 0.5, 0.5, 1.0]),
    }
  }
}

/// Parse `#rgb`, `#rrggbb`, or `#rrggbbaa` into 0..1 channels.
pub fn parse_hex(text: &str) -> Option<[f32; 4]> {
  let hex = text.trim().trim_start_matches('#');
  let byte = |i: usize| {
    u8::from_str_radix(&hex[i..i + 2], 16)
      .ok()
      .map(|v| v as f32 / 255.0)
  };
  match hex.len() {
    3 => {
      let nib = |i: usize| {
        u8::from_str_radix(&hex[i..i + 1], 16)
          .ok()
          .map(|v| (v * 17) as f32 / 255.0)
      };
      Some([nib(0)?, nib(1)?, nib(2)?, 1.0])
    }
    6 => Some([byte(0)?, byte(2)?, byte(4)?, 1.0]),
    8 => Some([byte(0)?, byte(2)?, byte(4)?, byte(6)?]),
    _ => None,
  }
}

/// Format 0..1 channels back to `#rrggbb` (or `#rrggbbaa` when translucent).
pub fn to_hex(rgba: [f32; 4]) -> String {
  let ch = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
  if rgba[3] >= 0.999 {
    format!("#{:02x}{:02x}{:02x}", ch(rgba[0]), ch(rgba[1]), ch(rgba[2]))
  } else {
    format!(
      "#{:02x}{:02x}{:02x}{:02x}",
      ch(rgba[0]),
      ch(rgba[1]),
      ch(rgba[2]),
      ch(rgba[3])
    )
  }
}

token! {
    /// The entrance a node plays when it appears. Mirrors guise's
    /// `TransitionKind`, which is what generated code names.
    EnterToken {
        Fade => "fade",
        SlideUp => "slideup",
        SlideDown => "slidedown",
        SlideLeft => "slideleft",
        SlideRight => "slideright",
    }
}

impl EnterToken {
  /// What the inspector calls it.
  pub fn title(self) -> &'static str {
    match self {
      EnterToken::Fade => "Fade",
      EnterToken::SlideUp => "Slide up",
      EnterToken::SlideDown => "Slide down",
      EnterToken::SlideLeft => "Slide left",
      EnterToken::SlideRight => "Slide right",
    }
  }

  /// Whether the `distance` setting does anything for this entrance.
  pub fn travels(self) -> bool {
    !matches!(self, EnterToken::Fade)
  }

  /// The word the `motion!` macro spells it with. Not [`label`](Self::label),
  /// which is the file format's and predates the macro.
  pub fn word(self) -> &'static str {
    match self {
      EnterToken::Fade => "fade",
      EnterToken::SlideUp => "slide_up",
      EnterToken::SlideDown => "slide_down",
      EnterToken::SlideLeft => "slide_left",
      EnterToken::SlideRight => "slide_right",
    }
  }
}

/// A curated slice of guise's easing curves — the ones worth a picker row.
///
/// Written out by hand rather than through `token!` because the generated
/// path is not `Enum::Variant`: guise composes a direction with a shape
/// (`Easing::Out(Curve::Cubic)`), which is exactly the axis a designer wants
/// to pick along and exactly what a flat token list would flatten away.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EaseToken {
  Linear,
  OutQuad,
  #[default]
  OutCubic,
  OutQuint,
  OutExpo,
  OutCirc,
  OutBack,
  OutElastic,
  OutBounce,
  InQuad,
  InCubic,
  InExpo,
  InOutQuad,
  InOutCubic,
  InOutSine,
  Spring,
}

impl EaseToken {
  pub const ALL: &'static [EaseToken] = &[
    EaseToken::Linear,
    EaseToken::OutQuad,
    EaseToken::OutCubic,
    EaseToken::OutQuint,
    EaseToken::OutExpo,
    EaseToken::OutCirc,
    EaseToken::OutBack,
    EaseToken::OutElastic,
    EaseToken::OutBounce,
    EaseToken::InQuad,
    EaseToken::InCubic,
    EaseToken::InExpo,
    EaseToken::InOutQuad,
    EaseToken::InOutCubic,
    EaseToken::InOutSine,
    EaseToken::Spring,
  ];

  /// The lowercase name used in the file format.
  pub fn label(self) -> &'static str {
    match self {
      EaseToken::Linear => "linear",
      EaseToken::OutQuad => "out-quad",
      EaseToken::OutCubic => "out-cubic",
      EaseToken::OutQuint => "out-quint",
      EaseToken::OutExpo => "out-expo",
      EaseToken::OutCirc => "out-circ",
      EaseToken::OutBack => "out-back",
      EaseToken::OutElastic => "out-elastic",
      EaseToken::OutBounce => "out-bounce",
      EaseToken::InQuad => "in-quad",
      EaseToken::InCubic => "in-cubic",
      EaseToken::InExpo => "in-expo",
      EaseToken::InOutQuad => "in-out-quad",
      EaseToken::InOutCubic => "in-out-cubic",
      EaseToken::InOutSine => "in-out-sine",
      EaseToken::Spring => "spring",
    }
  }

  pub fn parse(s: &str) -> Option<Self> {
    EaseToken::ALL.iter().copied().find(|e| e.label() == s)
  }

  /// What the inspector shows.
  pub fn title(self) -> &'static str {
    match self {
      EaseToken::Linear => "Linear",
      EaseToken::OutQuad => "Ease out",
      EaseToken::OutCubic => "Ease out (soft)",
      EaseToken::OutQuint => "Ease out (long)",
      EaseToken::OutExpo => "Ease out (sharp)",
      EaseToken::OutCirc => "Ease out (circular)",
      EaseToken::OutBack => "Overshoot",
      EaseToken::OutElastic => "Elastic",
      EaseToken::OutBounce => "Bounce",
      EaseToken::InQuad => "Ease in",
      EaseToken::InCubic => "Ease in (soft)",
      EaseToken::InExpo => "Ease in (sharp)",
      EaseToken::InOutQuad => "Ease in-out",
      EaseToken::InOutCubic => "Ease in-out (soft)",
      EaseToken::InOutSine => "Ease in-out (gentle)",
      EaseToken::Spring => "Spring",
    }
  }

  /// The declaration the `motion!` macro spells it with — a direction and a
  /// shape, or one of the three words that stand alone.
  pub fn words(self) -> &'static str {
    match self {
      EaseToken::Linear => "linear",
      EaseToken::OutQuad => "out quad",
      EaseToken::OutCubic => "out cubic",
      EaseToken::OutQuint => "out quint",
      EaseToken::OutExpo => "out expo",
      EaseToken::OutCirc => "out circ",
      EaseToken::OutBack => "out back",
      EaseToken::OutElastic => "out elastic",
      EaseToken::OutBounce => "out bounce",
      EaseToken::InQuad => "in quad",
      EaseToken::InCubic => "in cubic",
      EaseToken::InExpo => "in expo",
      EaseToken::InOutQuad => "in_out quad",
      EaseToken::InOutCubic => "in_out cubic",
      EaseToken::InOutSine => "in_out sine",
      EaseToken::Spring => "spring",
    }
  }
}

/// How many times a motion runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoopToken {
  #[default]
  Once,
  Forever,
}

impl LoopToken {
  pub const ALL: &'static [LoopToken] = &[LoopToken::Once, LoopToken::Forever];

  pub fn label(self) -> &'static str {
    match self {
      LoopToken::Once => "once",
      LoopToken::Forever => "forever",
    }
  }

  pub fn parse(s: &str) -> Option<Self> {
    LoopToken::ALL.iter().copied().find(|l| l.label() == s)
  }

  pub fn title(self) -> &'static str {
    match self {
      LoopToken::Once => "Once",
      LoopToken::Forever => "Loop",
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn tokens_round_trip_through_their_labels() {
    for size in SizeToken::ALL {
      assert_eq!(SizeToken::parse(size.label()), Some(*size));
    }
    for color in ColorToken::ALL {
      assert_eq!(ColorToken::parse(color.label()), Some(*color));
    }
    assert_eq!(SizeToken::parse("huge"), None);
    for ease in EaseToken::ALL {
      assert_eq!(EaseToken::parse(ease.label()), Some(*ease));
    }
    for enter in EnterToken::ALL {
      assert_eq!(EnterToken::parse(enter.label()), Some(*enter));
    }
    for repeat in LoopToken::ALL {
      assert_eq!(LoopToken::parse(repeat.label()), Some(*repeat));
    }
  }

  #[test]
  fn a_token_knows_its_variant_but_not_its_type() {
    assert_eq!(SizeToken::Md.variant(), "Md");
    assert_eq!(VariantToken::Outline.variant(), "Outline");
    assert_eq!(ColorToken::Grape.variant(), "Grape");
    assert_eq!(EnterToken::SlideUp.variant(), "SlideUp");
    assert_eq!(EaseToken::OutBack.words(), "out back");
    assert_eq!(EaseToken::InOutSine.words(), "in_out sine");
    assert_eq!(EnterToken::SlideUp.word(), "slide_up");
  }

  #[test]
  fn hex_parses_every_accepted_length() {
    assert_eq!(parse_hex("#fff"), Some([1.0, 1.0, 1.0, 1.0]));
    assert_eq!(parse_hex("000000"), Some([0.0, 0.0, 0.0, 1.0]));
    assert_eq!(parse_hex("#00000080").unwrap()[3], 128.0 / 255.0);
    assert_eq!(parse_hex("#ggg"), None);
    assert_eq!(parse_hex("#12345"), None);
  }

  #[test]
  fn hex_round_trips() {
    assert_eq!(to_hex(parse_hex("#3b82f6").unwrap()), "#3b82f6");
    assert_eq!(to_hex([1.0, 0.0, 0.0, 0.5]), "#ff000080");
  }
}
