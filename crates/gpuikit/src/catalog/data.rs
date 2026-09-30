//! Things that show a value rather than take one.

use tailor_model::catalog::{ComponentSpec, Ctor, CHILDREN};
use tailor_model::comp;
use tailor_model::props::{boolean, float, icon, int, text, Emit, PropValue};

use super::SIZE;

pub static SPECS: &[ComponentSpec] = &[
  comp!(
      "icon", "Icon", "Icon", Media, "shapes",
      "One of gpui-kit's bundled icons.",
      Ctor::Special,
      props: &[icon("icon", "Icon", Emit::None), SIZE],
      required: &[("icon", "Pick one from the icon picker.")],
      aliases: &["glyph", "symbol"],
  ),
  comp!(
      "avatar", "Avatar", "Avatar", Data, "circle-user",
      "A person's initials or picture.",
      Ctor::Unit,
      props: &[text("name", "Name", Emit::Method("name")), SIZE],
      docs: "With no image it draws the initials of Name.",
      aliases: &["profile picture", "user"],
  ),
  comp!(
      "avatargroup", "Avatar group", "AvatarGroup", Data, "users",
      "Avatars overlapped into a stack.",
      Ctor::Unit,
      props: &[
          int("limit", "Limit", Emit::Method("limit"), || PropValue::Int(3)),
          boolean("ellipsis", "Ellipsis", Emit::Flag("ellipsis"), false),
          SIZE,
      ],
      slots: &[CHILDREN],
      docs: "Children must be Avatars.",
  ),
  comp!(
      "progress", "Progress", "Progress", Data, "loader",
      "A horizontal progress bar.",
      Ctor::Id,
      props: &[
          float("value", "Value", Emit::Method("value"), || PropValue::Float(50.0)),
          boolean("loading", "Indeterminate", Emit::Method("loading"), false),
          SIZE,
      ],
      docs: "Value is a percentage, 0 to 100.",
      aliases: &["bar", "meter"],
  ),
  comp!(
      "progresscircle", "Progress circle", "ProgressCircle", Data, "loader-circle",
      "A circular progress ring.",
      Ctor::Id,
      props: &[
          float("value", "Value", Emit::Method("value"), || PropValue::Float(50.0)),
          boolean("loading", "Indeterminate", Emit::Method("loading"), false),
          SIZE,
      ],
      docs: "Value is a percentage, 0 to 100.",
      aliases: &["ring", "gauge"],
  ),
  comp!(
      "pagination", "Pagination", "Pagination", Data, "ellipsis",
      "Page numbers with previous and next.",
      Ctor::Id,
      props: &[
          int("current", "Current page", Emit::Method("current_page"), || PropValue::Int(1)),
          int("total", "Total pages", Emit::Method("total_pages"), || PropValue::Int(10)),
          int("visible", "Visible pages", Emit::Method("visible_pages"), || PropValue::Int(5)),
          boolean("compact", "Compact", Emit::Flag("compact"), false),
          boolean("disabled", "Disabled", Emit::Method("disabled"), false),
          SIZE,
      ],
      events: &[super::CLICK_INDEX],
      aliases: &["pager", "pages"],
  ),
];
