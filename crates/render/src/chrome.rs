//! The canvas's own drawing: selection, hover, drop indicators, and the
//! placeholders that stand in for an empty container.
//!
//! All of it reads from the theme, so the chrome sits on top of your design
//! without ever being mistaken for part of it.

use gpui::prelude::*;
use gpui::{div, px, App, Div, ElementId, Empty, Hsla, MouseButton, SharedString, Window};
use guise::prelude::*;
use tailor_model::NodeId;

use crate::hooks::{GrabDrag, Handle, Hooks};

/// The accent the canvas uses for selection. The project's own primary colour
/// would be indistinguishable from a selected primary button, so this is fixed.
pub fn accent(cx: &App) -> Hsla {
    theme(cx).color(ColorName::Blue, 5).hsla()
}

pub fn hover_tint(cx: &App) -> Hsla {
    theme(cx).color(ColorName::Blue, 4).alpha(0.5)
}

/// The dashed outline every container carries in blueprint mode, and that an
/// empty container carries in design mode — a box with nothing in it and no
/// border is invisible, and invisible is not droppable.
pub fn outline(cx: &App) -> Hsla {
    theme(cx).border().alpha(0.9)
}

/// A message where a component should have been.
pub fn missing(text: impl Into<SharedString>) -> Div {
    div()
        .p(px(12.))
        .child(Text::new(text.into()).size(Size::Sm).dimmed())
}

/// The stand-in for an empty container: a dashed box that says what it is, so
/// there is something to aim a drop at.
pub fn empty_slot(label: impl Into<SharedString>, cx: &App) -> Div {
    let border = outline(cx);
    let text = theme(cx).dimmed().hsla();
    div()
        .flex()
        .items_center()
        .justify_center()
        .min_h(px(44.))
        .min_w(px(64.))
        .w_full()
        .rounded(px(4.))
        .border(px(1.))
        .border_dashed()
        .border_color(border)
        .text_color(text)
        .text_size(px(11.))
        .child(label.into())
}

/// The strip that appears between children while a drag is in flight.
pub fn drop_strip(active: bool, horizontal: bool, cx: &App) -> Div {
    let color = if active {
        accent(cx)
    } else {
        theme(cx).border().alpha(0.35)
    };
    let thickness = if active { 3.0 } else { 2.0 };
    let base = div().rounded(px(2.)).bg(color);
    if horizontal {
        base.h_full().w(px(thickness)).min_h(px(16.))
    } else {
        base.w_full().h(px(thickness)).min_w(px(16.))
    }
}

/// A blueprint box: the node's footprint and its name, no content.
pub fn blueprint_box(label: impl Into<SharedString>, cx: &App) -> Div {
    let border = outline(cx);
    let text = theme(cx).dimmed().hsla();
    div()
        .flex()
        .items_center()
        .justify_center()
        .min_h(px(28.))
        .min_w(px(48.))
        .rounded(px(3.))
        .border(px(1.))
        .border_dashed()
        .border_color(border)
        .text_color(text)
        .text_size(px(10.))
        .child(label.into())
}

/// What a hidden node leaves behind on the canvas.
pub fn ghost(label: impl Into<SharedString>, cx: &App) -> Div {
    blueprint_box(format!("{} (hidden)", label.into()), cx).opacity(0.5)
}

/// The eight knobs around a selection — Interface Builder's, in guise's
/// colours. Centred on the edges rather than tucked inside them, which is what
/// makes the corner ones feel like they belong to both edges at once.
pub fn handles(id: NodeId, hooks: &Hooks, cx: &App) -> Div {
    let accent = accent(cx);
    // White in both schemes, the way every design tool draws them: a knob has
    // to read against whatever the node behind it is painted.
    let fill = gpui::white();

    let knob = |handle: Handle| {
        let grab = hooks.grab.clone();
        let mut dot = div()
            .id(ElementId::Name(SharedString::from(format!(
                "knob-{id}-{}",
                handle.label()
            ))))
            .size(px(9.))
            .rounded(px(2.))
            .bg(fill)
            .border(px(1.))
            .border_color(accent);
        dot = match handle {
            Handle::NorthWest | Handle::SouthEast => dot.cursor_nwse_resize(),
            Handle::NorthEast | Handle::SouthWest => dot.cursor_nesw_resize(),
            Handle::North | Handle::South => dot.cursor_ns_resize(),
            Handle::East | Handle::West => dot.cursor_ew_resize(),
        };
        dot.on_mouse_down(MouseButton::Left, move |event, _window, cx| {
            cx.stop_propagation();
            grab(id, Some(handle), event.position, cx);
        })
        .on_drag(
            GrabDrag {
                node: id,
                handle: Some(handle),
            },
            |_, _, _, cx| cx.new(|_| Empty),
        )
    };

    let row = || {
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .w_full()
    };
    let gap = || div().size(px(9.));

    // The overlay reaches half a knob beyond the node on every side, so a knob
    // sits centred on its edge rather than inside it.
    div()
        .absolute()
        .top(px(-4.5))
        .left(px(-4.5))
        .right(px(-4.5))
        .bottom(px(-4.5))
        .flex()
        .flex_col()
        .justify_between()
        .child(
            row()
                .child(knob(Handle::NorthWest))
                .child(knob(Handle::North))
                .child(knob(Handle::NorthEast)),
        )
        .child(
            row()
                .child(knob(Handle::West))
                .child(gap())
                .child(knob(Handle::East)),
        )
        .child(
            row()
                .child(knob(Handle::SouthWest))
                .child(knob(Handle::South))
                .child(knob(Handle::SouthEast)),
        )
}

/// The readout that follows a resize or a move: `240 × 96`, or `x, y`.
pub fn measure(label: impl Into<SharedString>, cx: &App) -> Div {
    let fill = accent(cx);
    div()
        .px(px(6.))
        .py(px(2.))
        .rounded(px(4.))
        .bg(fill)
        .text_color(gpui::white())
        .text_size(px(10.))
        .child(label.into())
}

/// The drag preview: a small chip naming what is being dragged.
pub struct DragGhost {
    pub label: SharedString,
}

impl Render for DragGhost {
    fn render(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let fill = theme(cx).color(ColorName::Blue, 6).hsla();
        div()
            .px(px(8.))
            .py(px(4.))
            .rounded(px(4.))
            .bg(fill)
            .text_color(gpui::white())
            .text_size(px(11.))
            .child(self.label.clone())
    }
}
