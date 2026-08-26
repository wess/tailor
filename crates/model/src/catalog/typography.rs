//! Text components.
//!
//! `Text` and `Title` take guise's `Color` rather than a `ColorName`, so their
//! colour props are declared with `color_value` — the generator resolves a
//! palette family through the theme and an explicit colour through `Color::hex`.

use crate::props::{
  boolean, color_name, color_value, enums, int, multiline, size, text, Emit, PropValue,
};
use crate::tokens::{ColorToken, SizeToken};

use super::spec::{ComponentSpec, Ctor, CHILDREN};

const WEIGHTS: &[&str] = &["normal", "medium", "semibold", "bold"];

pub static SPECS: &[ComponentSpec] = &[
  comp!(
      "text", "Text", "Text", Typography, "type",
      "A run of themed body text.",
      Ctor::Arg("content"),
      props: &[
          multiline("content", "Content", Emit::None),
          size("size", "Size", Emit::Method("size"), SizeToken::Md),
          enums("weight", "Weight", Emit::Method("weight"), "FontWeight", WEIGHTS, || {
              PropValue::Choice("normal".into())
          }),
          color_value("color", "Color", Emit::Method("color"), ColorToken::Gray),
          boolean("dimmed", "Dimmed", Emit::Flag("dimmed"), false),
      ],
  ),
  comp!(
      "title", "Title", "Title", Typography, "heading",
      "A heading, h1 through h6 by order.",
      Ctor::Arg("content"),
      props: &[
          text("content", "Content", Emit::None),
          int("order", "Order", Emit::Method("order"), || PropValue::Int(2)),
          color_value("color", "Color", Emit::Method("color"), ColorToken::Gray),
      ],
  ),
  comp!(
      "anchor", "Link", "Anchor", Typography, "link",
      "A text link.",
      Ctor::IdAnd("label"),
      props: &[
          text("label", "Label", Emit::None),
          color_name("color", "Color", Emit::Method("color"), ColorToken::Blue),
          size("size", "Size", Emit::Method("size"), SizeToken::Md),
          text("href", "URL", Emit::None),
      ],
      events: &[crate::node::CLICK],
  ),
  comp!(
      "code", "Inline code", "Code", Typography, "code",
      "Monospace inline code.",
      Ctor::Arg("content"),
      props: &[
          text("content", "Content", Emit::None),
          color_name("color", "Color", Emit::Method("color"), ColorToken::Gray),
      ],
  ),
  comp!(
      "kbd", "Keyboard key", "Kbd", Typography, "keyboard",
      "A key cap.",
      Ctor::Arg("key"),
      props: &[text("key", "Key", Emit::None)],
  ),
  comp!(
      "mark", "Highlight", "Mark", Typography, "highlighter",
      "Highlighted text.",
      Ctor::Arg("content"),
      props: &[
          text("content", "Content", Emit::None),
          color_name("color", "Color", Emit::Method("color"), ColorToken::Yellow),
          size("size", "Size", Emit::Method("size"), SizeToken::Md),
      ],
  ),
  comp!(
      "blockquote", "Blockquote", "Blockquote", Typography, "quote",
      "A pull quote with an optional citation.",
      Ctor::Unit,
      props: &[
          multiline("text", "Text", Emit::Method("text")),
          text("cite", "Citation", Emit::Method("cite")),
          color_name("color", "Color", Emit::Method("color"), ColorToken::Gray),
          crate::props::icon("icon", "Icon", Emit::Method("icon")),
          size("padding", "Padding", Emit::Method("padding"), SizeToken::Md),
          size("radius", "Radius", Emit::Method("radius"), SizeToken::Md),
      ],
  ),
  comp!(
      "markdown", "Markdown", "Markdown", Typography, "file-text",
      "Rendered markdown — headings, lists, code, links.",
      Ctor::Arg("source"),
      props: &[
          multiline("source", "Source", Emit::None),
          size("size", "Size", Emit::Method("size"), SizeToken::Md),
          color_name("accent", "Accent", Emit::Method("accent"), ColorToken::Blue),
      ],
  ),
  comp!(
      "spoiler", "Spoiler", "Spoiler", Typography, "eye-off",
      "Content collapsed behind a show/hide toggle.",
      Ctor::Id,
      props: &[
          crate::props::float("max_height", "Collapsed height", Emit::Method("max_height"), || {
              PropValue::Float(80.0)
          }),
          boolean("expanded", "Expanded", Emit::Method("expanded"), false),
          text("show_label", "Show label", Emit::Method("show_label")),
          text("hide_label", "Hide label", Emit::Method("hide_label")),
          color_name("color", "Color", Emit::Method("color"), ColorToken::Blue),
          size("size", "Size", Emit::Method("size"), SizeToken::Sm),
      ],
      slots: &[CHILDREN],
      events: &[crate::node::TOGGLE],
  ),
];
