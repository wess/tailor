//! Buttons, toggles, and the boolean controls.
//!
//! Every builder here takes an element id, which the generator fills with the
//! node's own (`Ctor::Id`) — gpui-kit uses it to key focus and hover state, so
//! it has to be stable across renders.

use tailor_model::catalog::{ComponentSpec, Ctor, CHILDREN};
use tailor_model::comp;
use tailor_model::node::{CHANGE_BOOL, CHANGE_INDEX, CLICK};
use tailor_model::props::{boolean, enums, icon, int, items, text, Emit, PropValue};

use super::{CLICK_BOOL, CLICK_INDEX, CLICK_SELECTION, SIZE};

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

pub static SPECS: &[ComponentSpec] = &[
  comp!(
      "button", "Button", "Button", Controls, "square-mouse-pointer",
      "A labelled action.",
      Ctor::Id,
      props: &[
          text("label", "Label", Emit::Method("label")),
          enums("variant", "Variant", Emit::Method("with_variant"), "ButtonVariant",
            BUTTON_VARIANTS, || PropValue::Choice("default".into())),
          icon("icon", "Icon", Emit::Method("icon")),
          SIZE,
          boolean("outline", "Outline", Emit::Flag("outline"), false),
          boolean("compact", "Compact", Emit::Flag("compact"), false),
          boolean("loading", "Loading", Emit::Method("loading"), false),
          boolean("disabled", "Disabled", Emit::Method("disabled"), false),
          text("tooltip", "Tooltip", Emit::Method("tooltip")),
      ],
      // a label and an icon cover it; a child slot would flag every labelled
      // button as empty and offer a drop target on it
      events: &[CLICK],
      required: &[("label", "Set one in the Attributes inspector, or give it an icon.")],
      aliases: &["cta", "submit", "action"],
  ),
  comp!(
      "buttongroup", "Button group", "ButtonGroup", Controls, "layout-list",
      "Buttons joined into one control.",
      Ctor::Id,
      props: &[
          boolean("compact", "Compact", Emit::Flag("compact"), false),
          boolean("outline", "Outline", Emit::Flag("outline"), false),
          boolean("multiple", "Multiple", Emit::Method("multiple"), false),
          boolean("disabled", "Disabled", Emit::Method("disabled"), false),
          SIZE,
      ],
      slots: &[CHILDREN],
      events: &[CLICK_SELECTION],
      docs: "Children must be Buttons; anything else will not compile.",
      aliases: &["segmented buttons"],
  ),
  comp!(
      "toggle", "Toggle", "Toggle", Controls, "toggle-left",
      "A button that stays pressed.",
      Ctor::Id,
      props: &[
          text("label", "Label", Emit::Method("label")),
          icon("icon", "Icon", Emit::Method("icon")),
          boolean("checked", "Pressed", Emit::Method("checked"), false),
          boolean("disabled", "Disabled", Emit::Method("disabled"), false),
          SIZE,
      ],
      events: &[CLICK_BOOL],
      aliases: &["toggle button"],
  ),
  comp!(
      "togglegroup", "Toggle group", "ToggleGroup", Controls, "toggle-right",
      "Toggles laid out together.",
      Ctor::Id,
      props: &[
          boolean("segmented", "Segmented", Emit::Flag("segmented"), false),
          boolean("disabled", "Disabled", Emit::Method("disabled"), false),
          SIZE,
      ],
      slots: &[CHILDREN],
      docs: "Children must be Toggles.",
  ),
  comp!(
      "switch", "Switch", "Switch", Controls, "toggle-left",
      "An on/off switch.",
      Ctor::Id,
      props: &[
          boolean("checked", "On", Emit::Method("checked"), false),
          text("label", "Label", Emit::Method("label")),
          boolean("disabled", "Disabled", Emit::Method("disabled"), false),
          SIZE,
      ],
      events: &[CHANGE_BOOL],
      aliases: &["toggle switch", "on off"],
  ),
  comp!(
      "checkbox", "Checkbox", "Checkbox", Controls, "square-check",
      "A tick box.",
      Ctor::Id,
      props: &[
          boolean("checked", "Checked", Emit::Method("checked"), false),
          text("label", "Label", Emit::Method("label")),
          boolean("disabled", "Disabled", Emit::Method("disabled"), false),
          SIZE,
      ],
      events: &[CHANGE_BOOL],
      aliases: &["tick", "check box"],
  ),
  comp!(
      "radio", "Radio", "Radio", Controls, "circle-dot",
      "A single option. Use a Radio group to make them exclusive.",
      Ctor::Id,
      props: &[
          boolean("checked", "Checked", Emit::Method("checked"), false),
          text("label", "Label", Emit::Method("label")),
          boolean("disabled", "Disabled", Emit::Method("disabled"), false),
          SIZE,
      ],
      events: &[CHANGE_BOOL],
  ),
  comp!(
      "radiogroup", "Radio group", "RadioGroup", Controls, "list-checks",
      "Options of which exactly one is picked.",
      Ctor::Id,
      props: &[
          items("options", "Options", Emit::Custom, || {
            PropValue::Items(vec!["One".into(), "Two".into()])
          }),
          int("selected", "Selected", Emit::Custom, || PropValue::Int(-1)),
          enums("layout", "Layout", Emit::Method("layout"), "Axis",
            &["vertical", "horizontal"], || PropValue::Choice("vertical".into())),
          boolean("disabled", "Disabled", Emit::Method("disabled"), false),
      ],
      events: &[CHANGE_INDEX],
      docs: "Selected is a zero-based index; -1 selects nothing.",
      imports: &["use gpui::Axis;"],
      aliases: &["options", "choice", "one of"],
  ),
  comp!(
      "rating", "Rating", "Rating", Controls, "star",
      "A row of stars.",
      Ctor::Id,
      props: &[
          int("value", "Value", Emit::Method("value"), || PropValue::Int(0)),
          int("max", "Max", Emit::Method("max"), || PropValue::Int(5)),
          boolean("disabled", "Disabled", Emit::Method("disabled"), false),
          SIZE,
      ],
      events: &[CLICK_INDEX],
      aliases: &["stars", "review"],
  ),
];
