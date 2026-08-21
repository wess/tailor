//! The theme tokens a document can carry.
//!
//! These mirror guise's `Size` / `Variant` / `ColorName` / `Align` / `Justify`
//! without depending on guise: the model crate stays free of gpui so it can be
//! unit-tested, serialized, and reasoned about on its own. `tailor-render` maps
//! them onto the real enums; `tailor-codegen` prints them as Rust paths.

use serde::{Deserialize, Serialize};

/// Generate a token enum plus its `label` (what the UI and the file format
/// spell it) and `path` (the Rust it generates).
macro_rules! token {
    (
        $(#[$meta:meta])*
        $name:ident : $rust:literal {
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

            /// The Rust path this token generates, e.g. `Size::Md`.
            pub fn path(self) -> String {
                let variant = match self { $( $name::$variant => stringify!($variant) ),* };
                format!("{}::{}", $rust, variant)
            }
        }
    };
}

token! {
    /// The `xs..xl` scale used for spacing, radius, and font size.
    SizeToken: "Size" {
        Xs => "xs",
        Sm => "sm",
        Md => "md",
        Lg => "lg",
        Xl => "xl",
    }
}

token! {
    /// How a component fills itself against its color.
    VariantToken: "Variant" {
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
    ColorToken: "ColorName" {
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
    AlignToken: "Align" {
        Start => "start",
        Center => "center",
        End => "end",
        Stretch => "stretch",
    }
}

token! {
    /// Main-axis distribution of flex children.
    JustifyToken: "Justify" {
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
    /// The Rust expression for this color in generated code.
    pub fn path(&self) -> String {
        match self {
            ColorSpec::Named(name) => name.path(),
            ColorSpec::Custom(hex) => format!("css({hex:?}).unwrap()"),
        }
    }

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
    }

    #[test]
    fn tokens_print_their_rust_path() {
        assert_eq!(SizeToken::Md.path(), "Size::Md");
        assert_eq!(VariantToken::Outline.path(), "Variant::Outline");
        assert_eq!(ColorToken::Grape.path(), "ColorName::Grape");
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
