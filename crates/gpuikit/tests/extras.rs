//! The command palette, the input group, and the questionnaire's freeform
//! answers: the parts that are written by their parent rather than placed.
//!
//! What matters here is the shape the generator writes, because that is where a
//! design turns into behaviour: a palette reports a confirmed item to the host
//! by position, so the table from position back to action has to count the way
//! gpui-kit does.

use tailor_codegen::file::document;
use tailor_model::node::DEFAULT_SLOT;
use tailor_model::props::PropValue;
use tailor_model::{DocKind, NodeId, Project};

fn project(name: &str) -> Project {
  tailor_gpuikit::register();
  let mut project = Project::new(name);
  project.library = "gpuikit".into();
  project.docs[0].kind = DocKind::Screen;
  project
}

fn add(
  project: &mut Project,
  parent: NodeId,
  slot: &str,
  kind: &str,
  set: &[(&str, PropValue)],
  events: &[(&str, &str)],
) -> NodeId {
  let doc = &mut project.docs[0];
  let mut node = tailor_gpuikit::library()
    .get(kind)
    .unwrap()
    .build(doc.ids.next());
  for (key, value) in set {
    node.set_prop(*key, value.clone());
  }
  for (key, action) in events {
    node.events.insert((*key).into(), (*action).into());
  }
  doc.insert(parent, slot, usize::MAX, node)
}

fn text(value: &str) -> PropValue {
  PropValue::Text(value.into())
}

fn source(project: &Project) -> String {
  document(project, &project.docs[0]).source
}

fn root(project: &Project) -> NodeId {
  project.docs[0].root
}

fn actions(project: &mut Project, names: &[&str]) {
  for name in names {
    project.docs[0]
      .actions
      .push(tailor_model::ActionDef::new(*name));
  }
}

/// A palette with one loose item, a separator, and a group of two.
fn palette() -> Project {
  let mut project = project("Palette");
  palette_into(&mut project);
  project
}

fn palette_into(project: &mut Project) {
  actions(project, &["open", "copy", "dismiss"]);
  let r = root(project);
  let cmd = add(
    project,
    r,
    DEFAULT_SLOT,
    "command",
    &[("placeholder", text("Type a command"))],
    &[("cancel", "dismiss")],
  );
  add(
    project,
    cmd,
    DEFAULT_SLOT,
    "commanditem",
    &[
      ("label", text("Open file")),
      ("keywords", text("load, browse")),
    ],
    &[("select", "open")],
  );
  add(project, cmd, DEFAULT_SLOT, "commandseparator", &[], &[]);
  let group = add(
    project,
    cmd,
    DEFAULT_SLOT,
    "commandgroup",
    &[("label", text("Edit"))],
    &[],
  );
  add(
    project,
    group,
    DEFAULT_SLOT,
    "commanditem",
    &[("label", text("Copy"))],
    &[("select", "copy")],
  );
  add(
    project,
    group,
    DEFAULT_SLOT,
    "commanditem",
    &[("label", text("Paste"))],
    &[],
  );
}

#[test]
fn a_palette_is_a_state_and_an_element_with_its_entries_on_the_element() {
  let file = source(&palette());
  assert!(
    file.contains("pub command_palette: Entity<CommandState>,"),
    "{file}"
  );
  assert!(file.contains("CommandState::new(window, cx)"), "{file}");
  assert!(
    file.contains("Command::new(&self.command_palette)"),
    "{file}"
  );
  assert!(file.contains(".placeholder(\"Type a command\")"), "{file}");
  assert!(file.contains("CommandItem::new()"), "{file}");
  assert!(file.contains(".label(\"Open file\")"), "{file}");
  assert!(file.contains(".keywords([\"load\", \"browse\"])"), "{file}");
  assert!(file.contains(".separator()"), "{file}");
  assert!(file.contains("CommandGroup::new()"), "{file}");
  assert!(file.contains(".label(\"Edit\")"), "{file}");
  // The state is built in `new`, and none of the entries are its business.
  let build = file
    .split("pub fn new(")
    .nth(1)
    .and_then(|rest| rest.split("impl Render").next())
    .unwrap();
  assert!(!build.contains("CommandItem"), "{file}");
  let render = file.split("impl Render").nth(1).unwrap();
  assert!(render.contains("CommandItem"), "{file}");
}

#[test]
fn a_confirmed_item_runs_the_action_bound_at_its_position() {
  let file = source(&palette());
  // One loose item, so it is section 0 and the group is section 1.
  assert!(file.contains("(0, 0) => this.open(cx),"), "{file}");
  assert!(file.contains("(1, 0) => this.copy(cx),"), "{file}");
  // Paste has no action, so it has no arm.
  assert!(!file.contains("(1, 1)"), "{file}");
  assert!(file.contains("match (path.section, path.row)"), "{file}");
  assert!(file.contains("view.update(cx, |this, cx|"), "{file}");
  // The palette's callbacks are `'static`: the screen is a weak handle.
  assert!(file.contains("cx.entity().downgrade()"), "{file}");
  assert!(file.contains(".on_cancel({"), "{file}");
  assert!(file.contains("this.dismiss(cx);"), "{file}");
}

#[test]
fn groups_are_section_zero_when_nothing_is_loose() {
  let mut project = project("Grouped");
  actions(&mut project, &["go"]);
  let r = root(&project);
  let cmd = add(&mut project, r, DEFAULT_SLOT, "command", &[], &[]);
  let group = add(&mut project, cmd, DEFAULT_SLOT, "commandgroup", &[], &[]);
  add(
    &mut project,
    group,
    DEFAULT_SLOT,
    "commanditem",
    &[("label", text("Go"))],
    &[("select", "go")],
  );
  let file = source(&project);
  assert!(file.contains("(0, 0) => this.go(cx),"), "{file}");
}

#[test]
fn an_item_does_not_print_its_event_on_itself() {
  let file = source(&palette());
  // The item's action belongs to the palette; `CommandItem` has no handler.
  assert!(!file.contains(".on_select("), "{file}");
  assert!(!file.contains(".select("), "{file}");
  assert_eq!(file.matches(".on_confirm(").count(), 1, "{file}");
  // And nothing subscribes to the palette's state for it.
  assert!(!file.contains("cx.subscribe("), "{file}");
}

#[test]
fn an_item_outside_a_palette_is_a_lint_error() {
  let mut project = project("Loose");
  let r = root(&project);
  let item = add(&mut project, r, DEFAULT_SLOT, "commanditem", &[], &[]);
  let problems = tailor_model::lint::check(&project);
  assert!(
    problems.iter().any(|p| p.node == Some(item)
      && p.severity == tailor_model::lint::Severity::Error
      && p.message.contains("only works inside")),
    "{problems:?}"
  );
  // In a palette or a group it is fine.
  assert!(!tailor_model::lint::check(&palette())
    .iter()
    .any(|p| p.message.contains("only works inside")));
}

/// A URL field: `https://` before it, a copy button after it.
fn url_field() -> Project {
  let mut project = project("Url");
  url_field_into(&mut project);
  project
}

fn url_field_into(project: &mut Project) {
  actions(project, &["copy_url"]);
  let r = root(project);
  let group = add(
    project,
    r,
    DEFAULT_SLOT,
    "inputgroup",
    &[("invalid", PropValue::Bool(true))],
    &[],
  );
  add(
    project,
    group,
    "control",
    "input",
    &[("placeholder", text("example.com"))],
    &[],
  );
  let lead = add(project, group, "addons", "inputgroupaddon", &[], &[]);
  add(
    project,
    lead,
    DEFAULT_SLOT,
    "inputgrouptext",
    &[("text", text("https://"))],
    &[],
  );
  let tail = add(
    project,
    group,
    "addons",
    "inputgroupaddon",
    &[("align", PropValue::Choice("inline-end".into()))],
    &[],
  );
  add(
    project,
    tail,
    DEFAULT_SLOT,
    "inputgroupbutton",
    &[("label", text("Copy"))],
    &[("click", "copy_url")],
  );
}

#[test]
fn an_input_group_frames_a_field_and_its_addons() {
  let file = source(&url_field());
  assert!(file.contains("InputGroup::new("), "{file}");
  assert!(file.contains(".invalid(true)"), "{file}");
  // The field is the group's control, over the state the screen keeps.
  assert!(file.contains(".input(Input::new(&self.input))"), "{file}");
  assert!(file.contains("pub input: Entity<InputState>,"), "{file}");
  assert_eq!(file.matches(".addon(").count(), 2, "{file}");
  assert!(file.contains("InputGroupAddon::new("), "{file}");
  assert!(
    file.contains(".align(InputGroupAddonAlignment::InlineEnd)"),
    "{file}"
  );
  assert!(file.contains("InputGroupText::new()"), "{file}");
  assert!(file.contains(".child(\"https://\")"), "{file}");
  assert!(file.contains("InputGroupButton::new("), "{file}");
  assert!(file.contains(".label(\"Copy\")"), "{file}");
  assert!(file.contains(".on_click(cx.listener("), "{file}");
}

#[test]
fn input_group_parts_only_work_where_they_belong() {
  let mut project = project("Stray");
  let r = root(&project);
  let text = add(&mut project, r, DEFAULT_SLOT, "inputgrouptext", &[], &[]);
  let addon = add(&mut project, r, DEFAULT_SLOT, "inputgroupaddon", &[], &[]);
  let problems = tailor_model::lint::check(&project);
  for id in [text, addon] {
    assert!(
      problems
        .iter()
        .any(|p| p.node == Some(id) && p.message.contains("only works inside")),
      "{problems:?}"
    );
  }
  assert!(!tailor_model::lint::check(&url_field())
    .iter()
    .any(|p| p.message.contains("only works inside")));
}

#[test]
fn a_question_line_says_choices_descriptions_and_a_freeform_answer() {
  let q = tailor_gpuikit::extras::question(
    "Where will it run? | Cloud :: runs on our servers | Local | + Something else",
  );
  assert_eq!(q.text, "Where will it run?");
  assert_eq!(
    q.choices,
    [
      ("Cloud".to_string(), "runs on our servers".to_string()),
      ("Local".to_string(), String::new()),
    ]
  );
  assert_eq!(q.freeform.as_deref(), Some("Something else"));

  // A bare `+` is still a freeform answer, with a default label.
  assert_eq!(
    tailor_gpuikit::extras::question("Why? | +")
      .freeform
      .as_deref(),
    Some("Other")
  );
  // And a plain line is what it always was.
  let plain = tailor_gpuikit::extras::question("Sure? | Yes | No");
  assert_eq!(plain.choices.len(), 2);
  assert!(plain.freeform.is_none());
}

#[test]
fn a_questionnaire_builds_descriptions_and_a_freeform_input() {
  let mut project = project("Survey");
  let r = root(&project);
  add(
    &mut project,
    r,
    DEFAULT_SLOT,
    "questionnaire",
    &[(
      "questions",
      PropValue::Items(vec![
        "Where will it run? | Cloud :: runs on our servers | Local | + Something else".into(),
        "Sure? | Yes | No".into(),
      ]),
    )],
    &[],
  );
  let file = source(&project);
  assert!(
    file.contains(".with_description(\"runs on our servers\")"),
    "{file}"
  );
  assert!(
    file.contains("QuestionnaireInputDefinition::new("),
    "{file}"
  );
  assert!(
    file.contains("cx.new(|cx| InputState::new(window, cx).placeholder(\"Something else\"))"),
    "{file}"
  );
  // Only the question that asked for one gets the input.
  assert_eq!(
    file.matches("QuestionnaireInput::new(").count(),
    1,
    "{file}"
  );
  assert!(
    file.contains("QuestionnaireInput::new(&self.questionnaire, \"q1\")"),
    "{file}"
  );
}

/// Everything here on one screen, written to `$TAILOR_EXPORT_DIR` so it can be
/// built against the real gpui-kit. Does nothing when the variable is unset: a
/// build of gpui-kit is minutes, not a unit test.
#[test]
fn export_for_compilation() {
  let Ok(dir) = std::env::var("TAILOR_EXPORT_DIR") else {
    return;
  };
  let mut project = project("Kit Extras");
  palette_into(&mut project);
  url_field_into(&mut project);
  // Bodies that say they ran, so a confirmed item can be seen to reach its action.
  for action in &mut project.docs[0].actions {
    action.body = format!("println!(\"ACTION {}\");", action.name);
  }
  let r = root(&project);
  add(
    &mut project,
    r,
    DEFAULT_SLOT,
    "questionnaire",
    &[(
      "questions",
      PropValue::Items(vec![
        "Where will it run? | Cloud :: runs on our servers | Local | + Something else".into(),
        "Sure? | Yes | No".into(),
      ]),
    )],
    &[],
  );
  let dir = std::path::Path::new(&dir);
  let mut files = tailor_codegen::project_files(&project);
  let mut manifest = tailor_codegen::app::cargo_toml(&project);
  manifest.path = "Cargo.toml".into();
  files.push(manifest);
  for file in files {
    let path = dir.join(&file.path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, file.source).unwrap();
  }
}
