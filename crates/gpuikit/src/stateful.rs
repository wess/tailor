//! How the state-backed controls are built.
//!
//! Most states are `State::new(window, cx)` plus the props the catalog marks
//! `Emit::State`, and the emitter does that itself. These are the ones that
//! are not: a select is built over its list of options, a slider over a range
//! and no window, a code over its length, and some props only mean something
//! when they are present (a number input's bounds, a picker's starting colour).
//!
//! [`state`] is the state's constructor, [`special`] the one control here that
//! is a plain builder rather than a state (`Form`).

use tailor_codegen::node::Emitter;
use tailor_codegen::rust::string;
use tailor_model::Node;

/// The state's constructor and the calls chained onto it, for the kinds that
/// need more than `State::new(window, cx)`. Chained lines carry their own
/// indent, the way `special` returns them.
pub fn state(em: &mut Emitter, node: &Node) -> Option<Vec<String>> {
  let text = |em: &Emitter, key: &str| em.prop_value(node, key).as_str().unwrap_or("").to_string();
  let int = |em: &Emitter, key: &str| em.prop_value(node, key).as_i64().unwrap_or(0);

  Some(match node.kind.as_str() {
    "select" => {
      let selected = int(em, "selected");
      let picked = if selected >= 0 {
        format!("Some(IndexPath::default().row({selected}))")
      } else {
        "None".into()
      };
      vec![format!(
        "SelectState::new({}, {picked}, window, cx)",
        options(em, node)
      )]
    }
    // Nothing is selected to begin with: which options start ticked is a
    // decision the person using the form makes, not the designer.
    "combobox" => vec![format!(
      "ComboboxState::new({}, Vec::new(), window, cx)",
      options(em, node)
    )],
    // A slider's range needs no window, and `StatefulCx` says so.
    "slider" => vec!["SliderState::new()".into()],
    "otpinput" => {
      let length = int(em, "length").max(1);
      vec![format!("OtpState::new({length}, window, cx)")]
    }
    "numberinput" => {
      let mut lines = vec!["InputState::new(window, cx)".to_string()];
      for (key, method) in [("min", "min"), ("max", "max"), ("step", "step")] {
        if let Some(number) = number(&text(em, key)) {
          lines.push(format!("    .{method}({number})"));
        }
      }
      lines
    }
    "colorpicker" => {
      let mut lines = vec!["ColorPickerState::new(window, cx)".to_string()];
      if let Some(rgb) = hex(&text(em, "value")) {
        lines.push(format!("    .default_value(gpui::rgb(0x{rgb}))"));
      }
      lines
    }
    "datepicker" => {
      let mut lines = vec!["DatePickerState::new(window, cx)".to_string()];
      match text(em, "time").as_str() {
        "minute" => lines.push("    .time_precision(TimePrecision::Minute)".into()),
        "second" => lines.push("    .time_precision(TimePrecision::Second)".into()),
        _ => {}
      }
      lines
    }
    _ => return None,
  })
}

/// The controls here that are a builder, not a state.
pub fn special(em: &mut Emitter, node: &Node) -> Option<Vec<String>> {
  Some(match node.kind.as_str() {
    "form" => {
      let horizontal = em.prop_value(node, "layout").as_str() == Some("horizontal");
      vec![if horizontal {
        "Form::horizontal()".into()
      } else {
        "Form::vertical()".into()
      }]
    }
    _ => return None,
  })
}

/// `SearchableVec::new(vec!["One".to_string(), ..])`. The type is spelled out
/// on the element side through the field, so an empty list still infers.
fn options(em: &Emitter, node: &Node) -> String {
  let items = em.items(node, "items").unwrap_or_default();
  if items.is_empty() {
    return "SearchableVec::new(Vec::<String>::new())".into();
  }
  let parts: Vec<String> = items
    .iter()
    .map(|item| format!("{}.to_string()", string(item)))
    .collect();
  format!("SearchableVec::new(vec![{}])", parts.join(", "))
}

/// A number the way a literal spells an `f64`, or `None` when the field is
/// blank or not a number — the bound is then simply absent.
fn number(text: &str) -> Option<String> {
  let value: f64 = text.trim().parse().ok()?;
  Some(if value.fract() == 0.0 {
    format!("{value:.1}")
  } else {
    value.to_string()
  })
}

/// Six hex digits from `#3b82f6` or `3b82f6`, uppercase-insensitive.
fn hex(text: &str) -> Option<String> {
  let digits = text.trim().trim_start_matches('#');
  (digits.len() == 6 && digits.chars().all(|c| c.is_ascii_hexdigit()))
    .then(|| digits.to_lowercase())
}
