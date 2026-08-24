//! Reading a node's props as the types guise's setters want.
//!
//! Three things happen here that a plain map lookup would not do. A prop that
//! was never set falls back to the catalog's default, so a freshly dropped
//! component looks like something. A prop bound to a state variable resolves to
//! that variable's starting value, so the canvas shows what the app will show on
//! its first frame. And a colour resolves through the live theme, which is why
//! switching the project's scheme re-paints the canvas for free.

use gpui::{App, Hsla, SharedString};
use guise::prelude::*;
use tailor_model::catalog::{self, ComponentSpec};
use tailor_model::props::PropValue;
use tailor_model::tokens::{ColorSpec, ColorToken};
use tailor_model::{Document, Node, SizeToken, VariantToken};

/// The palette shade a named colour resolves to. Matches `tailor-codegen`, so
/// the canvas and the export agree.
pub const SHADE: usize = 6;

pub struct Reader<'a> {
    node: &'a Node,
    spec: Option<&'static ComponentSpec>,
    doc: &'a Document,
}

impl<'a> Reader<'a> {
    pub fn new(node: &'a Node, doc: &'a Document) -> Self {
        Reader {
            node,
            spec: catalog::get(&node.kind),
            doc,
        }
    }

    /// The value of a prop: what the node set, or what the catalog defaults to.
    /// A binding resolves to its variable's starting value.
    pub fn get(&self, key: &str) -> PropValue {
        let raw = self
            .node
            .prop(key)
            .cloned()
            .or_else(|| self.spec.and_then(|spec| spec.default_prop(key)))
            .unwrap_or(PropValue::Text(String::new()));
        match raw.as_binding() {
            Some(name) => self.resolve_binding(name, key),
            None => raw,
        }
    }

    fn resolve_binding(&self, name: &str, key: &str) -> PropValue {
        let Some(var) = self.doc.var(name) else {
            return PropValue::Text(format!("{{{name}}}"));
        };
        let initial = var.initial.trim();
        match var.ty {
            tailor_model::VarType::Text => PropValue::Text(if initial.is_empty() {
                format!("{{{name}}}")
            } else {
                initial.to_string()
            }),
            tailor_model::VarType::Bool => PropValue::Bool(matches!(initial, "true" | "yes" | "1")),
            tailor_model::VarType::Int => PropValue::Int(initial.parse().unwrap_or(0)),
            tailor_model::VarType::Float => PropValue::Float(initial.parse().unwrap_or(0.0)),
            tailor_model::VarType::Items => PropValue::Items(
                initial
                    .split(',')
                    .map(|part| part.trim().to_string())
                    .filter(|p| !p.is_empty())
                    .collect(),
            ),
            #[allow(unreachable_patterns)]
            _ => self
                .spec
                .and_then(|spec| spec.default_prop(key))
                .unwrap_or(PropValue::Bool(false)),
        }
    }

    pub fn text(&self, key: &str) -> SharedString {
        SharedString::from(self.get(key).as_str().unwrap_or("").to_string())
    }

    pub fn choice(&self, key: &str) -> String {
        self.get(key).as_str().unwrap_or("").to_string()
    }

    pub fn bool(&self, key: &str) -> bool {
        self.get(key).as_bool().unwrap_or(false)
    }

    pub fn f64(&self, key: &str) -> f64 {
        self.get(key).as_f64().unwrap_or(0.0)
    }

    pub fn f32(&self, key: &str) -> f32 {
        self.f64(key) as f32
    }

    pub fn usize(&self, key: &str) -> usize {
        self.get(key).as_i64().unwrap_or(0).max(0) as usize
    }

    pub fn size(&self, key: &str) -> Size {
        size_of(self.get(key).as_size().unwrap_or(SizeToken::Md))
    }

    pub fn variant(&self, key: &str) -> Variant {
        variant_of(self.get(key).as_variant().unwrap_or(VariantToken::Filled))
    }

    /// A colour for a setter that takes `impl Into<ColorValue>`.
    pub fn color(&self, key: &str, cx: &App) -> ColorValue {
        match self.get(key) {
            PropValue::Color(ColorSpec::Named(token)) => ColorValue::Named(color_name_of(token)),
            PropValue::Color(ColorSpec::Custom(hex)) => ColorValue::Custom(hex_or(&hex, cx)),
            _ => ColorValue::Named(ColorName::Blue),
        }
    }

    /// A colour for a setter that takes a bare `ColorName`.
    pub fn color_name(&self, key: &str) -> ColorName {
        match self.get(key) {
            PropValue::Color(ColorSpec::Named(token)) => color_name_of(token),
            _ => ColorName::Blue,
        }
    }

    /// A colour for `Text` and `Title`, which take guise's own `Color`.
    pub fn color_value(&self, key: &str, cx: &App) -> Color {
        match self.get(key) {
            PropValue::Color(ColorSpec::Named(token)) => {
                Color::from_hsla(theme(cx).color(color_name_of(token), SHADE).hsla())
            }
            PropValue::Color(ColorSpec::Custom(hex)) => Color::from_hsla(hex_or(&hex, cx)),
            _ => Color::from_hsla(theme(cx).text().hsla()),
        }
    }

    pub fn icon(&self, key: &str) -> Option<IconName> {
        icon_named(self.get(key).as_str().unwrap_or(""))
    }

    pub fn items(&self, key: &str) -> Vec<SharedString> {
        self.raw_items(key)
            .into_iter()
            .map(SharedString::from)
            .collect()
    }

    pub fn raw_items(&self, key: &str) -> Vec<String> {
        match self.get(key) {
            PropValue::Items(values) => values,
            PropValue::Text(text) if !text.is_empty() => vec![text],
            _ => Vec::new(),
        }
    }

    pub fn numbers(&self, key: &str) -> Vec<f32> {
        match self.get(key) {
            PropValue::Numbers(values) => values.into_iter().map(|v| v as f32).collect(),
            _ => Vec::new(),
        }
    }
}

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
fn hex_or(hex: &str, cx: &App) -> Hsla {
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

pub fn size_of(token: SizeToken) -> Size {
    match token {
        SizeToken::Xs => Size::Xs,
        SizeToken::Sm => Size::Sm,
        SizeToken::Md => Size::Md,
        SizeToken::Lg => Size::Lg,
        SizeToken::Xl => Size::Xl,
    }
}

pub fn variant_of(token: VariantToken) -> Variant {
    match token {
        VariantToken::Filled => Variant::Filled,
        VariantToken::Light => Variant::Light,
        VariantToken::Outline => Variant::Outline,
        VariantToken::Subtle => Variant::Subtle,
        VariantToken::Default => Variant::Default,
        VariantToken::Transparent => Variant::Transparent,
        VariantToken::White => Variant::White,
    }
}

pub fn align_of(token: tailor_model::AlignToken) -> Align {
    match token {
        tailor_model::AlignToken::Start => Align::Start,
        tailor_model::AlignToken::Center => Align::Center,
        tailor_model::AlignToken::End => Align::End,
        tailor_model::AlignToken::Stretch => Align::Stretch,
    }
}

pub fn justify_of(token: tailor_model::JustifyToken) -> Justify {
    match token {
        tailor_model::JustifyToken::Start => Justify::Start,
        tailor_model::JustifyToken::Center => Justify::Center,
        tailor_model::JustifyToken::End => Justify::End,
        tailor_model::JustifyToken::Between => Justify::Between,
        tailor_model::JustifyToken::Around => Justify::Around,
    }
}

/// Find an icon by its kebab-case name. The table is sorted, so this is a
/// binary search over 1991 entries rather than a scan.
pub fn icon_named(name: &str) -> Option<IconName> {
    if name.is_empty() {
        return None;
    }
    let all = IconName::all();
    all.binary_search_by(|candidate| candidate.name().cmp(name))
        .ok()
        .map(|index| all[index])
}

pub fn language(name: &str) -> Language {
    match name {
        "rust" => Language::Rust,
        "sql" => Language::Sql,
        "json" => Language::Json,
        "toml" => Language::Toml,
        "python" => Language::Python,
        "javascript" => Language::JavaScript,
        "typescript" => Language::TypeScript,
        "go" => Language::Go,
        "c" => Language::C,
        "markdown" => Language::Markdown,
        _ => Language::None,
    }
}

/// Turn indented lines into a `TreeNode` forest — two spaces per level.
pub fn tree_nodes(lines: &[String]) -> Vec<TreeNode> {
    fn depth(line: &str) -> usize {
        (line.len() - line.trim_start().len()) / 2
    }
    fn build(lines: &[String], index: &mut usize, level: usize) -> Vec<TreeNode> {
        let mut out = Vec::new();
        while *index < lines.len() {
            let line = &lines[*index];
            if line.trim().is_empty() {
                *index += 1;
                continue;
            }
            if depth(line) < level {
                break;
            }
            let label = line.trim().to_string();
            *index += 1;
            let children = build(lines, index, level + 1);
            let node = TreeNode::new(tailor_model::snake_case(&label), label).children(children);
            out.push(node);
        }
        out
    }
    let mut index = 0;
    build(lines, &mut index, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icons_are_found_by_their_kebab_name() {
        assert_eq!(icon_named("arrow-up"), Some(IconName::ArrowUp));
        assert_eq!(icon_named("a-arrow-down"), Some(IconName::AArrowDown));
        assert_eq!(icon_named(""), None);
        assert_eq!(icon_named("not-an-icon"), None);
    }

    #[test]
    fn languages_fall_back_to_plain() {
        assert!(matches!(language("rust"), Language::Rust));
        assert!(matches!(language("klingon"), Language::None));
    }

    #[test]
    fn indented_lines_build_a_forest() {
        let lines: Vec<String> = ["src", "  main.rs", "Cargo.toml"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let nodes = tree_nodes(&lines);
        assert_eq!(nodes.len(), 2);
    }
}
