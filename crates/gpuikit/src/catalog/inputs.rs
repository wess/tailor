//! Text fields: an element over a state entity.
//!
//! gpui-kit's fields are two types. The state (`InputState`) owns the buffer,
//! the cursor and focus, and is built with a `Window`; the element
//! (`Input::new(&state)`) is drawn from it. That is [`Ctor::Stateful`]: the
//! screen keeps the state as a field, and each frame builds the element over a
//! borrow of it. A prop lands on whichever half owns it — a placeholder is the
//! buffer's, `cleanable` is the element's.
//!
//! Reading what was typed is an action's job: `self.<field>.read(cx).value()`.

use tailor_model::catalog::{ComponentSpec, Ctor};
use tailor_model::comp;
use tailor_model::props::{boolean, int, text, Emit, PropValue};

use super::SIZE;

pub static SPECS: &[ComponentSpec] = &[
  comp!(
      "input", "Input", "Input", Inputs, "text-cursor-input",
      "A single-line text field.",
      Ctor::Stateful("InputState"),
      props: &[
          text("placeholder", "Placeholder", Emit::State("placeholder")),
          text("value", "Initial value", Emit::State("default_value")),
          boolean("masked", "Masked", Emit::State("masked"), false),
          boolean("mask_toggle", "Show/hide toggle", Emit::Flag("mask_toggle"), false),
          boolean("cleanable", "Clear button", Emit::Method("cleanable"), false),
          boolean("disabled", "Disabled", Emit::Method("disabled"), false),
          boolean("readonly", "Read only", Emit::Method("readonly"), false),
          boolean("bordered", "Bordered", Emit::Method("bordered"), true),
          SIZE,
      ],
      docs: "The text lives in an `InputState` field on the screen. Read it in an action \
             with `self.<field>.read(cx).value()`. For a password, set Masked and Show/hide \
             toggle. Use Textarea for more than one line.",
      aliases: &["text field", "textbox", "text input", "password", "search"],
  ),
  comp!(
      "textarea", "Textarea", "Textarea", Inputs, "text",
      "A multi-line text field.",
      Ctor::Stateful("TextareaState"),
      props: &[
          text("placeholder", "Placeholder", Emit::State("placeholder")),
          text("value", "Initial value", Emit::State("default_value")),
          int("rows", "Rows", Emit::State("rows"), || PropValue::Int(3)),
          boolean("disabled", "Disabled", Emit::Method("disabled"), false),
          boolean("readonly", "Read only", Emit::Method("readonly"), false),
          boolean("bordered", "Bordered", Emit::Method("bordered"), true),
          SIZE,
      ],
      docs: "The text lives in a `TextareaState` field on the screen. Read it in an action \
             with `self.<field>.read(cx).value()`.",
      aliases: &["multiline", "text area", "notes", "message"],
  ),
];
