//! The live entity for a node that owns state.
//!
//! Half of guise is gpui entities — a text field owns a focus handle and a
//! buffer, a picker owns its open state — and an entity cannot be created
//! inside `render`. [`tailor_render::PreviewStore`] keeps one per node, rebuilt
//! when the node's props change; this file is the arm that builds each one.
//!
//! They go into the cache as `Rc<dyn Any>`, because what a live component *is*
//! is this crate's business and not the cache's. [`entity_element`] takes one
//! back out and downcasts it.

use std::any::Any;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{AnyElement, Context, Entity};
use guise::prelude::*;
use tailor_model::{Document, Node};

use tailor_render::PreviewStore;

use crate::read::Reader;

/// A live guise component the canvas is holding on behalf of a node.
#[derive(Clone)]
pub enum Preview {
  TextInput(Entity<TextInput>),
  TextArea(Entity<TextArea>),
  NumberInput(Entity<NumberInput>),
  PasswordInput(Entity<PasswordInput>),
  PinInput(Entity<PinInput>),
  Select(Entity<Select>),
  Combobox(Entity<Combobox>),
  Autocomplete(Entity<Autocomplete>),
  Segmented(Entity<SegmentedControl>),
  Slider(Entity<Slider>),
  RangeSlider(Entity<RangeSlider>),
  ColorInput(Entity<ColorInput>),
  TagsInput(Entity<TagsInput>),
  DatePicker(Entity<DatePicker>),
  TimePicker(Entity<TimePicker>),
  FileInput(Entity<FileInput>),
  Transfer(Entity<Transfer>),
  TreeView(Entity<TreeView>),
  TabBar(Entity<TabBar>),
  Pagination(Entity<Pagination>),
  NavigationMenu(Entity<NavigationMenu>),
  Editor(Entity<Editor>),
  MarkdownEditor(Entity<MarkdownEditor>),
  WebView(Entity<WebView>),
  CopyButton(Entity<CopyButton>),
  AIChatView(Entity<AIChatView>),
  AIComposer(Entity<AIComposer>),
  AIModelPicker(Entity<AIModelPicker>),
  AISettings(Entity<AISettings>),
  Menu(Entity<Menu>),
  ContextMenu(Entity<ContextMenu>),
}

/// Build the entity for a node. `None` for the kinds the canvas draws itself —
/// a `SplitPanel` takes its two regions as `'static` closures, so what the
/// canvas shows is drawn from the theme and there is no entity to hold.
pub fn build(node: &Node, doc: &Document, cx: &mut Context<PreviewStore>) -> Option<Rc<dyn Any>> {
  let read = Reader::new(node, doc);
  let preview = match node.kind.as_str() {
    "textinput" => Preview::TextInput(cx.new(|cx| {
      let mut field = TextInput::new(cx)
        .value(&read.text("value"))
        .placeholder(read.text("placeholder"))
        .size(read.size("size"))
        .radius(read.size("radius"))
        .disabled(read.bool("disabled"))
        .read_only(read.bool("read_only"))
        .password(read.bool("password"));
      if !read.text("label").is_empty() {
        field = field.label(read.text("label"));
      }
      if !read.text("description").is_empty() {
        field = field.description(read.text("description"));
      }
      if !read.text("error").is_empty() {
        field = field.error(read.text("error"));
      }
      let max = read.usize("max_length");
      if max > 0 {
        field = field.max_length(max);
      }
      field
    })),
    "textarea" => Preview::TextArea(cx.new(|cx| {
      let mut field = TextArea::new(cx)
        .value(&read.text("value"))
        .placeholder(read.text("placeholder"))
        .rows(read.usize("rows").max(1))
        .size(read.size("size"))
        .disabled(read.bool("disabled"))
        .submit_on_enter(read.bool("submit_on_enter"));
      if !read.text("label").is_empty() {
        field = field.label(read.text("label"));
      }
      let max = read.usize("max_rows");
      if max > 0 {
        field = field.max_rows(max);
      }
      field
    })),
    "numberinput" => Preview::NumberInput(cx.new(|cx| {
      let mut field = NumberInput::new(cx)
        .value(read.f64("value"))
        .min(read.f64("min"))
        .max(read.f64("max"))
        .step(read.f64("step").max(0.0001))
        .size(read.size("size"))
        .disabled(read.bool("disabled"));
      if !read.text("label").is_empty() {
        field = field.label(read.text("label"));
      }
      field
    })),
    "passwordinput" => Preview::PasswordInput(cx.new(|cx| {
      let mut field = PasswordInput::new(cx)
        .placeholder(read.text("placeholder"))
        .size(read.size("size"))
        .disabled(read.bool("disabled"))
        .visible(read.bool("visible"))
        .read_only(read.bool("read_only"));
      if !read.text("label").is_empty() {
        field = field.label(read.text("label"));
      }
      field
    })),
    "pininput" => Preview::PinInput(cx.new(|cx| {
      PinInput::new(cx)
        .value(&read.text("value"))
        .length(read.usize("length").clamp(1, 12))
        .mask(read.bool("mask"))
        .size(read.size("size"))
        .disabled(read.bool("disabled"))
    })),
    "select" => Preview::Select(cx.new(|cx| {
      let mut field = Select::new(cx)
        .data(read.items("data"))
        .selected(read.usize("selected"))
        .placeholder(read.text("placeholder"))
        .size(read.size("size"))
        .disabled(read.bool("disabled"));
      if !read.text("label").is_empty() {
        field = field.label(read.text("label"));
      }
      field
    })),
    "combobox" => Preview::Combobox(cx.new(|cx| {
      let mut field = Combobox::new(cx)
        .data(read.items("data"))
        .multiple(read.bool("multiple"))
        .placeholder(read.text("placeholder"))
        .size(read.size("size"))
        .disabled(read.bool("disabled"));
      if !read.text("label").is_empty() {
        field = field.label(read.text("label"));
      }
      field
    })),
    "autocomplete" => Preview::Autocomplete(cx.new(|cx| {
      let mut field = Autocomplete::new(cx)
        .suggestions(read.items("suggestions"))
        .value(read.text("value").to_string())
        .max_shown(read.usize("max_shown").max(1))
        .placeholder(read.text("placeholder"))
        .size(read.size("size"))
        .disabled(read.bool("disabled"));
      if !read.text("label").is_empty() {
        field = field.label(read.text("label"));
      }
      field
    })),
    "segmented" => Preview::Segmented(cx.new(|cx| {
      SegmentedControl::new(cx)
        .data(read.items("data"))
        .selected(read.usize("selected"))
        .size(read.size("size"))
    })),
    "slider" => Preview::Slider(cx.new(|cx| {
      Slider::new(cx)
        .min(read.f64("min"))
        .max(read.f64("max"))
        .step(read.f64("step").max(0.0001))
        .color(read.color_name("color"))
        .disabled(read.bool("disabled"))
    })),
    "rangeslider" => Preview::RangeSlider(cx.new(|cx| {
      RangeSlider::new(cx)
        .min(read.f64("min"))
        .max(read.f64("max"))
        .step(read.f64("step").max(0.0001))
        .min_gap(read.f64("min_gap"))
        .color(read.color_name("color"))
        .size(read.size("size"))
        .disabled(read.bool("disabled"))
    })),
    "colorinput" => Preview::ColorInput(cx.new(|cx| {
      let mut field = ColorInput::new(cx)
        .size(read.size("size"))
        .disabled(read.bool("disabled"));
      if !read.text("label").is_empty() {
        field = field.label(read.text("label"));
      }
      field
    })),
    "tagsinput" => Preview::TagsInput(cx.new(|cx| {
      let mut field = TagsInput::new(cx)
        .tags(read.items("tags"))
        .placeholder(read.text("placeholder"))
        .size(read.size("size"))
        .disabled(read.bool("disabled"));
      if !read.text("label").is_empty() {
        field = field.label(read.text("label"));
      }
      let max = read.usize("max_tags");
      if max > 0 {
        field = field.max_tags(max);
      }
      field
    })),
    "datepicker" => Preview::DatePicker(cx.new(|cx| {
      let mut field = DatePicker::new(cx)
        .placeholder(read.text("placeholder"))
        .size(read.size("size"))
        .disabled(read.bool("disabled"));
      if read.bool("range_mode") {
        field = field.range_mode();
      }
      if !read.text("format").is_empty() {
        field = field.format(read.text("format"));
      }
      if !read.text("label").is_empty() {
        field = field.label(read.text("label"));
      }
      field
    })),
    "timepicker" => Preview::TimePicker(cx.new(|cx| {
      let mut field = TimePicker::new(cx)
        .minute_step(read.usize("minute_step").clamp(1, 30) as u32)
        .placeholder(read.text("placeholder"))
        .size(read.size("size"))
        .disabled(read.bool("disabled"));
      if read.bool("twenty_four_hour") {
        field = field.twenty_four_hour();
      }
      if !read.text("label").is_empty() {
        field = field.label(read.text("label"));
      }
      field
    })),
    "fileinput" => Preview::FileInput(cx.new(|cx| {
      let mut field = FileInput::new(cx)
        .placeholder(read.text("placeholder"))
        .size(read.size("size"))
        .disabled(read.bool("disabled"))
        .accept(read.items("accept"));
      if read.bool("multiple") {
        field = field.multiple();
      }
      if read.bool("directories") {
        field = field.directories();
      }
      if !read.text("label").is_empty() {
        field = field.label(read.text("label"));
      }
      field
    })),
    "transfer" => Preview::Transfer(cx.new(|cx| {
      Transfer::new(cx)
        .data(read.items("data"))
        .height(read.f32("height"))
        .disabled(read.bool("disabled"))
    })),
    "treeview" => Preview::TreeView(cx.new(|cx| {
      TreeView::new(cx)
        .nodes(crate::read::tree_nodes(&read.raw_items("nodes")))
        .height(read.f32("height"))
        .default_expanded(read.bool("default_expanded"))
    })),
    "tabbar" => Preview::TabBar(cx.new(|cx| {
      TabBar::new(cx)
        .tabs(read.items("tabs"))
        .active(read.usize("active"))
        .with_add_button(read.bool("with_add_button"))
    })),
    "pagination" => Preview::Pagination(cx.new(|cx| {
      Pagination::new(cx, read.usize("total").max(1))
        .active(read.usize("active").max(1))
        .color(read.color_name("color"))
    })),
    "navigationmenu" => Preview::NavigationMenu(cx.new(|cx| {
      let mut menu = NavigationMenu::new(cx);
      for entry in read.raw_items("items") {
        let (id, label) = entry.split_once(':').unwrap_or((&entry, &entry));
        menu = menu.item(id.trim().to_string(), label.trim().to_string());
      }
      if !read.text("active").is_empty() {
        menu = menu.active(read.text("active"));
      }
      menu
    })),
    "editor" => Preview::Editor(cx.new(|cx| {
      Editor::new(cx)
        .value(&read.text("value"))
        .language(crate::read::language(&read.choice("language")))
        .rows(((read.f32("height") / 20.0).round() as usize).max(3))
    })),
    "markdowneditor" => Preview::MarkdownEditor(cx.new(|cx| {
      MarkdownEditor::new(cx)
        .value(&read.text("value"))
        .placeholder(read.text("placeholder"))
        .rows(read.usize("rows").max(3))
        .read_only(read.bool("read_only"))
    })),
    "copybutton" => Preview::CopyButton(cx.new(|_| {
      let mut button = CopyButton::new(read.text("value"));
      if !read.text("label").is_empty() {
        button = button.label(read.text("label"));
      }
      button
    })),
    "webview" => Preview::WebView(cx.new(|cx| {
      let view = WebView::new(cx);
      if !read.text("url").is_empty() {
        view.url(read.text("url"))
      } else {
        view
      }
    })),
    "aichatview" => Preview::AIChatView(cx.new(|cx| {
      // Typed one turn per line, alternating the way a transcript reads.
      let turns = read
        .raw_items("turns")
        .into_iter()
        .enumerate()
        .map(|(index, body)| {
          let role = if index % 2 == 0 {
            AIRole::User
          } else {
            AIRole::Assistant
          };
          AITurn::new(role, body)
        });
      AIChatView::new(cx).turns(turns)
    })),
    "aicomposer" => Preview::AIComposer(cx.new(|cx| {
      let mut composer = AIComposer::new(cx)
        .attachments(read.bool("attachments"))
        .disabled(read.bool("disabled"))
        .size(read.size("size"));
      if !read.text("hint").is_empty() {
        composer = composer.hint(read.text("hint"));
      }
      composer
    })),
    "aimodelpicker" => Preview::AIModelPicker(cx.new(|cx| {
      let models = read
        .raw_items("models")
        .into_iter()
        .map(|label| AIModel::new(tailor_model::snake_case(&label), label));
      let mut picker = AIModelPicker::new(cx)
        .models(models)
        .disabled(read.bool("disabled"))
        .size(read.size("size"));
      if !read.text("label").is_empty() {
        picker = picker.label(read.text("label"));
      }
      picker
    })),
    "aisettings" => Preview::AISettings(cx.new(|cx| AISettings::new(cx).size(read.size("size")))),
    "menu" => Preview::Menu(cx.new(|cx| {
      menu_items(
        Menu::new(cx, read.text("trigger")).size(read.size("size")),
        &read,
      )
    })),
    "contextmenu" => Preview::ContextMenu(cx.new(|cx| {
      context_menu_items(
        ContextMenu::new(cx)
          .width(read.f32("width"))
          .size(read.size("size")),
        &read,
      )
    })),
    _ => return None,
  };
  Some(Rc::new(preview))
}

/// One `.item(..)` per line, with `-` for a divider and a leading `#` for a
/// section heading. The handler is a no-op on the canvas: a designed menu has
/// nothing to do yet, and the generated code wires the events the node carries.
fn menu_items(mut menu: Menu, read: &Reader<'_>) -> Menu {
  for line in read.raw_items("items") {
    let line = line.trim();
    if line == "-" {
      menu = menu.divider();
    } else if let Some(label) = line.strip_prefix('#') {
      menu = menu.section(label.trim().to_string());
    } else {
      menu = menu.item(line.to_string(), |_, _| {});
    }
  }
  menu
}

fn context_menu_items(mut menu: ContextMenu, read: &Reader<'_>) -> ContextMenu {
  for line in read.raw_items("items") {
    let line = line.trim();
    if line == "-" {
      menu = menu.divider();
    } else if let Some(label) = line.strip_prefix('#') {
      menu = menu.section(label.trim().to_string());
    } else {
      menu = menu.item(line.to_string(), |_, _| {});
    }
  }
  menu
}

/// A cached entity, drawn as itself. `None` when the box holds something else,
/// which can only happen if two providers shared a store — they do not.
pub fn entity_element(preview: &Rc<dyn Any>) -> Option<AnyElement> {
  let preview = preview.downcast_ref::<Preview>()?.clone();
  Some(match preview {
    Preview::TextInput(e) => e.into_any_element(),
    Preview::TextArea(e) => e.into_any_element(),
    Preview::NumberInput(e) => e.into_any_element(),
    Preview::PasswordInput(e) => e.into_any_element(),
    Preview::PinInput(e) => e.into_any_element(),
    Preview::Select(e) => e.into_any_element(),
    Preview::Combobox(e) => e.into_any_element(),
    Preview::Autocomplete(e) => e.into_any_element(),
    Preview::Segmented(e) => e.into_any_element(),
    Preview::Slider(e) => e.into_any_element(),
    Preview::RangeSlider(e) => e.into_any_element(),
    Preview::ColorInput(e) => e.into_any_element(),
    Preview::TagsInput(e) => e.into_any_element(),
    Preview::DatePicker(e) => e.into_any_element(),
    Preview::TimePicker(e) => e.into_any_element(),
    Preview::FileInput(e) => e.into_any_element(),
    Preview::Transfer(e) => e.into_any_element(),
    Preview::TreeView(e) => e.into_any_element(),
    Preview::TabBar(e) => e.into_any_element(),
    Preview::Pagination(e) => e.into_any_element(),
    Preview::NavigationMenu(e) => e.into_any_element(),
    Preview::Editor(e) => e.into_any_element(),
    Preview::MarkdownEditor(e) => e.into_any_element(),
    Preview::WebView(e) => e.into_any_element(),
    Preview::CopyButton(e) => e.into_any_element(),
    Preview::AIChatView(e) => e.into_any_element(),
    Preview::AIComposer(e) => e.into_any_element(),
    Preview::AIModelPicker(e) => e.into_any_element(),
    Preview::AISettings(e) => e.into_any_element(),
    Preview::Menu(e) => e.into_any_element(),
    Preview::ContextMenu(e) => e.into_any_element(),
  })
}
