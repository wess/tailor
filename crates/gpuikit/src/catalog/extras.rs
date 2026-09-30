//! The command palette and the input group: the last two multi-part
//! components, both built from nodes rather than from a prop.
//!
//! A palette's entries and a group's addons are real designs — each item has
//! its own label, icon and action — so they are child nodes, not a list prop.
//! The parent's generator writes them (`Command::item(..)`, `InputGroup::addon(..)`)
//! and the parts are only valid inside it, which `Library::parents` says.

use tailor_model::catalog::{slot, ComponentSpec, Ctor, SlotSpec};
use tailor_model::comp;
use tailor_model::node::EventSpec;
use tailor_model::node::{CLICK, DEFAULT_SLOT};
use tailor_model::props::{boolean, enums, hinted, icon, text, Emit, PropValue};

use super::SIZE;

/// The palette's entries: items, groups and separators, in order.
const ENTRIES: SlotSpec = SlotSpec {
  key: DEFAULT_SLOT,
  label: "Entries",
  single: false,
  method: "item",
};

/// A group's items.
const GROUP_ITEMS: SlotSpec = SlotSpec {
  key: DEFAULT_SLOT,
  label: "Items",
  single: false,
  method: "item",
};

/// An addon's content: text, icons and buttons, in order.
const ADDON_CONTENT: SlotSpec = SlotSpec {
  key: DEFAULT_SLOT,
  label: "Content",
  single: false,
  method: "child",
};

/// Confirming an item — Enter, or a click — is reported to the palette by
/// position, and the palette's generator turns that back into this item's
/// action. The method is never printed.
const SELECT: EventSpec = EventSpec {
  key: "select",
  label: "On select",
  method: "on_confirm",
  args: &[],
};

const CANCEL: EventSpec = EventSpec {
  key: "cancel",
  label: "On cancel",
  method: "on_cancel",
  args: &[],
};

const QUERY: EventSpec = EventSpec {
  key: "query",
  label: "On search",
  method: "on_query",
  args: &[],
};

const ALIGNMENTS: &[&str] = &["inline-start", "inline-end", "block-start", "block-end"];

pub static SPECS: &[ComponentSpec] = &[
  comp!(
      "command", "Command palette", "Command", Navigation, "command",
      "A search field over a filtered list of commands, with groups.",
      Ctor::Stateful("CommandState"),
      props: &[
          text("placeholder", "Placeholder", Emit::Method("placeholder")),
          boolean("searchable", "Search field", Emit::Method("searchable"), true),
          boolean("filterable", "Filter as you type", Emit::Method("filterable"), true),
          boolean("bordered", "Bordered", Emit::Method("bordered"), true),
      ],
      slots: &[ENTRIES],
      events: &[CANCEL, QUERY],
      docs: "Drop Command item, Command group and Command separator nodes into it. Each \
             item's On select runs its action when confirmed. Put it in a Dialog to open it \
             from a button; on its own it is always showing.",
      aliases: &["palette", "command menu", "spotlight", "quick open", "cmdk"],
  ),
  comp!(
      "commandgroup", "Command group", "CommandGroup", Navigation, "folder",
      "A labelled run of command items.",
      Ctor::Unit,
      props: &[text("label", "Heading", Emit::Method("label"))],
      slots: &[GROUP_ITEMS],
      docs: "Only works inside a Command palette.",
  ),
  comp!(
      "commanditem", "Command item", "CommandItem", Navigation, "corner-down-left",
      "One command: a label, an icon, and what it does.",
      Ctor::Unit,
      props: &[
          text("label", "Label", Emit::Method("label")),
          icon("icon", "Icon", Emit::Method("icon")),
          hinted(
            text("keywords", "Search keywords", Emit::Custom),
            "Extra words that find it, separated by commas.",
          ),
          boolean("checked", "Checked", Emit::Method("checked"), false),
          boolean("disabled", "Disabled", Emit::Method("disabled"), false),
      ],
      events: &[SELECT],
      required: &[("label", "Type the command's name into Label.")],
      docs: "Only works inside a Command palette or a Command group.",
  ),
  comp!(
      "commandseparator", "Command separator", "", Navigation, "minus",
      "A rule between entries.",
      Ctor::Special,
      docs: "Only works inside a Command palette.",
  ),
  comp!(
      "inputgroup", "Input group", "InputGroup", Inputs, "text-cursor-input",
      "One frame around a text field and the icons, text and buttons beside it.",
      Ctor::Id,
      props: &[
          boolean("disabled", "Disabled", Emit::Method("disabled"), false),
          boolean("readonly", "Read only", Emit::Method("readonly"), false),
          boolean("invalid", "Invalid", Emit::Method("invalid"), false),
          SIZE,
      ],
      slots: &[
          slot("control", "Field", "input"),
          SlotSpec { key: "addons", label: "Addons", single: false, method: "addon" },
      ],
      docs: "Drop an Input or a Textarea into Field, and Input group addon nodes into Addons. \
             An addon sits on one side of the field — before or after it on the line, or above \
             or below it.",
      aliases: &["addon", "prefix", "suffix", "search box", "url field"],
  ),
  comp!(
      "inputgroupaddon", "Input group addon", "InputGroupAddon", Inputs, "panel-left",
      "Text, icons and buttons on one side of an input group's field.",
      Ctor::Id,
      props: &[enums(
        "align", "Side", Emit::Method("align"), "InputGroupAddonAlignment", ALIGNMENTS,
        || PropValue::Choice("inline-start".into()),
      )],
      slots: &[ADDON_CONTENT],
      docs: "Only works inside an Input group. Holds Input group text and Input group button \
             nodes, or an Icon.",
  ),
  comp!(
      "inputgrouptext", "Input group text", "InputGroupText", Inputs, "type",
      "Muted text inside an addon: a unit, a domain, a prefix.",
      Ctor::Unit,
      props: &[text("text", "Text", Emit::Custom)],
      required: &[("text", "Type the text into Text.")],
      docs: "Only works inside an Input group addon.",
  ),
  comp!(
      "inputgroupbutton", "Input group button", "InputGroupButton", Inputs, "square-mouse-pointer",
      "A compact button for an addon.",
      Ctor::Id,
      props: &[
          text("label", "Label", Emit::Method("label")),
          icon("icon", "Icon", Emit::Method("icon")),
          boolean("outline", "Outline", Emit::Flag("outline"), false),
          boolean("loading", "Loading", Emit::Method("loading"), false),
          boolean("caret", "Dropdown caret", Emit::Method("dropdown_caret"), false),
          boolean("disabled", "Disabled", Emit::Method("disabled"), false),
          text("tooltip", "Tooltip", Emit::Method("tooltip")),
      ],
      events: &[CLICK],
      required: &[("label", "Set one in the Attributes inspector, or give it an icon.")],
      docs: "Only works inside an Input group addon.",
  ),
];
