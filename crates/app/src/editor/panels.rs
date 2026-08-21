//! Panel chrome: the splitters between panels, the headers that collapse them,
//! and the rails a collapsed panel leaves behind.
//!
//! Every panel is resizable and every panel folds away, because a builder is a
//! tool you sit in front of for hours and the right layout is different for
//! laying out a screen than it is for wiring one up. Sizes and open/closed
//! state live in the settings file, so the layout you left is the layout you
//! come back to.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, DragMoveEvent, Empty, MouseButton, MouseDownEvent, SharedString};
use gpui::{Context, ElementId};
use guise::prelude::*;
use tailor_store::Panel;

use super::{icon, Workbench};
use crate::theme;

/// The drag payload a splitter carries. Only the panel is in it; the pointer
/// origin and the starting size are recorded on mouse-down, because `on_drag`
/// hands the constructor an offset within the handle rather than a window
/// position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitterDrag {
    pub panel: Panel,
}

/// How wide the grab area is. Wider than the line it draws: a one-pixel target
/// is a fine divider and a terrible handle.
const GRAB: f32 = 7.0;

impl Workbench {
    /// The draggable divider beside a panel.
    pub(super) fn splitter(&self, panel: Panel, cx: &mut Context<Self>) -> impl IntoElement {
        let chrome = theme::colors(cx);
        let vertical = panel.vertical();
        let active = self
            .splitter
            .map(|(dragging, _, _)| dragging == panel)
            .unwrap_or(false);
        let line = if active { chrome.accent } else { chrome.border };

        div()
            .id(ElementId::Name(SharedString::from(format!(
                "split-{}",
                panel.label()
            ))))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .when(vertical, |d| d.w(px(GRAB)).h_full().cursor_col_resize())
            .when(!vertical, |d| d.h(px(GRAB)).w_full().cursor_row_resize())
            .hover(move |style| style.bg(chrome.accent_soft))
            .child(
                div()
                    .bg(line)
                    .when(vertical, |d| d.w(px(1.)).h_full())
                    .when(!vertical, |d| d.h(px(1.)).w_full()),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _window, cx| {
                    let at = if vertical {
                        event.position.x
                    } else {
                        event.position.y
                    };
                    this.splitter = Some((panel, f32::from(at), this.settings.size(panel)));
                    cx.stop_propagation();
                }),
            )
            .on_drag(SplitterDrag { panel }, |_, _offset, _window, cx| {
                cx.new(|_| Empty)
            })
    }

    /// Apply a splitter drag. Deltas from where the drag started, rather than
    /// absolute positions: the panel does not jump when you grab the handle
    /// off-centre.
    pub(super) fn on_splitter_move(
        &mut self,
        event: &DragMoveEvent<SplitterDrag>,
        cx: &mut Context<Self>,
    ) {
        let Some((panel, from, start)) = self.splitter else {
            return;
        };
        if event.drag(cx).panel != panel {
            return;
        }
        let at = if panel.vertical() {
            event.event.position.x
        } else {
            event.event.position.y
        };
        let mut delta = f32::from(at) - from;
        if panel.grows_negative() {
            delta = -delta;
        }
        self.settings.set_size(panel, start + delta);
        cx.notify();
    }

    /// End of a splitter drag: keep the size.
    pub(super) fn end_splitter(&mut self, cx: &mut Context<Self>) {
        if self.splitter.take().is_some() {
            self.save_settings();
            cx.notify();
        }
    }

    pub fn toggle_panel(&mut self, panel: Panel, cx: &mut Context<Self>) {
        let open = self.settings.is_open(panel);
        self.settings.set_open(panel, !open);
        self.save_settings();
        cx.notify();
    }

    /// A panel's title bar: its name, whatever the panel wants beside it, and
    /// the button that folds it away.
    pub(super) fn panel_header(
        &self,
        panel: Panel,
        trailing: Option<AnyElement>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let chrome = theme::colors(cx);
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .flex_none()
            .h(px(30.))
            .pl(px(10.))
            .pr(px(6.))
            .gap(px(6.))
            .border_b(px(1.))
            .border_color(chrome.border)
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(chrome.dimmed)
                    .child(SharedString::from(panel.label())),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.))
                    .children(trailing)
                    .child(
                        div()
                            .id(ElementId::Name(SharedString::from(format!(
                                "fold-{}",
                                panel.label()
                            ))))
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(20.))
                            .rounded(px(4.))
                            .text_color(chrome.dimmed)
                            .hover(move |style| style.bg(chrome.raised).text_color(chrome.text))
                            .child(icon(fold_icon(panel)))
                            .tooltip(tooltip("Collapse"))
                            .on_click(cx.listener(move |this, _, _window, cx| {
                                this.toggle_panel(panel, cx);
                            })),
                    ),
            )
    }

    /// What a collapsed panel leaves behind: a rail you can click to bring it
    /// back. gpui cannot rotate text, so the icon and its tooltip carry the
    /// name.
    pub(super) fn rail(&self, panel: Panel, cx: &mut Context<Self>) -> impl IntoElement {
        let chrome = theme::colors(cx);
        let right = matches!(panel, Panel::Inspector | Panel::Code);
        div()
            .flex_none()
            .w(px(34.))
            .h_full()
            .flex()
            .flex_col()
            .items_center()
            .pt(px(6.))
            .gap(px(4.))
            .bg(chrome.surface)
            .when(!right, |d| d.border_r(px(1.)).border_color(chrome.border))
            .when(right, |d| d.border_l(px(1.)).border_color(chrome.border))
            .child(
                div()
                    .id(ElementId::Name(SharedString::from(format!(
                        "rail-{}",
                        panel.label()
                    ))))
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(26.))
                    .rounded(px(5.))
                    .text_color(chrome.dimmed)
                    .hover(move |style| style.bg(chrome.raised).text_color(chrome.text))
                    .child(icon(panel.icon()))
                    .tooltip(tooltip(panel.label()))
                    .on_click(cx.listener(move |this, _, _window, cx| {
                        this.toggle_panel(panel, cx);
                    })),
            )
    }

    /// A section header inside the inspector: click it to fold the section.
    pub(super) fn fold_header(
        &self,
        key: &'static str,
        title: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let chrome = theme::colors(cx);
        let folded = self.settings.is_folded(key);
        div()
            .id(ElementId::Name(SharedString::from(format!(
                "fold-section-{key}"
            ))))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(4.))
            .py(px(2.))
            .text_size(px(10.))
            .text_color(chrome.dimmed)
            .hover(move |style| style.text_color(chrome.text))
            .child(icon(if folded {
                "chevron-right"
            } else {
                "chevron-down"
            }))
            .child(title.into())
            .on_click(cx.listener(move |this, _, _window, cx| {
                this.settings.toggle_folded(key);
                this.save_settings();
                cx.notify();
            }))
    }
}

/// The chevron that points the way the panel will fold.
fn fold_icon(panel: Panel) -> &'static str {
    match panel {
        Panel::Palette | Panel::Outline => "chevrons-left",
        Panel::Inspector | Panel::Code => "chevrons-right",
        Panel::Problems => "chevrons-down",
    }
}
