//! The UI tree above the workbench: routing between the start screen and an
//! open project, and the toast stack that floats over both.
//!
//! Every menu action is registered here and forwarded into the workbench.
//! Registering them on the workbench itself would tie them to what has focus,
//! and in a builder the focus is usually inside a text field in the inspector —
//! which is exactly when you still want ⌘Z to undo the canvas.

use std::path::PathBuf;

use gpui::prelude::*;
use gpui::{div, Context, Div, Entity, FocusHandle, Window};
use guise::prelude::*;
use tailor_model::Project;
use tailor_store::{Recents, Settings};

use crate::editor::Workbench;
use crate::toasts::Toasts;
use crate::*;

pub struct Root {
  pub(crate) settings: Settings,
  pub(crate) toasts: Toasts,
  pub(crate) recents: Recents,
  workbench: Option<Entity<Workbench>>,
  toast_stack: Entity<ToastStack>,
  /// Somewhere for focus to live when nothing else has it. Every menu item
  /// and shortcut is registered on this element, and gpui only offers an
  /// action that is reachable from what is focused.
  focus: FocusHandle,
  /// The start screen's right-click menu. The workbench owns its own; this
  /// one only exists before a project is open.
  menu: Option<Entity<ContextMenu>>,
}

impl Root {
  pub fn new(settings: Settings, open: Option<PathBuf>, cx: &mut Context<Self>) -> Self {
    let toasts = Toasts::new(cx);
    let toast_stack = toasts.stack();
    let mut recents = Recents::load();
    recents.prune();
    let mut root = Root {
      settings,
      toasts,
      recents,
      workbench: None,
      toast_stack,
      focus: cx.focus_handle(),
      menu: None,
    };
    if let Some(path) = open {
      root.load(path, cx);
    }
    root
  }

  fn open_project(&mut self, project: Project, path: Option<PathBuf>, cx: &mut Context<Self>) {
    crate::theme::install(&project, cx);
    if let Some(path) = &path {
      let mut recents = Recents::load();
      recents.touch(path, &project.name);
      recents.save();
      self.recents = recents.clone();
      refresh_menus(&recents, cx);
    }
    let settings = self.settings.clone();
    let toasts = self.toasts.clone();
    let workbench = cx.new(|cx| Workbench::new(project, path, settings, toasts, cx));
    self.workbench = Some(workbench);
    cx.notify();
  }

  /// Open the nth entry of the recents list, if it is still there.
  fn open_recent(&mut self, index: usize, cx: &mut Context<Self>) {
    let Some(recent) = self.recents.entries.get(index).cloned() else {
      return;
    };
    self.load(recent.path, cx);
  }

  fn clear_recents(&mut self, cx: &mut Context<Self>) {
    self.recents = Recents::default();
    self.recents.save();
    refresh_menus(&self.recents, cx);
    cx.notify();
  }

  fn close_project(&mut self, cx: &mut Context<Self>) {
    // The workbench owns the settings while a project is open — take back
    // whatever the settings screen changed, or the start screen wears the
    // scheme the app launched with instead of the one just picked.
    if let Some(workbench) = &self.workbench {
      self.settings = workbench.read(cx).settings.clone();
    }
    crate::theme::wear_chrome(&self.settings, cx);
    self.workbench = None;
    self.recents = Recents::load();
    self.recents.prune();
    cx.notify();
  }

  fn browse_and_open(&mut self, cx: &mut Context<Self>) {
    let receiver = cx.prompt_for_paths(gpui::PathPromptOptions {
      files: true,
      directories: false,
      multiple: false,
      prompt: None,
    });
    cx.spawn(async move |this, cx| {
      if let Ok(Ok(Some(paths))) = receiver.await {
        if let Some(path) = paths.into_iter().next() {
          this.update(cx, |this, cx| this.load(path, cx)).ok();
        }
      }
    })
    .detach();
  }

  fn load(&mut self, path: PathBuf, cx: &mut Context<Self>) {
    match tailor_store::open(&path) {
      Ok(project) => self.open_project(project, Some(path), cx),
      Err(err) => {
        let mut recents = Recents::load();
        recents.remove(&path);
        recents.save();
        refresh_menus(&recents, cx);
        self
          .toasts
          .failed(format!("Could not open that project: {err}"), cx);
        self.recents = Recents::load();
        self.recents.prune();
        cx.notify();
      }
    }
  }

  /// Run `f` against the open workbench, if there is one.
  fn with_workbench(
    &mut self,
    window: &mut Window,
    cx: &mut Context<Self>,
    f: impl FnOnce(&mut Workbench, &mut Window, &mut Context<Workbench>),
  ) {
    if let Some(workbench) = self.workbench.clone() {
      workbench.update(cx, |workbench, cx| f(workbench, window, cx));
    }
  }

  fn wire(&self, mut root: Div, cx: &mut Context<Self>) -> Div {
    macro_rules! forward {
            ($($action:ty => $method:ident),* $(,)?) => {
                $(
                    root = root.on_action(cx.listener(|this, _: &$action, window, cx| {
                        this.with_workbench(window, cx, |workbench, window, cx| {
                            workbench.$method(window, cx)
                        });
                    }));
                )*
            };
        }

    root = root.on_action(cx.listener(|this, _: &NewProject, _, cx| {
      this.open_project(Project::new("Untitled"), None, cx);
    }));
    root = root.on_action(cx.listener(|this, _: &OpenProject, _, cx| {
      this.browse_and_open(cx);
    }));
    root = root.on_action(cx.listener(|this, _: &CloseProject, _, cx| {
      this.close_project(cx);
    }));

    // Window management. gpui 0.2.2's only system submenu is Services, so
    // these are ordinary actions over the window handle.
    root = root.on_action(
      cx.listener(|_, _: &MinimizeWindow, window: &mut Window, _| {
        window.minimize_window();
      }),
    );
    root = root.on_action(cx.listener(|_, _: &ZoomWindow, window: &mut Window, _| {
      window.zoom_window();
    }));
    root = root.on_action(
      cx.listener(|_, _: &ToggleFullScreen, window: &mut Window, _| {
        window.toggle_fullscreen();
      }),
    );

    // About opens the settings sheet on its About page, which is where the
    // version and the settings path already live — a second window saying the
    // same thing would be a second place to keep it right.
    root = root.on_action(cx.listener(|this, _: &AboutTailor, window, cx| {
      this.with_workbench(window, cx, |workbench, _window, cx| {
        workbench.open_about(cx);
      });
    }));

    // File → Open Recent. One arm per slot, because a `no_json` action
    // carries nothing and the index has to live in the name.
    macro_rules! recent {
            ($($action:ty => $index:expr),* $(,)?) => {
                $(
                    root = root.on_action(cx.listener(|this, _: &$action, _, cx| {
                        this.open_recent($index, cx);
                    }));
                )*
            };
        }
    recent!(
        OpenRecent0 => 0, OpenRecent1 => 1, OpenRecent2 => 2, OpenRecent3 => 3,
        OpenRecent4 => 4, OpenRecent5 => 5, OpenRecent6 => 6, OpenRecent7 => 7,
    );
    root = root.on_action(cx.listener(|this, _: &ClearRecents, _, cx| {
      this.clear_recents(cx);
    }));

    forward!(
        Save => save,
        SaveAs => save_as,
        ExportCode => export_code,
        OpenQuickly => open_quickly,
        FindInCode => find_in_code,
        Dismiss => dismiss,
        AcceptSuggestion => accept_completion,
        NextSuggestion => next_completion,
        PreviousSuggestion => previous_completion,
        FindNext => find_forward,
        FindPrevious => find_backward,
        RunProject => run_project,
        BuildProject => build_project,
        StopProject => stop_project,
        CleanBuild => clean_build,
        RevealBuild => reveal_build,
        UseDebug => use_debug,
        UseRelease => use_release,
        NewScreen => new_screen,
        NewComponent => new_component,
        Undo => undo,
        Redo => redo,
        Cut => cut,
        Copy => copy,
        Paste => paste,
        Duplicate => duplicate,
        Delete => delete_selection,
        SelectAll => select_all,
        SelectParent => select_parent,
        Rename => begin_rename,
        EmbedFrame => embed_frame,
        EmbedStack => embed_stack,
        EmbedCard => embed_card,
        EmbedScroll => embed_scroll,
        Unwrap => unwrap_selection,
        MoveUp => move_up,
        MoveDown => move_down,
        AlignLeft => align_left,
        AlignCenterH => align_center_h,
        AlignRight => align_right,
        AlignTop => align_top,
        AlignMiddle => align_middle,
        AlignBottom => align_bottom,
        DistributeH => distribute_h,
        DistributeV => distribute_v,
        ModeDesign => mode_design,
        ModeBlueprint => mode_blueprint,
        ModeSplit => mode_split,
        ModePreview => mode_preview,
        TogglePalette => toggle_palette,
        ToggleOutline => toggle_outline,
        ToggleInspector => toggle_inspector,
        ToggleProblems => toggle_problems,
        ToggleGrid => toggle_grid,
        ToggleSnap => toggle_snap,
        ToggleSnapObjects => toggle_snap_objects,
        ToggleFreeForm => toggle_free_form,
        ToggleSelectionLayout => toggle_selection_layout,
        ToggleOutlines => toggle_outlines,
        NudgeLeft => nudge_left,
        NudgeRight => nudge_right,
        NudgeUp => nudge_up,
        NudgeDown => nudge_down,
        NudgeLeftBig => nudge_left_big,
        NudgeRightBig => nudge_right_big,
        NudgeUpBig => nudge_up_big,
        NudgeDownBig => nudge_down_big,
        OpenLiveWindow => open_live_window,
        ToggleDevTools => toggle_devtools,
        OpenInEditor => open_in_editor,
        InstallEditorTask => install_editor_task,
        OpenSettings => open_settings,
    );
    root
  }
}

impl Render for Root {
  fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let chrome = crate::theme::colors(cx);
    let font = theme(cx).font_family.clone();

    // Self-healing: whenever focus goes nowhere — a field was destroyed, a
    // menu closed — it comes back here, and the menu bar stays alive.
    if window.focused(cx).is_none() {
      window.focus(&self.focus);
    }

    let mut root = div()
      .track_focus(&self.focus)
      .relative()
      .size_full()
      .bg(chrome.body)
      .text_color(chrome.text)
      .font_family(font);
    root = self.wire(root, cx);

    match self.workbench.clone() {
      Some(workbench) => root = root.child(workbench),
      None => root = root.child(self.render_start(cx)),
    }

    root = root.children(self.menu.clone());
    root.child(self.toast_stack.clone())
  }
}

/// The start screen drives these; it is a render method on `Root`, not a view
/// of its own, so it can reach the project it is about to open.
impl Root {
  pub(crate) fn start_project(&mut self, project: Project, cx: &mut Context<Self>) {
    self.open_project(project, None, cx);
  }

  pub(crate) fn start_open(&mut self, path: PathBuf, cx: &mut Context<Self>) {
    self.load(path, cx);
  }

  pub(crate) fn start_browse(&mut self, cx: &mut Context<Self>) {
    self.browse_and_open(cx);
  }

  /// The menu on a row in Recent.
  pub(crate) fn open_recent_menu(
    &mut self,
    path: PathBuf,
    position: gpui::Point<gpui::Pixels>,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let weak = cx.entity().downgrade();
    let menu = cx.new(move |cx| {
      let (open, reveal, copy, forget) = (path.clone(), path.clone(), path.clone(), path.clone());
      ContextMenu::new(cx)
        .width(220.0)
        .item_icon(IconName::FolderOpen, "Open", {
          let weak = weak.clone();
          move |_window, cx| {
            weak
              .update(cx, |this, cx| this.start_open(open.clone(), cx))
              .ok();
          }
        })
        .item_icon(
          IconName::FolderSearch,
          "Reveal in Finder",
          move |_window, cx| {
            cx.reveal_path(&reveal);
          },
        )
        .item_icon(IconName::Copy, "Copy the path", move |_window, cx| {
          cx.write_to_clipboard(gpui::ClipboardItem::new_string(
            copy.to_string_lossy().to_string(),
          ));
        })
        .divider()
        .item("Remove from the list", {
          let weak = weak.clone();
          move |_window, cx| {
            weak
              .update(cx, |this, cx| {
                this.recents.remove(&forget);
                this.recents.save();
                refresh_menus(&this.recents, cx);
                cx.notify();
              })
              .ok();
          }
        })
    });
    menu.update(cx, |menu, cx| menu.show(position, window, cx));
    self.menu = Some(menu);
    cx.notify();
  }
}

/// Rebuild the menu bar so **File → Open Recent** matches the list.
///
/// A menu is a snapshot: `set_menus` is the only way to move it on, and the
/// recents list changes whenever a project is opened, saved or forgotten.
pub(crate) fn refresh_menus(recents: &Recents, cx: &mut gpui::App) {
  cx.set_menus(crate::menus(&recents.entries));
}
