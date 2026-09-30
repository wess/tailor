//! Overlays, run through the generator and — when asked — the compiler.
//!
//! [`export_for_compilation`] is the ground truth, as in `codegen.rs`: it writes
//! a screen with one of every overlay, nested the awkward ways, to
//! `$TAILOR_OVERLAY_DIR` so it can be built against the real gpui-kit. The
//! rest assert the shapes that a compiler would only catch in a build.

use tailor_codegen::app::{cargo_toml, main_rs};
use tailor_codegen::file::document;
use tailor_codegen::project_files;
use tailor_model::node::DEFAULT_SLOT;
use tailor_model::props::PropValue;
use tailor_model::{ActionDef, DocKind, NodeId, Project};

fn project(name: &str) -> Project {
  tailor_gpuikit::register();
  let mut project = Project::new(name);
  project.library = "gpuikit".into();
  project.docs[0].kind = DocKind::Screen;
  project
}

/// Add a node of `kind` under `parent`'s `slot`, with props and bound events.
fn add(
  project: &mut Project,
  parent: NodeId,
  slot: &str,
  kind: &str,
  props: &[(&str, PropValue)],
  events: &[(&str, &str)],
) -> NodeId {
  let doc = &mut project.docs[0];
  // Through the catalog, so a node has the defaults placing it would give it.
  let mut node = match tailor_gpuikit::library().get(kind) {
    Some(spec) => spec.build(doc.ids.next()),
    None => doc.create(kind),
  };
  for (key, value) in props {
    node.set_prop(*key, value.clone());
  }
  for (event, action) in events {
    node.events.insert((*event).into(), (*action).into());
  }
  doc.insert(parent, slot, usize::MAX, node)
}

fn text(value: &str) -> PropValue {
  PropValue::Text(value.into())
}

fn button(project: &mut Project, parent: NodeId, slot: &str, label: &str, action: &str) -> NodeId {
  let events: Vec<(&str, &str)> = if action.is_empty() {
    Vec::new()
  } else {
    vec![("click", action)]
  };
  add(
    project,
    parent,
    slot,
    "button",
    &[("label", text(label))],
    &events,
  )
}

/// Every overlay, and a text field inside two of them.
fn overlays() -> Project {
  let mut p = project("Overlays");
  let root = p.docs[0].root;
  for name in [
    "save",
    "confirmed",
    "declined",
    "picked",
    "closed",
    "changed",
    "note",
  ] {
    p.docs[0].actions.push(ActionDef::new(name));
  }

  // A dialog with a field and a button in its body, and handlers.
  let dialog = add(
    &mut p,
    root,
    DEFAULT_SLOT,
    "dialog",
    &[
      ("title", text("Rename")),
      ("description", text("Pick a new name.")),
    ],
    &[
      ("ok", "confirmed"),
      ("cancel", "declined"),
      ("close", "closed"),
    ],
  );
  button(&mut p, dialog, "trigger", "Rename", "");
  add(
    &mut p,
    dialog,
    DEFAULT_SLOT,
    "input",
    &[("placeholder", text("Name"))],
    &[],
  );
  button(&mut p, dialog, DEFAULT_SLOT, "Suggest", "save");

  // A dialog that brings its own footer.
  let custom = add(
    &mut p,
    root,
    DEFAULT_SLOT,
    "dialog",
    &[("title", text("Custom"))],
    &[],
  );
  button(&mut p, custom, "trigger", "Custom footer", "");
  button(&mut p, custom, "footer", "Done", "save");

  add(
    &mut p,
    root,
    DEFAULT_SLOT,
    "alertdialog",
    &[
      ("title", text("Delete it?")),
      ("description", text("This cannot be undone.")),
      ("ok_label", text("Delete")),
      ("destructive", PropValue::Bool(true)),
    ],
    &[("ok", "confirmed"), ("cancel", "declined")],
  );
  let alert = p.docs[0]
    .node(root)
    .unwrap()
    .slot(DEFAULT_SLOT)
    .last()
    .copied()
    .unwrap();
  button(&mut p, alert, "trigger", "Delete", "");

  // A popover holding a button with a handler, and a dialog: an overlay inside
  // an overlay's closure.
  let popover = add(
    &mut p,
    root,
    DEFAULT_SLOT,
    "popover",
    &[("arrow", PropValue::Bool(true))],
    &[("open_change", "changed")],
  );
  button(&mut p, popover, "trigger", "Options", "");
  button(&mut p, popover, DEFAULT_SLOT, "Save", "save");
  let inner = add(
    &mut p,
    popover,
    DEFAULT_SLOT,
    "dialog",
    &[("title", text("Inner"))],
    &[("ok", "confirmed")],
  );
  button(&mut p, inner, "trigger", "More", "");

  let hover = add(
    &mut p,
    root,
    DEFAULT_SLOT,
    "hovercard",
    &[("open_delay", PropValue::Int(200))],
    &[],
  );
  add(
    &mut p,
    hover,
    "trigger",
    "label",
    &[("text", text("Hover me"))],
    &[],
  );
  add(
    &mut p,
    hover,
    DEFAULT_SLOT,
    "label",
    &[("text", text("Details"))],
    &[],
  );

  // A sheet with a field and a button, opened from a button of its own.
  let sheet = add(
    &mut p,
    root,
    DEFAULT_SLOT,
    "sheet",
    &[
      ("label", text("Open sheet")),
      ("title", text("Filters")),
      ("placement", PropValue::Choice("left".into())),
      ("size", PropValue::Int(300)),
    ],
    &[("close", "closed")],
  );
  add(
    &mut p,
    sheet,
    DEFAULT_SLOT,
    "input",
    &[("placeholder", text("Search"))],
    &[],
  );
  button(&mut p, sheet, DEFAULT_SLOT, "Apply", "save");
  button(&mut p, sheet, "footer", "Reset", "save");

  add(
    &mut p,
    root,
    DEFAULT_SLOT,
    "notification",
    &[
      ("label", text("Notify")),
      ("title", text("Saved")),
      ("message", text("Your changes were saved.")),
      ("kind", PropValue::Choice("success".into())),
    ],
    &[],
  );

  // A menu with every entry kind, one of them a submenu.
  let menu = add(
    &mut p,
    root,
    DEFAULT_SLOT,
    "dropdownbutton",
    &[("label", text("Actions"))],
    &[],
  );
  add(
    &mut p,
    menu,
    "items",
    "menulabel",
    &[("label", text("File"))],
    &[],
  );
  add(
    &mut p,
    menu,
    "items",
    "menuitem",
    &[("label", text("Save"))],
    &[("click", "save")],
  );
  add(
    &mut p,
    menu,
    "items",
    "menuitem",
    &[
      ("label", text("Autosave")),
      ("checked", PropValue::Bool(true)),
    ],
    &[("click", "picked")],
  );
  add(&mut p, menu, "items", "menuseparator", &[], &[]);
  let more = add(
    &mut p,
    menu,
    "items",
    "submenu",
    &[("label", text("More"))],
    &[],
  );
  add(
    &mut p,
    more,
    "items",
    "menuitem",
    &[("label", text("Export"))],
    &[("click", "note")],
  );
  add(
    &mut p,
    menu,
    "items",
    "menuitem",
    &[("label", text("Off")), ("disabled", PropValue::Bool(true))],
    &[],
  );

  let context = add(&mut p, root, DEFAULT_SLOT, "contextmenu", &[], &[]);
  add(
    &mut p,
    context,
    DEFAULT_SLOT,
    "label",
    &[("text", text("Right-click me"))],
    &[],
  );
  add(
    &mut p,
    context,
    "items",
    "menuitem",
    &[("label", text("Copy"))],
    &[("click", "note")],
  );

  let tip = add(
    &mut p,
    root,
    DEFAULT_SLOT,
    "tooltip",
    &[("text", text("Saves the file"))],
    &[],
  );
  button(&mut p, tip, DEFAULT_SLOT, "Hover for a hint", "");
  p
}

fn file() -> String {
  let p = overlays();
  document(&p, &p.docs[0]).source
}

#[test]
fn a_dialog_is_a_declarative_trigger_and_a_content_closure() {
  let file = file();
  assert!(file.contains("Dialog::new(cx)"), "{file}");
  assert!(file.contains(".trigger("), "{file}");
  assert!(
    file.contains(".content(") && file.contains("DialogHeader::new()"),
    "{file}"
  );
  assert!(
    file.contains("DialogTitle::new().child(\"Rename\")"),
    "{file}"
  );
  // The confirm and cancel buttons are written when there is no footer, and a
  // destructive confirm is marked.
  assert!(
    file.contains("DialogClose::new()") && file.contains("DialogAction::new()"),
    "{file}"
  );
  assert!(file.contains(".danger()"), "{file}");
}

#[test]
fn a_footer_region_replaces_the_default_buttons() {
  let file = file();
  let custom = file.split("\"Custom\"").nth(1).unwrap();
  let custom = &custom[..custom.find("Alert").unwrap_or(custom.len()).min(900)];
  assert!(custom.contains("Done"), "{file}");
  assert!(!custom.contains("DialogClose"), "{file}");
}

#[test]
fn dialog_handlers_return_whether_to_close() {
  let file = file();
  // `on_ok` and `on_cancel` decide whether the dialog closes; `on_close` does not.
  assert!(file.contains(".on_ok({"), "{file}");
  assert!(file.contains("this.confirmed(cx)"), "{file}");
  assert!(file.contains(".on_cancel({"), "{file}");
  let ok = file.split(".on_ok({").nth(1).unwrap();
  assert!(ok[..ok.find("})").unwrap()].contains("true"), "{file}");
  let close = file.split(".on_close({").nth(1).unwrap();
  assert!(
    !close[..close.find("})").unwrap()].contains("true"),
    "{file}"
  );
}

#[test]
fn a_sheet_and_a_notification_are_buttons_that_call_the_window() {
  let file = file();
  assert!(
    file.contains("window.open_sheet_at(Placement::Left, cx,"),
    "{file}"
  );
  assert!(file.contains(".size(px(300.))"), "{file}");
  assert!(file.contains("window.push_notification("), "{file}");
  assert!(file.contains("NotificationType::Success"), "{file}");
}

#[test]
fn what_a_nested_closure_reaches_for_is_cloned_at_every_level() {
  let file = file();
  // The sheet's field is used in the sheet's closure, which is inside the
  // click handler's: it is cloned into the handler, then into the sheet.
  let sheet = file.split("open_sheet_at").nth(1).unwrap();
  assert!(sheet.contains("Input::new(&"), "{file}");
  let before = file.split("window.open_sheet_at").next().unwrap();
  let handler = &before[before.rfind(".on_click(").unwrap()..];
  assert!(handler.contains(".clone()"), "{file}");
}

#[test]
fn menu_entries_become_calls_on_the_menu() {
  let file = file();
  assert!(file.contains(".dropdown_menu("), "{file}");
  assert!(file.contains("PopupMenuItem::new(\"Save\")"), "{file}");
  assert!(file.contains(".checked(true)"), "{file}");
  assert!(file.contains(".disabled(true)"), "{file}");
  assert!(file.contains(".separator()"), "{file}");
  assert!(file.contains(".label(\"File\")"), "{file}");
  assert!(file.contains(".submenu(\"More\", window, cx,"), "{file}");
  assert!(file.contains(".context_menu("), "{file}");
  assert!(
    file.contains(".tooltip(|window, cx| Tooltip::new(\"Saves the file\")"),
    "{file}"
  );
}

#[test]
fn a_closure_parameter_the_body_never_uses_is_left_unnamed() {
  let file = file();
  // The alert's content reaches for neither the window nor the context.
  assert!(
    file.contains(".content(|content, _window, _cx| {"),
    "{file}"
  );
  // A popover's panel that holds a dialog does use the context, for
  // `Dialog::new(cx)`, and its state it does not.
  assert!(file.contains("move |_state, _window, cx|"), "{file}");
  // Nothing is named with two underscores.
  assert!(!file.contains("__"), "{file}");
}

#[test]
fn a_menu_entry_outside_a_menu_is_noted_not_broken() {
  let mut p = project("Stray");
  let root = p.docs[0].root;
  add(
    &mut p,
    root,
    DEFAULT_SLOT,
    "menuitem",
    &[("label", text("Lost"))],
    &[],
  );
  let generated = document(&p, &p.docs[0]);
  assert!(
    generated.notes.iter().any(|n| n.contains("not inside")),
    "{:?}",
    generated.notes
  );
}

#[test]
fn an_empty_trigger_still_exports_something_you_can_click() {
  let mut p = project("Bare");
  let root = p.docs[0].root;
  add(&mut p, root, DEFAULT_SLOT, "popover", &[], &[]);
  add(&mut p, root, DEFAULT_SLOT, "dialog", &[], &[]);
  let file = document(&p, &p.docs[0]).source;
  assert!(
    file.contains("Button::new(\"node-") || file.contains("Button::new("),
    "{file}"
  );
  assert!(file.contains(".trigger(Button::new("), "{file}");
}

#[test]
fn export_for_compilation() {
  let Ok(dir) = std::env::var("TAILOR_OVERLAY_DIR") else {
    return;
  };
  let p = overlays();
  let dir = std::path::Path::new(&dir);
  let mut files = project_files(&p);
  let mut manifest = cargo_toml(&p);
  manifest.path = "Cargo.toml".into();
  files.push(manifest);
  let mut main = main_rs(&p);
  main.path = "src/main.rs".into();
  let _ = main;
  for file in files {
    let path = dir.join(&file.path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, file.source).unwrap();
  }
}
