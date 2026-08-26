//! The workbench: one entity that owns the open project and every panel around
//! it.
//!
//! Panels are render methods in sibling files rather than views of their own.
//! In a builder almost every interaction is a document mutation — drag a node,
//! type in the inspector, pick a tab — and splitting that across six entities
//! would mean six copies of "tell the workbench, then tell everyone else". One
//! owner, `cx.notify()`, done.

pub mod analysis;
pub mod canvas;
pub mod code;
pub mod commands;
pub mod docs;
pub mod grab;
pub mod inspector;
pub mod live;
pub mod menu;
pub mod outline;
pub mod palette;
pub mod panels;
pub mod problems;
pub mod toolbar;
pub mod watch;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use gpui::prelude::*;
use gpui::{
  div, Context, DragMoveEvent, Entity, FocusHandle, MouseButton, Subscription, Task, Window,
};
use guise::prelude::*;
use tailor_model::lint::Problem;
use tailor_model::{Document, History, NodeId, Project};
use tailor_render::{DropSpot, Hooks, PreviewStore};
use tailor_store::{CanvasMode, Panel, Settings};

use crate::theme;
use crate::toasts::Toasts;

/// A Lucide glyph by name, falling back to a dot so a bad name in the catalog
/// never blanks a row.
pub fn icon(name: &str) -> Glyph {
  tailor_render::read::icon_named(name)
    .map(Glyph::from)
    .unwrap_or(Glyph::from(IconName::Circle))
}

/// Which inspector is showing. Interface Builder's split, minus the ones that
/// have no meaning here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inspector {
  /// Component props.
  Attributes,
  /// Layout, size, spacing, position.
  Size,
  /// Fill, border, radius, shadow, text.
  Style,
  /// The entrance this node plays, and how a container staggers its
  /// children into view.
  Motion,
  /// Events to actions, props to state variables.
  Connections,
  /// Name, kind, and what it generates as.
  Identity,
}

impl Inspector {
  pub const ALL: &'static [Inspector] = &[
    Inspector::Attributes,
    Inspector::Size,
    Inspector::Style,
    Inspector::Motion,
    Inspector::Connections,
    Inspector::Identity,
  ];

  pub fn label(self) -> &'static str {
    match self {
      Inspector::Attributes => "Attributes",
      Inspector::Size => "Size",
      Inspector::Style => "Style",
      Inspector::Motion => "Motion",
      Inspector::Connections => "Connections",
      Inspector::Identity => "Identity",
    }
  }

  pub fn icon(self) -> &'static str {
    match self {
      Inspector::Attributes => "sliders-horizontal",
      Inspector::Size => "ruler",
      Inspector::Style => "paintbrush",
      Inspector::Motion => "play",
      Inspector::Connections => "cable",
      Inspector::Identity => "tag",
    }
  }
}

pub struct Workbench {
  /// Shared, not owned: the canvas, the history, and the background tasks all
  /// hold the same allocation. `Arc::make_mut` pays for a copy once per edit
  /// rather than once per commit and once per frame.
  pub project: Arc<Project>,
  pub history: History,
  pub path: Option<PathBuf>,
  pub dirty: bool,
  pub doc_id: String,
  pub selection: Vec<NodeId>,
  pub hovered: Option<NodeId>,
  pub drop: Option<DropSpot>,
  pub settings: Settings,
  pub inspector: Inspector,
  pub problems: Vec<Problem>,
  pub store: Entity<PreviewStore>,
  pub toasts: Toasts,

  /// The palette's search field, and the category filter beside it.
  pub search: Entity<TextInput>,
  pub category: Option<tailor_model::Category>,
  /// Nodes collapsed in the outline.
  pub collapsed: HashSet<NodeId>,
  /// The node being renamed inline, and the field it is typed into.
  pub renaming: Option<NodeId>,
  pub rename_field: Option<Entity<TextInput>>,
  /// Inspector text fields, keyed `<node>/<prop>`, rebuilt when the
  /// selection changes.
  pub fields: HashMap<String, Entity<TextInput>>,
  /// A field the next frame should focus, by its `fields` key, with how many
  /// frames it has waited. The field is created by whichever panel renders
  /// it, so the focus has to wait for that panel and give up if it never
  /// comes — the user can select something else before it lands.
  pub focus_field: Option<(String, u8)>,
  /// The multi-line siblings of `fields`, for list props.
  pub areas: HashMap<String, Entity<TextArea>>,
  /// The generated code, regenerated whenever the document changes.
  pub generated: String,
  pub code_view: Entity<Editor>,
  /// The detached live window, while it is open.
  pub live: Option<gpui::WindowHandle<live::LiveView>>,
  /// The open right-click menu, if there is one.
  pub menu: Option<Entity<ContextMenu>>,
  /// The preferences sheet, while it is up.
  pub settings_sheet: Option<Entity<SettingsView>>,
  /// A splitter drag in flight: which panel, where the pointer started, and
  /// how big the panel was then.
  pub splitter: Option<(Panel, f32, f32)>,
  /// A canvas drag in flight — a move or a resize.
  pub grab: Option<grab::Grab>,
  /// A drag that will place a node somewhere. Drop strips only appear for
  /// these; a resize must leave every container's layout alone.
  pub placing: bool,
  /// Alignment guides for the drag in flight.
  pub guides: Vec<grab::Guide>,
  /// Bumped on every change. A background result that carries an older one
  /// is discarded rather than overwriting something newer.
  pub revision: u64,
  /// The regenerate-and-lint task, if one is in flight. Holding it is what
  /// cancels superseded work.
  pub analysis: Option<Task<()>>,
  /// The debounced autosave.
  pub autosave: Option<Task<()>>,
  /// How long an edit settles before either runs. Zero under test.
  pub analysis_delay: std::time::Duration,
  pub autosave_delay: std::time::Duration,
  /// Whether editor settings are written back to disk. Off under test, so a
  /// test run never rewrites the settings of whoever is running it.
  pub persist_settings: bool,
  /// The file's modified time as of our last read or write, so the watcher
  /// can tell somebody else's change from our own.
  pub file_seen: Option<std::time::SystemTime>,
  /// Whether the "changed on disk" warning has already been said.
  pub warned_about_file: bool,
  /// Portrait or landscape for the current device preset.
  pub landscape: bool,
  /// Bumped to replay the canvas's entrance animations. Editing any motion
  /// setting bumps it, so an adjustment plays back the moment it is made.
  pub motion_epoch: usize,
  /// The canvas's focus. Not decoration: gpui builds the dispatch path from
  /// whatever is focused, and an app where nothing is focused has no path —
  /// so every action registered on an element goes unreachable and the whole
  /// menu bar greys out.
  pub focus: FocusHandle,
  /// Whether the canvas has claimed focus yet.
  focused: bool,
  pub subs: Vec<Subscription>,
}

impl Workbench {
  pub fn new(
    project: Project,
    path: Option<PathBuf>,
    settings: Settings,
    toasts: Toasts,
    cx: &mut Context<Self>,
  ) -> Self {
    let doc_id = project
      .docs
      .first()
      .map(|doc| doc.id.clone())
      .unwrap_or_default();
    let store = cx.new(PreviewStore::new);
    let search = cx.new(|cx| {
      TextInput::new(cx)
        .placeholder("Search components")
        .size(Size::Sm)
    });
    let code_view = cx.new(|cx| {
      Editor::new(cx)
        .language(Language::Rust)
        .read_only(true)
        .line_numbers(true)
        .font_size(12.0)
    });

    let mut subs = Vec::new();
    subs.push(cx.subscribe(
      &search,
      |this: &mut Workbench, _, _: &TextInputEvent, cx| {
        cx.notify();
        let _ = this;
      },
    ));

    let mut workbench = Workbench {
      project: Arc::new(project),
      history: History::default(),
      path,
      dirty: false,
      doc_id,
      selection: Vec::new(),
      hovered: None,
      drop: None,
      settings,
      inspector: Inspector::Attributes,
      problems: Vec::new(),
      store,
      toasts,
      search,
      category: None,
      collapsed: HashSet::new(),
      renaming: None,
      rename_field: None,
      fields: HashMap::new(),
      focus_field: None,
      areas: HashMap::new(),
      generated: String::new(),
      code_view,
      live: None,
      menu: None,
      settings_sheet: None,
      splitter: None,
      grab: None,
      guides: Vec::new(),
      placing: false,
      revision: 0,
      analysis: None,
      autosave: None,
      analysis_delay: analysis::ANALYSIS_DELAY,
      autosave_delay: analysis::AUTOSAVE_DELAY,
      persist_settings: true,
      file_seen: None,
      warned_about_file: false,
      landscape: false,
      motion_epoch: 0,
      focus: cx.focus_handle(),
      focused: false,
      subs,
    };
    workbench.mark_file_seen();
    workbench.watch_file(cx);
    workbench.refresh(cx);
    workbench
  }

  pub fn doc(&self) -> Option<&Document> {
    self.project.doc(&self.doc_id)
  }

  pub fn doc_mut(&mut self) -> Option<&mut Document> {
    let id = self.doc_id.clone();
    Arc::make_mut(&mut self.project).doc_mut(&id)
  }

  pub fn title(&self) -> String {
    let name = self
      .path
      .as_ref()
      .and_then(|path| {
        path
          .file_stem()
          .map(|stem| stem.to_string_lossy().to_string())
      })
      .unwrap_or_else(|| self.project.name.clone());
    if self.dirty {
      format!("{name} — edited")
    } else {
      name
    }
  }

  /// Record the current state so the next mutation can be undone.
  pub fn commit(&mut self, label: &str) {
    let before = self.project.clone();
    self.history.commit(label, &before);
    self.dirty = true;
  }

  /// Everything that has to happen after the document changes: the entity
  /// cache, the lint pass, the generated code, and the live window.
  pub fn refresh(&mut self, cx: &mut Context<Self>) {
    self.close_menu();
    self.revision = self.revision.wrapping_add(1);

    // The entity cache is the one part that cannot leave the main thread:
    // it builds gpui entities. It is also the cheap part.
    if let Some(doc) = self.project.doc(&self.doc_id).cloned() {
      self.store.update(cx, |store, cx| store.sync(&doc, cx));
    }
    self.push_live(cx);
    self.analyse(cx);
    self.schedule_autosave(cx);
    cx.notify();
  }

  /// The project, shared. Free — the canvas gets the same allocation the
  /// workbench is holding, which is what stops a drag from deep-copying the
  /// whole document sixty times a second.
  pub fn snapshot(&self) -> Arc<Project> {
    Arc::clone(&self.project)
  }

  /// The canvas's callbacks, built from a weak handle so a live component
  /// tree never keeps the workbench alive.
  pub fn hooks(&self, cx: &mut Context<Self>) -> Hooks {
    let weak = cx.entity().downgrade();

    let select = {
      let weak = weak.clone();
      Rc::new(
        move |id: NodeId, additive: bool, _window: &mut Window, cx: &mut gpui::App| {
          weak
            .update(cx, |this, cx| this.select(id, additive, cx))
            .ok();
        },
      )
    };
    let hover = {
      let weak = weak.clone();
      Rc::new(move |id: Option<NodeId>, cx: &mut gpui::App| {
        weak
          .update(cx, |this, cx| {
            if this.hovered != id {
              this.hovered = id;
              cx.notify();
            }
          })
          .ok();
      })
    };
    let drop = {
      let weak = weak.clone();
      Rc::new(
        move |spot: DropSpot,
              payload: tailor_render::DragPayload,
              _window: &mut Window,
              cx: &mut gpui::App| {
          weak
            .update(cx, |this, cx| this.accept_drop(spot, payload, cx))
            .ok();
        },
      )
    };
    let over = {
      let weak = weak.clone();
      Rc::new(move |spot: Option<DropSpot>, cx: &mut gpui::App| {
        weak
          .update(cx, |this, cx| {
            if this.drop != spot {
              this.drop = spot;
              cx.notify();
            }
          })
          .ok();
      })
    };
    let reveal = {
      let weak = weak.clone();
      Rc::new(move |id: NodeId, page: usize, cx: &mut gpui::App| {
        weak
          .update(cx, |this, cx| {
            this
              .store
              .update(cx, |store, cx| store.set_page(id, page, cx));
            cx.notify();
          })
          .ok();
      })
    };

    let context = {
      let weak = weak.clone();
      Rc::new(
        move |id: Option<NodeId>,
              position: gpui::Point<gpui::Pixels>,
              window: &mut Window,
              cx: &mut gpui::App| {
          weak
            .update(cx, |this, cx| {
              this.open_context_menu(id, position, window, cx)
            })
            .ok();
        },
      )
    };

    let place = {
      let weak = weak.clone();
      Rc::new(move |cx: &mut gpui::App| {
        weak
          .update(cx, |this, cx| {
            if !this.placing {
              this.placing = true;
              cx.notify();
            }
          })
          .ok();
      })
    };

    let grab = {
      let weak = weak.clone();
      Rc::new(
        move |id: NodeId,
              handle: Option<tailor_render::Handle>,
              position: gpui::Point<gpui::Pixels>,
              cx: &mut gpui::App| {
          weak
            .update(cx, |this, cx| this.begin_grab(id, handle, position, cx))
            .ok();
        },
      )
    };

    Hooks {
      select,
      hover,
      drop,
      over,
      reveal,
      context,
      grab,
      place,
      live: true,
    }
  }

  /// Write the editor settings back, unless persistence is off.
  pub fn save_settings(&self) {
    if self.persist_settings {
      self.settings.save();
    }
  }

  pub fn mode(&self) -> tailor_render::Mode {
    match self.settings.canvas_mode {
      CanvasMode::Blueprint => tailor_render::Mode::Blueprint,
      CanvasMode::Preview => tailor_render::Mode::Preview,
      _ => tailor_render::Mode::Design,
    }
  }
}

impl Render for Workbench {
  fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let chrome = theme::colors(cx);
    let focus = self.focus.clone();
    if !self.focused {
      self.focused = true;
      window.focus(&focus);
    }
    let settings = self.settings.clone();
    let split = settings.canvas_mode == CanvasMode::Split;

    let mut columns = div().flex().flex_row().flex_grow().overflow_hidden();

    if settings.palette_open {
      columns = columns
        .child(self.render_palette(window, cx))
        .child(self.splitter(Panel::Palette, cx));
    } else {
      columns = columns.child(self.rail(Panel::Palette, cx));
    }

    let mut middle = div()
      .flex()
      .flex_col()
      .flex_grow()
      .overflow_hidden()
      .child(self.render_doc_tabs(cx));

    let mut stage = div().flex().flex_row().flex_grow().overflow_hidden();
    if settings.outline_open {
      stage = stage
        .child(self.render_outline(window, cx))
        .child(self.splitter(Panel::Outline, cx));
    } else {
      stage = stage.child(self.rail(Panel::Outline, cx));
    }
    stage = stage.child(self.render_canvas(window, cx));
    if split {
      stage = stage
        .child(self.splitter(Panel::Code, cx))
        .child(self.render_code(cx));
    }
    middle = middle.child(stage);

    if settings.problems_open {
      middle = middle
        .child(self.splitter(Panel::Problems, cx))
        .child(self.render_problems(cx));
    }
    middle = middle.child(self.render_status(cx));
    columns = columns.child(middle);

    if settings.inspector_open {
      columns = columns
        .child(self.splitter(Panel::Inspector, cx))
        .child(self.render_inspector(window, cx));
    } else {
      columns = columns.child(self.rail(Panel::Inspector, cx));
    }

    // Every panel has been built by now, so a field the last command asked
    // for exists if it is going to.
    self.take_pending_focus(window, cx);

    div()
      .key_context("canvas")
      .track_focus(&focus)
      .relative()
      .size_full()
      .flex()
      .flex_col()
      .bg(chrome.body)
      // One handler for every splitter: the drag payload says which.
      .on_drag_move(cx.listener(
        |this, event: &DragMoveEvent<panels::SplitterDrag>, _window, cx| {
          this.on_splitter_move(event, cx);
        },
      ))
      .on_drag_move(cx.listener(
        |this, event: &DragMoveEvent<tailor_render::GrabDrag>, _window, cx| {
          this.on_grab_move(event, cx);
        },
      ))
      .on_mouse_up(
        MouseButton::Left,
        cx.listener(|this, _, _window, cx| {
          this.end_splitter(cx);
          this.end_grab(cx);
          // A drag that ended anywhere at all ends the indicator too.
          if this.drop.take().is_some() || this.placing {
            this.placing = false;
            cx.notify();
          }
        }),
      )
      .child(self.render_toolbar(cx))
      .child(columns)
      .children(self.render_guides(cx))
      .children(self.render_readout(cx))
      .children(self.menu.clone())
      .children(self.render_settings_sheet(cx))
  }
}
