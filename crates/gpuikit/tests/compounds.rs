//! Tables, lists, trees and the multi-part containers, generated for gpui-kit.
//!
//! The assertions are about shape — where a delegate lands, what goes inside an
//! accordion item — because the compiler is the judge of the rest.
//! [`export_showcase`] is that judge's input: a screen using each of these the
//! way a person would, written to `$TAILOR_EXPORT_DIR` so it can be built
//! against the real gpui-kit and run.

use tailor_codegen::app::{cargo_toml, main_rs};
use tailor_codegen::file::document;
use tailor_codegen::project_files;
use tailor_model::node::DEFAULT_SLOT;
use tailor_model::props::PropValue;
use tailor_model::style::{Dimension, Direction};
use tailor_model::{DocKind, NodeId, Project};

fn project(name: &str) -> Project {
  tailor_gpuikit::register();
  let mut project = Project::new(name);
  project.library = "gpuikit".into();
  project.docs[0].kind = DocKind::Screen;
  project
}

/// Place a component under `parent`'s `slot` and set some props on it.
fn add(
  project: &mut Project,
  parent: NodeId,
  slot: &str,
  kind: &str,
  props: &[(&str, PropValue)],
) -> NodeId {
  let doc = &mut project.docs[0];
  let spec = tailor_gpuikit::library()
    .get(kind)
    .unwrap_or_else(|| panic!("{kind} is not catalogued"));
  let mut node = spec.build(doc.ids.next());
  for (key, value) in props {
    node.set_prop(*key, value.clone());
  }
  doc.insert(parent, slot, usize::MAX, node)
}

fn text(value: &str) -> PropValue {
  PropValue::Text(value.into())
}

fn items(values: &[&str]) -> PropValue {
  PropValue::Items(values.iter().map(|value| value.to_string()).collect())
}

fn source(project: &Project) -> String {
  document(project, &project.docs[0]).source
}

#[test]
fn a_table_writes_its_caption_header_and_rows() {
  let mut project = project("Tables");
  let root = project.docs[0].root;
  add(
    &mut project,
    root,
    DEFAULT_SLOT,
    "table",
    &[
      ("caption", text("Team")),
      ("head", items(&["Name", "Role"])),
      ("rows", items(&["Ada | Engineer", "Grace | Admiral"])),
    ],
  );
  let file = source(&project);
  assert!(
    file.contains("TableCaption::new().child(\"Team\")"),
    "{file}"
  );
  assert!(file.contains("TableHead::new().child(\"Role\")"), "{file}");
  assert!(
    file.contains("TableCell::new().child(\"Admiral\")"),
    "{file}"
  );
  // The header is one row of heads and the body one row per line.
  assert_eq!(file.matches("TableRow::new()").count(), 3, "{file}");
}

#[test]
fn a_cell_aligns_with_a_method_that_takes_no_argument() {
  let mut project = project("Cells");
  let root = project.docs[0].root;
  add(
    &mut project,
    root,
    DEFAULT_SLOT,
    "tablecell",
    &[
      ("text", text("42")),
      ("align", PropValue::Choice("right".into())),
    ],
  );
  let file = source(&project);
  assert!(file.contains(".text_right()"), "{file}");
  assert!(!file.contains("TextAlign"), "{file}");
}

#[test]
fn an_accordion_item_holds_what_was_dropped_into_its_section() {
  let mut project = project("Faq");
  let root = project.docs[0].root;
  let accordion = add(
    &mut project,
    root,
    DEFAULT_SLOT,
    "accordion",
    &[
      ("items", items(&["Shipping", "Returns"])),
      ("open", PropValue::Int(1)),
    ],
  );
  add(
    &mut project,
    accordion,
    "item:0",
    "label",
    &[("text", text("Two days."))],
  );
  add(
    &mut project,
    accordion,
    "item:1",
    "label",
    &[("text", text("Thirty days."))],
  );
  let file = source(&project);
  assert!(file.contains(".title(\"Shipping\")"), "{file}");
  assert!(file.contains("Two days."), "{file}");
  // Only the section named by Open starts expanded.
  assert_eq!(file.matches(".open(true)").count(), 1, "{file}");
  let returns = file.split(".title(\"Returns\")").nth(1).unwrap();
  assert!(returns.trim_start().starts_with(".open(true)"), "{file}");
  // The section's content is inside the closure, before it closes.
  let shipping = file.split(".title(\"Shipping\")").nth(1).unwrap();
  let closes = shipping.find("})").unwrap();
  assert!(shipping[..closes].contains("Two days."), "{file}");
}

#[test]
fn a_data_table_is_a_state_over_a_delegate_written_once() {
  let mut project = project("Grid");
  let root = project.docs[0].root;
  for _ in 0..2 {
    add(
      &mut project,
      root,
      DEFAULT_SLOT,
      "datatable",
      &[
        ("columns", items(&["Name", "Role"])),
        ("rows", items(&["Ada | Engineer"])),
      ],
    );
  }
  let file = source(&project);
  assert!(
    file.contains("pub data_table: Entity<TableState<TableRows>>,"),
    "{file}"
  );
  assert!(file.contains("&[\"Name\", \"Role\"],"), "{file}");
  assert!(file.contains("&[\"Ada\", \"Engineer\"],"), "{file}");
  assert!(file.contains("DataTable::new(&self.data_table)"), "{file}");
  // Two tables share the one delegate type.
  assert_eq!(file.matches("pub struct TableRows").count(), 1, "{file}");
  assert!(file.contains("impl TableDelegate for TableRows"), "{file}");
  // The screen takes the window the state is built with.
  assert!(file.contains("pub fn new(window: &mut Window,"), "{file}");
}

#[test]
fn a_table_option_lands_on_the_half_that_owns_it() {
  let mut project = project("Options");
  let root = project.docs[0].root;
  add(
    &mut project,
    root,
    DEFAULT_SLOT,
    "datatable",
    &[
      ("stripe", PropValue::Bool(true)),
      ("resizable", PropValue::Bool(false)),
    ],
  );
  let file = source(&project);
  // Striping is drawn by the element; resizing is the state's.
  let (state, element) = file.split_once("DataTable::new(").unwrap();
  assert!(state.contains(".col_resizable(false)"), "{file}");
  assert!(element.contains(".stripe(true)"), "{file}");
  assert!(!element.contains("col_resizable"), "{file}");
}

#[test]
fn a_list_filters_by_search_and_is_built_over_its_rows() {
  let mut project = project("Mail");
  let root = project.docs[0].root;
  add(
    &mut project,
    root,
    DEFAULT_SLOT,
    "list",
    &[
      ("items", items(&["Inbox", "Sent"])),
      ("searchable", PropValue::Bool(true)),
    ],
  );
  let file = source(&project);
  assert!(
    file.contains("ListState::new(ListRows::new(&[\"Inbox\", \"Sent\"]), window, cx)"),
    "{file}"
  );
  assert!(file.contains(".searchable(true)"), "{file}");
  assert!(file.contains("List::new(&self.list)"), "{file}");
  assert!(file.contains("fn perform_search"), "{file}");
}

#[test]
fn a_tree_nests_by_indentation_and_ids_are_paths() {
  let mut project = project("Files");
  let root = project.docs[0].root;
  add(
    &mut project,
    root,
    DEFAULT_SLOT,
    "tree",
    &[(
      "items",
      items(&["src", "  bin", "    main.rs", "  lib.rs", "Cargo.toml"]),
    )],
  );
  let file = source(&project);
  assert!(
    file.contains("TreeItem::new(\"src/bin/main.rs\", \"main.rs\")"),
    "{file}"
  );
  assert!(
    file.contains("TreeItem::new(\"src/lib.rs\", \"lib.rs\")"),
    "{file}"
  );
  // Two roots, one of them a folder: the folder opens, the file has no children.
  assert!(
    file.contains("TreeItem::new(\"Cargo.toml\", \"Cargo.toml\"),"),
    "{file}"
  );
  assert!(file.contains(".expanded(true)"), "{file}");
  assert!(file.contains("Tree::new(&self.tree,"), "{file}");
  // A tree needs no window of its own, so the screen does not name one it
  // never reads.
  assert!(file.contains("_window: &mut Window"), "{file}");
}

#[test]
fn a_carousel_has_one_item_per_slide_and_shows_the_title_when_empty() {
  let mut project = project("Gallery");
  let root = project.docs[0].root;
  let carousel = add(
    &mut project,
    root,
    DEFAULT_SLOT,
    "carousel",
    &[("slides", items(&["Sunrise", "Noon"]))],
  );
  add(
    &mut project,
    carousel,
    "slide:1",
    "label",
    &[("text", text("Bright."))],
  );
  let file = source(&project);
  assert!(file.contains("CarouselState::new(2)"), "{file}");
  assert!(file.contains("CarouselItem::new((\"node-"), "{file}");
  // The empty slide is labelled; the one with content shows the content.
  assert!(file.contains(".child(\"Sunrise\")"), "{file}");
  assert!(file.contains("Bright."), "{file}");
  assert!(!file.contains(".child(\"Noon\")"), "{file}");
  assert!(
    file.contains("CarouselPrevious::new(&self.carousel)"),
    "{file}"
  );
  assert!(file.contains("(0..2).map(|index|"), "{file}");
}

#[test]
fn a_carousel_can_drop_its_controls() {
  let mut project = project("Quiet");
  let root = project.docs[0].root;
  add(
    &mut project,
    root,
    DEFAULT_SLOT,
    "carousel",
    &[
      ("arrows", PropValue::Bool(false)),
      ("dots", PropValue::Bool(false)),
    ],
  );
  let file = source(&project);
  assert!(!file.contains("CarouselPrevious::new"), "{file}");
  assert!(!file.contains("CarouselPagination::new"), "{file}");
}

#[test]
fn a_collection_event_matches_only_its_own_variant() {
  let mut project = project("Events");
  let root = project.docs[0].root;
  let table = add(&mut project, root, DEFAULT_SLOT, "datatable", &[]);
  let doc = &mut project.docs[0];
  doc.actions.push(tailor_model::ActionDef::new("open_row"));
  doc
    .node_mut(table)
    .unwrap()
    .events
    .insert("open".into(), "open_row".into());
  let file = source(&project);
  assert!(file.contains("event: &TableEvent"), "{file}");
  assert!(
    file.contains("matches!(event, TableEvent::DoubleClickedRow(_))"),
    "{file}"
  );
}

#[test]
fn a_toolbar_takes_any_child_through_content() {
  let mut project = project("Bar");
  let root = project.docs[0].root;
  let toolbar = add(&mut project, root, DEFAULT_SLOT, "toolbar", &[]);
  add(
    &mut project,
    toolbar,
    DEFAULT_SLOT,
    "button",
    &[("label", text("Bold"))],
  );
  add(
    &mut project,
    toolbar,
    DEFAULT_SLOT,
    "label",
    &[("text", text("12pt"))],
  );
  let file = source(&project);
  // `Toolbar::child` only accepts sized controls, so a label would not compile.
  assert_eq!(file.matches(".content(").count(), 2, "{file}");
  assert!(!file.contains(".child(Button"), "{file}");
}

#[test]
fn a_status_bar_pins_left_and_right_and_centres_the_rest() {
  let mut project = project("Status");
  let root = project.docs[0].root;
  let bar = add(&mut project, root, DEFAULT_SLOT, "statusbar", &[]);
  add(
    &mut project,
    bar,
    "left",
    "label",
    &[("text", text("Ready"))],
  );
  add(
    &mut project,
    bar,
    "right",
    "label",
    &[("text", text("UTF-8"))],
  );
  add(
    &mut project,
    bar,
    DEFAULT_SLOT,
    "label",
    &[("text", text("main"))],
  );
  let file = source(&project);
  assert!(file.contains(".left("), "{file}");
  assert!(file.contains(".right("), "{file}");
  assert!(file.contains(".child("), "{file}");
}

#[test]
fn a_description_list_reads_label_value_and_span_per_line() {
  let mut project = project("Details");
  let root = project.docs[0].root;
  add(
    &mut project,
    root,
    DEFAULT_SLOT,
    "descriptionlist",
    &[
      (
        "items",
        items(&["Name | Ada", "---", "Bio | Wrote the first program | 3"]),
      ),
      ("layout", PropValue::Choice("vertical".into())),
    ],
  );
  let file = source(&project);
  assert!(file.contains(".item(\"Name\", \"Ada\", 1)"), "{file}");
  assert!(file.contains(".separator()"), "{file}");
  assert!(
    file.contains(".item(\"Bio\", \"Wrote the first program\", 3)"),
    "{file}"
  );
  assert!(file.contains(".layout(gpui::Axis::Vertical)"), "{file}");
}

#[test]
fn a_sidebar_is_groups_of_menus_with_active_items_and_icons() {
  let mut project = project("Nav");
  let root = project.docs[0].root;
  let sidebar = add(
    &mut project,
    root,
    DEFAULT_SLOT,
    "sidebar",
    &[(
      "menu",
      items(&[
        "Main",
        "  *Inbox | inbox",
        "  Tools",
        "    Search | search",
        "Other",
      ]),
    )],
  );
  add(
    &mut project,
    sidebar,
    "header",
    "label",
    &[("text", text("Acme"))],
  );
  let file = source(&project);
  assert!(
    file.contains("Sidebar::<SidebarGroup<SidebarMenu>>::new(\"node-"),
    "{file}"
  );
  assert!(file.contains("SidebarGroup::new(\"Main\")"), "{file}");
  assert!(
    file.contains("SidebarMenuItem::new(\"Inbox\").icon(IconName::Inbox).active(true)"),
    "{file}"
  );
  // A sub-item nests under its parent rather than sitting beside it.
  assert!(
    file.contains("SidebarMenuItem::new(\"Tools\").children([SidebarMenuItem::new(\"Search\").icon(IconName::Search)])"),
    "{file}"
  );
  // A group with no items is still a group.
  assert!(file.contains("SidebarGroup::new(\"Other\")"), "{file}");
  assert!(file.contains(".header("), "{file}");
}

#[test]
fn an_icon_outside_the_default_set_is_replaced_and_reported() {
  let mut project = project("Icons");
  let root = project.docs[0].root;
  add(
    &mut project,
    root,
    DEFAULT_SLOT,
    "sidebar",
    &[("menu", items(&["Main", "  Home | house"]))],
  );
  let generated = document(&project, &project.docs[0]);
  // `House` is Lucide's, not gpui-kit's default set: it would not compile.
  assert!(
    !generated.source.contains("IconName::House"),
    "{}",
    generated.source
  );
  assert!(
    generated.source.contains("IconName::Info"),
    "{}",
    generated.source
  );
  assert!(
    generated.notes.iter().any(|note| note.contains("`house`")),
    "{:?}",
    generated.notes
  );
}

#[test]
fn every_settings_page_is_listed_even_when_empty() {
  let mut project = project("Prefs");
  let root = project.docs[0].root;
  let settings = add(
    &mut project,
    root,
    DEFAULT_SLOT,
    "settings",
    &[("pages", items(&["General", "About"]))],
  );
  let label = add(
    &mut project,
    settings,
    "page:0",
    "label",
    &[("text", text("Hello"))],
  );
  let _ = label;
  let file = source(&project);
  assert!(file.contains("SettingPage::new(\"General\")"), "{file}");
  assert!(file.contains("SettingItem::render("), "{file}");
  assert!(file.contains("Hello"), "{file}");
  // gpui-kit drops a page with nothing on it, so About gets a blank row.
  let about = file.split("SettingPage::new(\"About\")").nth(1).unwrap();
  assert!(about.contains("gpui::Empty"), "{file}");
  // The item's closure takes the options first, then the window and context.
  assert!(file.contains("|_options, _window, _cx|"), "{file}");
}

#[test]
fn a_settings_item_that_uses_a_field_clones_it_in() {
  let mut project = project("Prefs");
  let root = project.docs[0].root;
  let settings = add(&mut project, root, DEFAULT_SLOT, "settings", &[]);
  add(
    &mut project,
    settings,
    "page:0",
    "input",
    &[("placeholder", text("Name"))],
  );
  let file = source(&project);
  // The closure outlives the render that made it, so it owns a handle.
  assert!(file.contains("let input = self.input.clone();"), "{file}");
  assert!(file.contains("move |_options, _window, _cx|"), "{file}");
  assert!(file.contains("Input::new(&input)"), "{file}");
}

#[test]
fn a_message_writes_its_bubble_and_lets_a_dropped_part_replace_it() {
  let mut project = project("Chat");
  let root = project.docs[0].root;
  let message = add(
    &mut project,
    root,
    DEFAULT_SLOT,
    "message",
    &[
      ("alignment", PropValue::Choice("end".into())),
      ("header", text("Ada")),
      ("text", text("Hi")),
      ("variant", PropValue::Choice("tinted".into())),
      ("footer", text("Sent")),
    ],
  );
  let file = source(&project);
  assert!(file.contains(".alignment(MessageAlignment::End)"), "{file}");
  assert!(
    file.contains(".header(MessageHeader::new().child(\"Ada\"))"),
    "{file}"
  );
  assert!(
    file.contains(
      ".content(MessageContent::new().bubble(Bubble::new().with_variant(BubbleVariant::Tinted).child(\"Hi\")))"
    ),
    "{file}"
  );
  assert!(
    file.contains(".footer(MessageFooter::new().child(\"Sent\"))"),
    "{file}"
  );

  // A part in the Content region comes after the text's bubble, so it wins.
  add(&mut project, message, "content", "messagecontent", &[]);
  let file = source(&project);
  let text_at = file.find("MessageContent::new().bubble").unwrap();
  let part_at = file.rfind("MessageContent::new()").unwrap();
  assert!(part_at > text_at, "{file}");
}

#[test]
fn a_marker_carries_text_and_an_icon_as_parts() {
  let mut project = project("Notice");
  let root = project.docs[0].root;
  add(
    &mut project,
    root,
    DEFAULT_SLOT,
    "marker",
    &[
      ("text", text("Chat resumed")),
      ("icon", PropValue::Icon("info".into())),
      ("variant", PropValue::Choice("separator".into())),
      ("alignment", PropValue::Choice("center".into())),
      ("loading", PropValue::Bool(true)),
    ],
  );
  let file = source(&project);
  assert!(
    file.contains(".content(MarkerContent::new().text(\"Chat resumed\"))"),
    "{file}"
  );
  assert!(
    file.contains(".icon(MarkerIcon::new().child(Icon::new(IconName::Info)))"),
    "{file}"
  );
  assert!(
    file.contains(".with_variant(MarkerVariant::Separator)"),
    "{file}"
  );
  assert!(
    file.contains(".alignment(MarkerAlignment::Center)"),
    "{file}"
  );
  assert!(file.contains(".loading(true)"), "{file}");
}

#[test]
fn an_attachment_writes_its_content_status_and_progress() {
  let mut project = project("Files");
  let root = project.docs[0].root;
  add(
    &mut project,
    root,
    DEFAULT_SLOT,
    "attachment",
    &[
      ("title", text("a.pdf")),
      ("description", text("1 MB")),
      ("status", PropValue::Choice("uploading".into())),
      ("progress", PropValue::Float(40.0)),
      ("icon", PropValue::Icon("file".into())),
    ],
  );
  let file = source(&project);
  assert!(file.contains(".id(\"node-"), "{file}");
  assert!(
    file.contains(".title(AttachmentTitle::new(\"a.pdf\"))"),
    "{file}"
  );
  assert!(
    file.contains(".description(AttachmentDescription::new(\"1 MB\"))"),
    "{file}"
  );
  assert!(
    file.contains(".status(AttachmentStatus::Uploading)"),
    "{file}"
  );
  assert!(file.contains(".progress(40.)"), "{file}");
  assert!(file.contains(".media(AttachmentMedia::new()"), "{file}");
}

#[test]
fn an_attachment_without_progress_writes_none() {
  let mut project = project("Files");
  let root = project.docs[0].root;
  add(&mut project, root, DEFAULT_SLOT, "attachment", &[]);
  let file = source(&project);
  assert!(!file.contains(".progress("), "{file}");
}

#[test]
fn a_questionnaire_is_definitions_in_its_state_and_parts_over_it() {
  let mut project = project("Survey");
  let root = project.docs[0].root;
  add(
    &mut project,
    root,
    DEFAULT_SLOT,
    "questionnaire",
    &[
      ("questions", items(&["Pick one | A | B", "Free? | Yes"])),
      ("required", PropValue::Bool(true)),
    ],
  );
  let file = source(&project);
  assert!(
    file.contains("pub questionnaire: Entity<QuestionnaireState>,"),
    "{file}"
  );
  assert!(
    file.contains("QuestionnaireItemDefinition::new(\"q1\", \"Pick one\")"),
    "{file}"
  );
  assert!(
    file.contains("QuestionnaireChoiceDefinition::new(\"choice-2\", \"B\")"),
    "{file}"
  );
  assert!(file.contains(".with_required(true)"), "{file}");
  assert!(file.contains(".expect("), "{file}");
  // The element side: one item per question, its choices, and the action row.
  assert!(
    file.contains("QuestionnaireChoice::new(&self.questionnaire, \"q2\", \"choice-1\")"),
    "{file}"
  );
  assert!(
    file.contains("QuestionnaireSubmit::new(&self.questionnaire)"),
    "{file}"
  );
  assert!(!file.contains("choice-2\"))\n                    .child(QuestionnaireChoice::new(&self.questionnaire, \"q2\""), "{file}");
}

#[test]
fn a_transcript_renders_the_element_dropped_into_each_row() {
  let mut project = project("Log");
  let root = project.docs[0].root;
  let scroller = add(
    &mut project,
    root,
    DEFAULT_SLOT,
    "messagescroller",
    &[("rows", items(&["one", "two", "three"]))],
  );
  add(
    &mut project,
    scroller,
    "row:0",
    "message",
    &[("text", text("First"))],
  );
  add(
    &mut project,
    scroller,
    "row:2",
    "label",
    &[("text", text("Third"))],
  );
  let file = source(&project);
  assert!(file.contains("MessageScrollerState::new(3, cx)"), "{file}");
  assert!(file.contains("MessageScroller::new(\"node-"), "{file}");
  assert!(file.contains("|ix, _window, _cx|"), "{file}");
  assert!(file.contains("0 => {"), "{file}");
  assert!(file.contains("First"), "{file}");
  // A row with nothing in it is a blank row, not a missing arm.
  assert!(file.contains("1 => {"), "{file}");
  assert!(file.contains("_ => div().into_any_element(),"), "{file}");
  assert!(file.contains("Third"), "{file}");
}

#[test]
fn an_empty_state_can_carry_a_framed_icon() {
  let mut project = project("Blank");
  let root = project.docs[0].root;
  add(
    &mut project,
    root,
    DEFAULT_SLOT,
    "empty",
    &[
      ("icon", PropValue::Icon("inbox".into())),
      ("title", text("Nothing here")),
    ],
  );
  let file = source(&project);
  assert!(
    file.contains(".media(EmptyMedia::new().with_variant(EmptyMediaVariant::Icon).child(Icon::new(IconName::Inbox)))"),
    "{file}"
  );
}

/// A screen using each of these the way a person would, so a real build can be
/// looked at. Does nothing unless `$TAILOR_EXPORT_DIR` is set.
#[test]
fn export_showcase() {
  let Ok(dir) = std::env::var("TAILOR_EXPORT_DIR_SHOWCASE") else {
    return;
  };
  let mut project = project("Showcase");
  project.docs[0].canvas.width = 1100.0;
  project.docs[0].canvas.height = 900.0;
  let root = project.docs[0].root;
  {
    let node = project.docs[0].node_mut(root).unwrap();
    node.style.padding = tailor_model::style::Edges::all(16.0);
    node.style.gap = Some(16.0);
  }

  // Accordion with real content in its sections.
  let accordion = add(
    &mut project,
    root,
    DEFAULT_SLOT,
    "accordion",
    &[
      ("items", items(&["Shipping", "Returns", "Warranty"])),
      ("open", PropValue::Int(0)),
    ],
  );
  add(
    &mut project,
    accordion,
    "item:0",
    "label",
    &[("text", text("Orders ship within two working days."))],
  );
  add(
    &mut project,
    accordion,
    "item:1",
    "button",
    &[("label", text("Start a return"))],
  );

  // A row holding the table and the list side by side.
  let row = add(&mut project, root, DEFAULT_SLOT, "hstack", &[]);
  {
    let node = project.docs[0].node_mut(row).unwrap();
    node.style.gap = Some(16.0);
    node.style.direction = Direction::Row;
  }
  let table = add(&mut project, row, DEFAULT_SLOT, "datatable", &[]);
  project.docs[0].node_mut(table).unwrap().style.width = Dimension::Px(480.0);
  let list = add(
    &mut project,
    row,
    DEFAULT_SLOT,
    "list",
    &[("searchable", PropValue::Bool(true))],
  );
  project.docs[0].node_mut(list).unwrap().style.width = Dimension::Px(200.0);
  let tree = add(&mut project, row, DEFAULT_SLOT, "tree", &[]);
  project.docs[0].node_mut(tree).unwrap().style.width = Dimension::Px(200.0);

  add(
    &mut project,
    root,
    DEFAULT_SLOT,
    "table",
    &[("caption", text("A static table"))],
  );
  add(&mut project, root, DEFAULT_SLOT, "descriptionlist", &[]);
  let toolbar = add(&mut project, root, DEFAULT_SLOT, "toolbar", &[]);
  add(
    &mut project,
    toolbar,
    DEFAULT_SLOT,
    "button",
    &[("label", text("Bold"))],
  );
  add(
    &mut project,
    toolbar,
    DEFAULT_SLOT,
    "button",
    &[("label", text("Italic"))],
  );

  let carousel = add(
    &mut project,
    root,
    DEFAULT_SLOT,
    "carousel",
    &[("slides", items(&["Sunrise", "Noon", "Dusk"]))],
  );
  add(
    &mut project,
    carousel,
    "slide:1",
    "label",
    &[("text", text("Bright and high."))],
  );

  let bar = add(&mut project, root, DEFAULT_SLOT, "statusbar", &[]);
  add(
    &mut project,
    bar,
    "left",
    "label",
    &[("text", text("Ready"))],
  );
  add(
    &mut project,
    bar,
    "right",
    "label",
    &[("text", text("UTF-8"))],
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
  // Keep `main_rs` in use so a signature change here is caught.
  let _ = main_rs(&project);
}

/// A settings page and a transcript, written the way a person would build them,
/// so a real build can be looked at. Does nothing unless
/// `$TAILOR_EXPORT_DIR_SHOWCASE2` is set.
#[test]
fn export_showcase_two() {
  let Ok(dir) = std::env::var("TAILOR_EXPORT_DIR_SHOWCASE2") else {
    return;
  };
  let mut project = project("Showcase Two");
  project.docs[0].canvas.width = 1200.0;
  project.docs[0].canvas.height = 900.0;
  let root = project.docs[0].root;
  {
    let node = project.docs[0].node_mut(root).unwrap();
    node.style.padding = tailor_model::style::Edges::all(16.0);
    node.style.gap = Some(16.0);
  }

  let row = add(&mut project, root, DEFAULT_SLOT, "hstack", &[]);
  {
    let node = project.docs[0].node_mut(row).unwrap();
    node.style.gap = Some(16.0);
    node.style.direction = Direction::Row;
    node.style.height = Dimension::Px(250.0);
  }
  let sidebar = add(&mut project, row, DEFAULT_SLOT, "sidebar", &[]);
  add(
    &mut project,
    sidebar,
    "header",
    "label",
    &[("text", text("Acme"))],
  );
  add(
    &mut project,
    sidebar,
    "footer",
    "label",
    &[("text", text("v1.0"))],
  );

  let settings = add(&mut project, row, DEFAULT_SLOT, "settings", &[]);
  project.docs[0].node_mut(settings).unwrap().style.width = Dimension::Px(560.0);
  add(
    &mut project,
    settings,
    "page:0",
    "label",
    &[("text", text("General preferences live here."))],
  );
  add(
    &mut project,
    settings,
    "page:0",
    "switch",
    &[("label", text("Send usage reports"))],
  );
  add(
    &mut project,
    settings,
    "page:1",
    "button",
    &[("label", text("Pick a theme"))],
  );

  let scroller = add(&mut project, root, DEFAULT_SLOT, "messagescroller", &[]);
  project.docs[0].node_mut(scroller).unwrap().style.height = Dimension::Px(150.0);
  let ada = add(
    &mut project,
    scroller,
    "row:0",
    "message",
    &[
      ("header", text("Ada · 9:41")),
      ("text", text("Can you send the report?")),
      ("footer", text("Delivered")),
    ],
  );
  let _ = ada;
  add(
    &mut project,
    scroller,
    "row:1",
    "message",
    &[
      ("alignment", PropValue::Choice("end".into())),
      ("text", text("On its way.")),
      ("variant", PropValue::Choice("tinted".into())),
    ],
  );

  add(
    &mut project,
    root,
    DEFAULT_SLOT,
    "marker",
    &[
      ("text", text("Today")),
      ("variant", PropValue::Choice("separator".into())),
    ],
  );
  let files = add(&mut project, root, DEFAULT_SLOT, "attachmentgroup", &[]);
  add(
    &mut project,
    files,
    DEFAULT_SLOT,
    "attachment",
    &[
      ("title", text("report.pdf")),
      ("description", text("2.4 MB")),
      ("icon", PropValue::Icon("file".into())),
    ],
  );
  add(
    &mut project,
    files,
    DEFAULT_SLOT,
    "attachment",
    &[
      ("title", text("photo.png")),
      ("description", text("Uploading")),
      ("status", PropValue::Choice("uploading".into())),
      ("progress", PropValue::Float(40.0)),
    ],
  );
  add(&mut project, root, DEFAULT_SLOT, "questionnaire", &[]);
  add(
    &mut project,
    root,
    DEFAULT_SLOT,
    "empty",
    &[
      ("icon", PropValue::Icon("inbox".into())),
      ("title", text("Nothing here yet")),
      ("description", text("Messages you receive show up here.")),
    ],
  );

  let dir = std::path::Path::new(&dir);
  let mut written = project_files(&project);
  let mut manifest = cargo_toml(&project);
  manifest.path = "Cargo.toml".into();
  written.push(manifest);
  for file in written {
    let path = dir.join(&file.path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, file.source).unwrap();
  }
}
