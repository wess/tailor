//! Prop values as Rust expressions.
//!
//! The awkward part is colour. guise's setters take three different types —
//! `impl Into<ColorValue>`, a bare `ColorName`, and guise's own `Color` — and
//! two of those need a value resolved out of the theme. Resolving inline would
//! hold a `&Theme` borrow across the rest of the builder chain, which is the
//! one thing guise's own conventions tell you not to do, so every resolved
//! colour is hoisted into a `let` at the top of `render` instead.

use tailor_model::props::{PropSpec, PropType, PropValue};
use tailor_model::tokens::{ColorSpec, ColorToken};
use tailor_model::{Document, SizeToken, VariantToken};

use crate::rust::{float, string};

/// The palette shade a named colour resolves to. Matches what `tailor-render`
/// paints on the canvas, so the export looks like what you designed.
pub const SHADE: usize = 6;

/// Colours pulled out of the builder chain into locals at the top of `render`.
#[derive(Debug, Default)]
pub struct Hoist {
    entries: Vec<(String, String)>,
}

impl Hoist {
    /// A local holding `theme(cx).color(ColorName::Blue, 6).hsla()`.
    pub fn named(&mut self, token: ColorToken) -> String {
        let name = format!("{}_{SHADE}", token.label());
        let expr = format!("theme(cx).color({}, {SHADE}).hsla()", token.path());
        self.push(name, expr)
    }

    /// A local holding an explicit colour parsed from its hex.
    pub fn custom(&mut self, hex: &str) -> String {
        let cleaned: String = hex
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_lowercase();
        let name = format!("hex_{cleaned}");
        let expr = format!("css({}).unwrap()", string(hex));
        self.push(name, expr)
    }

    fn push(&mut self, name: String, expr: String) -> String {
        if !self.entries.iter().any(|(existing, _)| *existing == name) {
            self.entries.push((name.clone(), expr));
        }
        name
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The `let` lines, in the order they were first needed.
    pub fn lines(&self) -> Vec<String> {
        self.entries
            .iter()
            .map(|(name, expr)| format!("let {name} = {expr};"))
            .collect()
    }
}

/// An `Hsla` expression — what gpui's `bg`, `border_color`, and `text_color`
/// take.
pub fn hsla(hoist: &mut Hoist, color: &ColorSpec) -> String {
    match color {
        ColorSpec::Named(token) => hoist.named(*token),
        ColorSpec::Custom(hex) => hoist.custom(hex),
    }
}

/// A colour for a component prop, in whichever type its setter takes.
pub fn color_arg(hoist: &mut Hoist, rust_enum: &str, color: &ColorSpec) -> String {
    match (rust_enum, color) {
        // `impl Into<ColorValue>`: a palette family passes as itself, an
        // explicit colour as the Hsla it resolves to.
        ("ColorValue", ColorSpec::Named(token)) => token.path(),
        ("ColorValue", ColorSpec::Custom(hex)) => hoist.custom(hex),
        // A bare `ColorName` has no room for an explicit colour. The inspector
        // only offers palette families for these, so Custom is a stale file.
        ("ColorName", ColorSpec::Named(token)) => token.path(),
        ("ColorName", ColorSpec::Custom(_)) => ColorToken::Blue.path(),
        // guise's `Color`.
        (_, ColorSpec::Named(token)) => format!("Color::from_hsla({})", hoist.named(*token)),
        (_, ColorSpec::Custom(hex)) => format!("Color::hex({})", string(hex)),
    }
}

/// A prop value as the argument its setter takes.
pub fn value(hoist: &mut Hoist, spec: &PropSpec, value: &PropValue, doc: &Document) -> String {
    if let Some(var) = value.as_binding() {
        return binding(spec, var, doc);
    }
    match (spec.ty, value) {
        (PropType::Color, PropValue::Color(color)) => color_arg(hoist, spec.rust_enum, color),
        (_, PropValue::Bool(v)) => v.to_string(),
        (_, PropValue::Int(v)) => v.to_string(),
        (_, PropValue::Float(v)) => float(*v as f32),
        (_, PropValue::Text(v)) => string(v),
        (_, PropValue::Icon(v)) => icon_path(v),
        (_, PropValue::Size(v)) => v.path(),
        (_, PropValue::Variant(v)) => v.path(),
        (_, PropValue::Choice(v)) => choice(spec, v),
        (_, PropValue::Items(v)) => items(v),
        (_, PropValue::Numbers(v)) => numbers(v),
        (_, PropValue::Color(color)) => color_arg(hoist, spec.rust_enum, color),
        (_, PropValue::Binding(_)) => unreachable!("handled above"),
    }
}

/// A prop that reads a state variable. `Signal<T>` hands back an owned value,
/// which is what every setter here wants.
fn binding(spec: &PropSpec, var: &str, doc: &Document) -> String {
    let read = format!("self.{}.get(cx)", tailor_model::snake_case(var));
    let ty = doc.var(var).map(|v| v.ty);
    match (spec.ty, ty) {
        // A text setter takes `impl Into<SharedString>`; a `String` qualifies.
        (PropType::Text | PropType::MultilineText, _) => read,
        (PropType::Int, Some(tailor_model::VarType::Float)) => format!("{read} as i64"),
        (PropType::Float, Some(tailor_model::VarType::Int)) => format!("{read} as f64"),
        (PropType::Int, _) => format!("{read} as usize"),
        _ => read,
    }
}

/// `IconName::ArrowUp` from the kebab-case name the document carries.
pub fn icon_path(name: &str) -> String {
    if name.is_empty() {
        return "IconName::Circle".into();
    }
    format!("IconName::{}", tailor_model::pascal_case(name))
}

/// A choice value as its Rust enum path, or as a string when it has no enum.
pub fn choice(spec: &PropSpec, value: &str) -> String {
    if spec.rust_enum.is_empty() {
        string(value)
    } else if spec.rust_enum == "FontWeight" {
        format!("FontWeight::{}", value.to_uppercase())
    } else {
        format!("{}::{}", spec.rust_enum, tailor_model::pascal_case(value))
    }
}

/// `["a", "b"]` — every `data`/`options`/`items` setter takes an iterator of
/// `impl Into<SharedString>`, and an array literal is the least noisy of those.
pub fn items(values: &[String]) -> String {
    let parts: Vec<String> = values.iter().map(|v| string(v)).collect();
    format!("[{}]", parts.join(", "))
}

pub fn numbers(values: &[f64]) -> String {
    let parts: Vec<String> = values.iter().map(|v| float(*v as f32)).collect();
    format!("[{}]", parts.join(", "))
}

/// The default a prop falls back to when the document never set it.
pub fn is_default(spec: &PropSpec, value: &PropValue) -> bool {
    spec.default_value() == *value
}

/// Sizes and variants print as their token path; kept here so the node emitter
/// never reaches into `tokens` directly.
pub fn size_path(size: SizeToken) -> String {
    size.path()
}

pub fn variant_path(variant: VariantToken) -> String {
    variant.path()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tailor_model::props::{color, color_name, color_value, Emit};

    #[test]
    fn a_hoisted_colour_is_declared_once() {
        let mut hoist = Hoist::default();
        assert_eq!(hoist.named(ColorToken::Blue), "blue_6");
        assert_eq!(hoist.named(ColorToken::Blue), "blue_6");
        assert_eq!(hoist.named(ColorToken::Red), "red_6");
        assert_eq!(
            hoist.lines(),
            [
                "let blue_6 = theme(cx).color(ColorName::Blue, 6).hsla();",
                "let red_6 = theme(cx).color(ColorName::Red, 6).hsla();"
            ]
        );
    }

    #[test]
    fn a_custom_colour_hoists_under_its_hex() {
        let mut hoist = Hoist::default();
        assert_eq!(hoist.custom("#3B82F6"), "hex_3b82f6");
        assert_eq!(
            hoist.lines(),
            ["let hex_3b82f6 = css(\"#3B82F6\").unwrap();"]
        );
    }

    #[test]
    fn colour_arguments_match_the_setter_they_feed() {
        let mut hoist = Hoist::default();
        let named = ColorSpec::Named(ColorToken::Grape);
        let custom = ColorSpec::Custom("#101010".into());

        assert_eq!(
            color_arg(&mut hoist, "ColorValue", &named),
            "ColorName::Grape"
        );
        assert_eq!(color_arg(&mut hoist, "ColorValue", &custom), "hex_101010");
        assert_eq!(
            color_arg(&mut hoist, "ColorName", &named),
            "ColorName::Grape"
        );
        assert_eq!(
            color_arg(&mut hoist, "ColorName", &custom),
            "ColorName::Blue"
        );
        assert_eq!(
            color_arg(&mut hoist, "Color", &named),
            "Color::from_hsla(grape_6)"
        );
        assert_eq!(
            color_arg(&mut hoist, "Color", &custom),
            "Color::hex(\"#101010\")"
        );
    }

    #[test]
    fn choices_print_as_their_enum() {
        let spec = tailor_model::props::enums(
            "align",
            "Align",
            Emit::Method("align"),
            "Align",
            &["start", "space_between"],
            || PropValue::Choice("start".into()),
        );
        assert_eq!(choice(&spec, "start"), "Align::Start");
        assert_eq!(choice(&spec, "space_between"), "Align::SpaceBetween");

        let plain = tailor_model::props::enums("axis", "Axis", Emit::None, "", &["x", "y"], || {
            PropValue::Choice("x".into())
        });
        assert_eq!(choice(&plain, "y"), "\"y\"");
    }

    #[test]
    fn lists_print_as_array_literals() {
        assert_eq!(items(&["a".into(), "b".into()]), "[\"a\", \"b\"]");
        assert_eq!(numbers(&[1.0, 2.5]), "[1., 2.5]");
    }

    #[test]
    fn icons_become_their_variant() {
        assert_eq!(icon_path("arrow-up"), "IconName::ArrowUp");
        assert_eq!(icon_path("a-arrow-down"), "IconName::AArrowDown");
        assert_eq!(icon_path(""), "IconName::Circle");
    }

    #[test]
    fn the_three_colour_constructors_carry_their_target_type() {
        assert_eq!(
            color("c", "C", Emit::None, ColorToken::Blue).rust_enum,
            "ColorValue"
        );
        assert_eq!(
            color_name("c", "C", Emit::None, ColorToken::Blue).rust_enum,
            "ColorName"
        );
        assert_eq!(
            color_value("c", "C", Emit::None, ColorToken::Blue).rust_enum,
            "Color"
        );
    }
}
