//! The Rust for what floats: popovers, dialogs, sheets, notifications, menus.
//!
//! gpui-kit hands a closure a builder for all of these — a dialog's content, a
//! popover's panel, a menu's items — and the closure is `'static`, which a
//! designer cannot drop into. So the catalog gives each one regions a person
//! can fill (a trigger, a body, a footer, menu items) and this writes the
//! closure around them, cloning in whatever the regions reach for.
//!
//! Every kind here is `Generator::writes`: its regions and its handlers are
//! written whole, because a dialog's `on_ok` returns a `bool`, a popover's
//! `on_open_change` is handed the new state, and none of them is a `.child(..)`.
//! `Tooltip` is the exception: it wraps a box, so the generic walk writes the
//! children and this only adds the call that shows the text.

use tailor_codegen::node::{prefixed_call, Emitter};
use tailor_codegen::rust::{indent, string};
use tailor_codegen::style::Placement;
use tailor_model::node::DEFAULT_SLOT;
use tailor_model::{Node, NodeId};

/// Kinds whose regions and handlers this file writes.
const WRITTEN: &[&str] = &[
  "popover",
  "hovercard",
  "dialog",
  "alertdialog",
  "sheet",
  "notification",
  "dropdownbutton",
  "contextmenu",
];

/// Kinds that only mean something inside a menu.
const MENU_PARTS: &[&str] = &["menuitem", "menuseparator", "menulabel", "submenu"];

pub fn writes(kind: &str) -> bool {
  WRITTEN.contains(&kind)
}

/// The constructor line for a kind this file owns.
pub fn special(em: &mut Emitter, node: &Node) -> Option<Vec<String>> {
  let id = string(&node.id.element_id());
  Some(match node.kind.as_str() {
    "dialog" => vec!["Dialog::new(cx)".into()],
    "alertdialog" => vec!["AlertDialog::new(cx)".into()],
    // gpui-kit opens these by calling the window, so what is on the canvas is
    // the button that calls it.
    "sheet" | "notification" => vec![format!("Button::new({id})")],
    "dropdownbutton" => vec![format!("DropdownButton::new({id})")],
    // Both wrap what is inside them in an interactive box.
    "contextmenu" | "tooltip" => vec![format!("div().id({id})")],
    kind if MENU_PARTS.contains(&kind) => {
      em.notes.push(format!(
        "{} is a menu entry, but it is not inside a Dropdown button, Context menu or Submenu, \
         so it generates as an empty box",
        kind
      ));
      vec!["div()".into()]
    }
    _ => return None,
  })
}

/// The regions and handlers of a kind this file owns, appended after the
/// constructor and the props the generic walk prints.
pub fn slots(em: &mut Emitter, node: &Node, placement: Placement) -> Option<Vec<String>> {
  Some(match node.kind.as_str() {
    "popover" => floating(em, node, placement, "popover"),
    "hovercard" => floating(em, node, placement, "hovercard"),
    "dialog" => dialog(em, node, placement, false),
    "alertdialog" => dialog(em, node, placement, true),
    "sheet" => sheet(em, node, placement),
    "notification" => notification(em, node),
    "dropdownbutton" => dropdown(em, node),
    "contextmenu" => context_menu(em, node, placement),
    "tooltip" => vec![format!(
      ".tooltip(|window, cx| Tooltip::new({}).build(window, cx))",
      string(&text(em, node, "text"))
    )],
    _ => return None,
  })
}

// Small readers

fn text(em: &Emitter, node: &Node, key: &str) -> String {
  em.prop_value(node, key).as_str().unwrap_or("").to_string()
}

fn flag(em: &Emitter, node: &Node, key: &str) -> bool {
  em.prop_value(node, key).as_bool().unwrap_or(false)
}

fn int(em: &Emitter, node: &Node, key: &str) -> i64 {
  em.prop_value(node, key).as_f64().unwrap_or(0.0) as i64
}

/// Whether a name is used as an identifier in some lines — decides whether a
/// closure parameter is called `cx` or `_cx`, so the file does not open with
/// warnings about arguments a closure has to take and did not use.
///
/// A nested closure that names its own `cx` shadows this one's, so the lines
/// inside it do not count: the body of `move |_, _window, cx| { .. }` is using
/// *its* `cx`. Blocks are found by indentation, which the generator keeps
/// consistent, and a closure's header line counts only up to its first `|`
/// — `.submenu("More", window, cx, move |menu, window, cx| {` uses the outer
/// `window` and `cx` and declares two more.
fn uses(lines: &[String], name: &str) -> bool {
  let word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
  let mentions = |text: &str| {
    text.match_indices(name).any(|(at, _)| {
      !word(text[..at].chars().next_back()) && !word(text[at + name.len()..].chars().next())
    })
  };
  let depth = |line: &str| line.len() - line.trim_start().len();

  let mut shadow: Option<usize> = None;
  for line in lines {
    if let Some(at) = shadow {
      if depth(line) > at {
        continue;
      }
      shadow = None;
    }
    if let Some(open) = line.find('|') {
      let close = line[open + 1..].find('|').map(|c| open + 1 + c);
      let declared = close.is_some_and(|close| mentions(&line[open..=close]));
      if declared {
        if mentions(&line[..open]) {
          return true;
        }
        // A one-line closure is over already; a block one shadows what follows.
        if line.trim_end().ends_with('{') {
          shadow = Some(depth(line));
        }
        continue;
      }
    }
    if mentions(line) {
      return true;
    }
  }
  false
}

/// `|a, b, c|`, with each name left bare when the body uses it and given a
/// leading underscore when it does not.
fn params(names: &[&str], body: &[String]) -> String {
  let named: Vec<String> = names
    .iter()
    .map(|name| {
      if uses(body, name) {
        (*name).to_string()
      } else {
        format!("_{name}")
      }
    })
    .collect();
  format!("|{}|", named.join(", "))
}

/// The children of a region as lines of `.child(..)` calls.
fn children(em: &mut Emitter, node: &Node, key: &str, placement: Placement) -> Vec<String> {
  let mut out = Vec::new();
  for child in node.slot(key).to_vec() {
    let inner = em.emit(child, placement);
    out.extend(prefixed_call("child", inner));
  }
  out
}

/// A closure region: `head` then a closure over `names` whose body is built by
/// `build`, with whatever the body reached for cloned in first.
fn closure(
  em: &mut Emitter,
  head: &str,
  names: &[&str],
  build: impl FnOnce(&mut Emitter) -> Vec<String>,
) -> Vec<String> {
  em.push_scope();
  let body = build(em);
  let captured = em.pop_scope();
  let params = params(names, &body);
  em.wrap_closure_with(head.to_string(), &params, captured, body)
}

/// An action bound to one of a node's events, as the method it calls.
fn bound(node: &Node, event: &str) -> Option<String> {
  node
    .events
    .get(event)
    .filter(|action| !action.is_empty())
    .map(|action| tailor_model::snake_case(action))
}

/// A handler for `event`, or nothing when no action is bound. `args` is the
/// closure's parameter list, `tail` what it returns after calling the action
/// — `true` for the callbacks that decide whether a dialog closes.
///
/// A component with no screen to call back into (a `RenderOnce` builder) has
/// nowhere to send it, so the handler is a stub that says which action it
/// would have run, the way every other generated handler is.
fn handler(
  em: &mut Emitter,
  node: &Node,
  event: &str,
  method: &str,
  args: &str,
  tail: Option<&str>,
) -> Vec<String> {
  let Some(action) = bound(node, event) else {
    return Vec::new();
  };
  let tail_line = tail.map(|t| vec![format!("    {t}")]).unwrap_or_default();
  match em.view_handle() {
    Some(view) => {
      let mut out = vec![format!(".{method}({{")];
      out.push(format!("    let view = {view};"));
      out.push(format!("    move |{args}| {{"));
      out.push(format!(
        "        view.update(cx, |this, cx| this.{action}(cx)).ok();"
      ));
      out.extend(tail_line.iter().map(|l| format!("    {l}")));
      out.push("    }".into());
      out.push("})".into());
      out
    }
    None => {
      let mut out = vec![format!(".{method}(|{args}| {{ /* {action} */")];
      if let Some(t) = tail {
        out.push(format!("    {t}"));
      }
      out.push("})".into());
      out
    }
  }
}

/// The trigger region as `.trigger(..)`, or `fallback` when nothing is in it: a
/// popover or dialog with no trigger draws nothing, and a design that exports
/// to nothing is worse than one that exports to an obvious placeholder.
fn trigger(em: &mut Emitter, node: &Node, placement: Placement, fallback: &str) -> Vec<String> {
  match node.slot("trigger").first().copied() {
    Some(child) => {
      let inner = em.emit(child, placement);
      prefixed_call("trigger", inner)
    }
    None => vec![format!(".trigger({fallback})")],
  }
}

// Popover and hover card

fn floating(em: &mut Emitter, node: &Node, placement: Placement, kind: &str) -> Vec<String> {
  let mut out = Vec::new();
  let fallback = if kind == "popover" {
    format!(
      "Button::new({}).label(\"Open\")",
      string(&format!("{}-trigger", node.id.element_id()))
    )
  } else {
    "div().child(\"Hover me\")".to_string()
  };
  out.extend(trigger(em, node, placement, &fallback));

  if kind == "hovercard" {
    for (key, method, default) in [
      ("open_delay", "open_delay", 600),
      ("close_delay", "close_delay", 300),
    ] {
      let ms = int(em, node, key);
      if ms != default {
        out.push(format!(".{method}(std::time::Duration::from_millis({ms}))"));
      }
    }
  }

  out.extend(closure(em, ".content(", &["state", "window", "cx"], |em| {
    let kids = node.slot(DEFAULT_SLOT).to_vec();
    if kids.is_empty() {
      return vec!["div()".into()];
    }
    let mut body = vec!["div().flex().flex_col().gap(px(8.))".to_string()];
    for child in kids {
      let inner = em.emit(child, placement);
      body.extend(indent(&prefixed_call("child", inner)));
    }
    body
  }));

  out.extend(handler(
    em,
    node,
    "open_change",
    "on_open_change",
    "_open, _window, cx",
    None,
  ));
  out
}

// Dialog and alert dialog

fn dialog(em: &mut Emitter, node: &Node, placement: Placement, alert: bool) -> Vec<String> {
  let mut out = Vec::new();
  let id = node.id.element_id();

  let fallback = format!(
    "Button::new({}).outline().label(\"Open\")",
    string(&format!("{id}-open"))
  );
  out.extend(trigger(em, node, placement, &fallback));

  let width = int(em, node, "width");
  if width != 448 && width > 0 {
    out.push(format!(
      ".{}(px({width}.))",
      if alert { "width" } else { "w" }
    ));
  }

  let title = text(em, node, "title");
  let description = text(em, node, "description");
  let ok = text(em, node, "ok_label");
  let cancel = text(em, node, "cancel_label");
  let danger = flag(em, node, "destructive");
  let has_footer = !node.slot("footer").is_empty();

  out.extend(closure(
    em,
    ".content(",
    &["content", "window", "cx"],
    |em| {
      let mut chain = vec!["content".to_string()];

      if !title.is_empty() || !description.is_empty() {
        let mut header = vec!["DialogHeader::new()".to_string()];
        if !title.is_empty() {
          header.push(format!(
            "    .child(DialogTitle::new().child({}))",
            string(&title)
          ));
        }
        if !description.is_empty() {
          header.push(format!(
            "    .child(DialogDescription::new().child({}))",
            string(&description)
          ));
        }
        chain.extend(indent(&prefixed_call("child", header)));
      }

      // The body: a dialog's own children. An alert has none, only its text.
      if !alert && !node.slot(DEFAULT_SLOT).is_empty() {
        let mut body = vec!["v_flex().gap_3()".to_string()];
        body.extend(indent(&children(em, node, DEFAULT_SLOT, placement)));
        chain.extend(indent(&prefixed_call("child", body)));
      }

      let mut footer = vec!["DialogFooter::new()".to_string()];
      if !alert && has_footer {
        footer.extend(indent(&children(em, node, "footer", placement)));
      } else {
        if !cancel.is_empty() {
          footer.push(format!(
            "    .child(DialogClose::new().child(Button::new({}).outline().label({})))",
            string(&format!("{id}-cancel")),
            string(&cancel)
          ));
        }
        if !ok.is_empty() {
          footer.push(format!(
            "    .child(DialogAction::new().child(Button::new({}).label({}){}))",
            string(&format!("{id}-ok")),
            string(&ok),
            if danger { ".danger()" } else { "" }
          ));
        }
      }
      if footer.len() > 1 {
        chain.extend(indent(&prefixed_call("child", footer)));
      }
      chain
    },
  ));

  out.extend(handler(
    em,
    node,
    "ok",
    "on_ok",
    "_, _window, cx",
    Some("true"),
  ));
  out.extend(handler(
    em,
    node,
    "cancel",
    "on_cancel",
    "_, _window, cx",
    Some("true"),
  ));
  if !alert {
    out.extend(handler(
      em,
      node,
      "close",
      "on_close",
      "_, _window, cx",
      None,
    ));
  }
  out
}

// Sheet and notification

/// The button's click, calling the window. `body` is what the click does; it
/// is handed the enclosing closure's window and context by name.
fn on_click(em: &mut Emitter, body: impl FnOnce(&mut Emitter) -> Vec<String>) -> Vec<String> {
  closure(em, ".on_click(", &["event", "window", "cx"], |em| body(em))
}

fn sheet(em: &mut Emitter, node: &Node, placement: Placement) -> Vec<String> {
  let title = text(em, node, "title");
  let edge = match em.prop_value(node, "placement").as_str() {
    Some("left") => "Left",
    Some("top") => "Top",
    Some("bottom") => "Bottom",
    _ => "Right",
  };
  let size = int(em, node, "size");
  let overlay = flag(em, node, "overlay");
  let closable = flag(em, node, "overlay_closable");
  let resizable = flag(em, node, "resizable");

  on_click(em, |em| {
    // The sheet's own builder is a second closure inside the click handler, so
    // what its body reaches for is cloned once more, inside the first.
    em.push_scope();
    let mut inner = vec!["sheet".to_string()];
    if !title.is_empty() {
      inner.push(format!("    .title({})", string(&title)));
    }
    if size != 350 && size > 0 {
      inner.push(format!("    .size(px({size}.))"));
    }
    if !overlay {
      inner.push("    .overlay(false)".into());
    }
    if !closable {
      inner.push("    .overlay_closable(false)".into());
    }
    if !resizable {
      inner.push("    .resizable(false)".into());
    }
    inner.extend(indent(&handler(
      em,
      node,
      "close",
      "on_close",
      "_, _window, cx",
      None,
    )));
    inner.extend(indent(&children(em, node, DEFAULT_SLOT, placement)));
    if !node.slot("footer").is_empty() {
      let mut footer = vec!["h_flex().gap_2().justify_end()".to_string()];
      footer.extend(indent(&children(em, node, "footer", placement)));
      inner.extend(indent(&prefixed_call("footer", footer)));
    }
    let captured = em.pop_scope();
    let inner_params = params(&["sheet", "window", "cx"], &inner);

    // Cloned in at the top of the click handler, then moved into the sheet's
    // closure: a `Fn` closure cannot give away what it captured.
    let mut body: Vec<String> = captured
      .iter()
      .map(|name| format!("let {name} = {name}.clone();"))
      .collect();
    if let Some(scope) = em.scope_mut() {
      scope.extend(captured.iter().cloned());
    }
    body.push(format!(
      "window.open_sheet_at(Placement::{edge}, cx, move {inner_params} {{"
    ));
    body.extend(indent(&inner));
    body.push("});".into());
    body
  })
}

fn notification(em: &mut Emitter, node: &Node) -> Vec<String> {
  let title = text(em, node, "title");
  let message = text(em, node, "message");
  let kind = match em.prop_value(node, "kind").as_str() {
    Some("success") => "Success",
    Some("warning") => "Warning",
    Some("error") => "Error",
    _ => "Info",
  };
  let autohide = flag(em, node, "autohide");

  on_click(em, |_| {
    let mut note = vec!["Notification::new()".to_string()];
    if !title.is_empty() {
      note.push(format!("    .title({})", string(&title)));
    }
    if !message.is_empty() {
      note.push(format!("    .message({})", string(&message)));
    }
    if kind != "Info" {
      note.push(format!("    .with_type(NotificationType::{kind})"));
    }
    if !autohide {
      note.push("    .autohide(false)".into());
    }
    let mut body = vec!["window.push_notification(".to_string()];
    body.extend(indent(
      &note
        .iter()
        .enumerate()
        .map(|(i, l)| {
          if i + 1 == note.len() {
            format!("{l},")
          } else {
            l.clone()
          }
        })
        .collect::<Vec<_>>(),
    ));
    body.push("    cx,".into());
    body.push(");".into());
    body
  })
}

// Menus

/// The items of a menu as calls on `menu`, in order.
fn menu_items(em: &mut Emitter, node: &Node, key: &str) -> Vec<String> {
  let mut out = Vec::new();
  for id in node.slot(key).to_vec() {
    let Some(item) = em.doc.node(id).cloned() else {
      continue;
    };
    out.extend(menu_item(em, id, &item));
  }
  out
}

fn menu_item(em: &mut Emitter, _id: NodeId, item: &Node) -> Vec<String> {
  match item.kind.as_str() {
    "menuseparator" => vec![".separator()".into()],
    "menulabel" => vec![format!(".label({})", string(&text(em, item, "label")))],
    "submenu" => {
      let label = text(em, item, "label");
      let icon = text(em, item, "icon");
      let head = if icon.is_empty() {
        format!(".submenu({}, window, cx, ", string(&label))
      } else {
        format!(
          ".submenu_with_icon(Some({}.into()), {}, window, cx, ",
          tailor_codegen::expr::icon_path(&icon),
          string(&label)
        )
      };
      closure(em, &head, &["menu", "window", "cx"], |em| {
        let mut body = vec!["menu".to_string()];
        body.extend(indent(&menu_items(em, item, "items")));
        body
      })
    }
    "menuitem" => {
      let mut chain = vec![format!(
        "PopupMenuItem::new({})",
        string(&text(em, item, "label"))
      )];
      let icon = text(em, item, "icon");
      if !icon.is_empty() {
        chain.push(format!(
          "    .icon({})",
          tailor_codegen::expr::icon_path(&icon)
        ));
      }
      if flag(em, item, "checked") {
        chain.push("    .checked(true)".into());
      }
      if flag(em, item, "disabled") {
        chain.push("    .disabled(true)".into());
      }
      chain.extend(indent(&handler(
        em,
        item,
        "click",
        "on_click",
        "_, _window, cx",
        None,
      )));
      prefixed_call("item", chain)
    }
    // Anything else dropped in a menu is not an entry.
    _ => Vec::new(),
  }
}

/// The closure over a menu, whose body is `menu` and one call per item.
fn menu_closure(em: &mut Emitter, head: &str, node: &Node) -> Vec<String> {
  closure(em, head, &["menu", "window", "cx"], |em| {
    let mut body = vec!["menu".to_string()];
    body.extend(indent(&menu_items(em, node, "items")));
    body
  })
}

fn dropdown(em: &mut Emitter, node: &Node) -> Vec<String> {
  let id = node.id.element_id();
  let mut button = vec![format!("Button::new({})", string(&format!("{id}-button")))];
  let label = text(em, node, "label");
  if !label.is_empty() {
    button.push(format!("    .label({})", string(&label)));
  }
  let icon = text(em, node, "icon");
  if !icon.is_empty() {
    button.push(format!(
      "    .icon({})",
      tailor_codegen::expr::icon_path(&icon)
    ));
  }
  let mut out = prefixed_call("button", button);
  out.extend(menu_closure(em, ".dropdown_menu(", node));
  out
}

fn context_menu(em: &mut Emitter, node: &Node, placement: Placement) -> Vec<String> {
  let mut out = children(em, node, DEFAULT_SLOT, placement);
  out.extend(menu_closure(em, ".context_menu(", node));
  out
}
