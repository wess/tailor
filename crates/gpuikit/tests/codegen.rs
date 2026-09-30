//! The generator, run against gpui-kit.
//!
//! An integration test rather than a `#[cfg(test)]` module for the same reason
//! as guise's: the registries live in `tailor-codegen`, and a dev-dependency
//! back onto a provider would compile that crate twice.
//!
//! [`export_for_compilation`] is the ground truth. It writes a full project —
//! one node for every catalogued kind and one more for every value of every
//! choice — to `$TAILOR_EXPORT_DIR`, so it can be built against the real
//! gpui-kit. It does nothing when the variable is unset, since a build of
//! gpui-kit is minutes, not a unit test.

use tailor_codegen::app::{cargo_toml, main_rs};
use tailor_codegen::file::document;
use tailor_codegen::{preview, project_files};
use tailor_model::node::DEFAULT_SLOT;
use tailor_model::props::{PropType, PropValue};
use tailor_model::{DocKind, Project};

fn project(name: &str) -> Project {
  tailor_gpuikit::register();
  let mut project = Project::new(name);
  project.library = "gpuikit".into();
  project
}

#[test]
fn the_manifest_renames_gpui_kit_to_gpui() {
  let manifest = cargo_toml(&project("My App"));
  assert!(manifest.source.contains("name = \"my_app\""));
  assert!(
    manifest
      .source
      .contains("gpui = { package = \"gpui-kit\", version = \"0.7\" }"),
    "{}",
    manifest.source
  );
  // One dependency, not gpui and gpui-kit side by side: two gpuis is the
  // problem the rename exists to avoid.
  assert!(!manifest.source.contains("0.2.2"));
  assert_eq!(manifest.source.matches("gpui").count(), 2);
}

#[test]
fn main_opens_its_window_through_the_facade() {
  let main = main_rs(&project("Demo")).source;
  assert!(
    main.contains("gpui::application().with_assets(gpui::assets::Assets).run("),
    "{main}"
  );
  assert!(main.contains("gpui::open_window("), "{main}");
  assert!(main.contains("theme::init(cx);"), "{main}");
  assert!(!main.contains("Application::new()"), "{main}");
  assert!(!main.contains("cx.open_window("), "{main}");
  // The facade takes `cx` second.
  let window = main.find("gpui::open_window(").unwrap();
  assert!(
    main[window..].contains("\n      cx,\n") || main[window..].contains("cx,"),
    "{main}"
  );
}

#[test]
fn theme_init_initialises_the_library_and_applies_the_scheme() {
  let files = project_files(&project("Demo"));
  let theme = files.iter().find(|f| f.path == "src/theme.rs").unwrap();
  assert!(theme.source.contains("gpui::init(cx);"));
  assert!(theme.source.contains("Theme::change(ThemeMode::"));
  assert!(theme.notes.is_empty());
}

#[test]
fn a_guise_theme_file_is_reported_not_applied() {
  let mut project = project("Demo");
  project.theme.json = "{}".into();
  let theme = tailor_codegen::generator::for_project(&project).theme_rs(&project);
  assert_eq!(theme.notes.len(), 1);
  assert!(!theme.source.contains("include_str!"));
}

#[test]
fn every_catalog_kind_generates_something() {
  for spec in tailor_gpuikit::library().components() {
    let mut project = project("Demo");
    let doc = &mut project.docs[0];
    let root = doc.root;
    let node = doc.create(spec.kind);
    doc.insert(root, DEFAULT_SLOT, 0, node);

    let file = preview(&project, &project.docs[0]);
    assert!(!file.source.is_empty(), "{} generated nothing", spec.kind);
    if spec.ctor != tailor_model::catalog::Ctor::Special && !spec.rust.is_empty() {
      assert!(
        file.source.contains(spec.rust),
        "{} generated no reference to {}",
        spec.kind,
        spec.rust
      );
    }
  }
}

#[test]
fn the_hand_written_kinds_name_their_component() {
  for (kind, expected) in [
    ("frame", "div()"),
    ("hstack", "h_flex()"),
    ("vstack", "v_flex()"),
    ("separator", "Separator::horizontal()"),
    ("kbd", "Kbd::new(gpui::Keystroke::parse(\"cmd-k\")"),
    ("link", "Link::new(\"node-"),
    ("tag", "Tag::new()"),
    ("empty", "Empty::new()"),
    ("breadcrumb", "BreadcrumbItem::new(\"Home\")"),
    ("tabbar", "Tab::new().label(\"One\")"),
    ("stepper", "StepperItem::new().child(\"Account\")"),
    ("radiogroup", "Radio::new(\"node-"),
  ] {
    let mut project = project("Demo");
    let doc = &mut project.docs[0];
    let root = doc.root;
    let node = doc.create(kind);
    doc.insert(root, DEFAULT_SLOT, 0, node);
    let source = preview(&project, &project.docs[0]).source;
    assert!(
      source.contains(expected),
      "{kind} did not emit {expected}:\n{source}"
    );
  }
}

#[test]
fn a_variant_and_a_size_are_gpui_kits_own_enums() {
  let mut project = project("Demo");
  let doc = &mut project.docs[0];
  let root = doc.root;
  let mut button = doc.create("button");
  button.set_prop("label", PropValue::Text("Go".into()));
  button.set_prop("variant", PropValue::Choice("primary".into()));
  button.set_prop("size", PropValue::Choice("x-small".into()));
  doc.insert(root, DEFAULT_SLOT, 0, button);
  let source = document(&project, &project.docs[0]).source;
  assert!(
    source.contains(".with_variant(ButtonVariant::Primary)"),
    "{source}"
  );
  assert!(source.contains(".with_size(Size::XSmall)"), "{source}");
}

#[test]
fn the_default_choice_prints_nothing() {
  let mut project = project("Demo");
  let doc = &mut project.docs[0];
  let root = doc.root;
  let button = doc.create("button");
  doc.insert(root, DEFAULT_SLOT, 0, button);
  let source = document(&project, &project.docs[0]).source;
  assert!(!source.contains("with_variant"), "{source}");
  assert!(!source.contains("with_size"), "{source}");
}

/// A screen holding every kind twice over: once with its props all set, and
/// once more per choice value, so every enum the catalog names is spelled at
/// least once in code a compiler reads.
/// Where a kind lives: the root, or a fresh parent (built the same way, so a
/// cell gets a row gets a table) and the slot of it the part belongs in.
fn home(
  doc: &mut tailor_model::Document,
  root: tailor_model::NodeId,
  kind: &str,
) -> (tailor_model::NodeId, String) {
  let library = tailor_gpuikit::library();
  let Some(parent_kind) = library.parents(kind).first().copied() else {
    return (root, DEFAULT_SLOT.to_string());
  };
  let node = doc.create(parent_kind);
  let (grand, grand_slot) = home(doc, root, parent_kind);
  let id = doc.insert(grand, &grand_slot, usize::MAX, node);
  let slots = library
    .get(parent_kind)
    .map(|spec| spec.slots)
    .unwrap_or(&[]);
  let slot = slots
    .iter()
    .find(|s| kind.contains(s.key.trim_end_matches('s')) && s.key != DEFAULT_SLOT)
    .or_else(|| slots.iter().find(|s| s.key.contains("item")))
    .map(|s| s.key.to_string())
    .unwrap_or_else(|| DEFAULT_SLOT.to_string());
  (id, slot)
}

fn everything() -> Project {
  let mut project = project("Kit Export");
  project.docs[0].kind = DocKind::Screen;
  let root = project.docs[0].root;
  let mut at = 0;

  // `TAILOR_EXPORT_ONLY=table,list` narrows the export to those kinds, so one
  // area can be compiled without waiting on another's work in progress.
  let only = std::env::var("TAILOR_EXPORT_ONLY").ok();
  for spec in tailor_gpuikit::library().components() {
    if let Some(only) = &only {
      if !only.split(',').any(|kind| kind == spec.kind) {
        continue;
      }
    }
    let mut variants: Vec<Vec<(&str, PropValue)>> = Vec::new();

    let mut loaded = Vec::new();
    for prop in spec.props {
      let value = match prop.ty {
        PropType::Bool => PropValue::Bool(true),
        PropType::Text | PropType::MultilineText => PropValue::Text("Sample".into()),
        PropType::Int => PropValue::Int(3),
        PropType::Float => PropValue::Float(30.0),
        PropType::Icon => PropValue::Icon("check".into()),
        PropType::Choice => match prop.choices.first() {
          Some(first) => PropValue::Choice((*first).into()),
          None => continue,
        },
        _ => continue,
      };
      loaded.push((prop.key, value));
    }
    variants.push(loaded);
    for prop in spec.props.iter().filter(|p| !p.choices.is_empty()) {
      for choice in prop.choices {
        variants.push(vec![(prop.key, PropValue::Choice((*choice).into()))]);
      }
    }
    variants.push(Vec::new());

    for props in variants {
      let doc = &mut project.docs[0];
      let mut node = doc.create(spec.kind);
      for (key, value) in props {
        node.set_prop(key, value);
      }
      for event in spec.events {
        node.events.insert(event.key.into(), "handle".into());
      }
      // A part goes inside the thing it is a part of, as it would in a design.
      let (parent, slot) = home(doc, root, spec.kind);
      let index = if parent == root { at } else { usize::MAX };
      doc.insert(parent, &slot, index, node);
      at += 1;
    }
  }
  let doc = &mut project.docs[0];
  doc.actions.push(tailor_model::ActionDef::new("handle"));
  // State goes through the compiler too: plain fields, in every type.
  for (name, ty) in [
    ("note", tailor_model::VarType::Text),
    ("done", tailor_model::VarType::Bool),
    ("count", tailor_model::VarType::Int),
    ("ratio", tailor_model::VarType::Float),
    ("tags", tailor_model::VarType::Items),
  ] {
    doc.state.push(tailor_model::StateVar::new(name, ty));
  }
  project
}

#[test]
fn export_for_compilation() {
  let Ok(dir) = std::env::var("TAILOR_EXPORT_DIR") else {
    return;
  };
  let mut project = everything();
  // A placed stateful component, so the nested shape is compiled too.
  let nested = nested();
  project.docs.push(nested.docs[1].clone());
  let root = project.docs[0].root;
  let id = project.docs[0].ids.next();
  project.docs[0].insert(
    root,
    DEFAULT_SLOT,
    usize::MAX,
    tailor_model::Node::new(id, "@EmailField"),
  );
  let dir = std::path::Path::new(&dir);
  let mut files = project_files(&project);
  let mut manifest = cargo_toml(&project);
  manifest.path = "Cargo.toml".into();
  files.push(manifest);
  for file in files {
    let path = dir.join(&file.path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, file.source).unwrap();
  }
}

#[test]
fn a_state_variable_is_a_plain_field_not_a_signal() {
  use tailor_model::{StateVar, VarType};
  let mut project = project("Stateful");
  let doc = &mut project.docs[0];
  doc.state.push(StateVar::new("email", VarType::Text));
  let mut count = StateVar::new("count", VarType::Int);
  count.initial = "3".into();
  doc.state.push(count);
  // gpui-kit has no signal type, so nothing here may name one.
  let file = document(&project, &project.docs[0]).source;
  assert!(!file.contains("Signal"), "{file}");
  assert!(file.contains("pub email: String,"), "{file}");
  assert!(file.contains("let email = \"\".to_string();"), "{file}");
  assert!(file.contains("pub count: i64,"), "{file}");
  assert!(file.contains("let count = 3;"), "{file}");
  // And it lints clean: state is supported, not refused.
  assert!(!tailor_model::lint::check(&project)
    .iter()
    .any(|p| p.severity == tailor_model::lint::Severity::Error));
}

#[test]
fn a_bound_prop_reads_the_field_directly() {
  use tailor_model::{StateVar, VarType};
  let mut project = project("Bound");
  let doc = &mut project.docs[0];
  doc.state.push(StateVar::new("title", VarType::Text));
  let root = doc.root;
  let mut node = tailor_gpuikit::library()
    .get("label")
    .unwrap()
    .build(doc.ids.next());
  node.set_prop("text", PropValue::Binding("title".into()));
  doc.insert(root, DEFAULT_SLOT, usize::MAX, node);
  let file = document(&project, &project.docs[0]).source;
  assert!(file.contains("self.title.clone()"), "{file}");
  assert!(!file.contains(".get(cx)"), "{file}");
}

#[test]
fn an_empty_icon_falls_back_to_one_gpui_kit_has() {
  let project = project("Icons");
  let mut project = project;
  let doc = &mut project.docs[0];
  let root = doc.root;
  let node = tailor_gpuikit::library()
    .get("icon")
    .unwrap()
    .build(doc.ids.next());
  doc.insert(root, DEFAULT_SLOT, usize::MAX, node);
  let file = document(&project, &project.docs[0]).source;
  assert!(file.contains("IconName::Info"), "{file}");
  assert!(!file.contains("IconName::Circle"), "{file}");
}

/// A screen with one text field on it, as the emitter sees it.
fn form() -> Project {
  let mut project = project("Form");
  let doc = &mut project.docs[0];
  let root = doc.root;
  let mut node = tailor_gpuikit::library()
    .get("input")
    .unwrap()
    .build(doc.ids.next());
  node.set_prop("placeholder", PropValue::Text("Email".into()));
  node.set_prop("cleanable", PropValue::Bool(true));
  doc.insert(root, DEFAULT_SLOT, usize::MAX, node);
  project
}

#[test]
fn a_text_field_is_a_state_field_and_an_element_over_it() {
  let project = form();
  let file = document(&project, &project.docs[0]).source;
  // The field holds the state, not the element.
  assert!(file.contains("pub input: Entity<InputState>,"), "{file}");
  // The state is built with the window and takes the state-side props...
  assert!(file.contains("InputState::new(window, cx)"), "{file}");
  assert!(file.contains(".placeholder(\"Email\")"), "{file}");
  // ...and the element, built each frame over a borrow of it, takes the rest.
  assert!(file.contains("Input::new(&self.input)"), "{file}");
  assert!(file.contains(".cleanable(true)"), "{file}");
  // Neither half wears the other's props.
  let (state, element) = file.split_once("Input::new(&self.input)").unwrap();
  assert!(!state.contains(".cleanable("), "{file}");
  assert!(!element.contains(".placeholder("), "{file}");
}

#[test]
fn a_screen_that_owns_window_state_takes_the_window() {
  let project = form();
  let file = document(&project, &project.docs[0]).source;
  assert!(
    file.contains("pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self"),
    "{file}"
  );
  let main = main_rs(&project).source;
  assert!(
    main.contains("|window, cx| cx.new(|cx| ui::MainScreen::new(window, cx)),"),
    "{main}"
  );
}

#[test]
fn a_screen_without_window_state_keeps_the_plain_shape() {
  let project = project("Plain");
  let file = document(&project, &project.docs[0]).source;
  assert!(!file.contains("pub fn new(window"), "{file}");
  assert!(main_rs(&project).source.contains("|_, cx| cx.new("));
}

#[test]
fn a_field_event_matches_only_the_event_it_was_bound_to() {
  let mut project = form();
  let doc = &mut project.docs[0];
  doc.actions.push(tailor_model::ActionDef::new("validate"));
  let id = doc.descendants(doc.root)[0];
  doc
    .node_mut(id)
    .unwrap()
    .events
    .insert("change".into(), "validate".into());
  let file = document(&project, &project.docs[0]).source;
  // One subscription, filtered: focus and blur must not run the handler too.
  assert!(file.contains("event: &InputEvent"), "{file}");
  assert!(
    file.contains("if matches!(event, InputEvent::Change) {"),
    "{file}"
  );
  assert!(file.contains("this.validate(cx);"), "{file}");
}

/// A screen that places a component which owns a text field.
fn nested() -> Project {
  use tailor_model::{Document, Node};
  let mut project = project("Nested");
  let mut field = Document::new("field", "EmailField", DocKind::Component);
  let root = field.root;
  let input = tailor_gpuikit::library()
    .get("input")
    .unwrap()
    .build(field.ids.next());
  field.insert(root, DEFAULT_SLOT, usize::MAX, input);
  project.docs.push(field);

  let screen = &mut project.docs[0];
  let root = screen.root;
  let id = screen.ids.next();
  screen.insert(root, DEFAULT_SLOT, usize::MAX, Node::new(id, "@EmailField"));
  project
}

#[test]
fn a_placed_component_that_holds_a_field_is_built_once_in_new() {
  let project = nested();
  let parent = document(&project, &project.docs[0]).source;
  // The component owns window state, so it is an entity, held in a field,
  // and the window is passed up through whoever places it.
  assert!(
    parent.contains("pub email_field: Entity<EmailField>,"),
    "{parent}"
  );
  assert!(
    parent.contains("cx.new(|cx| EmailField::new(window, cx))"),
    "{parent}"
  );
  assert!(parent.contains("self.email_field.clone()"), "{parent}");
  assert!(
    parent.contains("pub fn new(window: &mut Window, cx"),
    "{parent}"
  );
  assert!(!parent.contains("EmailField::new()"), "{parent}");
  let child = document(&project, &project.docs[1]).source;
  assert!(child.contains("impl Render for EmailField"), "{child}");
  // main hands the screen its window too.
  assert!(main_rs(&project).source.contains("::new(window, cx)"));
}

fn placed(project: &mut Project, kind: &str, into: Option<&str>) -> tailor_model::NodeId {
  let doc = &mut project.docs[0];
  let root = doc.root;
  let mut parent = root;
  if let Some(into) = into {
    let node = doc.create(into);
    parent = doc.insert(root, DEFAULT_SLOT, usize::MAX, node);
  }
  let node = doc.create(kind);
  doc.insert(parent, DEFAULT_SLOT, usize::MAX, node)
}

#[test]
fn a_part_outside_its_parent_is_a_lint_error() {
  let mut project = project("Parts");
  let cell = placed(&mut project, "tablecell", None);
  let problems = tailor_model::lint::check(&project);
  assert!(
    problems.iter().any(|p| p.node == Some(cell)
      && p.severity == tailor_model::lint::Severity::Error
      && p.message.contains("only works inside")),
    "{problems:?}"
  );

  // The same cell in a row is fine.
  let project = project_with_row();
  let row = tailor_model::lint::check(&project);
  assert!(
    !row.iter().any(|p| p.message.contains("only works inside")),
    "{row:?}"
  );
}

fn project_with_row() -> Project {
  let mut project = project("Row");
  let doc = &mut project.docs[0];
  let root = doc.root;
  let mut parent = root;
  for kind in ["table", "tablerow", "tablecell"] {
    let node = doc.create(kind);
    parent = doc.insert(parent, DEFAULT_SLOT, usize::MAX, node);
  }
  project
}

#[test]
fn an_icon_gpui_kit_lacks_warns_and_falls_back() {
  let mut project = project("Icons");
  let id = placed(&mut project, "button", None);
  project.docs[0]
    .node_mut(id)
    .unwrap()
    .set_prop("icon", PropValue::Icon("zzz-not-an-icon".into()));
  let problems = tailor_model::lint::check(&project);
  assert!(
    problems
      .iter()
      .any(|p| p.node == Some(id) && p.message.contains("no icon")),
    "{problems:?}"
  );
  let file = document(&project, &project.docs[0]).source;
  assert!(!file.contains("ZzzNotAnIcon"), "{file}");
  assert!(file.contains(".icon(IconName::Info)"), "{file}");

  // One it has is kept.
  project.docs[0]
    .node_mut(id)
    .unwrap()
    .set_prop("icon", PropValue::Icon("arrow-up".into()));
  let file = document(&project, &project.docs[0]).source;
  assert!(file.contains(".icon(IconName::ArrowUp)"), "{file}");
}
