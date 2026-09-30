//! The generator's half of the command palette, the input group's text, and the
//! questionnaire's freeform answers.
//!
//! A palette is a stack of entries the parent writes — an item is
//! `CommandItem::new().label(..)`, a group holds items, a separator is a bare
//! call — and what confirming an item does is not on the item at all. gpui-kit
//! reports a confirmed item to the palette by its position, `(section, row)`, so
//! the palette's generator keeps that table and turns a position back into the
//! action the item was bound to. Section numbering is gpui-kit's: ungrouped
//! items are section 0, counted across the whole palette, and groups follow in
//! order — after the ungrouped section only when there is one.

use tailor_codegen::node::{prefixed_call, Emitter};
use tailor_codegen::rust::string;
use tailor_codegen::style::Placement;
use tailor_model::node::DEFAULT_SLOT;
use tailor_model::{Node, NodeId};

/// Kinds whose slots and handlers the generator writes itself. An item's event
/// belongs to the palette that holds it, so the item must not print it.
pub fn writes(kind: &str) -> bool {
  matches!(kind, "command" | "commanditem")
}

/// A constructor that is not `Type::new(..)`.
pub fn special(em: &mut Emitter, node: &Node) -> Option<Vec<String>> {
  match node.kind.as_str() {
    "inputgrouptext" => {
      let text = em
        .prop_value(node, "text")
        .as_str()
        .unwrap_or("")
        .to_string();
      Some(vec![
        "InputGroupText::new()".into(),
        format!("    .child({})", string(&text)),
      ])
    }
    _ => None,
  }
}

/// Calls for the props whose shape is not one setter.
pub fn custom_props(em: &mut Emitter, node: &Node) -> Vec<String> {
  match node.kind.as_str() {
    "commanditem" => {
      let words = em
        .prop_value(node, "keywords")
        .as_str()
        .unwrap_or("")
        .to_string();
      let words: Vec<String> = words
        .split(',')
        .map(str::trim)
        .filter(|word| !word.is_empty())
        .map(string)
        .collect();
      if words.is_empty() {
        Vec::new()
      } else {
        vec![format!(".keywords([{}])", words.join(", "))]
      }
    }
    _ => Vec::new(),
  }
}

/// A palette's entries and handlers; an item has nothing of its own to write.
pub fn slots(em: &mut Emitter, node: &Node, placement: Placement) -> Option<Vec<String>> {
  match node.kind.as_str() {
    "command" => Some(command(em, node, placement)),
    "commanditem" => Some(Vec::new()),
    _ => None,
  }
}

fn bound<'a>(node: &'a Node, event: &str) -> Option<&'a str> {
  node
    .events
    .get(event)
    .map(String::as_str)
    .filter(|action| !action.is_empty())
}

/// One arm of the confirm `match`: the action an item runs at a position.
fn arm(arms: &mut Vec<String>, section: usize, row: usize, item: &Node) {
  if let Some(action) = bound(item, "select") {
    arms.push(format!(
      "    ({section}, {row}) => this.{}(cx),",
      tailor_model::snake_case(action)
    ));
  }
}

fn command(em: &mut Emitter, node: &Node, placement: Placement) -> Vec<String> {
  let doc = em.doc;
  let kind = |id: NodeId| doc.node(id).map(|n| n.kind.as_str()).unwrap_or("");
  let entries: Vec<NodeId> = node.slot(DEFAULT_SLOT).to_vec();
  // Ungrouped items are section 0 only if there are any; groups come after.
  let ungrouped_section = entries.iter().any(|id| kind(*id) == "commanditem");

  let mut out = Vec::new();
  let mut arms: Vec<String> = Vec::new();
  let (mut row, mut group) = (0usize, 0usize);
  for id in entries {
    let Some(entry) = doc.node(id) else { continue };
    match entry.kind.as_str() {
      "commanditem" => {
        arm(&mut arms, 0, row, entry);
        row += 1;
        let inner = em.emit(id, placement);
        out.extend(prefixed_call("item", inner));
      }
      "commandgroup" => {
        let section = group + usize::from(ungrouped_section);
        group += 1;
        for (at, item) in entry.slot(DEFAULT_SLOT).iter().enumerate() {
          if let Some(item) = doc.node(*item) {
            arm(&mut arms, section, at, item);
          }
        }
        let inner = em.emit(id, placement);
        out.extend(prefixed_call("group", inner));
      }
      "commandseparator" => out.push(".separator()".into()),
      // Anything else has no place in a palette; the linter says so.
      _ => {}
    }
  }

  if !arms.is_empty() {
    arms.push("    _ => {}".into());
    let mut body = vec!["match (path.section, path.row) {".to_string()];
    body.extend(arms);
    body.push("}".into());
    out.extend(handler(em, "on_confirm", "path, _window, cx", &body));
  }
  for (key, method, params) in [
    ("query", "on_query", "_query, _window, cx"),
    ("cancel", "on_cancel", "_window, cx"),
  ] {
    if let Some(action) = bound(node, key) {
      let call = format!("this.{}(cx);", tailor_model::snake_case(action));
      out.extend(handler(em, method, params, &[call]));
    }
  }
  out
}

/// `.method({ let view = ..; move |params| { view.update(cx, |this, cx| { .. }).ok(); } })`.
/// The palette's callbacks are `'static` and take an `App`, not the screen's
/// context, so the screen is reached through a weak handle.
fn handler(em: &mut Emitter, method: &str, params: &str, body: &[String]) -> Vec<String> {
  let Some(view) = em.view_handle() else {
    // A `RenderOnce` component has no screen to call back into.
    return vec![format!(".{method}(|{params}| {{ /* handler */ }})")];
  };
  let mut out = vec![format!(".{method}({{")];
  out.push(format!("    let view = {view};"));
  out.push(format!("    move |{params}| {{"));
  out.push("        view.update(cx, |this, cx| {".into());
  out.extend(body.iter().map(|line| format!("            {line}")));
  out.push("        }).ok();".into());
  out.push("    }".into());
  out.push("})".into());
  out
}

/// One question line, parsed: `Question | Choice | Choice :: description | + Other`.
///
/// A cell after the question is a choice; `::` splits a choice from the
/// description shown under it, and a cell starting with `+` is not a choice but
/// a freeform answer, labelled with what follows.
pub struct Question {
  pub text: String,
  pub choices: Vec<(String, String)>,
  pub freeform: Option<String>,
}

pub fn question(line: &str) -> Question {
  let mut cells = line.split('|').map(str::trim);
  let text = cells.next().unwrap_or("").to_string();
  let mut choices = Vec::new();
  let mut freeform = None;
  for cell in cells.filter(|cell| !cell.is_empty()) {
    if let Some(label) = cell.strip_prefix('+') {
      let label = label.trim();
      freeform = Some(if label.is_empty() { "Other" } else { label }.to_string());
    } else {
      let (label, description) = cell.split_once("::").unwrap_or((cell, ""));
      choices.push((label.trim().to_string(), description.trim().to_string()));
    }
  }
  Question {
    text,
    choices,
    freeform,
  }
}
