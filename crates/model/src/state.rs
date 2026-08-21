//! State variables and actions — the half of a document that is not layout.
//!
//! A screen you can only look at is a mockup. These two tables are what make
//! the generated file a component you can wire up: every variable becomes a
//! `Signal<T>` field, every action becomes a method, and the inspector's
//! Events tab is how a button finds one.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VarType {
    #[default]
    Text,
    Bool,
    Int,
    Float,
    /// `Vec<String>` — list data for a Select, a Table, tab titles.
    Items,
}

impl VarType {
    pub const ALL: &'static [VarType] = &[
        VarType::Text,
        VarType::Bool,
        VarType::Int,
        VarType::Float,
        VarType::Items,
    ];

    pub fn label(self) -> &'static str {
        match self {
            VarType::Text => "text",
            VarType::Bool => "bool",
            VarType::Int => "int",
            VarType::Float => "float",
            VarType::Items => "items",
        }
    }

    /// The Rust type the signal holds.
    pub fn rust(self) -> &'static str {
        match self {
            VarType::Text => "String",
            VarType::Bool => "bool",
            VarType::Int => "i64",
            VarType::Float => "f64",
            VarType::Items => "Vec<String>",
        }
    }

    /// A literal of this type built from the variable's `initial` text. Bad
    /// input falls back to the type's zero rather than failing generation —
    /// the file still compiles, and the wrong number is visible in the code.
    pub fn literal(self, initial: &str) -> String {
        let text = initial.trim();
        match self {
            VarType::Text => format!("{:?}.to_string()", text),
            VarType::Bool => match text {
                "true" | "yes" | "1" => "true".into(),
                _ => "false".into(),
            },
            VarType::Int => text.parse::<i64>().unwrap_or(0).to_string(),
            VarType::Float => {
                let value = text.parse::<f64>().unwrap_or(0.0);
                if value.fract() == 0.0 {
                    format!("{value:.1}")
                } else {
                    value.to_string()
                }
            }
            VarType::Items => {
                if text.is_empty() {
                    "Vec::new()".into()
                } else {
                    let parts: Vec<String> = text
                        .split(',')
                        .map(|p| format!("{:?}.to_string()", p.trim()))
                        .collect();
                    format!("vec![{}]", parts.join(", "))
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateVar {
    pub name: String,
    pub ty: VarType,
    /// The starting value, as typed in the inspector.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub initial: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
}

impl StateVar {
    pub fn new(name: impl Into<String>, ty: VarType) -> Self {
        StateVar {
            name: name.into(),
            ty,
            initial: String::new(),
            note: String::new(),
        }
    }

    /// The `Signal::new(cx, ..)` initializer for this variable.
    pub fn initializer(&self) -> String {
        format!("Signal::new(cx, {})", self.ty.literal(&self.initial))
    }
}

/// A named handler. The body is whatever the user typed; Tailor never runs it,
/// it only places it in the generated method so the file is a starting point
/// and not a stub you have to re-wire by hand.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionDef {
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub body: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
}

impl ActionDef {
    pub fn new(name: impl Into<String>) -> Self {
        ActionDef {
            name: name.into(),
            body: String::new(),
            note: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literals_match_their_declared_type() {
        assert_eq!(VarType::Text.literal("hi"), "\"hi\".to_string()");
        assert_eq!(VarType::Bool.literal("true"), "true");
        assert_eq!(VarType::Bool.literal("nonsense"), "false");
        assert_eq!(VarType::Int.literal("42"), "42");
        assert_eq!(VarType::Int.literal("4.2"), "0");
        assert_eq!(VarType::Float.literal("2"), "2.0");
        assert_eq!(
            VarType::Items.literal("a, b"),
            r#"vec!["a".to_string(), "b".to_string()]"#
        );
        assert_eq!(VarType::Items.literal(""), "Vec::new()");
    }

    #[test]
    fn a_variable_prints_its_own_initializer() {
        let mut var = StateVar::new("query", VarType::Text);
        var.initial = "hello".into();
        assert_eq!(var.initializer(), "Signal::new(cx, \"hello\".to_string())");
    }
}
