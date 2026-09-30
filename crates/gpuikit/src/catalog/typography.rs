//! Text, keys, and the small labels that sit beside other things.

use tailor_model::catalog::{ComponentSpec, Ctor, CHILDREN};
use tailor_model::comp;
use tailor_model::node::CLICK;
use tailor_model::props::{boolean, enums, hinted, int, text, Emit, PropValue};

use super::SIZE;

pub static SPECS: &[ComponentSpec] = &[
  comp!(
      "label", "Label", "Label", Typography, "type",
      "A run of text, optionally with a quieter second line.",
      Ctor::Arg("text"),
      props: &[
          text("text", "Text", Emit::None),
          text("secondary", "Secondary", Emit::Method("secondary")),
          boolean("masked", "Masked", Emit::Method("masked"), false),
      ],
      required: &[("text", "Type into Text.")],
      aliases: &["text", "heading", "paragraph"],
  ),
  comp!(
      "kbd", "Keystroke", "Kbd", Typography, "keyboard",
      "A keycap, written as a gpui keystroke like `cmd-k`.",
      Ctor::Special,
      props: &[
          hinted(text("keys", "Keys", Emit::Custom), "gpui syntax: cmd-shift-p, ctrl-c."),
          boolean("outline", "Outline", Emit::Flag("outline"), false),
      ],
      required: &[("keys", "Type a keystroke such as cmd-k.")],
      aliases: &["shortcut", "keycap", "hotkey"],
  ),
  comp!(
      "link", "Link", "Link", Typography, "link",
      "Text that opens a URL or runs a handler.",
      Ctor::Special,
      props: &[
          text("label", "Label", Emit::Custom),
          text("href", "URL", Emit::Method("href")),
          boolean("disabled", "Disabled", Emit::Method("disabled"), false),
      ],
      events: &[CLICK],
      required: &[("label", "Type the link text into Label.")],
      aliases: &["anchor", "hyperlink"],
  ),
  comp!(
      "tag", "Tag", "Tag", Typography, "tag",
      "A small coloured label.",
      Ctor::Special,
      props: &[
          text("label", "Label", Emit::Custom),
          enums("variant", "Variant", Emit::Custom, "",
            &["default", "primary", "secondary", "danger", "success", "warning", "info"],
            || PropValue::Choice("default".into())),
          boolean("outline", "Outline", Emit::Flag("outline"), false),
          boolean("rounded_full", "Pill", Emit::Flag("rounded_full"), false),
          SIZE,
      ],
      required: &[("label", "Type the tag text into Label.")],
      aliases: &["chip", "pill"],
  ),
  comp!(
      "badge", "Badge", "Badge", Typography, "circle-dot",
      "A count or dot pinned to the corner of whatever it wraps.",
      Ctor::Unit,
      props: &[
          int("count", "Count", Emit::Method("count"), || PropValue::Int(0)),
          int("max", "Max", Emit::Method("max"), || PropValue::Int(99)),
          boolean("dot", "Dot", Emit::Flag("dot"), false),
          SIZE,
      ],
      slots: &[CHILDREN],
      docs: "Wraps one element — put an Icon or Avatar inside. A zero count and \
             no dot draws nothing.",
      aliases: &["notification count", "counter"],
  ),
];
