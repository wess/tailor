//! Tables, lists, trees and the multi-part containers.
//!
//! Most of these are a state entity or a stack of parts rather than one
//! builder, so the catalog only says what a designer sets — columns, rows,
//! titles, whether a control shows — and `compounds` in the generator writes the
//! parts (`TableHeader`, `AccordionItem`, `CarouselContent`) around them.
//!
//! Items props are one row per line, with `|` between the fields of a row; each
//! prop says which fields in its hint.

use tailor_model::catalog::{ComponentSpec, Ctor, SlotSpec, CHILDREN};
use tailor_model::comp;
use tailor_model::node::{EventSpec, DEFAULT_SLOT};
use tailor_model::props::{boolean, enums, hinted, icon, int, items, text, Emit, PropValue};
use tailor_model::style::Dimension;
use tailor_model::Node;

use super::SIZE;

/// Children that are not sized controls, which is what `Toolbar::child` insists
/// on: `content` takes anything.
const TOOLBAR_ITEMS: SlotSpec = SlotSpec {
  key: DEFAULT_SLOT,
  label: "Items",
  single: false,
  method: "content",
};

const LEFT: SlotSpec = SlotSpec {
  key: "left",
  label: "Left",
  single: false,
  method: "left",
};

const RIGHT: SlotSpec = SlotSpec {
  key: "right",
  label: "Right",
  single: false,
  method: "right",
};

/// `Accordion::on_toggle_click` hands back the indices of the open items.
const TOGGLE_ITEMS: EventSpec = EventSpec {
  key: "toggle",
  label: "On toggle",
  method: "on_toggle_click",
  args: &["open"],
};

/// A scrolling collection has no height of its own, so a fresh one would
/// collapse to nothing. Give it enough to show a few rows.
fn tall(node: &mut Node) {
  node.style.height = Dimension::Px(240.0);
}

/// A carousel measures its slides against its own width.
fn slide_frame(node: &mut Node) {
  node.style.width = Dimension::Px(360.0);
}

// What a collection's state emits; the handler is bound to one of these, and the
// method is the pattern the subscription matches (see `Emitter::emit_subscriptions`).
const ROW_SELECTED: EventSpec = EventSpec {
  key: "select",
  label: "On row select",
  method: "TableEvent::SelectRow(_)",
  args: &[],
};

const ROW_OPENED: EventSpec = EventSpec {
  key: "open",
  label: "On row double-click",
  method: "TableEvent::DoubleClickedRow(_)",
  args: &[],
};

const ITEM_SELECTED: EventSpec = EventSpec {
  key: "select",
  label: "On select",
  method: "ListEvent::Select(_)",
  args: &[],
};

const ITEM_CONFIRMED: EventSpec = EventSpec {
  key: "confirm",
  label: "On click or enter",
  method: "ListEvent::Confirm(_)",
  args: &[],
};

const SLIDE_CHANGED: EventSpec = EventSpec {
  key: "change",
  label: "On slide change",
  method: "CarouselEvent::Change(_)",
  args: &[],
};

const HEADER: SlotSpec = SlotSpec {
  key: "header",
  label: "Header",
  single: true,
  method: "header",
};

const FOOTER: SlotSpec = SlotSpec {
  key: "footer",
  label: "Footer",
  single: true,
  method: "footer",
};

const AVATAR: SlotSpec = SlotSpec {
  key: "avatar",
  label: "Avatar",
  single: true,
  method: "avatar",
};

const CONTENT: SlotSpec = SlotSpec {
  key: "content",
  label: "Content",
  single: true,
  method: "content",
};

const REACTIONS: SlotSpec = SlotSpec {
  key: "reactions",
  label: "Reactions",
  single: true,
  method: "reactions",
};

const ACTIONS: SlotSpec = SlotSpec {
  key: "actions",
  label: "Actions",
  single: true,
  method: "actions",
};

/// The fixed height a settings screen needs to lay its sidebar and page out.
fn page_frame(node: &mut Node) {
  node.style.height = Dimension::Px(360.0);
}

const SUBMITTED: EventSpec = EventSpec {
  key: "submit",
  label: "On submit",
  method: "QuestionnaireEvent::Submit(_)",
  args: &[],
};

const COMPLETED: EventSpec = EventSpec {
  key: "complete",
  label: "On complete",
  method: "QuestionnaireEvent::Completed(_)",
  args: &[],
};

const ANSWERED: EventSpec = EventSpec {
  key: "answer",
  label: "On answer",
  method: "QuestionnaireEvent::AnswerChanged(_)",
  args: &[],
};

/// `Attachment` takes three click-shaped handlers.
const ATTACHMENT_EVENTS: &[EventSpec] = &[
  EventSpec {
    key: "click",
    label: "On click",
    method: "on_click",
    args: &["_event"],
  },
  EventSpec {
    key: "remove",
    label: "On remove",
    method: "on_remove",
    args: &["_event"],
  },
  EventSpec {
    key: "retry",
    label: "On retry",
    method: "on_retry",
    args: &["_event"],
  },
];

pub static SPECS: &[ComponentSpec] = &[
  // Static tables
  comp!(
      "table", "Table", "Table", Data, "table",
      "A table from a caption, a header row and body rows.",
      Ctor::Unit,
      props: &[
          text("caption", "Caption", Emit::Custom),
          hinted(items("head", "Header", Emit::Custom, || {
            PropValue::Items(vec!["Name".into(), "Role".into(), "Status".into()])
          }), "One entry per column."),
          hinted(items("rows", "Rows", Emit::Custom, || {
            PropValue::Items(vec![
              "Ada Lovelace | Engineer | Active".into(),
              "Grace Hopper | Admiral | Active".into(),
            ])
          }), "One row per line, cells separated by `|`."),
          SIZE,
      ],
      slots: &[CHILDREN],
      docs: "Header and rows come from the props. Drop TableHeader, TableBody, TableFooter \
             or TableCaption inside for a table those props cannot express; children must \
             be table parts, not arbitrary elements. For thousands of rows use Data table.",
      aliases: &["grid", "spreadsheet", "rows and columns"],
  ),
  comp!(
      "tableheader", "Table header", "TableHeader", Data, "table-properties",
      "The header section of a Table, holding a row of heads.",
      Ctor::Unit,
      props: &[SIZE],
      slots: &[CHILDREN],
      docs: "Children are TableRow. Only place it inside a Table.",
  ),
  comp!(
      "tablebody", "Table body", "TableBody", Data, "table-rows-split",
      "The body section of a Table, holding rows.",
      Ctor::Unit,
      props: &[SIZE],
      slots: &[CHILDREN],
      docs: "Children are TableRow. Only place it inside a Table.",
  ),
  comp!(
      "tablefooter", "Table footer", "TableFooter", Data, "table-cells-merge",
      "The footer section of a Table.",
      Ctor::Unit,
      props: &[SIZE],
      slots: &[CHILDREN],
      docs: "Children are TableRow. Only place it inside a Table.",
  ),
  comp!(
      "tablerow", "Table row", "TableRow", Data, "rows-3",
      "A row of heads or cells.",
      Ctor::Unit,
      props: &[SIZE],
      slots: &[CHILDREN],
      docs: "Children are TableHead in a header and TableCell elsewhere.",
  ),
  comp!(
      "tablehead", "Table head", "TableHead", Data, "heading",
      "A header cell.",
      Ctor::Unit,
      props: &[
          text("text", "Text", Emit::Custom),
          int("span", "Column span", Emit::Method("col_span"), || PropValue::Int(1)),
          enums("align", "Align", Emit::Custom, "TextAlign", &["left", "center", "right"],
            || PropValue::Choice("left".into())),
          SIZE,
      ],
      slots: &[CHILDREN],
  ),
  comp!(
      "tablecell", "Table cell", "TableCell", Data, "square",
      "A body cell.",
      Ctor::Unit,
      props: &[
          text("text", "Text", Emit::Custom),
          int("span", "Column span", Emit::Method("col_span"), || PropValue::Int(1)),
          enums("align", "Align", Emit::Custom, "TextAlign", &["left", "center", "right"],
            || PropValue::Choice("left".into())),
          SIZE,
      ],
      slots: &[CHILDREN],
      docs: "Text, or any elements dropped inside.",
  ),
  comp!(
      "tablecaption", "Table caption", "TableCaption", Data, "captions",
      "A caption under a Table.",
      Ctor::Unit,
      props: &[text("text", "Text", Emit::Custom), SIZE],
  ),
  // Accordion
  comp!(
      "accordion", "Accordion", "Accordion", Data, "chevrons-down-up",
      "Sections that fold open one at a time or together.",
      Ctor::Id,
      props: &[
          hinted(items("items", "Sections", Emit::Custom, || {
            PropValue::Items(vec!["First".into(), "Second".into(), "Third".into()])
          }), "One title per section; each gets its own drop region."),
          int("open", "Open section", Emit::Custom, || PropValue::Int(-1)),
          boolean("multiple", "Allow several open", Emit::Method("multiple"), false),
          boolean("bordered", "Bordered", Emit::Method("bordered"), true),
          boolean("disabled", "Disabled", Emit::Method("disabled"), false),
          SIZE,
      ],
      dynamic: Some(tailor_model::catalog::DynamicSlots {
          from_prop: "items",
          prefix: "item",
          method: "",
      }),
      events: &[TOGGLE_ITEMS],
      docs: "Set Open section to the index of the one that starts expanded, -1 for none.",
      aliases: &["collapse", "faq", "expandable sections"],
  ),
  // Toolbars and bars
  comp!(
      "toolbar", "Toolbar", "Toolbar", Navigation, "wrench",
      "A row of controls with arrow-key navigation.",
      Ctor::Id,
      props: &[boolean("disabled", "Disable navigation", Emit::Method("disabled"), false), SIZE],
      slots: &[TOOLBAR_ITEMS],
      docs: "Arrow keys move between the controls. Group related ones in a Toolbar group.",
      aliases: &["tool bar", "action bar", "ribbon"],
  ),
  comp!(
      "toolbargroup", "Toolbar group", "ToolbarGroup", Navigation, "group",
      "A labelled subgroup inside a Toolbar.",
      Ctor::Id,
      props: &[text("label", "Label", Emit::Method("label")), SIZE],
      slots: &[TOOLBAR_ITEMS],
  ),
  comp!(
      "statusbar", "Status bar", "StatusBar", Layout, "panel-bottom",
      "A footer strip with left, centre and right regions.",
      Ctor::Unit,
      slots: &[CHILDREN, LEFT, RIGHT],
      docs: "Children sit in the middle; Left and Right are pinned to their edges.",
      aliases: &["footer", "bottom bar", "info bar"],
  ),
  comp!(
      "descriptionlist", "Description list", "DescriptionList", Data, "list-collapse",
      "Labels and their values, in a grid.",
      Ctor::Unit,
      props: &[
          hinted(items("items", "Items", Emit::Custom, || {
            PropValue::Items(vec![
              "Name | Ada Lovelace".into(),
              "Role | Engineer".into(),
              "Team | Analytical Engines".into(),
            ])
          }), "`Label | Value`, or `Label | Value | span`. A line of `---` is a separator."),
          enums("layout", "Layout", Emit::Custom, "Axis", &["horizontal", "vertical"],
            || PropValue::Choice("horizontal".into())),
          int("columns", "Columns", Emit::Method("columns"), || PropValue::Int(3)),
          boolean("bordered", "Bordered", Emit::Method("bordered"), true),
          SIZE,
      ],
      aliases: &["key value", "details", "properties", "definition list"],
  ),
  // Collections backed by a state entity
  comp!(
      "datatable", "Data table", "DataTable", Data, "table-2",
      "A scrolling table with row selection, sorting and resizable columns.",
      Ctor::Stateful("TableState<TableRows>"),
      props: &[
          hinted(items("columns", "Columns", Emit::Custom, || {
            PropValue::Items(vec!["Name".into(), "Role".into(), "Status".into()])
          }), "One entry per column."),
          hinted(items("rows", "Rows", Emit::Custom, || {
            PropValue::Items(vec![
              "Ada Lovelace | Engineer | Active".into(),
              "Grace Hopper | Admiral | Active".into(),
              "Alan Turing | Mathematician | Away".into(),
            ])
          }), "One row per line, cells separated by `|`."),
          boolean("stripe", "Striped", Emit::Method("stripe"), false),
          boolean("bordered", "Bordered", Emit::Method("bordered"), true),
          boolean("resizable", "Resizable columns", Emit::State("col_resizable"), true),
          boolean("movable", "Movable columns", Emit::State("col_movable"), true),
          boolean("sortable", "Sortable", Emit::State("sortable"), true),
          boolean("selectable", "Selectable rows", Emit::State("row_selectable"), true),
          SIZE,
      ],
      events: &[ROW_SELECTED, ROW_OPENED],
      on_place: Some(tall),
      docs: "Holds its rows as text in a delegate the file defines, so the numbers and names \
             you type are what ships. To feed it live data, edit the generated delegate. \
             For a handful of static rows use Table.",
      aliases: &["grid", "spreadsheet", "datagrid", "sortable table"],
  ),
  comp!(
      "list", "List", "List", Data, "list",
      "A scrolling, selectable, optionally searchable list.",
      Ctor::Stateful("ListState<ListRows>"),
      props: &[
          hinted(items("items", "Items", Emit::Custom, || {
            PropValue::Items(vec!["Inbox".into(), "Drafts".into(), "Sent".into(), "Archive".into()])
          }), "One entry per row."),
          boolean("searchable", "Searchable", Emit::State("searchable"), false),
          boolean("selectable", "Selectable", Emit::State("selectable"), true),
          boolean("scrollbar", "Show scrollbar", Emit::Method("scrollbar_visible"), true),
          text("search_placeholder", "Search placeholder", Emit::Method("search_placeholder")),
          SIZE,
      ],
      events: &[ITEM_SELECTED, ITEM_CONFIRMED],
      on_place: Some(tall),
      docs: "Rows are text in a delegate the file defines; search filters them. Selecting \
             a row does not run anything by itself: bind On click or enter.",
      aliases: &["menu list", "listbox", "sidebar list"],
  ),
  comp!(
      "tree", "Tree", "Tree", Data, "list-tree",
      "Nested rows that fold open, like a file browser.",
      Ctor::Stateful("TreeState"),
      props: &[
          hinted(items("items", "Items", Emit::Custom, || {
            PropValue::Items(vec![
              "src".into(),
              "  main.rs".into(),
              "  lib.rs".into(),
              "docs".into(),
              "  readme.md".into(),
              "Cargo.toml".into(),
            ])
          }), "One node per line; two spaces of indent per level."),
          boolean("expanded", "Start expanded", Emit::Custom, true),
          boolean("icons", "Folder and file icons", Emit::Custom, true),
      ],
      on_place: Some(tall),
      docs: "Nest by indenting two spaces per level. Read the selection in an action with \
             `self.<field>.read(cx).selected_item()`.",
      aliases: &["file tree", "outline", "hierarchy", "folders"],
  ),
  comp!(
      "carousel", "Carousel", "Carousel", Media, "gallery-horizontal",
      "Slides you page through, with arrows and dots.",
      Ctor::Stateful("CarouselState"),
      props: &[
          hinted(items("slides", "Slides", Emit::Custom, || {
            PropValue::Items(vec!["One".into(), "Two".into(), "Three".into()])
          }), "One per slide; each gets its own drop region. The title shows when the slide is empty."),
          int("selected", "Starts on", Emit::State("with_selected_index"), || PropValue::Int(0)),
          boolean("looping", "Loop", Emit::State("with_looping"), false),
          enums("axis", "Direction", Emit::State("with_axis"), "gpui::Axis",
            &["horizontal", "vertical"], || PropValue::Choice("horizontal".into())),
          boolean("arrows", "Previous and next", Emit::Custom, true),
          boolean("dots", "Page dots", Emit::Custom, true),
      ],
      dynamic: Some(tailor_model::catalog::DynamicSlots {
          from_prop: "slides",
          prefix: "slide",
          method: "",
      }),
      events: &[SLIDE_CHANGED],
      on_place: Some(slide_frame),
      aliases: &["slideshow", "gallery", "pager", "image slider"],
  ),
  // Navigation and settings
  comp!(
      "sidebar", "Sidebar", "Sidebar", Navigation, "panel-left",
      "A collapsible side navigation with groups of menu items.",
      Ctor::Special,
      props: &[
          hinted(items("menu", "Menu", Emit::Custom, || {
            PropValue::Items(vec![
              "Platform".into(),
              "  *Overview | layout-dashboard".into(),
              "  Inbox | inbox".into(),
              "  Settings | settings".into(),
              "Projects".into(),
              "  Website".into(),
              "  Mobile app".into(),
            ])
          }), "A group per unindented line, its items indented two spaces, sub-items four. \
               `*` marks the active item; `| name` picks an icon from gpui-kit's default set."),
          enums("collapsible", "Collapses to", Emit::Method("collapsible"), "SidebarCollapsible",
            &["icon", "offcanvas", "none"], || PropValue::Choice("icon".into())),
          enums("side", "Side", Emit::Method("side"), "Side", &["left", "right"],
            || PropValue::Choice("left".into())),
          boolean("collapsed", "Collapsed", Emit::Method("collapsed"), false),
      ],
      slots: &[HEADER, FOOTER],
      docs: "Fills the height it is given, so place it in a row beside the page. Pair it \
             with a Sidebar toggle to collapse it. Icons come from gpui-kit's default set \
             (inbox, settings, user, folder, star, bell, search, calendar, globe, heart …); \
             a name outside it is replaced and reported in the export's notes.",
      aliases: &["side nav", "navigation drawer", "menu", "left nav"],
  ),
  comp!(
      "sidebartoggle", "Sidebar toggle", "SidebarToggleButton", Navigation, "panel-left-close",
      "The button that collapses and expands a Sidebar.",
      Ctor::Unit,
      props: &[
          boolean("collapsed", "Shows collapsed", Emit::Method("collapsed"), false),
          enums("side", "Side", Emit::Method("side"), "Side", &["left", "right"],
            || PropValue::Choice("left".into())),
          text("label", "Accessible label", Emit::Method("accessibility_label")),
      ],
      events: &[tailor_model::node::CLICK],
      docs: "Bind On click to an action that flips the state the Sidebar's Collapsed reads.",
  ),
  comp!(
      "settings", "Settings", "Settings", Layout, "settings",
      "A settings screen: a page list on the left and the chosen page on the right.",
      Ctor::Id,
      props: &[hinted(items("pages", "Pages", Emit::Custom, || {
        PropValue::Items(vec!["General".into(), "Appearance".into(), "About".into()])
      }), "One title per page; each gets its own drop region.")],
      dynamic: Some(tailor_model::catalog::DynamicSlots {
          from_prop: "pages",
          prefix: "page",
          method: "",
      }),
      on_place: Some(page_frame),
      docs: "Whatever is dropped into a page is drawn in it, in order. For a labelled row \
             of controls put a Label and the control side by side in a row.",
      aliases: &["preferences", "options", "config screen"],
  ),
  // Conversation parts
  comp!(
      "marker", "Marker", "Marker", Ai, "separator-horizontal",
      "A status line in a transcript: 'Today', 'Thinking…', 'Chat resumed'.",
      Ctor::Unit,
      props: &[
          text("text", "Text", Emit::Custom),
          icon("icon", "Icon", Emit::Custom),
          enums("variant", "Variant", Emit::Method("with_variant"), "MarkerVariant",
            &["plain", "separator", "border"], || PropValue::Choice("plain".into())),
          enums("alignment", "Alignment", Emit::Method("alignment"), "MarkerAlignment",
            &["auto", "start", "center", "end"], || PropValue::Choice("auto".into())),
          boolean("loading", "Loading", Emit::Method("loading"), false),
          enums("loading_style", "Loading style", Emit::Method("with_loading_style"),
            "MarkerLoadingStyle", &["spinner", "shimmer"],
            || PropValue::Choice("spinner".into())),
      ],
      slots: &[CHILDREN],
      aliases: &["divider label", "system message", "date separator", "status line"],
  ),
  comp!(
      "message", "Message", "Message", Ai, "message-square",
      "One turn of a conversation: an avatar, a header, a bubble and a footer.",
      Ctor::Unit,
      props: &[
          enums("alignment", "Alignment", Emit::Method("alignment"), "MessageAlignment",
            &["start", "end"], || PropValue::Choice("start".into())),
          text("header", "Header text", Emit::Custom),
          text("text", "Message text", Emit::Custom),
          enums("variant", "Bubble style", Emit::Custom, "BubbleVariant",
            &["filled", "secondary", "muted", "tinted", "outline", "ghost", "destructive"],
            || PropValue::Choice("filled".into())),
          text("footer", "Footer text", Emit::Custom),
      ],
      slots: &[AVATAR, HEADER, CONTENT, FOOTER],
      docs: "The text props write the usual bubble. For anything else drop parts: a Message \
             header, Message content or Message footer into their regions (they replace the \
             text), and any element into Avatar. Start is the other party, End is you.",
      aliases: &["chat message", "chat bubble row", "comment"],
  ),
  comp!(
      "messagegroup", "Message group", "MessageGroup", Ai, "messages-square",
      "Consecutive messages from one sender.",
      Ctor::Unit,
      slots: &[CHILDREN],
      docs: "Children are Message.",
  ),
  comp!(
      "messageheader", "Message header", "MessageHeader", Ai, "heading",
      "The line above a message: a name and a time.",
      Ctor::Unit,
      props: &[
          text("text", "Text", Emit::Custom),
          boolean("inset", "Align with the bubble", Emit::Method("content_inset"), true),
      ],
      slots: &[CHILDREN],
      docs: "Place it in a Message's Header region.",
  ),
  comp!(
      "messagecontent", "Message content", "MessageContent", Ai, "text-quote",
      "The body of a message: bubbles and anything else.",
      Ctor::Unit,
      slots: &[CHILDREN],
      docs: "Place it in a Message's Content region. Children are usually Bubble.",
  ),
  comp!(
      "messagefooter", "Message footer", "MessageFooter", Ai, "captions",
      "The line under a message: status and actions.",
      Ctor::Unit,
      props: &[
          text("text", "Text", Emit::Custom),
          boolean("inset", "Align with the bubble", Emit::Method("content_inset"), true),
      ],
      slots: &[CHILDREN],
      docs: "Place it in a Message's Footer region.",
  ),
  comp!(
      "bubble", "Bubble", "Bubble", Ai, "message-circle",
      "A rounded chat bubble around text or anything else.",
      Ctor::Unit,
      props: &[
          text("text", "Text", Emit::Custom),
          enums("variant", "Style", Emit::Method("with_variant"), "BubbleVariant",
            &["filled", "secondary", "muted", "tinted", "outline", "ghost", "destructive"],
            || PropValue::Choice("filled".into())),
          enums("alignment", "Alignment", Emit::Method("alignment"), "MessageAlignment",
            &["start", "end"], || PropValue::Choice("start".into())),
      ],
      slots: &[CHILDREN, REACTIONS],
      docs: "Text, or any elements dropped inside. Reactions takes a Bubble reactions row.",
  ),
  comp!(
      "bubblegroup", "Bubble group", "BubbleGroup", Ai, "layers",
      "Bubbles stacked as one run.",
      Ctor::Unit,
      slots: &[CHILDREN],
      docs: "Children are Bubble.",
  ),
  comp!(
      "bubblereactions", "Bubble reactions", "BubbleReactions", Ai, "smile-plus",
      "A row of reactions on the edge of a Bubble.",
      Ctor::Unit,
      props: &[
          enums("side", "Edge", Emit::Method("side"), "BubbleReactionSide",
            &["bottom", "top"], || PropValue::Choice("bottom".into())),
          enums("alignment", "Alignment", Emit::Method("alignment"), "MessageAlignment",
            &["end", "start"], || PropValue::Choice("end".into())),
      ],
      slots: &[CHILDREN],
      docs: "Place it in a Bubble's Reactions region. Children are small Buttons or emoji.",
  ),
  comp!(
      "attachment", "Attachment", "Attachment", Ai, "paperclip",
      "A file chip: an icon, a name, a size, and a status.",
      Ctor::Unit,
      props: &[
          text("title", "Title", Emit::Custom),
          text("description", "Description", Emit::Custom),
          icon("icon", "Icon", Emit::Custom),
          enums("status", "Status", Emit::Method("status"), "AttachmentStatus",
            &["complete", "pending", "uploading", "processing", "failed"],
            || PropValue::Choice("complete".into())),
          tailor_model::props::float("progress", "Progress", Emit::Custom, || PropValue::Float(-1.0)),
          enums("axis", "Layout", Emit::Method("axis"), "gpui::Axis",
            &["horizontal", "vertical"], || PropValue::Choice("horizontal".into())),
          text("tooltip", "Tooltip", Emit::Method("tooltip")),
          SIZE,
      ],
      slots: &[ACTIONS],
      events: ATTACHMENT_EVENTS,
      docs: "Set Progress from 0 to 100 while Uploading; leave it at -1 for none.",
      aliases: &["file chip", "upload", "file card"],
  ),
  comp!(
      "attachmentgroup", "Attachment group", "AttachmentGroup", Ai, "paperclip",
      "A scrolling row of Attachment.",
      Ctor::Id,
      slots: &[CHILDREN],
      docs: "Children are Attachment.",
  ),
  comp!(
      "attachmentactions", "Attachment actions", "AttachmentActions", Ai, "ellipsis",
      "Buttons on an Attachment.",
      Ctor::Unit,
      slots: &[CHILDREN],
      docs: "Place it in an Attachment's Actions region. Children are small Buttons.",
  ),
  comp!(
      "messagescroller", "Message scroller", "MessageScroller", Ai, "scroll-text",
      "A transcript that scrolls, follows the newest message and offers a jump back.",
      Ctor::Stateful("MessageScrollerState"),
      props: &[
          hinted(items("rows", "Rows", Emit::Custom, || {
            PropValue::Items(vec!["First message".into(), "Second message".into()])
          }), "One per row; each gets its own drop region. The title is only a label on the canvas."),
          boolean("scrollbar", "Scrollbar", Emit::Method("scrollbar"), true),
          boolean("jump_button", "Jump to latest", Emit::Method("jump_button"), true),
          text("jump_label", "Jump label", Emit::Method("with_jump_button_label")),
      ],
      dynamic: Some(tailor_model::catalog::DynamicSlots {
          from_prop: "rows",
          prefix: "row",
          method: "",
      }),
      on_place: Some(tall),
      docs: "Drop a Message into each row. It draws only the rows it needs, so it stays \
             fast with a long transcript; add and remove rows in your own code.",
      aliases: &["chat log", "transcript", "conversation", "message list"],
  ),
  comp!(
      "questionnaire", "Questionnaire", "Questionnaire", Ai, "list-checks",
      "A stepped set of multiple-choice questions with progress and Next / Submit.",
      Ctor::Stateful("QuestionnaireState"),
      props: &[
          hinted(items("questions", "Questions", Emit::Custom, || {
            PropValue::Items(vec![
              "Where will it run? | Cloud | Local | Both".into(),
              "How should we summarise it? | Short | Detailed".into(),
            ])
          }), "One question per line: the question, then its choices, separated by `|`. Follow a choice \
             with `::` and a sentence to describe it; add a cell starting with `+` for a \
             freeform text answer, labelled with what follows."),
          boolean("required", "Answers required", Emit::Custom, false),
          boolean("multiple", "Allow several choices", Emit::Custom, false),
          SIZE,
      ],
      events: &[SUBMITTED, COMPLETED, ANSWERED],
      docs: "Read the answers in the On submit action. Choices become the values `choice-1`, \
             `choice-2` … in order.",
      aliases: &["survey", "quiz", "form wizard", "multiple choice"],
  ),
];
