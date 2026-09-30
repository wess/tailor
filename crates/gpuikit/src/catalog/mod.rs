//! gpui-kit's catalog: the components Tailor can place, in palette order.
//!
//! A curated slice, not the whole crate. gpui-kit is about two hundred types,
//! most of them the parts of another (`DialogTitle`, `QuestionnaireChoice`) or
//! entities whose state is not a plain field. What is here is what a builder,
//! or a text field's state-and-element pair, can express; `coverage` in the tests says why
//! every other type in `libraries/gpuikit.surface` is not.
//!
//! Nothing here imports gpui-kit — a catalog describes a library, and a
//! description compiles without it.

use tailor_model::catalog::ComponentSpec;

mod compounds;
mod controls;
mod data;
mod extras;
mod feedback;
mod inputs;
mod layout;
mod nav;
mod overlays;
mod stateful;
mod typography;

/// The catalog, flattened once and kept for the life of the process.
pub fn registry() -> &'static [&'static ComponentSpec] {
  use std::sync::OnceLock;
  static REGISTRY: OnceLock<Vec<&'static ComponentSpec>> = OnceLock::new();
  REGISTRY.get_or_init(|| {
    let groups: [&'static [ComponentSpec]; 11] = [
      layout::SPECS,
      typography::SPECS,
      controls::SPECS,
      inputs::SPECS,
      data::SPECS,
      feedback::SPECS,
      nav::SPECS,
      stateful::SPECS,
      overlays::SPECS,
      compounds::SPECS,
      extras::SPECS,
    ];
    groups.into_iter().flatten().collect()
  })
}

use tailor_model::node::EventSpec;
use tailor_model::props::{enums, Emit, PropSpec, PropValue};

/// `.with_size(Size::Small)` — gpui-kit's scale is four steps and a pixel
/// escape hatch, not Tailor's `xs..xl`, so it is a choice rather than a size
/// token. `x-small` because `pascal_case` is what turns it into `XSmall`.
pub(crate) const SIZE: PropSpec = enums(
  "size",
  "Size",
  Emit::Method("with_size"),
  "Size",
  &["x-small", "small", "medium", "large"],
  || PropValue::Choice("medium".into()),
);

// Handlers that are not the shared ones in `tailor_model::node`: gpui-kit's
// list-ish controls hand back the whole selection rather than one value.
pub(crate) const CLICK_BOOL: EventSpec = EventSpec {
  key: "click",
  label: "On click",
  method: "on_click",
  args: &["checked"],
};

pub(crate) const CLICK_INDEX: EventSpec = EventSpec {
  key: "click",
  label: "On click",
  method: "on_click",
  args: &["index"],
};

pub(crate) const CLICK_SELECTION: EventSpec = EventSpec {
  key: "click",
  label: "On click",
  method: "on_click",
  args: &["selected"],
};

/// Alert's close handler is handed the click that dismissed it, unlike guise's
/// parameterless `on_close`.
pub(crate) const CLOSE_CLICK: EventSpec = EventSpec {
  key: "close",
  label: "On close",
  method: "on_close",
  args: &["event"],
};

// A text field's state emits these; the handler is bound to one. The method is
// the pattern the subscription matches, not a builder call — see
// `Emitter::emit_subscriptions`.
pub(crate) const INPUT_CHANGE: EventSpec = EventSpec {
  key: "change",
  label: "On change",
  method: "InputEvent::Change",
  args: &[],
};
pub(crate) const INPUT_ENTER: EventSpec = EventSpec {
  key: "enter",
  label: "On enter",
  method: "InputEvent::PressEnter { .. }",
  args: &[],
};
pub(crate) const INPUT_FOCUS: EventSpec = EventSpec {
  key: "focus",
  label: "On focus",
  method: "InputEvent::Focus",
  args: &[],
};
pub(crate) const INPUT_BLUR: EventSpec = EventSpec {
  key: "blur",
  label: "On blur",
  method: "InputEvent::Blur",
  args: &[],
};
pub(crate) const INPUT_EVENTS: &[EventSpec] = &[INPUT_CHANGE, INPUT_ENTER, INPUT_FOCUS, INPUT_BLUR];
