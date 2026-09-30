//! gpui-kit, described so Tailor can target it.
//!
//! gpui-kit is Longbridge's component library for gpui — a styled set built on
//! an unstyled base, shipped as one facade crate. This crate is everything
//! Tailor needs to know about it that does not need a window: the components
//! and their props ([`catalog`]), the Rust each one generates ([`codegen`]),
//! and the type names a document must not shadow.
//!
//! **There is no renderer here, and that is not an omission.** gpui-kit is
//! built on `gpui-pre`, a snapshot of Zed's gpui that is a different crate from
//! the `gpui 0.2` Tailor draws with. Two gpuis in one binary means two sets of
//! `Element` and `Render` types that do not unify, so a gpui-kit component
//! cannot be handed to Tailor's canvas. The catalog and generator are
//! unaffected — generated code compiles against gpui-kit in the user's own
//! crate — and the canvas draws a placeholder for what it cannot build.
//!
//! Exports depend on `gpui-kit` alone, renamed to `gpui`: the facade
//! re-exports gpui and itself as `gpui`, so `gpui::prelude`, `gpui::div` and
//! `gpui::component::button::Button` all resolve against the one snapshot the
//! library was built with.

pub mod catalog;
pub mod codegen;
pub mod compounds;
pub mod extras;
pub mod overlays;
pub mod stateful;

use tailor_model::catalog::ComponentSpec;
use tailor_model::library::{Library, StateStyle, TokenPaths};
use tailor_model::project::ThemePreset;

/// The library handle. Zero-sized: everything it answers is `static`.
pub struct Gpuikit;

pub fn library() -> &'static dyn Library {
  &Gpuikit
}

/// Make gpui-kit available to open documents and export them. Idempotent.
pub fn register() {
  tailor_model::library::register(library());
  tailor_codegen::register(&codegen::GpuikitGenerator);
}

impl Library for Gpuikit {
  fn id(&self) -> &'static str {
    "gpuikit"
  }

  fn label(&self) -> &'static str {
    "gpui-kit"
  }

  fn krate(&self) -> &'static str {
    "gpui-kit"
  }

  fn version_req(&self) -> &'static str {
    // `libraries/gpuikit.surface` is generated from the exact version the
    // surface tool pins; `the_surface_matches_the_requirement` holds this to it.
    "0.7"
  }

  fn dependencies(&self) -> Vec<String> {
    // The one dependency, under the name generated code already uses for gpui.
    vec!["gpui = { package = \"gpui-kit\", version = \"0.7\" }".into()]
  }

  fn prelude(&self) -> &'static [&'static str] {
    &[
      "use gpui::prelude::*;",
      // Tables, lists and the multi-part containers, named for the same reason.
      "#[allow(unused_imports)] use gpui::component::{accordion::Accordion, description_list::DescriptionList, status_bar::StatusBar, toolbar::{Toolbar, ToolbarGroup}};",
      "#[allow(unused_imports)] use gpui::component::table::{Column, DataTable, TableDelegate, TableEvent, TableState};",
      "#[allow(unused_imports)] use gpui::component::list::{List, ListDelegate, ListEvent, ListItem, ListState};",
      "#[allow(unused_imports)] use gpui::component::tree::{Tree, TreeItem, TreeState};",
      "#[allow(unused_imports)] use gpui::component::carousel::{Carousel, CarouselContent, CarouselEvent, CarouselItem, CarouselNext, CarouselPagination, CarouselPaginationItem, CarouselPrevious, CarouselState};",
      "#[allow(unused_imports)] use gpui::component::sidebar::{Sidebar, SidebarCollapsible, SidebarGroup, SidebarMenu, SidebarMenuItem, SidebarToggleButton};",
      "#[allow(unused_imports)] use gpui::component::setting::{SettingGroup, SettingItem, SettingPage, Settings};",
      "#[allow(unused_imports)] use gpui::component::marker::{Marker, MarkerAlignment, MarkerContent, MarkerIcon, MarkerLoadingStyle, MarkerVariant};",
      "#[allow(unused_imports)] use gpui::component::message::{Message, MessageAlignment, MessageContent, MessageFooter, MessageGroup, MessageHeader};",
      "#[allow(unused_imports)] use gpui::component::message_scroller::{MessageScroller, MessageScrollerState};",
      "#[allow(unused_imports)] use gpui::component::bubble::{Bubble, BubbleGroup, BubbleReactionSide, BubbleReactions, BubbleVariant};",
      "#[allow(unused_imports)] use gpui::component::attachment::{Attachment, AttachmentActions, AttachmentContent, AttachmentDescription, AttachmentGroup, AttachmentMedia, AttachmentStatus, AttachmentTitle};",
      "#[allow(unused_imports)] use gpui::component::questionnaire::{Questionnaire, QuestionnaireActions, QuestionnaireChoice, QuestionnaireChoiceDefinition, QuestionnaireChoices, QuestionnaireError, QuestionnaireEvent, QuestionnaireInput, QuestionnaireInputDefinition, QuestionnaireItem, QuestionnaireItemDefinition, QuestionnaireNext, QuestionnairePrevious, QuestionnaireProgress, QuestionnaireSkip, QuestionnaireState, QuestionnaireSubmit, QuestionnaireTitle};",
      "#[allow(unused_imports)] use gpui::component::table::{Table, TableBody, TableCaption, TableCell, TableFooter, TableHead, TableHeader, TableRow};",
      // The root exports icons, `h_flex`/`v_flex`, the theme and the sizing
      // traits; each component lives in its own module. Every line that a small
      // file might not use carries an allow, or a component with one input in it
      // warns about thirty imports.
      "#[allow(unused_imports)] use gpui::component::*;",
      "#[allow(unused_imports)] use gpui::component::{alert::*, avatar::*, badge::*, breadcrumb::*, button::*, \
       checkbox::*, empty::*, group_box::*, kbd::*, label::*, link::*, \
       pagination::*, progress::*, radio::*, rating::*, separator::*, skeleton::*, \
       spinner::*, stepper::*, switch::*, tab::*, tag::*};",
      // Named, because gpui-kit has a `Collapsible` trait at the root as well
      // and two globs that both offer a name make it ambiguous.
      "#[allow(unused_imports)] use gpui::component::collapsible::Collapsible;",
      // A popover's anchor corner is gpui's, not a component's, so nothing
      // above brings it in.
      "#[allow(unused_imports)] use gpui::Anchor;",
      "#[allow(unused_imports)] use gpui::component::input::{InputGroup, InputGroupAddon, InputGroupAddonAlignment, InputGroupButton, InputGroupText};",
      "#[allow(unused_imports)] use gpui::component::command::{Command, CommandGroup, CommandItem, CommandState};",
      "#[allow(unused_imports)] use gpui::component::input::{Input, InputEvent, InputState, Textarea, TextareaState};",
      // The state-backed controls: each is a state, an element, and the events
      // the state emits, all named because the generated code names them.
      "#[allow(unused_imports)] use gpui::component::input::{EditorState, NumberInput, NumberInputEvent, OtpEvent, OtpInput, OtpState, Editor};",
      "#[allow(unused_imports)] use gpui::component::select::{Select, SelectEvent, SelectState};",
      "#[allow(unused_imports)] use gpui::component::combobox::{Combobox, ComboboxEvent, ComboboxState};",
      "#[allow(unused_imports)] use gpui::component::searchable_list::SearchableVec;",
      "#[allow(unused_imports)] use gpui::component::slider::{Slider, SliderEvent, SliderState};",
      "#[allow(unused_imports)] use gpui::component::color_picker::{ColorPicker, ColorPickerEvent, ColorPickerState};",
      "#[allow(unused_imports)] use gpui::component::date_picker::{DatePicker, DatePickerEvent, DatePickerState};",
      "#[allow(unused_imports)] use gpui::component::calendar::{Calendar, CalendarEvent, CalendarState};",
      "#[allow(unused_imports)] use gpui::component::time_field::{HourCycle, TimeField, TimeFieldEvent, TimeFieldState, TimePrecision};",
      "#[allow(unused_imports)] use gpui::component::clipboard::Clipboard;",
      "#[allow(unused_imports)] use gpui::component::form::{Field, Form};",
      // Overlays: named, because `menu::*` and `dialog::*` export a good deal that a
      // small file does not want in scope.
      "#[allow(unused_imports)] use gpui::component::dialog::{AlertDialog, Dialog, DialogAction, DialogClose, DialogDescription, DialogFooter, DialogHeader, DialogTitle};",
      "#[allow(unused_imports)] use gpui::component::{hover_card::HoverCard, popover::Popover, sheet::Sheet, tooltip::Tooltip, WindowExt};",
      "#[allow(unused_imports)] use gpui::component::notification::{Notification, NotificationType};",
      "#[allow(unused_imports)] use gpui::component::menu::{ContextMenuExt, PopupMenuItem};",
      "#[allow(unused_imports)] use gpui::component::button::DropdownButton;",
    ]
  }

  fn components(&self) -> &'static [&'static ComponentSpec] {
    catalog::registry()
  }

  fn presets(&self) -> &'static [ThemePreset] {
    // gpui-kit's themes are JSON sets a registry loads at run time, not
    // constructors a generator can name.
    &[]
  }

  fn token_paths(&self) -> TokenPaths {
    // Sizes are choices here (`Size::Small`), not tokens, so these are only
    // read for props the catalog does not declare — none today.
    TokenPaths {
      size: "Size",
      variant: "ButtonVariant",
      color: "ColorName",
      align: "Align",
      justify: "Justify",
    }
  }

  fn state_style(&self) -> StateStyle {
    // gpui-kit has no signal type. A variable is a plain field on the screen,
    // which an action assigns and follows with `cx.notify()`.
    StateStyle::Field
  }

  fn fallback_icon(&self) -> &'static str {
    // guise's `Circle` is not in gpui-kit's icon set; `Info` is.
    "IconName::Info"
  }

  fn has_icon(&self, name: &str) -> bool {
    compounds::DEFAULT_ICONS.contains(&name)
  }

  fn parents(&self, kind: &str) -> &'static [&'static str] {
    // The parts gpui-kit's generator writes inside a parent's own expression.
    // Anywhere else they have nothing to be written by.
    const MENU: &[&str] = &["dropdownbutton", "contextmenu", "submenu"];
    match kind {
      "menuitem" | "menuseparator" | "menulabel" | "submenu" => MENU,
      "tableheader" | "tablebody" | "tablefooter" | "tablecaption" => &["table"],
      "tablerow" => &["table", "tableheader", "tablebody", "tablefooter"],
      "tablehead" | "tablecell" => &["tablerow"],
      "commandgroup" | "commandseparator" => &["command"],
      "commanditem" => &["command", "commandgroup"],
      "inputgroupaddon" => &["inputgroup"],
      "inputgrouptext" | "inputgroupbutton" => &["inputgroupaddon"],
      "toolbargroup" => &["toolbar"],
      "messageheader" | "messagecontent" | "messagefooter" => &["message"],
      _ => &[],
    }
  }

  fn boxes(&self) -> &'static [&'static str] {
    &["frame", "hstack", "vstack", "spacer"]
  }

  fn reserved(&self) -> &'static [&'static str] {
    // What the two glob imports export besides components.
    &[
      "Anchor",
      "CommandEntry",
      "InputGroupAddonAlignment",
      "InputState",
      "TableRows",
      "ListRows",
      "TextareaState",
      "EditorState",
      "OtpState",
      "SelectState",
      "ComboboxState",
      "SliderState",
      "ColorPickerState",
      "DatePickerState",
      "CalendarState",
      "TimeFieldState",
      "SearchableVec",
      "Theme",
      "ThemeMode",
      "ActiveTheme",
      "Size",
      "Sizable",
      "Disableable",
      "Selectable",
      "Icon",
      "IconName",
      "Root",
      "Window",
      "App",
      "Axis",
      "ButtonVariant",
      "AlertVariant",
      "TabVariant",
      "TagVariant",
      "GroupBoxVariant",
    ]
  }
}
