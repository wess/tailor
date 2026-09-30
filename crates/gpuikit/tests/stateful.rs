//! The state-backed controls, generated.
//!
//! Shape tests read the generated Rust; [`export_stateful_for_compilation`] is
//! the ground truth. It writes a project holding only these kinds to
//! `$TAILOR_EXPORT_DIR_STATEFUL` so it can be built against the real gpui-kit —
//! minutes of compile, so it does nothing when the variable is unset.

use tailor_codegen::file::document;
use tailor_codegen::{app::cargo_toml, project_files};
use tailor_model::node::DEFAULT_SLOT;
use tailor_model::props::{PropType, PropValue};
use tailor_model::{DocKind, Project};

const KINDS: &[&str] = &[
  "numberinput",
  "otpinput",
  "select",
  "combobox",
  "slider",
  "colorpicker",
  "datepicker",
  "calendar",
  "timefield",
  "editor",
  "clipboard",
];

fn project(name: &str) -> Project {
  tailor_gpuikit::register();
  let mut project = Project::new(name);
  project.library = "gpuikit".into();
  project.docs[0].kind = DocKind::Screen;
  project
}

/// A screen with one node of `kind`, its props as given, returning the file.
fn one(kind: &str, props: &[(&str, PropValue)]) -> String {
  let mut project = project("One");
  let doc = &mut project.docs[0];
  let root = doc.root;
  let mut node = tailor_gpuikit::library()
    .get(kind)
    .unwrap_or_else(|| panic!("no kind {kind}"))
    .build(doc.ids.next());
  for (key, value) in props {
    node.set_prop(*key, value.clone());
  }
  doc.insert(root, DEFAULT_SLOT, usize::MAX, node);
  document(&project, &project.docs[0]).source
}

fn text(value: &str) -> PropValue {
  PropValue::Text(value.into())
}

#[test]
fn every_state_backed_kind_is_catalogued() {
  tailor_gpuikit::register();
  for kind in KINDS.iter().chain(&["form", "field"]) {
    assert!(tailor_gpuikit::library().get(kind).is_some(), "{kind}");
  }
}

#[test]
fn a_select_is_built_over_its_options_and_the_element_borrows_it() {
  let file = one(
    "select",
    &[
      (
        "items",
        PropValue::Items(vec!["Red".into(), "Green".into()]),
      ),
      ("selected", PropValue::Int(1)),
      ("searchable", PropValue::Bool(true)),
      ("placeholder", text("Colour")),
    ],
  );
  assert!(
    file.contains("pub select: Entity<SelectState<SearchableVec<String>>>,"),
    "{file}"
  );
  assert!(
    file.contains(
      "SelectState::new(SearchableVec::new(vec![\"Red\".to_string(), \"Green\".to_string()]), \
       Some(IndexPath::default().row(1)), window, cx)"
    ),
    "{file}"
  );
  // The state's prop on the state, the element's on the element.
  assert!(file.contains(".searchable(true)"), "{file}");
  assert!(file.contains("Select::new(&self.select)"), "{file}");
  assert!(file.contains(".placeholder(\"Colour\")"), "{file}");
  let (state, element) = file.split_once("Select::new(&self.select)").unwrap();
  assert!(!state.contains(".placeholder("), "{file}");
  assert!(!element.contains(".searchable("), "{file}");
}

#[test]
fn a_select_with_nothing_chosen_starts_with_none() {
  let file = one("select", &[]);
  assert!(file.contains(", None, window, cx)"), "{file}");
}

#[test]
fn a_slider_needs_no_window_and_leaves_no_unused_context() {
  let file = one(
    "slider",
    &[
      ("max", PropValue::Float(10.0)),
      ("value", PropValue::Float(3.0)),
    ],
  );
  assert!(file.contains("pub slider: Entity<SliderState>,"), "{file}");
  assert!(file.contains("cx.new(|_cx| {"), "{file}");
  assert!(file.contains("SliderState::new()"), "{file}");
  assert!(file.contains(".max(10.)"), "{file}");
  assert!(file.contains(".default_value(3.)"), "{file}");
  // Nothing here wants a window, so the screen is not handed one.
  assert!(!file.contains("pub fn new(window"), "{file}");
}

#[test]
fn a_screen_with_a_slider_and_a_select_takes_the_window_for_the_select() {
  let mut project = project("Both");
  let doc = &mut project.docs[0];
  let root = doc.root;
  for kind in ["slider", "select"] {
    let node = tailor_gpuikit::library()
      .get(kind)
      .unwrap()
      .build(doc.ids.next());
    doc.insert(root, DEFAULT_SLOT, usize::MAX, node);
  }
  let file = document(&project, &project.docs[0]).source;
  assert!(
    file.contains("pub fn new(window: &mut Window, cx"),
    "{file}"
  );
}

#[test]
fn a_number_input_carries_only_the_bounds_that_were_set() {
  let file = one("numberinput", &[("min", text("0")), ("step", text("0.5"))]);
  assert!(file.contains("InputState::new(window, cx)"), "{file}");
  assert!(file.contains(".min(0.0)"), "{file}");
  assert!(file.contains(".step(0.5)"), "{file}");
  assert!(!file.contains(".max("), "{file}");
  assert!(
    file.contains("NumberInput::new(&self.number_input)"),
    "{file}"
  );
}

#[test]
fn a_code_is_built_at_its_length() {
  let file = one("otpinput", &[("length", PropValue::Int(4))]);
  assert!(file.contains("OtpState::new(4, window, cx)"), "{file}");
  assert!(file.contains("OtpInput::new(&self."), "{file}");
}

#[test]
fn a_colour_starts_at_its_hex_and_ignores_junk() {
  let file = one("colorpicker", &[("value", text("#3B82F6"))]);
  assert!(
    file.contains(".default_value(gpui::rgb(0x3b82f6))"),
    "{file}"
  );
  let file = one("colorpicker", &[("value", text("not a colour"))]);
  assert!(!file.contains("default_value"), "{file}");
}

#[test]
fn a_date_picker_edits_a_time_only_when_asked() {
  let plain = one("datepicker", &[]);
  assert!(!plain.contains("time_precision"), "{plain}");
  let timed = one(
    "datepicker",
    &[("time", PropValue::Choice("second".into()))],
  );
  assert!(
    timed.contains(".time_precision(TimePrecision::Second)"),
    "{timed}"
  );
}

#[test]
fn a_time_field_names_its_enums() {
  let file = one(
    "timefield",
    &[("hour_cycle", PropValue::Choice("h12".into()))],
  );
  assert!(file.contains(".hour_cycle(HourCycle::H12)"), "{file}");
}

#[test]
fn an_event_on_a_generic_state_annotates_the_type_and_matches_without_it() {
  let mut project = project("Events");
  let doc = &mut project.docs[0];
  doc.actions.push(tailor_model::ActionDef::new("chosen"));
  let root = doc.root;
  let mut node = tailor_gpuikit::library()
    .get("select")
    .unwrap()
    .build(doc.ids.next());
  node.events.insert("confirm".into(), "chosen".into());
  doc.insert(root, DEFAULT_SLOT, usize::MAX, node);
  let file = document(&project, &project.docs[0]).source;
  // `SelectEvent` has a type parameter: the closure annotation needs `<_>`, the
  // pattern must not have it.
  assert!(file.contains("event: &SelectEvent<_>"), "{file}");
  assert!(
    file.contains("if matches!(event, SelectEvent::Confirm(_)) {"),
    "{file}"
  );
  assert!(file.contains("this.chosen(cx);"), "{file}");
}

#[test]
fn a_form_holds_fields_which_hold_controls() {
  let mut project = project("Forms");
  let doc = &mut project.docs[0];
  let root = doc.root;
  let library = tailor_gpuikit::library();
  let form = library.get("form").unwrap().build(doc.ids.next());
  let form = doc.insert(root, DEFAULT_SLOT, usize::MAX, form);
  let mut field = library.get("field").unwrap().build(doc.ids.next());
  field.set_prop("label", text("Email"));
  field.set_prop("required", PropValue::Bool(true));
  let field = doc.insert(form, DEFAULT_SLOT, usize::MAX, field);
  let input = library.get("input").unwrap().build(doc.ids.next());
  doc.insert(field, DEFAULT_SLOT, usize::MAX, input);
  let file = document(&project, &project.docs[0]).source;
  assert!(file.contains("Form::vertical()"), "{file}");
  assert!(file.contains("Field::new()"), "{file}");
  assert!(file.contains(".label(\"Email\")"), "{file}");
  assert!(file.contains(".required(true)"), "{file}");
  assert!(file.contains("Input::new(&self.input)"), "{file}");
}

/// Every state-backed kind, with a value on every prop and each choice tried.
fn everything() -> Project {
  let mut project = project("Stateful Export");
  let root = project.docs[0].root;
  let library = tailor_gpuikit::library();
  let mut at = 0;
  for kind in KINDS {
    let spec = library.get(kind).unwrap();
    let mut variants: Vec<Vec<(&str, PropValue)>> = Vec::new();
    let mut loaded = Vec::new();
    for prop in spec.props {
      let value = match prop.ty {
        PropType::Bool => PropValue::Bool(true),
        PropType::Text | PropType::MultilineText => match prop.key {
          "value" if spec.kind.starts_with("color") => text("#3b82f6"),
          "min" | "max" | "step" => text("2"),
          _ => text("Sample"),
        },
        PropType::Int => PropValue::Int(3),
        PropType::Float => PropValue::Float(30.0),
        PropType::Icon => PropValue::Icon("check".into()),
        PropType::Choice => match prop.choices.first() {
          Some(first) => PropValue::Choice((*first).into()),
          None => continue,
        },
        PropType::Items => PropValue::Items(vec!["One".into(), "Two".into()]),
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

  // A form of fields over controls, and the two containers by themselves.
  let doc = &mut project.docs[0];
  for layout in ["vertical", "horizontal"] {
    let mut form = doc.create("form");
    form.set_prop("layout", PropValue::Choice(layout.into()));
    form.set_prop("columns", PropValue::Int(2));
    let form = doc.insert(root, DEFAULT_SLOT, usize::MAX, form);
    for control in ["input", "select", "slider", "datepicker"] {
      let mut field = doc.create("field");
      field.set_prop("label", text("Row"));
      field.set_prop("description", text("Help"));
      field.set_prop("required", PropValue::Bool(true));
      let field = doc.insert(form, DEFAULT_SLOT, usize::MAX, field);
      let control = doc.create(control);
      doc.insert(field, DEFAULT_SLOT, usize::MAX, control);
    }
  }
  doc.actions.push(tailor_model::ActionDef::new("handle"));
  project
}

#[test]
fn export_stateful_for_compilation() {
  let Ok(dir) = std::env::var("TAILOR_EXPORT_DIR_STATEFUL") else {
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
