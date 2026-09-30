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
  assert!(main.contains("gpui::application().run("), "{main}");
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
fn everything() -> Project {
  let mut project = project("Kit Export");
  project.docs[0].kind = DocKind::Screen;
  let root = project.docs[0].root;
  let mut at = 0;

  for spec in tailor_gpuikit::library().components() {
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
      doc.insert(root, DEFAULT_SLOT, at, node);
      at += 1;
    }
  }
  project.docs[0]
    .actions
    .push(tailor_model::ActionDef::new("handle"));
  project
}

#[test]
fn export_for_compilation() {
  let Ok(dir) = std::env::var("TAILOR_EXPORT_DIR") else {
    return;
  };
  let project = everything();
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
fn state_variables_are_a_lint_error_not_a_failed_export() {
  use tailor_model::{StateVar, VarType};
  let mut project = project("Stateful");
  project.docs[0]
    .state
    .push(StateVar::new("email", VarType::Text));
  let problems = tailor_model::lint::check(&project);
  assert!(problems
    .iter()
    .any(|p| p.severity == tailor_model::lint::Severity::Error && p.message.contains("state")));
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
