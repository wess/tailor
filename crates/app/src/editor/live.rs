//! The live window.
//!
//! A second OS window showing the document at its real device size, with no
//! canvas chrome and every component interactive. It updates on the same edit
//! that updates the canvas, so you can leave it open on a second display and
//! watch the app take shape while you work — which is the closest a compiled
//! language gets to a live preview.
//!
//! It is also the one place guise's inspector shows the *design* and nothing
//! else: this window renders the document alone, where the workbench renders
//! the document inside Tailor's own chrome. ⌥⌘I opens it, and it stays closed
//! until then — the probe in every component costs a boolean check while no
//! inspector is alive.

use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    div, px, size, App, Bounds, Context, Entity, FocusHandle, MouseButton, MouseDownEvent, Pixels,
    Point, Size, TitlebarOptions, Window, WindowBounds, WindowHandle, WindowOptions,
};
use guise::devtools::{DevTools, DevToolsEvent, DevToolsState, Dock};
use guise::prelude::*;
use tailor_model::Project;
use tailor_render::{Hooks, Mode, PreviewStore, RenderCtx};

use super::Workbench;
use crate::ToggleDevTools;

/// How much of the window the inspector takes when it is open.
const INSPECTOR_H: f32 = 340.0;
const INSPECTOR_W: f32 = 420.0;
/// One frame at 60Hz, which is how often the held pick looks again.
const FRAME: Duration = Duration::from_millis(16);

pub struct LiveView {
    project: Arc<Project>,
    doc_id: String,
    store: Entity<PreviewStore>,
    focus: FocusHandle,
    /// The inspector, when the user has opened it. `None` is not just hidden:
    /// dropping the last `DevTools` is what stops the recorder, so a closed
    /// inspector costs the document nothing per frame.
    devtools: Option<Entity<DevTools>>,
    dock: Dock,
    /// The dock whose room has been added to the window, if any. The inspector
    /// takes its space from the window rather than from the document: this
    /// window exists to show the design at its real device size, and squeezing
    /// the design to make room would be the one thing it must not do.
    docked: Option<Dock>,
    menu: Option<Entity<ContextMenu>>,
}

/// How much bigger the window is while the inspector is docked.
fn inspector_room(dock: Dock) -> Size<Pixels> {
    match dock {
        Dock::Right => size(px(INSPECTOR_W), px(0.0)),
        _ => size(px(0.0), px(INSPECTOR_H)),
    }
}

impl LiveView {
    fn new(project: Arc<Project>, doc_id: String, cx: &mut Context<Self>) -> Self {
        let store = cx.new(PreviewStore::new);
        let mut view = LiveView {
            project,
            doc_id,
            store,
            focus: cx.focus_handle(),
            devtools: None,
            dock: Dock::Bottom,
            docked: None,
            menu: None,
        };
        view.sync(cx);
        view
    }

    /// Take the latest document. Called from the workbench on every edit.
    pub fn update_project(
        &mut self,
        project: Arc<Project>,
        doc_id: String,
        cx: &mut Context<Self>,
    ) {
        self.project = project;
        self.doc_id = doc_id;
        self.sync(cx);
        cx.notify();
    }

    /// Open the inspector, or close it if it is already open.
    pub fn toggle_devtools(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.devtools.take().is_none() {
            // Only the Timelines graph needs the host's side of the inspector,
            // and it needs it to exist before the first frame it records.
            if !cx.has_global::<DevToolsState>() {
                DevToolsState::new().init(cx);
            }
            let dock = self.dock;
            let devtools = cx.new(|cx| DevTools::new(cx).dock(dock));
            // `subscribe_in` rather than `subscribe` because closing and
            // re-docking both resize the window, and only this form hands the
            // handler one.
            cx.subscribe_in(
                &devtools,
                window,
                |this, _devtools, event: &DevToolsEvent, window, cx| {
                    match event {
                        DevToolsEvent::Close => this.devtools = None,
                        DevToolsEvent::Dock(dock) => this.dock = *dock,
                        // Sources here are guise's own, not the user's design;
                        // there is nothing for Tailor to reveal.
                        _ => return,
                    }
                    this.fit_to_dock(window, cx);
                    cx.notify();
                },
            )
            .detach();
            self.devtools = Some(devtools);
        }
        self.fit_to_dock(window, cx);
        window.focus(&self.focus);
        cx.notify();
    }

    /// Give the inspector its own room rather than taking it from the document.
    /// This window exists to show the design at its device size, and squeezing
    /// the design to make space would be the one thing it must not do.
    ///
    /// Called when the inspector opens, closes or moves — never from `render`.
    /// A window resized while rendering has already laid out at the old size,
    /// which leaves the selection's highlight a frame behind the design.
    fn fit_to_dock(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let wants = self.devtools.is_some().then_some(self.dock);
        if self.docked == wants {
            return;
        }
        let mut bounds = window.bounds().size;
        if let Some(old) = self.docked {
            let room = inspector_room(old);
            bounds = size(bounds.width - room.width, bounds.height - room.height);
        }
        if let Some(new) = wants {
            let room = inspector_room(new);
            bounds = size(bounds.width + room.width, bounds.height + room.height);
        }
        window.resize(bounds);
        self.docked = wants;
        self.settle(cx);
    }

    /// Draw a few more frames after the window changes size. The highlight is
    /// read from the recorder, which is a frame behind; without this the box
    /// sits over the old layout until something else asks for a frame, and in
    /// a window that is otherwise still, nothing does.
    fn settle(&self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            for _ in 0..3 {
                cx.background_executor().timer(FRAME).await;
                if this.update(cx, |_, cx| cx.notify()).is_err() {
                    break;
                }
            }
        })
        .detach();
    }

    /// The menu a right-click opens on the document, the way a browser's is
    /// the page's menu with Inspect at the bottom of it.
    fn open_menu(&mut self, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let weak = cx.entity().downgrade();
        let open = self.devtools.is_some();
        let menu = cx.new(move |cx| {
            let inspect = weak.clone();
            let toggle = weak.clone();
            ContextMenu::new(cx)
                .width(210.0)
                .item_icon(IconName::Crosshair, "Inspect element", move |window, cx| {
                    inspect
                        .update(cx, |this, cx| this.inspect_at(position, window, cx))
                        .ok();
                })
                .divider()
                .item_icon(
                    if open {
                        IconName::PanelBottomClose
                    } else {
                        IconName::PanelBottomOpen
                    },
                    if open {
                        "Hide the inspector"
                    } else {
                        "Show the inspector"
                    },
                    move |window, cx| {
                        toggle
                            .update(cx, |this, cx| this.toggle_devtools(window, cx))
                            .ok();
                    },
                )
        });
        menu.update(cx, |menu, cx| menu.show(position, window, cx));
        self.menu = Some(menu);
        cx.notify();
    }

    /// Select the element under a point, the way a browser's Inspect does.
    ///
    /// Opening the inspector is not enough on its own: the tree it selects from
    /// is recorded by the frames that follow, so the point has to wait for one.
    /// The wait is a task rather than a check inside `render` — a render cannot
    /// ask for the next frame, and nothing else about this window is changing.
    fn inspect_at(&mut self, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        if self.devtools.is_none() {
            self.toggle_devtools(window, cx);
        }
        cx.spawn(async move |this, cx| {
            for _ in 0..20 {
                cx.background_executor().timer(FRAME).await;
                let settled = this
                    .update(cx, |this, cx| {
                        let Some(devtools) = this.devtools.clone() else {
                            return true;
                        };
                        // Keep asking for frames: the recorder only rotates
                        // when the inspector renders.
                        cx.notify();
                        if devtools.read(cx).tree().is_empty() {
                            return false;
                        }
                        devtools.update(cx, |devtools, cx| devtools.pick_at(position, cx))
                    })
                    .unwrap_or(true);
                if settled {
                    break;
                }
            }
        })
        .detach();
    }

    fn sync(&mut self, cx: &mut Context<Self>) {
        if let Some(doc) = self.project.doc(&self.doc_id).cloned() {
            self.store.update(cx, |store, cx| store.sync(&doc, cx));
        }
    }
}

impl Render for LiveView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = theme(cx).body().hsla();
        let text = theme(cx).text().hsla();
        let border = theme(cx).border().hsla();
        let font = theme(cx).font_family.clone();

        // Same self-healing as the workbench root: an action reaches a window
        // through whatever is focused in it, so focus must land somewhere.
        if window.focused(cx).is_none() {
            window.focus(&self.focus);
        }

        let ctx = RenderCtx {
            project: self.project.clone(),
            doc_id: self.doc_id.clone(),
            mode: Mode::Preview,
            selected: Rc::new(Vec::new()),
            hovered: None,
            drop: None,
            dragging: None,
            store: self.store.clone(),
            hooks: Hooks::inert(),
            outlines: false,
            placing: false,
            depth: 0,
        };

        let root =
            div()
                .track_focus(&self.focus)
                .key_context("live")
                .on_action(cx.listener(|this, _: &ToggleDevTools, window, cx| {
                    this.toggle_devtools(window, cx)
                }))
                .flex()
                .size_full()
                .bg(body)
                .text_color(text)
                .font_family(font);

        // The document keeps its own size; the inspector takes a fixed edge,
        // the way it does in a browser.
        let content = div()
            .flex_1()
            .min_w(px(0.0))
            .min_h(px(0.0))
            // On the document rather than on the root: the inspector is the
            // root's other child, and it has menus of its own.
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    this.open_menu(event.position, window, cx);
                }),
            )
            .child(LiveBody { ctx });

        // What the inspector has selected, drawn over the design the way a
        // browser draws it over the page. Without it you can pick an element
        // and have nothing on screen tell you which one you got.
        let highlight = self
            .devtools
            .as_ref()
            .and_then(|devtools| devtools.read(cx).selected_bounds())
            .filter(|bounds| bounds.size.width > px(0.0) && bounds.size.height > px(0.0))
            .map(|bounds| {
                let accent = theme(cx).info().hsla();
                let mut fill = accent;
                fill.a = 0.18;
                div()
                    .absolute()
                    .left(bounds.origin.x)
                    .top(bounds.origin.y)
                    .w(bounds.size.width)
                    .h(bounds.size.height)
                    .bg(fill)
                    .border_1()
                    .border_color(accent)
            });

        let root = root.relative().children(self.menu.clone());
        let content = div()
            .flex_1()
            .min_w(px(0.0))
            .min_h(px(0.0))
            .relative()
            .child(content)
            .children(highlight);

        match self.devtools.clone() {
            None => root.flex_col().child(content),
            Some(devtools) => match self.dock {
                Dock::Right => root.flex_row().child(content).child(
                    div()
                        .flex_none()
                        .w(px(INSPECTOR_W))
                        .h_full()
                        .border_l_1()
                        .border_color(border)
                        .child(devtools),
                ),
                _ => root.flex_col().child(content).child(
                    div()
                        .flex_none()
                        .h(px(INSPECTOR_H))
                        .w_full()
                        .border_t_1()
                        .border_color(border)
                        .child(devtools),
                ),
            },
        }
    }
}

/// A one-shot element so the document renders with a live `&mut Window`.
#[derive(IntoElement)]
struct LiveBody {
    ctx: RenderCtx,
}

impl RenderOnce for LiveBody {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .size_full()
            .child(tailor_render::render_document(&self.ctx, window, cx))
    }
}

impl Workbench {
    /// Open the live window, or bring it forward if it is already up.
    pub fn open_live_window(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(handle) = self.live {
            if handle
                .update(cx, |_, window, _| window.activate_window())
                .is_ok()
            {
                return;
            }
            self.live = None;
        }
        let Some(doc) = self.doc().cloned() else {
            return;
        };
        let project = self.snapshot();
        let doc_id = self.doc_id.clone();
        let title = format!("{} — live", doc.name);

        let bounds = Bounds::centered(None, size(px(doc.canvas.width), px(doc.canvas.height)), cx);
        let opened: gpui::Result<WindowHandle<LiveView>> = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some(title.into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(|cx| LiveView::new(project, doc_id, cx)),
        );
        match opened {
            Ok(handle) => {
                self.live = Some(handle);
                if self.settings.live_devtools {
                    let _ = handle.update(cx, |view, window, cx| view.toggle_devtools(window, cx));
                }
                self.toasts.info(
                    "Live window open — it follows every edit · ⌥⌘I inspects",
                    cx,
                );
            }
            Err(err) => self
                .toasts
                .failed(format!("Could not open the live window: {err}"), cx),
        }
    }

    /// Show the inspector in the live window, opening the window first if it is
    /// closed. The live window handles this action itself while it is the key
    /// window; this is the same action arriving from the workbench's menu.
    pub fn toggle_devtools(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.live.is_none() {
            self.open_live_window(window, cx);
        }
        let Some(handle) = self.live else {
            return;
        };
        let alive = handle
            .update(cx, |view, window, cx| view.toggle_devtools(window, cx))
            .is_ok();
        if !alive {
            self.live = None;
        }
    }

    /// Push the current document into the live window, if it is open. A window
    /// the user closed makes `update` fail, which is how we notice.
    pub fn push_live(&mut self, cx: &mut Context<Self>) {
        let Some(handle) = self.live else {
            return;
        };
        let project = self.snapshot();
        let doc_id = self.doc_id.clone();
        let alive = handle
            .update(cx, |view, _window, cx| {
                view.update_project(project, doc_id, cx)
            })
            .is_ok();
        if !alive {
            self.live = None;
        }
    }
}
