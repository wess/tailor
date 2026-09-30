//! Controls backed by a state entity: pickers, selects, sliders, number and
//! one-time-code fields, the code editor.
//!
//! The same split as the text fields in `inputs`: the screen keeps the state as
//! a field and each frame builds the element over a borrow of it, so a prop is
//! the state's (`Emit::State`, applied when it is built) or the element's. Most
//! states are built the way an `InputState` is, with the window; the few that
//! need more — a select's items, a slider's range, a code's length — are built
//! by the generator (`crate::stateful`), which is where those props are read.
//!
//! Reading a value is an action's job, as with a text field: `.read(cx)` on
//! the field, then whatever the state calls it (`value()`, `selected_value()`).

use tailor_model::catalog::{slot, ComponentSpec, Ctor, CHILDREN};
use tailor_model::comp;
use tailor_model::node::EventSpec;
use tailor_model::props::{
  boolean, enums, float, hinted, icon, int, items, multiline, text, Emit, PropValue,
};

use super::{INPUT_BLUR, INPUT_CHANGE, INPUT_ENTER, INPUT_EVENTS, INPUT_FOCUS, SIZE};

/// A state's event, matched by pattern. `<_>` on the enum is the annotation
/// the subscription needs and is stripped from the pattern; see
/// `Emitter::emit_subscriptions`.
const fn event(key: &'static str, label: &'static str, pattern: &'static str) -> EventSpec {
  EventSpec {
    key,
    label,
    method: pattern,
    args: &[],
  }
}

const SELECT_EVENTS: &[EventSpec] = &[event("confirm", "On select", "SelectEvent<_>::Confirm(_)")];
const COMBOBOX_EVENTS: &[EventSpec] = &[
  event("change", "On toggle", "ComboboxEvent<_>::Change(_)"),
  event("confirm", "On close", "ComboboxEvent<_>::Confirm(_)"),
];
const SLIDER_EVENTS: &[EventSpec] = &[
  event("change", "While dragging", "SliderEvent::Change(_)"),
  event("release", "On release", "SliderEvent::Release(_)"),
];
const COLOR_EVENTS: &[EventSpec] = &[event("change", "On change", "ColorPickerEvent::Change(_)")];
const DATE_EVENTS: &[EventSpec] = &[event("change", "On change", "DatePickerEvent::Change(_)")];
const CALENDAR_EVENTS: &[EventSpec] = &[event("select", "On select", "CalendarEvent::Selected(_)")];
const TIME_EVENTS: &[EventSpec] = &[event("change", "On change", "TimeFieldEvent::Change(_)")];
const OTP_EVENTS: &[EventSpec] = &[
  event("change", "On change", "OtpEvent::Change"),
  event("complete", "On complete", "OtpEvent::Complete"),
  event("focus", "On focus", "OtpEvent::Focus"),
  event("blur", "On blur", "OtpEvent::Blur"),
];
const NUMBER_EVENTS: &[EventSpec] = &[
  INPUT_CHANGE,
  INPUT_ENTER,
  INPUT_FOCUS,
  INPUT_BLUR,
  event("step", "On step", "NumberInputEvent::Step(_)"),
];

const TIME_PRECISION: &[&str] = &["minute", "second"];
const HOUR_CYCLE: &[&str] = &["h23", "h12"];

pub static SPECS: &[ComponentSpec] = &[
  comp!(
      "numberinput", "Number input", "NumberInput", Inputs, "hash",
      "A number field with step buttons.",
      Ctor::Stateful("InputState"),
      props: &[
          text("value", "Initial value", Emit::State("default_value")),
          hinted(text("min", "Minimum", Emit::Custom), "Blank for no lower bound."),
          hinted(text("max", "Maximum", Emit::Custom), "Blank for no upper bound."),
          hinted(text("step", "Step", Emit::Custom), "What one press of + or − adds. Blank for 1."),
          text("placeholder", "Placeholder", Emit::Method("placeholder")),
          boolean("appearance", "Bordered", Emit::Method("appearance"), true),
          boolean("disabled", "Disabled", Emit::Method("disabled"), false),
          SIZE,
      ],
      events: NUMBER_EVENTS,
      docs: "An `InputState` field on the screen; read it with `self.<field>.read(cx).value()` \
             and parse. Use Slider for a bounded value you drag; use Input for free text.",
      aliases: &["stepper field", "quantity", "amount", "integer", "counter"],
  ),
  comp!(
      "otpinput", "One-time code", "OtpInput", Inputs, "key-round",
      "Boxes for a verification code.",
      Ctor::Stateful("OtpState"),
      props: &[
          int("length", "Digits", Emit::Custom, || PropValue::Int(6)),
          text("value", "Initial value", Emit::State("default_value")),
          boolean("masked", "Masked", Emit::State("masked"), false),
          int("groups", "Groups", Emit::Method("groups"), || PropValue::Int(2)),
          boolean("disabled", "Disabled", Emit::Method("disabled"), false),
          SIZE,
      ],
      events: OTP_EVENTS,
      docs: "An `OtpState` field. The Complete event fires when the last box is filled — the \
             moment to check the code. Read it with `self.<field>.read(cx).value()`.",
      aliases: &["pin", "verification code", "2fa", "sms code", "passcode"],
  ),
  comp!(
      "select", "Select", "Select", Inputs, "chevrons-up-down",
      "Choose one option from a list.",
      Ctor::Stateful("SelectState<SearchableVec<String>>"),
      props: &[
          items("items", "Options", Emit::Custom, || {
            PropValue::Items(vec!["One".into(), "Two".into(), "Three".into()])
          }),
          int("selected", "Selected", Emit::Custom, || PropValue::Int(-1)),
          boolean("searchable", "Searchable", Emit::State("searchable"), false),
          text("placeholder", "Placeholder", Emit::Method("placeholder")),
          text("search_placeholder", "Search placeholder", Emit::Method("search_placeholder")),
          text("title_prefix", "Label prefix", Emit::Method("title_prefix")),
          icon("icon", "Icon", Emit::Method("icon")),
          boolean("cleanable", "Clear button", Emit::Method("cleanable"), false),
          boolean("disabled", "Disabled", Emit::Method("disabled"), false),
          boolean("appearance", "Bordered", Emit::Method("appearance"), true),
          SIZE,
      ],
      events: SELECT_EVENTS,
      docs: "The options are a list of strings, fixed at design time; `Selected` is the index \
             chosen at first (-1 for none). Read the choice with \
             `self.<field>.read(cx).selected_value()`. Use Combobox for several at once, Radio \
             for a handful that should all be visible.",
      aliases: &["dropdown", "picker", "combo", "option list", "menu"],
  ),
  comp!(
      "combobox", "Combobox", "Combobox", Inputs, "list-checks",
      "Choose several options from a list.",
      Ctor::Stateful("ComboboxState<SearchableVec<String>>"),
      props: &[
          items("items", "Options", Emit::Custom, || {
            PropValue::Items(vec!["One".into(), "Two".into(), "Three".into()])
          }),
          boolean("multiple", "Several", Emit::State("multiple"), true),
          boolean("searchable", "Searchable", Emit::State("searchable"), false),
          text("placeholder", "Placeholder", Emit::Method("placeholder")),
          text("search_placeholder", "Search placeholder", Emit::Method("search_placeholder")),
          icon("icon", "Icon", Emit::Method("icon")),
          boolean("cleanable", "Clear button", Emit::Method("cleanable"), false),
          boolean("disabled", "Disabled", Emit::Method("disabled"), false),
          boolean("appearance", "Bordered", Emit::Method("appearance"), true),
          SIZE,
      ],
      events: COMBOBOX_EVENTS,
      docs: "A Select that can hold more than one choice. Read them with \
             `self.<field>.read(cx).selected_values()`.",
      aliases: &["multiselect", "multi select", "tags input", "chooser"],
  ),
  comp!(
      "slider", "Slider", "Slider", Inputs, "sliders-horizontal",
      "Drag to pick a value in a range.",
      Ctor::StatefulCx("SliderState"),
      props: &[
          float("min", "Minimum", Emit::State("min"), || PropValue::Float(0.0)),
          float("max", "Maximum", Emit::State("max"), || PropValue::Float(100.0)),
          float("step", "Step", Emit::State("step"), || PropValue::Float(1.0)),
          float("value", "Initial value", Emit::State("default_value"), || PropValue::Float(0.0)),
          enums("orientation", "Direction", Emit::Custom, "", &["horizontal", "vertical"],
            || PropValue::Choice("horizontal".into())),
          boolean("reverse", "Fill from the end", Emit::Flag("reverse"), false),
          boolean("disabled", "Disabled", Emit::Method("disabled"), false),
      ],
      events: SLIDER_EVENTS,
      docs: "A `SliderState` field. Change fires continuously while dragging, Release once at \
             the end. Read it with `self.<field>.read(cx).value().start()`.",
      aliases: &["range", "scrubber", "volume", "seek"],
  ),
  comp!(
      "colorpicker", "Color picker", "ColorPicker", Inputs, "palette",
      "A swatch that opens a color palette.",
      Ctor::Stateful("ColorPickerState"),
      props: &[
          hinted(text("value", "Initial color", Emit::Custom), "Hex, like #3b82f6."),
          text("label", "Label", Emit::Method("label")),
          icon("icon", "Icon", Emit::Method("icon")),
          SIZE,
      ],
      events: COLOR_EVENTS,
      docs: "A `ColorPickerState` field; read it with `self.<field>.read(cx).value()`, an \
             `Option<Hsla>`.",
      aliases: &["colour", "swatch", "eyedropper", "fill"],
  ),
  comp!(
      "datepicker", "Date picker", "DatePicker", Inputs, "calendar",
      "A date field with a calendar popover.",
      Ctor::Stateful("DatePickerState"),
      props: &[
          hinted(text("date_format", "Format", Emit::State("date_format")),
            "chrono syntax, like %Y-%m-%d. Blank for the default."),
          enums("time", "Time of day", Emit::Custom, "", &["none", "minute", "second"],
            || PropValue::Choice("none".into())),
          enums("hour_cycle", "Hours", Emit::State("hour_cycle"), "HourCycle", HOUR_CYCLE,
            || PropValue::Choice("h23".into())),
          int("months", "Months shown", Emit::State("number_of_months"), || PropValue::Int(1)),
          text("placeholder", "Placeholder", Emit::Method("placeholder")),
          boolean("cleanable", "Clear button", Emit::Method("cleanable"), false),
          boolean("appearance", "Bordered", Emit::Method("appearance"), true),
          boolean("disabled", "Disabled", Emit::Method("disabled"), false),
          SIZE,
      ],
      events: DATE_EVENTS,
      docs: "A `DatePickerState` field. Set Time of day to edit a time as well as the date.",
      aliases: &["date field", "calendar popover", "datetime", "birthday", "due date"],
  ),
  comp!(
      "calendar", "Calendar", "Calendar", Inputs, "calendar-days",
      "A month grid to pick a date from.",
      Ctor::Stateful("CalendarState"),
      props: &[
          int("months", "Months shown", Emit::Method("number_of_months"), || PropValue::Int(1)),
          SIZE,
      ],
      events: CALENDAR_EVENTS,
      docs: "A `CalendarState` field, always visible. Use Date picker for a field that opens \
             one on demand.",
      aliases: &["month view", "date grid", "schedule"],
  ),
  comp!(
      "timefield", "Time field", "TimeField", Inputs, "clock",
      "Edit an hour and minute by keyboard.",
      Ctor::Stateful("TimeFieldState"),
      props: &[
          enums("precision", "Precision", Emit::State("precision"), "TimePrecision", TIME_PRECISION,
            || PropValue::Choice("minute".into())),
          enums("hour_cycle", "Hours", Emit::State("hour_cycle"), "HourCycle", HOUR_CYCLE,
            || PropValue::Choice("h23".into())),
          boolean("invalid", "Show invalid", Emit::Method("invalid"), false),
          boolean("disabled", "Disabled", Emit::Method("disabled"), false),
          SIZE,
      ],
      events: TIME_EVENTS,
      docs: "A `TimeFieldState` field. Invalid only draws the error style; it does not stop \
             an edit.",
      aliases: &["time picker", "clock", "hour", "alarm"],
  ),
  comp!(
      "editor", "Code editor", "Editor", Inputs, "code",
      "A multi-line editor with line numbers.",
      Ctor::Stateful("EditorState"),
      props: &[
          hinted(text("language", "Language", Emit::State("language")),
            "A language name, like rust or json. Highlighting needs gpui-kit's tree-sitter feature."),
          multiline("value", "Initial text", Emit::State("default_value")),
          boolean("line_number", "Line numbers", Emit::State("line_number"), true),
          boolean("auto_close", "Auto-close brackets", Emit::State("auto_close"), true),
          boolean("appearance", "Chrome", Emit::Method("appearance"), true),
          boolean("bordered", "Bordered", Emit::Method("bordered"), true),
          boolean("disabled", "Disabled", Emit::Method("disabled"), false),
          boolean("readonly", "Read only", Emit::Method("readonly"), false),
      ],
      events: INPUT_EVENTS,
      docs: "An `EditorState` field. Read it with `self.<field>.read(cx).value()`. Use \
             Textarea for prose; this is for code.",
      aliases: &["code", "source", "monaco", "json editor", "script"],
  ),
  comp!(
      "clipboard", "Copy button", "Clipboard", Controls, "clipboard",
      "A button that copies a value.",
      Ctor::Id,
      props: &[
          text("value", "Value to copy", Emit::Method("value")),
          text("tooltip", "Tooltip", Emit::Method("tooltip")),
          SIZE,
      ],
      required: &[("value", "Set the text the button copies.")],
      docs: "Copies a fixed string. The value is fixed at design time; wire a Button to an \
             action when it has to be computed.",
      aliases: &["copy", "copy to clipboard"],
  ),
  comp!(
      "form", "Form", "Form", Layout, "form-input",
      "Labelled fields in a column or a grid.",
      Ctor::Special,
      props: &[
          enums("layout", "Layout", Emit::Custom, "", &["vertical", "horizontal"],
            || PropValue::Choice("vertical".into())),
          int("columns", "Columns", Emit::Method("columns"), || PropValue::Int(1)),
          SIZE,
      ],
      slots: &[CHILDREN, slot("footer", "Footer", "footer")],
      docs: "Children must be Field: a Form lays out labelled rows and rejects anything else. \
             Put the input inside the Field.",
      aliases: &["fields", "settings form", "sign in", "profile"],
  ),
  comp!(
      "field", "Field", "Field", Layout, "rectangle-horizontal",
      "One labelled row of a Form.",
      Ctor::Unit,
      props: &[
          text("label", "Label", Emit::Method("label")),
          text("description", "Description", Emit::Method("description")),
          boolean("required", "Required", Emit::Method("required"), false),
      ],
      slots: &[CHILDREN],
      docs: "Goes inside a Form. Holds the control — an Input, a Select — under a label.",
      aliases: &["form row", "labelled input", "form group"],
  ),
];
