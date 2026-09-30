//! Coverage: every gpui-kit component is either in the catalog or excluded on
//! purpose, and the exclusions are checked back against the library.
//!
//! What it reads is `libraries/gpuikit.surface`, generated from the published
//! `gpui-component` source by `cargo run -p tailor-surface -- gpuikit`. gpui-kit
//! is not a dependency of this workspace — it builds on a different gpui — so
//! that file is how a pinned release still fails a build when it grows a type.
//!
//! Exclusions are rules, not one line per type: gpui-kit is about two hundred
//! types and most are the parts of another. A rule is an exact name or a
//! `Prefix*` family, and each carries the reason, which is the only record of
//! the call.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

struct Surface {
  version: String,
  components: BTreeMap<String, String>,
}

fn surface() -> Surface {
  let path = root().join("libraries/gpuikit.surface");
  let source = std::fs::read_to_string(&path).unwrap_or_else(|e| {
    panic!(
      "{}: {e}\nGenerate it with `cargo run -p tailor-surface -- gpuikit`.",
      path.display()
    )
  });
  let mut version = None;
  let mut components = BTreeMap::new();
  for line in source.lines().map(str::trim) {
    if line.is_empty() || line.starts_with('#') {
      continue;
    }
    let mut parts = line.split_whitespace();
    match parts.next() {
      Some("version") => version = parts.next().map(str::to_string),
      Some("component") => {
        let name = parts.next().expect("component name").to_string();
        components.insert(name, parts.next().unwrap_or("").to_string());
      }
      other => panic!("unknown surface line: {other:?}"),
    }
  }
  Surface {
    version: version.expect("the surface file names a version"),
    components,
  }
}

/// Types the catalog deliberately does not offer, and why. `Name*` matches the
/// whole family and `*Name` a suffix.
const EXCLUDED: &[(&str, &str)] = &[
  // Parts the generator writes itself, inside their parent's expression.
  (
    "BreadcrumbItem",
    "written by Breadcrumb's generator, one per item",
  ),
  ("Tab", "written by TabBar's generator, one per tab"),
  (
    "StepperItem",
    "written by Stepper's generator, one per step",
  ),
  ("EmptyHeader", "written inside Empty's expression"),
  ("EmptyTitle", "written inside Empty's expression"),
  ("EmptyDescription", "written inside Empty's expression"),
  (
    "EmptyMedia",
    "written inside Empty's expression, from its Icon prop",
  ),
  (
    "EmptyContent",
    "not modelled: Empty's children follow the text instead",
  ),
  ("DescriptionText", "a value inside DescriptionList"),
  ("ButtonIcon*", "an argument type of Button::icon"),
  ("IconName", "an icon value, not a component (see Icon)"),
  // Kept out of the catalog until the pieces below have somewhere to live.
  (
    "AccordionItem",
    "written inside Accordion's expression, one per section",
  ),
  (
    "CarouselContent",
    "the track of slides, written inside Carousel's expression",
  ),
  (
    "CarouselItem",
    "one slide, written inside Carousel's expression",
  ),
  (
    "CarouselNext",
    "the next arrow, written inside Carousel's expression",
  ),
  (
    "CarouselPrevious",
    "the previous arrow, written inside Carousel's expression",
  ),
  (
    "CarouselPagination",
    "the dots, written inside Carousel's expression",
  ),
  (
    "CarouselPaginationItem",
    "one dot, written inside Carousel's expression",
  ),
  (
    "SidebarHeader",
    "Sidebar::header takes any element, so the wrapper adds nothing a frame does not",
  ),
  (
    "SidebarFooter",
    "Sidebar::footer takes any element, so the wrapper adds nothing a frame does not",
  ),
  (
    "MarkerContent",
    "written inside Marker's expression, from its Text prop",
  ),
  (
    "MarkerIcon",
    "written inside Marker's expression, from its Icon prop",
  ),
  (
    "StepperSeparator",
    "drawn between StepperItems by Stepper itself",
  ),
  (
    "BreadcrumbSeparator",
    "drawn between BreadcrumbItems by Breadcrumb itself",
  ),
  (
    "Loading*",
    "list loading rows, owned by the List that shows them",
  ),
  (
    "ShimmerText",
    "animated text that needs a running clock; nothing to design",
  ),
  (
    "StatelessMarkdown",
    "renders a markdown string a host supplies",
  ),
  (
    "Text",
    "a text value type that Label and friends convert from, not a component",
  ),
  (
    "Scrollable",
    "scrolling is a style on a box, not a component",
  ),
  ("MediaLayout", "layout mode enum for Attachment"),
  // Conversation UI: built from message data at run time, not laid out by hand.
  (
    "AttachmentContent",
    "written inside Attachment's expression, from its Title and Description",
  ),
  (
    "AttachmentDescription",
    "written inside Attachment's expression, from its Description prop",
  ),
  (
    "AttachmentMedia",
    "written inside Attachment's expression, from its Icon prop",
  ),
  (
    "AttachmentTitle",
    "written inside Attachment's expression, from its Title prop",
  ),
  ("BubbleContent", "a Bubble's own children are its content"),
  (
    "MessageAvatar",
    "Message::avatar wraps any element in one itself",
  ),
  (
    "QuestionnaireInput",
    "written by Questionnaire for a question whose line has a `+` freeform cell",
  ),
  (
    "QuestionnaireActions",
    "the previous / skip / next / submit row, written inside Questionnaire's expression",
  ),
  (
    "QuestionnaireChoice",
    "one choice, written inside Questionnaire's expression",
  ),
  (
    "QuestionnaireChoiceDescription",
    "the text a choice draws from its `::` description; the part is for hand-built choices",
  ),
  (
    "QuestionnaireChoices",
    "the choices of one question, written inside Questionnaire's expression",
  ),
  (
    "QuestionnaireError",
    "a question's error line, written inside Questionnaire's expression",
  ),
  (
    "QuestionnaireItem",
    "one question, written inside Questionnaire's expression",
  ),
  (
    "QuestionnaireProgress",
    "the progress line, written inside Questionnaire's expression",
  ),
  // Opened through the window's root at run time, not placed in a tree.
  // Overlays. The kinds themselves are catalogued (`Dialog`, `AlertDialog`, `Sheet`,
  // `Notification`, `Popover`, `HoverCard`, `Tooltip`, `DropdownButton`); what is
  // left is the parts the generator writes inside them.
  (
    "DialogAction",
    "the confirm button of a Dialog, written inside its content by the generator",
  ),
  (
    "DialogClose",
    "the cancel button of a Dialog, written inside its content by the generator",
  ),
  (
    "DialogButton",
    "a button the dialog draws itself when it has no footer",
  ),
  (
    "DialogContent",
    "the closure argument of Dialog::content, not a node",
  ),
  (
    "DialogHeader",
    "written inside a Dialog's content from its title and description",
  ),
  (
    "DialogTitle",
    "written inside a Dialog's content from its title",
  ),
  (
    "DialogDescription",
    "written inside a Dialog's content from its description",
  ),
  (
    "DialogFooter",
    "written inside a Dialog's content from its footer region",
  ),
  ("DialogHost", "internal: the window root's dialog layer"),
  (
    "NotificationList",
    "internal: the window root's stack of toasts, which Notification pushes onto",
  ),
  (
    "HoverPopover",
    "an editor's hover popup, built from language-server data",
  ),
  (
    "PopupMenu",
    "the menu itself, built inside the closure a Dropdown button or Context menu writes",
  ),
  (
    "DropdownMenuPopover",
    "what `.dropdown_menu(..)` returns; a Dropdown button generates it",
  ),
  (
    "MenuItem",
    "the row a PopupMenu draws for an entry; entries are Menu item nodes",
  ),
  (
    "AppMenu*",
    "reads the menus the application registers with `cx.set_menus` at start-up, which are \
     app-level actions rather than something placed in a layout",
  ),
  (
    "CommandState",
    "the state behind Command, held as its field",
  ),
  ("Completion*", "editor completion popups"),
  ("CodeActionMenu", "editor popup"),
  ("DiagnosticPopover", "editor popup"),
  ("FallbackMenuOverlay", "internal menu overlay"),
  // Stateful: an `Entity<State>` built with a Window, which the emitter's
  // entity seam (`cx.new(Type::new)`) has no way to pass.
  // Not `Input*`: that would swallow `Input` itself, which is catalogued.
  ("InputToken", "an inline token inside an editor"),
  // Catalogued as a state field plus an element: the state types are the field's
  // type, and the private trigger inside a picker is the picker's own.
  (
    "ColorPickerButton",
    "private: the swatch inside ColorPicker",
  ),
  (
    "DatePickerState",
    "the state behind DatePicker, held as its field",
  ),
  (
    "ListItem",
    "a row of a list, written by the ListRows delegate the file defines",
  ),
  ("SettingsHost", "internal test host"),
  (
    "SearchableListItemElement",
    "a row of a delegate-driven list",
  ),
  // Window chrome, meaningful only against a real borderless window.
  ("TitleBar*", "window chrome; needs a real borderless window"),
  (
    "WindowBorder",
    "window chrome; needs a real borderless window",
  ),
  (
    "WindowControls",
    "window chrome; needs a real borderless window",
  ),
  ("WindowState*", "window chrome plumbing"),
  ("WindowTouchSelectionOverlay", "window-level plumbing"),
  // Internals and scaffolding that are `RenderOnce` only because gpui says so.
  ("BaseOwnedTextViewSelection", "text-selection internals"),
  ("CachedInnerPanel", "dock internals"),
  ("CachedPanelRoot", "dock internals"),
  ("Caret", "an input's cursor"),
  ("Chromeless", "an internal wrapper"),
  ("ControlIcon", "window control glyph"),
  ("CrossLine", "internal drag guide"),
  ("CrossRendererVirtualView", "internal renderer scaffolding"),
  ("DivInspector", "the inspector itself"),
  ("Dot", "internal marker"),
  ("DragColumn", "table drag internals"),
  ("DragPanelPreview", "dock drag internals"),
  ("DragPreview", "drag internals"),
  ("ResizeColumn", "table resize internals"),
  ("ResizePanel", "resizable-panel internals"),
  ("InvalidPanel", "dock placeholder for a missing panel"),
  ("MixedAdapterView", "internal renderer scaffolding"),
  ("RowsRoot", "internal renderer scaffolding"),
  ("ScrollbarLayer", "scrollbar internals"),
  ("SpinnerHost", "internal test host"),
  ("SettingsHost", "internal test host"),
  ("VariantHost", "internal test host"),
  ("NarrowSwitch", "internal test wrapper"),
  ("*Probe", "test probe compiled into the crate"),
  ("*TestView", "test view compiled into the crate"),
  ("*TestRoot", "test root compiled into the crate"),
  ("UnfocusedRoot", "test root compiled into the crate"),
  ("*ProbeRoot", "test probe compiled into the crate"),
];

/// Does `name` fall under `rule`? `Name*` is a prefix, `*Name` a suffix.
fn matches(rule: &str, name: &str) -> bool {
  match (rule.strip_prefix('*'), rule.strip_suffix('*')) {
    (Some(suffix), _) => name.ends_with(suffix),
    (_, Some(prefix)) => name.starts_with(prefix),
    _ => rule == name,
  }
}

fn excluded(name: &str) -> Option<&'static str> {
  EXCLUDED
    .iter()
    .find_map(|(rule, reason)| matches(rule, name).then_some(*reason))
}

fn catalogued() -> Vec<&'static str> {
  tailor_gpuikit::library()
    .components()
    .iter()
    .map(|spec| spec.rust)
    .collect()
}

#[test]
fn every_component_is_catalogued_or_excused() {
  let surface = surface();
  let catalogued = catalogued();
  let missing: Vec<String> = surface
    .components
    .iter()
    .filter(|(name, _)| !catalogued.contains(&name.as_str()))
    .filter(|(name, _)| excluded(name).is_none())
    .map(|(name, file)| format!("  {name} ({file})"))
    .collect();
  assert!(
    missing.is_empty(),
    "{} gpui-kit type(s) are neither in the catalog nor excluded.\n\
     Add a `comp!` entry plus a generator arm, or an EXCLUDED rule saying why not:\n{}",
    missing.len(),
    missing.join("\n")
  );
}

#[test]
fn the_exclusion_list_has_not_gone_stale() {
  let surface = surface();
  let catalogued = catalogued();
  for (rule, reason) in EXCLUDED {
    let matched: Vec<&String> = surface
      .components
      .keys()
      .filter(|name| matches(rule, name))
      .collect();
    assert!(
      !matched.is_empty(),
      "EXCLUDED lists {rule}, which gpui-kit no longer defines — drop the line"
    );
    for name in matched {
      assert!(
        !catalogued.contains(&name.as_str()),
        "{name} is excluded ({reason}) but the catalog offers it — narrow the rule"
      );
    }
  }
}

#[test]
fn the_surface_matches_the_requirement() {
  // Generated code asks for `version_req`; the surface records the exact
  // release the catalog was written against. If those drift, an export
  // compiles against a library that is not the one described.
  let surface = surface();
  let library = tailor_gpuikit::library();
  let req = library.version_req();
  assert!(
    surface.version == req || surface.version.starts_with(&format!("{req}.")),
    "generated code asks for {} {req}, but the surface was generated from {}",
    library.krate(),
    surface.version
  );
  assert!(
    surface.components.len() > 40,
    "surface lists only {} components — regenerate it",
    surface.components.len()
  );
}
