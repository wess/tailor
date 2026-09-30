//! Loading states, empty states, and inline messages.

use tailor_model::catalog::{ComponentSpec, Ctor, CHILDREN};
use tailor_model::comp;
use tailor_model::props::{boolean, enums, icon, text, Emit, PropValue};

use super::{CLOSE_CLICK, SIZE};

pub static SPECS: &[ComponentSpec] = &[
  comp!(
      "alert", "Alert", "Alert", Feedback, "circle-alert",
      "An inline message with a tone.",
      Ctor::IdAnd("message"),
      props: &[
          text("message", "Message", Emit::None),
          text("title", "Title", Emit::Method("title")),
          enums("variant", "Tone", Emit::Method("with_variant"), "AlertVariant",
            &["default", "info", "success", "warning", "error"],
            || PropValue::Choice("default".into())),
          icon("icon", "Icon", Emit::Method("icon")),
          boolean("banner", "Banner", Emit::Flag("banner"), false),
          SIZE,
      ],
      events: &[CLOSE_CLICK],
      required: &[("message", "Type the message into Message.")],
      aliases: &["callout", "notice", "banner"],
  ),
  comp!(
      "spinner", "Spinner", "Spinner", Feedback, "loader-circle",
      "An indeterminate loading indicator.",
      Ctor::Unit,
      props: &[SIZE],
      aliases: &["loading", "busy"],
  ),
  comp!(
      "skeleton", "Skeleton", "Skeleton", Feedback, "rectangle-horizontal",
      "A grey placeholder shaped like what is loading.",
      Ctor::Unit,
      props: &[boolean("secondary", "Secondary", Emit::Flag("secondary"), false)],
      docs: "Give it a width and height in Layout; it has no size of its own.",
      aliases: &["placeholder", "shimmer"],
  ),
  comp!(
      "empty", "Empty state", "Empty", Feedback, "inbox",
      "A title and description for a screen with nothing on it yet.",
      Ctor::Special,
      props: &[
          icon("icon", "Icon", Emit::Custom),
          text("title", "Title", Emit::Custom),
          text("description", "Description", Emit::Custom),
      ],
      slots: &[CHILDREN],
      docs: "Children follow the text, so put the call-to-action Button inside.",
      aliases: &["no data", "zero state", "blank slate"],
  ),
];
