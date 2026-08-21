//! The artboard.
//!
//! A fixed-size frame at the document's device size, centred in a scrolling
//! field, with the document rendered inside it as live guise components.
//! Clicking the field around the artboard clears the selection; dropping on it
//! appends to the root.

use gpui::prelude::*;
use gpui::{div, px, relative, AnyElement, App, Context, MouseButton, Window};
use tailor_model::node::DEFAULT_SLOT;
use tailor_render::{DragPayload, DropSpot, Mode, RenderCtx};

use super::Workbench;
use crate::theme;

impl Workbench {
    pub(super) fn render_canvas(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let chrome = theme::colors(cx);
        let Some(doc) = self.doc() else {
            return div().flex_grow().into_any_element();
        };
        let (width, height) = (doc.canvas.width, doc.canvas.height);
        let background = doc
            .canvas
            .background
            .clone()
            .map(|color| tailor_render::read::resolve(&color, cx))
            .unwrap_or_else(|| theme(cx).body().hsla());
        let root = doc.root;
        let grid = self.settings.show_grid && self.mode() != Mode::Preview;
        let grid_size = self.settings.grid.max(2.0);

        let ctx = RenderCtx {
            project: self.snapshot(),
            doc_id: self.doc_id.clone(),
            mode: self.mode(),
            selected: std::rc::Rc::new(self.selection.clone()),
            hovered: self.hovered,
            drop: self.drop.clone(),
            dragging: None,
            store: self.store.clone(),
            hooks: self.hooks(cx),
            outlines: self.settings.show_outlines,
            placing: self.placing,
            depth: 0,
        };

        let weak = cx.entity().downgrade();
        let drop_spot = DropSpot::at(root, DEFAULT_SLOT, usize::MAX);

        // The artboard is centred by a row that is at least as wide as the
        // viewport rather than by `items_center` on the scroller: centred
        // content that overflows cannot be scrolled back to its left edge.
        div()
            .id("canvas-field")
            .flex()
            .flex_col()
            .flex_grow()
            .overflow_scroll()
            .bg(chrome.body)
            // Clicking the field, not the artboard, deselects.
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _window, cx| {
                    this.close_menu();
                    this.clear_selection(cx);
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, event: &gpui::MouseDownEvent, window, cx| {
                    this.open_context_menu(None, event.position, window, cx);
                }),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .justify_center()
                    // The row is the wider of the pane and the artboard, so
                    // `justify_center` centres a small artboard and leaves a
                    // large one flush left instead of hanging it off both
                    // edges where the left half cannot be scrolled to.
                    .w(relative(1.))
                    .min_w(px(width + PAD * 2.))
                    .p(px(PAD))
                    .child(
                        div()
                            .relative()
                            .w(px(width))
                            .h(px(height))
                            .flex_none()
                            .bg(background)
                            .rounded(px(6.))
                            .border(px(1.))
                            .border_color(chrome.border)
                            .shadow_lg()
                            .overflow_hidden()
                            .when(grid, |d| d.child(grid_overlay(grid_size, chrome.border)))
                            .child(ArtBoard { ctx })
                            .drag_over::<DragPayload>(move |style, _, _, _| {
                                style.bg(chrome.accent_soft)
                            })
                            .on_drop::<DragPayload>(move |payload, _window, cx: &mut App| {
                                let payload = payload.clone();
                                let spot = drop_spot.clone();
                                weak.update(cx, |this, cx| this.accept_drop(spot, payload, cx))
                                    .ok();
                            }),
                    ),
            )
            .into_any_element()
    }
}

/// The document itself. A `RenderOnce` wrapper so the tree is built with a live
/// `&mut Window`, which the renderer needs and a render method does not hand out.
#[derive(IntoElement)]
struct ArtBoard {
    ctx: RenderCtx,
}

impl RenderOnce for ArtBoard {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .size_full()
            .child(tailor_render::render_document(&self.ctx, window, cx))
    }
}

/// The margin between the artboard and the edge of its pane.
const PAD: f32 = 28.0;

/// The alignment grid behind the artboard. Drawn as rules rather than a
/// repeating image, because gpui has no tiling fill and forty divs is cheap.
fn grid_overlay(step: f32, color: gpui::Hsla) -> AnyElement {
    let faint = gpui::hsla(color.h, color.s, color.l, 0.28);
    let mut overlay = div().absolute().inset_0().overflow_hidden();
    // A fixed count keeps this bounded on a large artboard; the lines past the
    // edge are clipped and cost nothing.
    for index in 1..=80u32 {
        let offset = step * index as f32;
        overlay = overlay
            .child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(px(offset))
                    .w(px(1.))
                    .bg(faint),
            )
            .child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(px(offset))
                    .h(px(1.))
                    .bg(faint),
            );
    }
    overlay.into_any_element()
}
