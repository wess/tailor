//! The boxes a screen is built out of.
//!
//! gpui-kit has no layout components of its own: it lays out with gpui's
//! `div()` and adds `h_flex()`/`v_flex()` shorthands. So most of this table is
//! `Ctor::Special` boxes whose whole behaviour is their style.

use tailor_model::catalog::{slot, ComponentSpec, Ctor, CHILDREN};
use tailor_model::comp;
use tailor_model::node::Node;
use tailor_model::props::{boolean, enums, text, Emit, PropValue};
use tailor_model::style::Edges;

fn frame_defaults(node: &mut Node) {
  node.style.padding = Edges::all(16.0);
  node.style.gap = Some(12.0);
}

pub static SPECS: &[ComponentSpec] = &[
  comp!(
      "frame", "Frame", "div", Layout, "square-dashed",
      "A plain box. Everything about it comes from its style.",
      Ctor::Special,
      slots: &[CHILDREN],
      on_place: Some(frame_defaults),
  ),
  comp!(
      "hstack", "Row", "h_flex", Layout, "columns-3",
      "A box whose children run left to right.",
      Ctor::Special,
      slots: &[CHILDREN],
      on_place: Some(frame_defaults),
      aliases: &["row", "hflex", "horizontal"],
  ),
  comp!(
      "vstack", "Column", "v_flex", Layout, "rows-3",
      "A box whose children run top to bottom.",
      Ctor::Special,
      slots: &[CHILDREN],
      on_place: Some(frame_defaults),
      aliases: &["stack", "vflex", "vertical"],
  ),
  comp!(
    "spacer",
    "Spacer",
    "div",
    Layout,
    "unfold-horizontal",
    "Flexible space that pushes its siblings apart.",
    Ctor::Special,
  ),
  comp!(
      "groupbox", "Group box", "GroupBox", Layout, "square",
      "A titled container that groups related controls.",
      Ctor::Unit,
      props: &[
          text("title", "Title", Emit::Method("title")),
          enums("variant", "Variant", Emit::Method("with_variant"), "GroupBoxVariant",
            &["normal", "fill", "outline"], || PropValue::Choice("normal".into())),
      ],
      slots: &[CHILDREN, slot("footer", "Footer", "footer")],
      aliases: &["fieldset", "section"],
  ),
  comp!(
      "separator", "Separator", "Separator", Layout, "minus",
      "A rule, optionally with a label in the middle.",
      Ctor::Special,
      props: &[
          enums("orientation", "Orientation", Emit::Custom, "",
            &["horizontal", "vertical"], || PropValue::Choice("horizontal".into())),
          text("label", "Label", Emit::Method("label")),
          boolean("dashed", "Dashed", Emit::Flag("dashed"), false),
      ],
      aliases: &["divider", "rule", "hr"],
  ),
  comp!(
      "collapsible", "Collapsible", "Collapsible", Layout, "chevrons-down-up",
      "Content that expands and collapses under a trigger.",
      Ctor::Unit,
      props: &[boolean("open", "Open", Emit::Method("open"), false)],
      slots: &[CHILDREN, slot("content", "Content", "content")],
      docs: "Children are the always-visible trigger; the Content region is what \
             folds away. It does not toggle itself — a host flips `open`.",
      aliases: &["disclosure", "expander"],
  ),
];
