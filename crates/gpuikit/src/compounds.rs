//! The generator's half of the tables, lists, trees and multi-part containers.
//!
//! These are the components whose Rust is not one chained constructor: a table
//! is a stack of section and row types, an accordion's items are closures over
//! an item builder, a data table is a delegate type and a state entity over it.
//! `codegen` forwards the hooks here and this module answers for its own kinds,
//! returning `None` (or nothing) for anything else.
//!
//! Lines are written the way the emitter's callers expect: a hook that adds
//! chained calls returns them at column zero, and the caller indents them under
//! the constructor.

use tailor_codegen::node::{child_call, Emitter};
use tailor_codegen::rust::{indent, string};
use tailor_codegen::style::Placement;
use tailor_model::Node;

/// The state's constructor, when it is not `State::new(window, cx)`.
pub fn state(em: &mut Emitter, node: &Node) -> Option<Vec<String>> {
  match node.kind.as_str() {
    "datatable" => Some(data_table_state(em, node)),
    "list" => Some(list_state(em, node)),
    "tree" => Some(tree_state(em, node)),
    "carousel" => Some(vec![format!(
      "CarouselState::new({})",
      em.items(node, "slides").unwrap_or_default().len()
    )]),
    "questionnaire" => Some(questionnaire_state(em, node)),
    "messagescroller" => Some(vec![format!(
      "MessageScrollerState::new({}, cx)",
      em.items(node, "rows").unwrap_or_default().len()
    )]),
    _ => None,
  }
}

/// The element drawn over a state, when it is not `Type::new(&state)`.
pub fn element_over(em: &mut Emitter, node: &Node, borrow: &str) -> Option<Vec<String>> {
  match node.kind.as_str() {
    "tree" => Some(tree_element(em, node, borrow)),
    "carousel" => Some(carousel_element(em, node, borrow)),
    "questionnaire" => Some(questionnaire_element(em, node, borrow)),
    "messagescroller" => Some(message_scroller(em, node, borrow)),
    _ => None,
  }
}

/// Top-level items a node needs beside its file's impls: the delegate a data
/// table or a list is built over. One per file, however many there are.
pub fn support(em: &mut Emitter, node: &Node) -> Vec<String> {
  match node.kind.as_str() {
    "datatable" if first_of(em, node) => TABLE_ROWS.lines().map(str::to_string).collect(),
    "list" if first_of(em, node) => LIST_ROWS.lines().map(str::to_string).collect(),
    _ => Vec::new(),
  }
}

/// A constructor that is not `Type::new(..)`.
pub fn special(em: &mut Emitter, node: &Node) -> Option<Vec<String>> {
  let _ = em;
  match node.kind.as_str() {
    // The sidebar is generic over the items it holds; groups of menus is what
    // the `menu` prop describes.
    "sidebar" => Some(vec![format!(
      "Sidebar::<SidebarGroup<SidebarMenu>>::new({})",
      string(&node.id.element_id())
    )]),
    _ => None,
  }
}

/// Regions a component takes some way other than `.child(..)`. Answering at
/// all replaces the generic walk over a dynamic slot, so every kind that
/// declares one must answer here, even with nothing.
pub fn slots(em: &mut Emitter, node: &Node, placement: Placement) -> Option<Vec<String>> {
  match node.kind.as_str() {
    "accordion" => Some(accordion(em, node, placement)),
    // The slides are written by `carousel_element`, over the state it owns.
    "carousel" => Some(Vec::new()),
    "settings" => Some(settings(em, node)),
    // The rows are written by `message_scroller`, inside the row renderer.
    "messagescroller" => Some(Vec::new()),
    _ => None,
  }
}

/// Calls for the props the catalog leaves to the generator.
pub fn custom_props(em: &mut Emitter, node: &Node) -> Vec<String> {
  match node.kind.as_str() {
    "table" => table(em, node),
    "tablehead" | "tablecell" => cell(em, node),
    "tablecaption" => text_child(em, node, "text"),
    "descriptionlist" => description_list(em, node),
    "sidebar" => sidebar_menu(em, node),
    "marker" => marker(em, node),
    "message" => message(em, node),
    "messageheader" | "messagefooter" | "bubble" => text_child(em, node, "text"),
    "attachment" => attachment(em, node),
    _ => Vec::new(),
  }
}

/// `.child("text")` for a text prop that is a child rather than a setter.
fn text_child(em: &Emitter, node: &Node, key: &str) -> Vec<String> {
  match em.prop_value(node, key).as_str() {
    Some(text) if !text.is_empty() => vec![format!(".child({})", string(text))],
    _ => Vec::new(),
  }
}

/// A row's cells, split on `|` and trimmed.
fn cells(row: &str) -> Vec<String> {
  row.split('|').map(|cell| cell.trim().to_string()).collect()
}

/// The caption, header and body a `Table`'s props describe.
fn table(em: &mut Emitter, node: &Node) -> Vec<String> {
  let mut out = Vec::new();

  if let Some(caption) = em.prop_value(node, "caption").as_str() {
    if !caption.is_empty() {
      out.push(format!(
        ".child(TableCaption::new().child({}))",
        string(caption)
      ));
    }
  }

  let head = em.items(node, "head").unwrap_or_default();
  if !head.is_empty() {
    out.push(".child(".into());
    out.push("    TableHeader::new().child(".into());
    out.push("        TableRow::new()".into());
    for label in &head {
      out.push(format!(
        "            .child(TableHead::new().child({}))",
        string(label)
      ));
    }
    out.push("    )".into());
    out.push(")".into());
  }

  let rows = em.items(node, "rows").unwrap_or_default();
  if !rows.is_empty() {
    out.push(".child(".into());
    out.push("    TableBody::new()".into());
    for row in &rows {
      out.push("        .child(".into());
      out.push("            TableRow::new()".into());
      for cell in cells(row) {
        out.push(format!(
          "                .child(TableCell::new().child({}))",
          string(&cell)
        ));
      }
      out.push("        )".into());
    }
    out.push(")".into());
  }
  out
}

/// A header or body cell: its text, its span (a plain setter) and its
/// alignment (two methods, neither of which takes an argument).
fn cell(em: &mut Emitter, node: &Node) -> Vec<String> {
  let mut out = text_child(em, node, "text");
  match em.prop_value(node, "align").as_str() {
    Some("center") => out.push(".text_center()".into()),
    Some("right") => out.push(".text_right()".into()),
    _ => {}
  }
  out
}

/// `Label | Value | span` per line; a line of dashes is a separator.
fn description_list(em: &mut Emitter, node: &Node) -> Vec<String> {
  let mut out = Vec::new();
  if em.prop_value(node, "layout").as_str() == Some("vertical") {
    out.push(".layout(gpui::Axis::Vertical)".into());
  }
  for line in em.items(node, "items").unwrap_or_default() {
    if line.trim().chars().all(|c| c == '-') && !line.trim().is_empty() {
      out.push(".separator()".into());
      continue;
    }
    let parts = cells(&line);
    let label = parts.first().cloned().unwrap_or_default();
    let value = parts.get(1).cloned().unwrap_or_default();
    let span = parts
      .get(2)
      .and_then(|span| span.parse::<usize>().ok())
      .unwrap_or(1);
    out.push(format!(
      ".item({}, {}, {span})",
      string(&label),
      string(&value)
    ));
  }
  out
}

/// One `.item(|item| ..)` per section title, with whatever was dropped into
/// the section as the item's children. The closure is `FnOnce` and called on
/// the spot, so nothing has to be cloned into it.
fn accordion(em: &mut Emitter, node: &Node, placement: Placement) -> Vec<String> {
  let open = em.prop_value(node, "open").as_i64().unwrap_or(-1);
  let mut out = Vec::new();
  for (index, title) in em
    .items(node, "items")
    .unwrap_or_default()
    .iter()
    .enumerate()
  {
    out.push(".item(|item| {".into());
    out.push("    item".into());
    out.push(format!("        .title({})", string(title)));
    if open == index as i64 {
      out.push("        .open(true)".into());
    }
    for child in node.slot(&format!("item:{index}")).to_vec() {
      let inner = em.emit(child, placement);
      out.extend(indent(&indent(&child_call(inner))));
    }
    out.push("})".into());
  }
  out
}

/// Whether this is the first node of its kind in the document, which is the one
/// that writes the type its siblings share.
fn first_of(em: &Emitter, node: &Node) -> bool {
  em.fields
    .keys()
    .find(|id| em.doc.node(**id).is_some_and(|n| n.kind == node.kind))
    == Some(&node.id)
}

/// `&["a", "b"]`.
fn str_slice(values: &[String]) -> String {
  let parts: Vec<String> = values.iter().map(|value| string(value)).collect();
  format!("&[{}]", parts.join(", "))
}

fn data_table_state(em: &mut Emitter, node: &Node) -> Vec<String> {
  let columns = em.items(node, "columns").unwrap_or_default();
  let rows = em.items(node, "rows").unwrap_or_default();
  let mut out = vec![
    "TableState::new(".to_string(),
    "    TableRows::new(".to_string(),
    format!("        {},", str_slice(&columns)),
  ];
  if rows.is_empty() {
    out.push("        &[],".into());
  } else {
    out.push("        &[".into());
    for row in &rows {
      out.push(format!("            {},", str_slice(&cells(row))));
    }
    out.push("        ],".into());
  }
  out.push("    ),".into());
  out.push("    window,".into());
  out.push("    cx,".into());
  out.push(")".into());
  out
}

fn list_state(em: &mut Emitter, node: &Node) -> Vec<String> {
  let items = em.items(node, "items").unwrap_or_default();
  vec![format!(
    "ListState::new(ListRows::new({}), window, cx)",
    str_slice(&items)
  )]
}

/// A node of the tree an `Items` prop describes by indentation.
struct Branch {
  label: String,
  children: Vec<Branch>,
}

/// Two spaces of indent per level; a line indented deeper than the one above
/// it by more than a level is treated as one level deeper.
fn branches(lines: &[String]) -> Vec<Branch> {
  let mut roots: Vec<Branch> = Vec::new();
  // Indices from the root to the branch last added, one per depth.
  let mut trail: Vec<usize> = Vec::new();
  for line in lines {
    let label = line.trim().to_string();
    if label.is_empty() {
      continue;
    }
    let indent = line.len() - line.trim_start().len();
    let depth = (indent / 2).min(trail.len());
    trail.truncate(depth);
    let mut level = &mut roots;
    for &at in &trail {
      level = &mut level[at].children;
    }
    level.push(Branch {
      label,
      children: Vec::new(),
    });
    trail.push(level.len() - 1);
  }
  roots
}

/// `TreeItem::new(id, label)` and its children, with the path as the id so two
/// files of the same name in different folders stay distinct.
fn tree_item(branch: &Branch, path: &str, expanded: bool) -> Vec<String> {
  let id = if path.is_empty() {
    branch.label.clone()
  } else {
    format!("{path}/{}", branch.label)
  };
  let mut out = vec![format!(
    "TreeItem::new({}, {})",
    string(&id),
    string(&branch.label)
  )];
  if branch.children.is_empty() {
    return out;
  }
  if expanded {
    out.push("    .expanded(true)".into());
  }
  for child in &branch.children {
    let inner = tree_item(child, &id, expanded);
    if inner.len() == 1 {
      out.push(format!("    .child({})", inner[0]));
    } else {
      out.push("    .child(".into());
      out.extend(indent(&indent(&inner)));
      out.push("    )".into());
    }
  }
  out
}

fn tree_state(em: &mut Emitter, node: &Node) -> Vec<String> {
  let expanded = em.prop_value(node, "expanded").as_bool().unwrap_or(true);
  let lines = em.items(node, "items").unwrap_or_default();
  let mut out = vec!["TreeState::new(cx)".to_string()];
  out.push("    .items(vec![".into());
  for branch in branches(&lines) {
    let mut item = tree_item(&branch, "", expanded);
    // The last line of each item takes the trailing comma of a vec entry.
    if let Some(last) = item.last_mut() {
      last.push(',');
    }
    out.extend(indent(&indent(&indent(&item))));
  }
  out.push("    ])".into());
  out
}

fn tree_element(em: &mut Emitter, node: &Node, borrow: &str) -> Vec<String> {
  let icons = em.prop_value(node, "icons").as_bool().unwrap_or(true);
  let mut out = vec![
    format!("Tree::new({borrow}, |ix, entry, _selected, _window, _cx| {{"),
    "    ListItem::new(ix)".into(),
    "        .pl(px(16.0 * entry.depth() as f32))".into(),
  ];
  if icons {
    out.push("        .child(".into());
    out.push("            h_flex()".into());
    out.push("                .gap_2()".into());
    out.push("                .child(Icon::new(if entry.is_folder() { IconName::Folder } else { IconName::File }))".into());
    out.push("                .child(entry.item().label.clone())".into());
    out.push("        )".into());
  } else {
    out.push("        .child(entry.item().label.clone())".into());
  }
  out.push("})".into());
  out
}

/// The carousel: its content and one item per slide, the arrows and the dots
/// the props ask for, all over the one state entity.
fn carousel_element(em: &mut Emitter, node: &Node, borrow: &str) -> Vec<String> {
  let id = node.id.element_id();
  let slides = em.items(node, "slides").unwrap_or_default();
  let arrows = em.prop_value(node, "arrows").as_bool().unwrap_or(true);
  let dots = em.prop_value(node, "dots").as_bool().unwrap_or(true);
  let placement = Placement { absolute: false };

  let mut out = vec![format!("Carousel::new({}, {borrow})", string(&id))];
  out.push("    .child(".into());
  out.push(format!("        CarouselContent::new({borrow})"));
  for (index, label) in slides.iter().enumerate() {
    out.push("            .child(".into());
    out.push(format!(
      "                CarouselItem::new(({}, {index}usize), {index}, {borrow})",
      string(&format!("{id}-slide"))
    ));
    let children = node.slot(&format!("slide:{index}")).to_vec();
    if children.is_empty() {
      // A slide with nothing dropped in still shows what it is called.
      out.push(format!("                    .child({})", string(label)));
    }
    for child in children {
      let inner = em.emit(child, placement);
      out.extend(indent(&indent(&indent(&indent(&indent(&child_call(
        inner,
      )))))));
    }
    out.push("            )".into());
  }
  out.push("    )".into());
  if arrows {
    out.push(format!("    .child(CarouselPrevious::new({borrow}))"));
    out.push(format!("    .child(CarouselNext::new({borrow}))"));
  }
  if dots {
    out.push("    .child(".into());
    out.push(format!(
      "        CarouselPagination::new().children((0..{}).map(|index| {{",
      slides.len()
    ));
    out.push(format!(
      "            CarouselPaginationItem::new(({}, index), index, {borrow})",
      string(&format!("{id}-dot"))
    ));
    out.push("                .child((index + 1).to_string())".into());
    out.push("        }))".into());
    out.push("    )".into());
  }
  out
}

/// The delegate a data table is built over: columns and rows of text.
const TABLE_ROWS: &str = r#"/// The text a data table shows, as columns and rows of strings. Replace it with
/// your own `TableDelegate` to feed the table live data.
pub struct TableRows {
    columns: Vec<Column>,
    rows: Vec<Vec<gpui::SharedString>>,
}

impl TableRows {
    pub fn new(columns: &[&str], rows: &[&[&str]]) -> Self {
        Self {
            columns: columns
                .iter()
                .map(|name| Column::new(name.to_string(), name.to_string()))
                .collect(),
            rows: rows
                .iter()
                .map(|row| row.iter().map(|cell| gpui::SharedString::from(cell.to_string())).collect())
                .collect(),
        }
    }
}

impl TableDelegate for TableRows {
    fn columns_count(&self, _cx: &gpui::App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _cx: &gpui::App) -> usize {
        self.rows.len()
    }

    fn column(&self, col_ix: usize, _cx: &gpui::App) -> Column {
        self.columns[col_ix].clone()
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<TableState<Self>>,
    ) -> impl IntoElement {
        self.rows
            .get(row_ix)
            .and_then(|row| row.get(col_ix))
            .cloned()
            .unwrap_or_default()
    }
}"#;

/// The delegate a list is built over: rows of text, filtered by the search.
const LIST_ROWS: &str = r#"/// The rows a list shows, narrowed by what was searched for. Replace it with your
/// own `ListDelegate` to feed the list live data.
pub struct ListRows {
    items: Vec<gpui::SharedString>,
    shown: Vec<usize>,
}

impl ListRows {
    pub fn new(items: &[&str]) -> Self {
        let items: Vec<gpui::SharedString> = items
            .iter()
            .map(|item| gpui::SharedString::from(item.to_string()))
            .collect();
        Self {
            shown: (0..items.len()).collect(),
            items,
        }
    }
}

impl ListDelegate for ListRows {
    type Item = ListItem;

    fn perform_search(
        &mut self,
        query: &str,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<ListState<Self>>,
    ) -> gpui::Task<()> {
        let query = query.to_lowercase();
        self.shown = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.to_lowercase().contains(&query))
            .map(|(ix, _)| ix)
            .collect();
        gpui::Task::ready(())
    }

    fn items_count(&self, _section: usize, _cx: &gpui::App) -> usize {
        self.shown.len()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let item = self.items.get(*self.shown.get(ix.row)?)?.clone();
        Some(ListItem::new(("list-item", ix.row)).child(item))
    }

    fn set_selected_index(
        &mut self,
        _ix: Option<IndexPath>,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<ListState<Self>>,
    ) {
        cx.notify();
    }
}"#;

/// A label with its optional icon and active marker: `*Home | house`.
struct MenuLabel {
  label: String,
  icon: String,
  active: bool,
}

fn menu_label(text: &str) -> MenuLabel {
  let (active, rest) = match text.strip_prefix('*') {
    Some(rest) => (true, rest),
    None => (false, text),
  };
  let (label, icon) = match rest.split_once('|') {
    Some((label, icon)) => (label.trim(), icon.trim()),
    None => (rest.trim(), ""),
  };
  MenuLabel {
    label: label.to_string(),
    icon: icon.to_string(),
    active,
  }
}

/// `SidebarMenuItem::new("Home").icon(..).active(true)`, with sub-items inline.
fn menu_item(em: &mut Emitter, branch: &Branch) -> String {
  let label = menu_label(&branch.label);
  let mut out = format!("SidebarMenuItem::new({})", string(&label.label));
  if !label.icon.is_empty() {
    out.push_str(&format!(".icon({})", icon(em, &label.icon)));
  }
  if label.active {
    out.push_str(".active(true)");
  }
  if !branch.children.is_empty() {
    let subs: Vec<String> = branch
      .children
      .iter()
      .map(|child| menu_item(em, child))
      .collect();
    out.push_str(&format!(".children([{}])", subs.join(", ")));
  }
  out
}

/// One `SidebarGroup` per unindented line, holding one menu of its items.
fn sidebar_menu(em: &mut Emitter, node: &Node) -> Vec<String> {
  let mut out = Vec::new();
  for group in branches(&em.items(node, "menu").unwrap_or_default()) {
    let label = menu_label(&group.label);
    out.push(".child(".into());
    out.push(format!("    SidebarGroup::new({})", string(&label.label)));
    if !group.children.is_empty() {
      out.push("        .child(".into());
      out.push("            SidebarMenu::new()".into());
      for item in &group.children {
        let line = menu_item(em, item);
        out.push(format!("                .child({line})"));
      }
      out.push("        )".into());
    }
    out.push(")".into());
  }
  out
}

/// A marker's text, icon and the two setters that take a part.
fn marker(em: &mut Emitter, node: &Node) -> Vec<String> {
  let mut out = Vec::new();
  if let Some(name) = em
    .prop_value(node, "icon")
    .as_str()
    .filter(|n| !n.is_empty())
  {
    let icon = icon(em, name);
    out.push(format!(".icon(MarkerIcon::new().child(Icon::new({icon})))"));
  }
  if let Some(text) = em
    .prop_value(node, "text")
    .as_str()
    .filter(|t| !t.is_empty())
  {
    out.push(format!(
      ".content(MarkerContent::new().text({}))",
      string(text)
    ));
  }
  out
}

/// A message's header, bubble and footer written from its text props. A part
/// dropped into the matching region comes after these and replaces them.
fn message(em: &mut Emitter, node: &Node) -> Vec<String> {
  let mut out = Vec::new();
  let text = |em: &Emitter, key: &str| {
    em.prop_value(node, key)
      .as_str()
      .filter(|t| !t.is_empty())
      .map(str::to_string)
  };
  if let Some(header) = text(em, "header") {
    out.push(format!(
      ".header(MessageHeader::new().child({}))",
      string(&header)
    ));
  }
  if let Some(body) = text(em, "text") {
    let variant = match em.prop_value(node, "variant").as_str() {
      Some("filled") | None => String::new(),
      Some(other) => format!(
        ".with_variant(BubbleVariant::{})",
        tailor_model::pascal_case(other)
      ),
    };
    out.push(format!(
      ".content(MessageContent::new().bubble(Bubble::new(){variant}.child({})))",
      string(&body)
    ));
  }
  if let Some(footer) = text(em, "footer") {
    out.push(format!(
      ".footer(MessageFooter::new().child({}))",
      string(&footer)
    ));
  }
  out
}

/// An attachment's identity, media, and the title and description that make its
/// content.
fn attachment(em: &mut Emitter, node: &Node) -> Vec<String> {
  let mut out = vec![format!(".id({})", string(&node.id.element_id()))];
  if let Some(name) = em
    .prop_value(node, "icon")
    .as_str()
    .filter(|n| !n.is_empty())
  {
    let icon = icon(em, name);
    out.push(format!(
      ".media(AttachmentMedia::new().child(Icon::new({icon})))"
    ));
  }
  let title = em
    .prop_value(node, "title")
    .as_str()
    .unwrap_or("")
    .to_string();
  let description = em
    .prop_value(node, "description")
    .as_str()
    .unwrap_or("")
    .to_string();
  if !title.is_empty() || !description.is_empty() {
    out.push(".content(".into());
    out.push("    AttachmentContent::new()".into());
    if !title.is_empty() {
      out.push(format!(
        "        .title(AttachmentTitle::new({}))",
        string(&title)
      ));
    }
    if !description.is_empty() {
      out.push(format!(
        "        .description(AttachmentDescription::new({}))",
        string(&description)
      ));
    }
    out.push(")".into());
  }
  if let Some(progress) = em
    .prop_value(node, "progress")
    .as_f64()
    .filter(|p| *p >= 0.0)
  {
    out.push(format!(
      ".progress({})",
      tailor_codegen::rust::float(progress as f32)
    ));
  }
  out
}

/// One `.page(..)` per title, with a group holding one item per element dropped
/// into the page. Each item's content is a `'static` closure, so what it uses is
/// cloned in ahead of it, the way any region closure is.
fn settings(em: &mut Emitter, node: &Node) -> Vec<String> {
  let mut out = Vec::new();
  for (index, title) in em
    .items(node, "pages")
    .unwrap_or_default()
    .iter()
    .enumerate()
  {
    out.push(".page(".into());
    out.push(format!("    SettingPage::new({})", string(title)));
    let children = node.slot(&format!("page:{index}")).to_vec();
    // gpui-kit leaves a page with nothing to show out of the page list, so an
    // empty one gets a blank row: it is a page you have not filled in yet, not
    // a page you removed.
    out.push("        .group(".into());
    out.push("            SettingGroup::new()".into());
    if children.is_empty() {
      out.push(
        "                .item(SettingItem::render(|_options, _window, _cx| gpui::Empty))".into(),
      );
    }
    for child in children {
      let mut item = em.closure_region_with(
        ".item(SettingItem::render(".into(),
        "|_options, _window, _cx|",
        Some(child),
      );
      // `render(` is a second open paren the closure's own closer has to match.
      if let Some(last) = item.last_mut() {
        *last = "}))".into();
      }
      out.extend(indent(&indent(&indent(&indent(&item)))));
    }
    out.push("        )".into());
    out.push(")".into());
  }
  out
}

/// The question definitions the props describe, built into the state.
fn questionnaire_state(em: &mut Emitter, node: &Node) -> Vec<String> {
  let required = em.prop_value(node, "required").as_bool().unwrap_or(false);
  let multiple = em.prop_value(node, "multiple").as_bool().unwrap_or(false);
  let mut out = vec![
    "QuestionnaireState::new(".to_string(),
    "    vec![".to_string(),
  ];
  for (number, line) in em
    .items(node, "questions")
    .unwrap_or_default()
    .iter()
    .enumerate()
  {
    let parsed = crate::extras::question(line);
    out.push(format!(
      "        QuestionnaireItemDefinition::new({}, {})",
      string(&format!("q{}", number + 1)),
      string(&parsed.text)
    ));
    if required {
      out.push("            .with_required(true)".into());
    }
    if multiple {
      out.push("            .with_multiple(true)".into());
    }
    if !parsed.choices.is_empty() {
      out.push("            .with_choices([".into());
      for (at, (choice, description)) in parsed.choices.iter().enumerate() {
        let described = if description.is_empty() {
          String::new()
        } else {
          format!(".with_description({})", string(description))
        };
        out.push(format!(
          "                QuestionnaireChoiceDefinition::new({}, {}){described},",
          string(&format!("choice-{}", at + 1)),
          string(choice)
        ));
      }
      out.push("            ])".into());
    }
    // A freeform answer is its own text field, built here so the state owns it.
    if let Some(label) = &parsed.freeform {
      out.push("            .with_input(QuestionnaireInputDefinition::new(".into());
      out.push(format!(
        "                cx.new(|cx| InputState::new(window, cx).placeholder({})),",
        string(label)
      ));
      out.push(format!("                {},", string(label)));
      out.push("            ))".into());
    }
    if let Some(last) = out.last_mut() {
      last.push(',');
    }
  }
  out.push("    ],".into());
  out.push("    cx,".into());
  out.push(")".into());
  out.push(".expect(\"the questionnaire's questions are valid\")".into());
  out
}

/// The questionnaire: progress, each question as a title over its choices and
/// its error line, and the previous / skip / next / submit row.
fn questionnaire_element(em: &mut Emitter, node: &Node, borrow: &str) -> Vec<String> {
  let mut out = vec![
    format!("Questionnaire::new({borrow})"),
    format!("    .child(QuestionnaireProgress::new({borrow}))"),
  ];
  let count = em.items(node, "questions").unwrap_or_default().len();
  for (index, line) in em
    .items(node, "questions")
    .unwrap_or_default()
    .iter()
    .enumerate()
  {
    let name = string(&format!("q{}", index + 1));
    out.push("    .child(".into());
    out.push(format!("        QuestionnaireItem::new({borrow}, {name})"));
    out.push(format!(
      "            .child(QuestionnaireTitle::new({borrow}, {name}))"
    ));
    out.push("            .child(".into());
    out.push(format!(
      "                QuestionnaireChoices::new({borrow}, {name})"
    ));
    let parsed = crate::extras::question(line);
    for at in 0..parsed.choices.len() {
      out.push(format!(
        "                    .child(QuestionnaireChoice::new({borrow}, {name}, {}))",
        string(&format!("choice-{}", at + 1))
      ));
    }
    out.push("            )".into());
    if parsed.freeform.is_some() {
      out.push(format!(
        "            .child(QuestionnaireInput::new({borrow}, {name}))"
      ));
    }
    out.push(format!(
      "            .child(QuestionnaireError::new({borrow}, {name}))"
    ));
    out.push("    )".into());
  }
  let _ = count;
  out.push("    .child(".into());
  out.push(format!("        QuestionnaireActions::new({borrow})"));
  for part in [
    "QuestionnairePrevious",
    "QuestionnaireSkip",
    "QuestionnaireNext",
    "QuestionnaireSubmit",
  ] {
    out.push(format!("            .child({part}::new({borrow}))"));
  }
  out.push("    )".into());
  out
}

/// The scroller and its row renderer: a `match` over the row index, each arm
/// the element that was dropped into that row. The renderer is `'static`, so
/// what the rows reach for is cloned in ahead of it.
fn message_scroller(em: &mut Emitter, node: &Node, borrow: &str) -> Vec<String> {
  let rows = em.items(node, "rows").unwrap_or_default().len();
  let placement = Placement { absolute: false };
  let (arms, captured) = em.scoped(|em| {
    let mut arms = vec!["match ix {".to_string()];
    for index in 0..rows {
      let children = node.slot(&format!("row:{index}")).to_vec();
      arms.push(format!("    {index} => {{"));
      match children.as_slice() {
        [] => arms.push("        div().into_any_element()".into()),
        [only] => {
          arms.extend(indent(&indent(&em.emit(*only, placement))));
          arms.push("            .into_any_element()".into());
        }
        many => {
          arms.push("        v_flex()".into());
          for child in many {
            let inner = em.emit(*child, placement);
            arms.extend(indent(&indent(&indent(&child_call(inner)))));
          }
          arms.push("            .into_any_element()".into());
        }
      }
      arms.push("    }".into());
    }
    arms.push("    _ => div().into_any_element(),".into());
    arms.push("}".into());
    arms
  });
  let head = format!(
    "MessageScroller::new({}, {}.clone(), ",
    string(&node.id.element_id()),
    borrow.trim_start_matches('&')
  );
  em.wrap_closure_with(head, "|ix, _window, _cx|", captured, arms)
}

/// The icons gpui-kit's `IconName` has. Its enum is a subset of Lucide — the
/// rest of the catalog lives in a different type — so a name outside this list
/// would not compile.
pub(crate) const DEFAULT_ICONS: &[&str] = &[
  "a-large-small",
  "arrow-down",
  "arrow-left",
  "arrow-right",
  "arrow-up",
  "asterisk",
  "ban",
  "battery-charging",
  "battery-full",
  "battery-low",
  "battery-medium",
  "battery-warning",
  "battery",
  "bell",
  "book-open",
  "bot",
  "building-2",
  "calendar",
  "case-sensitive",
  "chart-pie",
  "check",
  "chevron-down",
  "chevron-left",
  "chevron-right",
  "chevron-up",
  "chevrons-up-down",
  "circle-alert",
  "circle-check",
  "circle-user",
  "circle-x",
  "close",
  "copy",
  "cpu",
  "dash",
  "delete",
  "ellipsis-vertical",
  "ellipsis",
  "external-link",
  "eye-off",
  "eye",
  "file-text",
  "file",
  "folder-closed",
  "folder-open",
  "folder",
  "frame",
  "gallery-vertical-end",
  "github",
  "globe",
  "hard-drive",
  "heart-off",
  "heart",
  "inbox",
  "info",
  "inspector",
  "layout-dashboard",
  "loader-circle",
  "loader",
  "map",
  "maximize",
  "memory-stick",
  "menu",
  "minimize",
  "minus",
  "moon",
  "network",
  "palette",
  "panel-bottom-open",
  "panel-bottom",
  "panel-left-close",
  "panel-left-open",
  "panel-left",
  "panel-right-close",
  "panel-right-open",
  "panel-right",
  "pause",
  "play",
  "plus",
  "redo-2",
  "redo",
  "refresh-cw",
  "replace",
  "resize-corner",
  "rotate-cw",
  "search",
  "settings-2",
  "settings",
  "sort-ascending",
  "sort-descending",
  "square-terminal",
  "star-fill",
  "star-off",
  "star",
  "sun",
  "thumbs-down",
  "thumbs-up",
  "triangle-alert",
  "undo-2",
  "undo",
  "user",
  "window-close",
  "window-maximize",
  "window-minimize",
  "window-restore",
];

/// `IconName::X` for a name the default set has. A name it lacks becomes the
/// library's fallback, and the export says so, because a generated file that
/// does not compile is a worse way to find out.
pub fn icon(em: &mut Emitter, name: &str) -> String {
  if DEFAULT_ICONS.contains(&name) {
    return tailor_codegen::expr::icon_path(name);
  }
  let fallback = crate::library().fallback_icon().to_string();
  em.notes.push(format!(
    "gpui-kit's default icon set has no `{name}`, so `{fallback}` was used instead"
  ));
  fallback
}
