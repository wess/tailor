//! Things that float above the layout: popovers, dialogs, sheets, menus.
//!
//! gpui-kit builds most of these by handing a closure a builder — a dialog's
//! content, a popover's panel, a menu's items — and a designer cannot drop
//! into a closure. The catalog models each as what a person actually lays out:
//! a trigger, and the regions of what it opens. The generator writes the
//! closure around them (`crate::overlays`), and the canvas draws every one as
//! a card whose regions are drop targets.
//!
//! Two of them have no declarative trigger in gpui-kit — a sheet and a
//! notification are opened by calling the window — so those are a button that
//! makes the call. A dialog, alert dialog, popover and hover card take any
//! trigger; a popover wants one that can show it is open, which in practice is
//! a button.

use tailor_model::catalog::{slot, ComponentSpec, Ctor};
use tailor_model::comp;
use tailor_model::node::{EventSpec, DEFAULT_SLOT};
use tailor_model::props::{boolean, enums, icon, int, text, Emit, PropSpec, PropValue};

use super::SIZE;

/// The default region, under the name that says what goes in it.
const CONTENT: tailor_model::catalog::SlotSpec = tailor_model::catalog::SlotSpec {
  key: DEFAULT_SLOT,
  label: "Content",
  single: false,
  method: "child",
};
const BODY: tailor_model::catalog::SlotSpec = tailor_model::catalog::SlotSpec {
  key: DEFAULT_SLOT,
  label: "Body",
  single: false,
  method: "child",
};
const FOOTER: tailor_model::catalog::SlotSpec = tailor_model::catalog::SlotSpec {
  key: "footer",
  label: "Footer",
  single: false,
  method: "footer",
};
const ITEMS: tailor_model::catalog::SlotSpec = tailor_model::catalog::SlotSpec {
  key: "items",
  label: "Menu items",
  single: false,
  method: "item",
};

const ANCHORS: &[&str] = &[
  "top-left",
  "top-right",
  "bottom-left",
  "bottom-right",
  "top-center",
  "bottom-center",
];

const BUTTON_VARIANTS: &[&str] = &[
  "default",
  "primary",
  "secondary",
  "danger",
  "info",
  "success",
  "warning",
  "ghost",
  "link",
  "text",
];

// Handlers the generator writes itself (`Generator::writes`); the specs are
// what the inspector's Events tab lists.
const OK: EventSpec = EventSpec {
  key: "ok",
  label: "On confirm",
  method: "on_ok",
  args: &[],
};
const CANCEL: EventSpec = EventSpec {
  key: "cancel",
  label: "On cancel",
  method: "on_cancel",
  args: &[],
};
const CLOSE: EventSpec = EventSpec {
  key: "close",
  label: "On close",
  method: "on_close",
  args: &[],
};
const OPEN_CHANGE: EventSpec = EventSpec {
  key: "open_change",
  label: "On open change",
  method: "on_open_change",
  args: &["open"],
};
const CLICK: EventSpec = EventSpec {
  key: "click",
  label: "On click",
  method: "on_click",
  args: &[],
};

// The button that opens a sheet or pushes a notification, as the generic button
// props so the generic emitter prints them. Functions rather than a shared
// array because a prop spec is not `Copy`.
const fn opener_label() -> PropSpec {
  text("label", "Button label", Emit::Method("label"))
}
const fn opener_variant() -> PropSpec {
  enums(
    "variant",
    "Button variant",
    Emit::Method("with_variant"),
    "ButtonVariant",
    BUTTON_VARIANTS,
    || PropValue::Choice("default".into()),
  )
}
const fn opener_icon() -> PropSpec {
  icon("icon", "Button icon", Emit::Method("icon"))
}
const fn opener_outline() -> PropSpec {
  boolean("outline", "Outline", Emit::Flag("outline"), false)
}
const fn opener_disabled() -> PropSpec {
  boolean("disabled", "Disabled", Emit::Method("disabled"), false)
}

pub static SPECS: &[ComponentSpec] = &[
  comp!(
      "popover", "Popover", "Popover", Feedback, "message-square",
      "A floating panel opened by a trigger.",
      Ctor::Id,
      props: &[
          enums("anchor", "Anchor", Emit::Method("anchor"), "Anchor", ANCHORS,
            || PropValue::Choice("top-left".into())),
          boolean("arrow", "Arrow", Emit::Method("arrow"), false),
          boolean("open", "Open at start", Emit::Method("default_open"), false),
          boolean("overlay_closable", "Click outside closes", Emit::Method("overlay_closable"), true),
          boolean("appearance", "Panel styling", Emit::Method("appearance"), true),
      ],
      slots: &[CONTENT, slot("trigger", "Trigger", "trigger")],
      events: &[OPEN_CHANGE],
      docs: "Drop a Button in Trigger and what the panel holds in Content. The trigger has to \
             be something that can show it is open — a Button, Checkbox or Dropdown button. \
             Use Hover card for a panel that opens on hover, and Dialog for one that blocks \
             the window.",
      aliases: &["popup", "flyout", "floating panel"],
  ),
  comp!(
      "hovercard", "Hover card", "HoverCard", Feedback, "mouse-pointer-2",
      "A card that opens when the pointer rests on its trigger.",
      Ctor::Id,
      props: &[
          enums("anchor", "Anchor", Emit::Method("anchor"), "Anchor", ANCHORS,
            || PropValue::Choice("top-center".into())),
          int("open_delay", "Open delay (ms)", Emit::Custom, || PropValue::Int(600)),
          int("close_delay", "Close delay (ms)", Emit::Custom, || PropValue::Int(300)),
          boolean("appearance", "Card styling", Emit::Method("appearance"), true),
      ],
      slots: &[CONTENT, slot("trigger", "Trigger", "trigger")],
      events: &[OPEN_CHANGE],
      docs: "Any element can be the trigger. For a panel that opens on click use Popover.",
      aliases: &["preview card", "hover popover"],
  ),
  comp!(
      "tooltip", "Tooltip", "Tooltip", Feedback, "message-square-text",
      "A short label that appears when the pointer rests on what is inside.",
      Ctor::Special,
      props: &[text("text", "Text", Emit::Custom)],
      slots: &[CONTENT],
      required: &[("text", "Type what the tooltip should say.")],
      docs: "Wraps whatever it holds in a box that shows the text on hover. A Button already \
             has its own Tooltip prop; use this for anything else.",
      aliases: &["hint", "title text"],
  ),
  comp!(
      "dialog", "Dialog", "Dialog", Feedback, "app-window",
      "A modal window with a title, a body and confirm and cancel buttons.",
      Ctor::Special,
      props: &[
          text("title", "Title", Emit::None),
          text("description", "Description", Emit::None),
          text("ok_label", "Confirm label", Emit::None),
          text("cancel_label", "Cancel label", Emit::None),
          boolean("destructive", "Destructive confirm", Emit::None, false),
          int("width", "Width", Emit::Custom, || PropValue::Int(448)),
          boolean("overlay", "Dim the window", Emit::Method("overlay"), true),
          boolean("close_button", "Close button", Emit::Method("close_button"), true),
          boolean("keyboard", "Escape closes", Emit::Method("keyboard"), true),
          boolean("overlay_closable", "Click outside closes", Emit::Method("overlay_closable"), true),
      ],
      slots: &[BODY, slot("trigger", "Trigger", "trigger"), FOOTER],
      events: &[OK, CANCEL, CLOSE],
      on_place: Some(|node| {
          node.set_prop("title", PropValue::Text("Dialog".into()));
          node.set_prop("ok_label", PropValue::Text("OK".into()));
          node.set_prop("cancel_label", PropValue::Text("Cancel".into()));
      }),
      docs: "Drop the Button that opens it in Trigger and the form or message in Body. The \
             confirm and cancel buttons are written for you; put your own in Footer to replace \
             them. Use Alert dialog for a short yes or no question.",
      aliases: &["modal", "popup window", "confirm"],
  ),
  comp!(
      "alertdialog", "Alert dialog", "AlertDialog", Feedback, "triangle-alert",
      "A short modal that asks for a decision.",
      Ctor::Special,
      props: &[
          text("title", "Title", Emit::None),
          text("description", "Description", Emit::None),
          text("ok_label", "Confirm label", Emit::None),
          text("cancel_label", "Cancel label", Emit::None),
          boolean("destructive", "Destructive confirm", Emit::None, false),
          int("width", "Width", Emit::Custom, || PropValue::Int(448)),
      ],
      slots: &[slot("trigger", "Trigger", "trigger")],
      events: &[OK, CANCEL],
      on_place: Some(|node| {
          node.set_prop("title", PropValue::Text("Are you sure?".into()));
          node.set_prop("ok_label", PropValue::Text("OK".into()));
          node.set_prop("cancel_label", PropValue::Text("Cancel".into()));
      }),
      docs: "Title and description are text only. It cannot be dismissed by clicking outside, \
             which is what separates it from Dialog. Clear the cancel label for an alert with \
             one button.",
      aliases: &["confirm dialog", "are you sure", "warning dialog"],
  ),
  comp!(
      "sheet", "Sheet", "Sheet", Feedback, "panel-right",
      "A panel that slides in from an edge, opened by a button.",
      Ctor::Special,
      props: &[
          opener_label(), opener_variant(), opener_icon(), opener_outline(), opener_disabled(),
          text("title", "Sheet title", Emit::None),
          enums("placement", "Edge", Emit::Custom, "Placement",
            &["right", "left", "top", "bottom"],
            || PropValue::Choice("right".into())),
          int("size", "Size", Emit::Custom, || PropValue::Int(350)),
          boolean("overlay", "Dim the window", Emit::Custom, true),
          boolean("overlay_closable", "Click outside closes", Emit::Custom, true),
          boolean("resizable", "Resizable", Emit::Custom, true),
      ],
      slots: &[BODY, FOOTER],
      events: &[CLOSE],
      on_place: Some(|node| {
          node.set_prop("label", PropValue::Text("Open sheet".into()));
          node.set_prop("title", PropValue::Text("Sheet".into()));
      }),
      docs: "gpui-kit opens a sheet by calling the window, so this is the button that makes \
             the call; its Body and Footer are what slides in.",
      aliases: &["drawer", "side panel", "slide over"],
  ),
  comp!(
      "notification", "Notification", "Notification", Feedback, "bell",
      "A button that pushes a toast onto the window.",
      Ctor::Special,
      props: &[
          opener_label(), opener_variant(), opener_icon(), opener_outline(), opener_disabled(),
          text("title", "Title", Emit::None),
          text("message", "Message", Emit::None),
          enums("kind", "Type", Emit::None, "NotificationType",
            &["info", "success", "warning", "error"],
            || PropValue::Choice("info".into())),
          boolean("autohide", "Hide by itself", Emit::None, true),
      ],
      on_place: Some(|node| {
          node.set_prop("label", PropValue::Text("Notify".into()));
          node.set_prop("message", PropValue::Text("Saved.".into()));
      }),
      docs: "gpui-kit shows a notification by calling the window, so this is the button that \
             makes the call. To notify from your own code, call `window.push_notification` \
             yourself.",
      aliases: &["toast", "snackbar", "banner"],
  ),
  comp!(
      "dropdownbutton", "Dropdown button", "DropdownButton", Navigation, "chevrons-up-down",
      "A button with a menu.",
      Ctor::Special,
      props: &[
          text("label", "Label", Emit::Custom),
          icon("icon", "Icon", Emit::Custom),
          enums("variant", "Variant", Emit::Method("with_variant"), "ButtonVariant",
            BUTTON_VARIANTS, || PropValue::Choice("default".into())),
          SIZE,
          boolean("outline", "Outline", Emit::Flag("outline"), false),
          boolean("disabled", "Disabled", Emit::Method("disabled"), false),
      ],
      slots: &[ITEMS],
      on_place: Some(|node| node.set_prop("label", PropValue::Text("Actions".into()))),
      docs: "Fill Menu items with Menu item, Menu separator and Submenu.",
      aliases: &["menu button", "actions menu", "select menu", "split button"],
  ),
  comp!(
      "contextmenu", "Context menu", "ContextMenu", Navigation, "mouse-pointer-click",
      "A region that opens a menu on right-click.",
      Ctor::Special,
      slots: &[CONTENT, ITEMS],
      docs: "Whatever is in Content answers a right-click with the items in Menu items.",
      aliases: &["right click menu"],
  ),
  comp!(
      "menuitem", "Menu item", "PopupMenuItem", Navigation, "list",
      "One entry of a menu.",
      Ctor::Special,
      props: &[
          text("label", "Label", Emit::Custom),
          icon("icon", "Icon", Emit::Custom),
          boolean("checked", "Checked", Emit::Custom, false),
          boolean("disabled", "Disabled", Emit::Custom, false),
      ],
      events: &[CLICK],
      required: &[("label", "Type what the entry says.")],
      on_place: Some(|node| node.set_prop("label", PropValue::Text("Item".into()))),
      docs: "Only means something inside a Dropdown button, Context menu or Submenu.",
  ),
  comp!(
      "menuseparator", "Menu separator", "", Navigation, "minus",
      "A rule between groups of menu items.",
      Ctor::Special,
      docs: "Only means something inside a Dropdown button, Context menu or Submenu.",
  ),
  comp!(
      "menulabel", "Menu label", "", Navigation, "tag",
      "A heading for a group of menu items.",
      Ctor::Special,
      props: &[text("label", "Label", Emit::Custom)],
      on_place: Some(|node| node.set_prop("label", PropValue::Text("Group".into()))),
      docs: "Only means something inside a Dropdown button, Context menu or Submenu.",
  ),
  comp!(
      "submenu", "Submenu", "", Navigation, "chevron-right",
      "A menu item that opens another menu.",
      Ctor::Special,
      props: &[
          text("label", "Label", Emit::Custom),
          icon("icon", "Icon", Emit::Custom),
      ],
      slots: &[ITEMS],
      on_place: Some(|node| node.set_prop("label", PropValue::Text("More".into()))),
      docs: "Only means something inside a Dropdown button, Context menu or another Submenu.",
  ),
];
