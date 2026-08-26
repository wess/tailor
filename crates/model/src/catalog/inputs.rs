//! Form fields.
//!
//! Almost everything here is a gpui entity rather than a builder — a text field
//! owns a focus handle and a buffer, a picker owns its open state — which is
//! why most rows are `Ctor::Entity`. That is not an implementation detail the
//! generator can hide: it is exactly why the generated screen is a `Render`
//! entity with a field per field, the way you would have written it.

use crate::node::{EventSpec, CHANGE_BOOL, CHANGE_INDEX, CHANGE_VALUE};
use crate::props::{
  boolean, color_name, enums, float, int, items, size, text, Emit, PropSpec, PropValue,
};
use crate::tokens::{ColorToken, SizeToken};

use super::spec::{ComponentSpec, Ctor, CHILDREN};

const LABEL: PropSpec = text("label", "Label", Emit::Method("label"));
const DESCRIPTION: PropSpec = text("description", "Description", Emit::Method("description"));
const ERROR: PropSpec = text("error", "Error", Emit::Method("error"));
const PLACEHOLDER: PropSpec = text("placeholder", "Placeholder", Emit::Method("placeholder"));
const DISABLED: PropSpec = boolean("disabled", "Disabled", Emit::Method("disabled"), false);
const FIELD_SIZE: PropSpec = size("size", "Size", Emit::Method("size"), SizeToken::Md);

fn options() -> PropValue {
  PropValue::Items(vec!["First".into(), "Second".into(), "Third".into()])
}

const CHANGES: &[EventSpec] = &[CHANGE_BOOL];
const INDEX_CHANGES: &[EventSpec] = &[CHANGE_INDEX];
const VALUE_CHANGES: &[EventSpec] = &[CHANGE_VALUE];

pub static SPECS: &[ComponentSpec] = &[
  comp!(
      "textinput", "Text field", "TextInput", Inputs, "text-cursor-input",
      "A single-line field with a real caret, IME, and selection.",
      Ctor::Entity,
      props: &[
          text("value", "Value", Emit::Method("value")),
          PLACEHOLDER, LABEL, DESCRIPTION, ERROR, FIELD_SIZE,
          size("radius", "Radius", Emit::Method("radius"), SizeToken::Md),
          DISABLED,
          boolean("read_only", "Read only", Emit::Method("read_only"), false),
          boolean("password", "Mask input", Emit::Method("password"), false),
          int("max_length", "Max length", Emit::Method("max_length"), || PropValue::Int(0)),
      ],
  ),
  comp!(
      "textarea", "Text area", "TextArea", Inputs, "text-quote",
      "A multi-line field that grows with its content.",
      Ctor::Entity,
      props: &[
          text("value", "Value", Emit::Method("value")),
          PLACEHOLDER, LABEL, DESCRIPTION, ERROR,
          int("rows", "Rows", Emit::Method("rows"), || PropValue::Int(3)),
          int("max_rows", "Max rows", Emit::Method("max_rows"), || PropValue::Int(0)),
          boolean("submit_on_enter", "Submit on enter", Emit::Method("submit_on_enter"), false),
          FIELD_SIZE, DISABLED,
      ],
  ),
  comp!(
      "numberinput", "Number field", "NumberInput", Inputs, "hash",
      "A numeric field with steppers.",
      Ctor::Entity,
      props: &[
          float("value", "Value", Emit::Method("value"), || PropValue::Float(0.0)),
          float("min", "Min", Emit::Method("min"), || PropValue::Float(0.0)),
          float("max", "Max", Emit::Method("max"), || PropValue::Float(100.0)),
          float("step", "Step", Emit::Method("step"), || PropValue::Float(1.0)),
          LABEL, DESCRIPTION, ERROR, FIELD_SIZE, DISABLED,
      ],
  ),
  comp!(
      "passwordinput", "Password field", "PasswordInput", Inputs, "key-round",
      "A masked field with a reveal toggle.",
      Ctor::Entity,
      props: &[
          PLACEHOLDER, LABEL, DESCRIPTION, ERROR, FIELD_SIZE, DISABLED,
          boolean("visible", "Revealed", Emit::Method("visible"), false),
          boolean("read_only", "Read only", Emit::Method("read_only"), false),
          int("max_length", "Max length", Emit::Method("max_length"), || PropValue::Int(0)),
      ],
  ),
  comp!(
      "pininput", "PIN field", "PinInput", Inputs, "lock",
      "A fixed-length code entry.",
      Ctor::Entity,
      props: &[
          text("value", "Value", Emit::Method("value")),
          int("length", "Length", Emit::Method("length"), || PropValue::Int(6)),
          boolean("mask", "Mask", Emit::Method("mask"), false),
          FIELD_SIZE, DISABLED,
      ],
  ),
  comp!(
      "select", "Select", "Select", Inputs, "chevron-down",
      "A dropdown over a fixed list.",
      Ctor::Entity,
      props: &[
          items("data", "Options", Emit::Method("data"), options),
          int("selected", "Selected", Emit::Method("selected"), || PropValue::Int(0)),
          PLACEHOLDER, LABEL, FIELD_SIZE, DISABLED,
      ],
      events: INDEX_CHANGES,
  ),
  comp!(
      "combobox", "Combobox", "Combobox", Inputs, "list-filter",
      "A searchable select, single or multiple.",
      Ctor::Entity,
      props: &[
          items("data", "Options", Emit::Method("data"), options),
          boolean("multiple", "Multiple", Emit::Method("multiple"), false),
          PLACEHOLDER, LABEL, FIELD_SIZE, DISABLED,
      ],
  ),
  comp!(
      "autocomplete", "Autocomplete", "Autocomplete", Inputs, "search",
      "A text field that suggests as you type.",
      Ctor::Entity,
      props: &[
          items("suggestions", "Suggestions", Emit::Method("suggestions"), options),
          text("value", "Value", Emit::Method("value")),
          int("max_shown", "Max shown", Emit::Method("max_shown"), || PropValue::Int(6)),
          PLACEHOLDER, LABEL, FIELD_SIZE, DISABLED,
      ],
  ),
  comp!(
      "checkbox", "Checkbox", "Checkbox", Inputs, "square-check",
      "A controlled checkbox — the parent owns the value.",
      Ctor::Id,
      props: &[
          boolean("checked", "Checked", Emit::Method("checked"), false),
          boolean("indeterminate", "Indeterminate", Emit::Method("indeterminate"), false),
          LABEL, FIELD_SIZE,
          color_name("color", "Color", Emit::Method("color"), ColorToken::Blue),
          DISABLED,
      ],
      events: CHANGES,
  ),
  comp!(
      "checkboxgroup", "Checkbox group", "CheckboxGroup", Inputs, "list-checks",
      "A labelled set of checkboxes over one list.",
      Ctor::Unit,
      props: &[
          items("options", "Options", Emit::Method("options"), options),
          LABEL,
          color_name("color", "Color", Emit::Method("color"), ColorToken::Blue),
          FIELD_SIZE,
      ],
  ),
  comp!(
      "switch", "Switch", "Switch", Inputs, "toggle-left",
      "A controlled on/off toggle.",
      Ctor::Id,
      props: &[
          boolean("checked", "On", Emit::Method("checked"), false),
          LABEL, FIELD_SIZE,
          color_name("color", "Color", Emit::Method("color"), ColorToken::Blue),
          DISABLED,
      ],
      events: CHANGES,
  ),
  comp!(
      "radio", "Radio", "Radio", Inputs, "circle-dot",
      "One radio button.",
      Ctor::Id,
      props: &[
          boolean("checked", "Checked", Emit::Method("checked"), false),
          LABEL, FIELD_SIZE,
          color_name("color", "Color", Emit::Method("color"), ColorToken::Blue),
          DISABLED,
      ],
      events: CHANGES,
  ),
  comp!(
      "radiogroup", "Radio group", "RadioGroup", Inputs, "circle-check-big",
      "A labelled set of radios over one list.",
      Ctor::Unit,
      props: &[
          items("options", "Options", Emit::Method("options"), options),
          LABEL,
          color_name("color", "Color", Emit::Method("color"), ColorToken::Blue),
          FIELD_SIZE,
      ],
      events: INDEX_CHANGES,
  ),
  comp!(
      "segmented", "Segmented control", "SegmentedControl", Inputs, "rows-2",
      "A row of mutually exclusive segments.",
      Ctor::Entity,
      props: &[
          items("data", "Segments", Emit::Method("data"), options),
          int("selected", "Selected", Emit::Method("selected"), || PropValue::Int(0)),
          FIELD_SIZE,
      ],
      events: INDEX_CHANGES,
  ),
  comp!(
      "slider", "Slider", "Slider", Inputs, "sliders-horizontal",
      "A single-value slider.",
      Ctor::Entity,
      props: &[
          float("min", "Min", Emit::Method("min"), || PropValue::Float(0.0)),
          float("max", "Max", Emit::Method("max"), || PropValue::Float(100.0)),
          float("step", "Step", Emit::Method("step"), || PropValue::Float(1.0)),
          color_name("color", "Color", Emit::Method("color"), ColorToken::Blue),
          DISABLED,
      ],
      events: VALUE_CHANGES,
  ),
  comp!(
      "rangeslider", "Range slider", "RangeSlider", Inputs, "sliders",
      "A two-handle range.",
      Ctor::Entity,
      props: &[
          float("min", "Min", Emit::Method("min"), || PropValue::Float(0.0)),
          float("max", "Max", Emit::Method("max"), || PropValue::Float(100.0)),
          float("step", "Step", Emit::Method("step"), || PropValue::Float(1.0)),
          float("min_gap", "Min gap", Emit::Method("min_gap"), || PropValue::Float(0.0)),
          color_name("color", "Color", Emit::Method("color"), ColorToken::Blue),
          FIELD_SIZE, DISABLED,
      ],
  ),
  comp!(
      "colorinput", "Color field", "ColorInput", Inputs, "palette",
      "A hex field with a swatch and a picker.",
      Ctor::Entity,
      props: &[LABEL, DESCRIPTION, ERROR, FIELD_SIZE, DISABLED],
  ),
  comp!(
      "tagsinput", "Tags field", "TagsInput", Inputs, "tags",
      "Free-text tags with a query field.",
      Ctor::Entity,
      props: &[
          items("tags", "Tags", Emit::Method("tags"), || PropValue::Items(Vec::new())),
          PLACEHOLDER, LABEL, DESCRIPTION, ERROR,
          int("max_tags", "Max tags", Emit::Method("max_tags"), || PropValue::Int(0)),
          FIELD_SIZE, DISABLED,
      ],
  ),
  comp!(
      "datepicker", "Date picker", "DatePicker", Inputs, "calendar",
      "A date field with a calendar popover.",
      Ctor::Entity,
      props: &[
          boolean("range_mode", "Range", Emit::Flag("range_mode"), false),
          text("format", "Format", Emit::Method("format")),
          PLACEHOLDER, LABEL, FIELD_SIZE, DISABLED,
      ],
  ),
  comp!(
      "timepicker", "Time picker", "TimePicker", Inputs, "clock",
      "A time field with hour and minute columns.",
      Ctor::Entity,
      props: &[
          boolean("twenty_four_hour", "24 hour", Emit::Flag("twenty_four_hour"), false),
          int("minute_step", "Minute step", Emit::Method("minute_step"), || PropValue::Int(5)),
          PLACEHOLDER, LABEL, FIELD_SIZE, DISABLED,
      ],
  ),
  comp!(
      "calendar", "Calendar", "Calendar", Inputs, "calendar-days",
      "An inline month grid.",
      Ctor::Id,
      props: &[
          int("year", "Year", Emit::None, || PropValue::Int(2026)),
          int("month", "Month", Emit::None, || PropValue::Int(1)),
          FIELD_SIZE,
      ],
  ),
  comp!(
      "fileinput", "File field", "FileInput", Inputs, "file-input",
      "A picker that opens the native file dialog.",
      Ctor::Entity,
      props: &[
          boolean("multiple", "Multiple", Emit::Flag("multiple"), false),
          boolean("directories", "Directories", Emit::Flag("directories"), false),
          items("accept", "Accept", Emit::Method("accept"), || PropValue::Items(Vec::new())),
          PLACEHOLDER, LABEL, FIELD_SIZE, DISABLED,
      ],
  ),
  comp!(
      "dropzone", "Dropzone", "Dropzone", Inputs, "upload",
      "A drop target for files.",
      Ctor::Id,
      props: &[
          LABEL,
          text("hint", "Hint", Emit::Method("hint")),
          crate::props::icon("icon", "Icon", Emit::Method("icon")),
          items("accept", "Accept", Emit::Method("accept"), || PropValue::Items(Vec::new())),
          boolean("single", "Single file", Emit::Flag("single"), false),
          float("height", "Height", Emit::Method("height"), || PropValue::Float(140.0)),
      ],
  ),
  comp!(
      "transfer", "Transfer", "Transfer", Inputs, "arrow-left-right",
      "Two lists you move items between.",
      Ctor::Entity,
      props: &[
          items("data", "Items", Emit::Method("data"), options),
          float("height", "Height", Emit::Method("height"), || PropValue::Float(240.0)),
          DISABLED,
      ],
  ),
  comp!(
      "field", "Field wrapper", "Field", Inputs, "square-pen",
      "Label, description, and error around any control.",
      Ctor::Unit,
      props: &[LABEL, DESCRIPTION, ERROR],
      slots: &[CHILDREN],
  ),
  comp!(
      "editor", "Code editor", "Editor", Inputs, "file-code",
      "A syntax-highlighted code buffer.",
      Ctor::Entity,
      props: &[
          crate::props::multiline("value", "Content", Emit::None),
          enums("language", "Language", Emit::None, "Language",
              &["none", "rust", "sql", "json", "toml", "python", "javascript", "typescript", "go", "c", "markdown"],
              || PropValue::Choice("rust".into())),
          float("height", "Height", Emit::None, || PropValue::Float(240.0)),
      ],
  ),
  comp!(
      "markdowneditor", "Markdown editor", "MarkdownEditor", Inputs, "pencil-line",
      "A live-preview markdown buffer.",
      Ctor::Entity,
      props: &[
          crate::props::multiline("value", "Content", Emit::Method("value")),
          PLACEHOLDER,
          int("rows", "Rows", Emit::Method("rows"), || PropValue::Int(10)),
          boolean("read_only", "Read only", Emit::Method("read_only"), false),
      ],
  ),
];
