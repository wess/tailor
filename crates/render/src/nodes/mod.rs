//! Turning a node into an element, and wrapping it in the canvas's chrome.
//!
//! Two layers. `build` makes the guise component; this module puts it in a box
//! that carries the node's style, its selection outline, its mouse handlers,
//! and — while a drag is in flight — the strips you drop between children.
//!
//! The chrome only exists outside preview mode. In preview the wrapper is a
//! plain styled `div` and every click goes where it would in the real app.

pub mod build;

use gpui::prelude::*;
use gpui::{
    div, px, AnyElement, App, Div, ElementId, Empty, MouseButton, SharedString, Stateful, Window,
};
use tailor_model::catalog;
use tailor_model::node::DEFAULT_SLOT;
use tailor_model::style::{Dimension, Direction, LayoutMode, StyleProps};
use tailor_model::{Node, NodeId};

use crate::chrome::{self, DragGhost};
use crate::hooks::{DragPayload, DropSpot, GrabDrag};
use crate::read;
use crate::{Mode, RenderCtx, MAX_DEPTH};

/// Render a node, working out from the document whether its parent pins it.
pub fn render(ctx: &RenderCtx, id: NodeId, window: &mut Window, cx: &mut App) -> AnyElement {
    let absolute = ctx
        .doc()
        .and_then(|doc| doc.parent_of(id))
        .map(|(parent, _, _)| {
            ctx.doc().map(|doc| doc.layout_of(parent)) == Some(LayoutMode::Absolute)
        })
        .unwrap_or(false);
    render_in(ctx, id, absolute, window, cx)
}

/// Render a node whose placement the caller already knows.
pub fn render_in(
    ctx: &RenderCtx,
    id: NodeId,
    absolute: bool,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let Some(doc) = ctx.doc() else {
        return chrome::missing("no document").into_any_element();
    };
    let Some(node) = doc.node(id) else {
        return chrome::missing("missing node").into_any_element();
    };

    if node.hidden && ctx.mode != Mode::Preview {
        return wrapper(ctx, node, absolute, cx)
            .child(chrome::ghost(label_of(node), cx))
            .into_any_element();
    }
    if node.hidden {
        return div().into_any_element();
    }

    let is_container = matches!(
        node.kind.as_str(),
        "frame" | "canvas" | "surface" | "spacer"
    );
    let mut root = wrapper(ctx, node, absolute, cx);

    if is_container {
        root = apply_container(root, &node.style);
        // A spacer is a gap, not a container: it takes no children and should
        // not offer a drop placeholder that would give it a size.
        if node.kind == "spacer" {
            return root
                .flex_grow()
                .min_h(px(8.))
                .min_w(px(8.))
                .children(overlays(ctx, node, cx))
                .into_any_element();
        }
        if node.kind == "surface" {
            let reader = read::Reader::new(node, doc);
            if let tailor_model::props::PropValue::Color(color) = reader.get("fill") {
                root = root.bg(read::resolve(&color, cx));
            }
        }
        let kids = slot_children(ctx, node, DEFAULT_SLOT, window, cx);
        root = root.children(kids);
    } else if ctx.mode == Mode::Blueprint {
        root = root.child(chrome::blueprint_box(label_of(node), cx));
        // A blueprint still shows structure, so containers keep their children.
        if let Some(spec) = catalog::get(&node.kind) {
            if spec.takes_children() {
                let kids = slot_children(ctx, node, DEFAULT_SLOT, window, cx);
                root = root.flex().flex_col().gap(px(4.)).children(kids);
            }
        }
    } else {
        root = root.child(build::element(ctx, node, window, cx));
    }

    root.children(overlays(ctx, node, cx)).into_any_element()
}

/// The node's own box: its style, its chrome, and its handlers.
fn wrapper(ctx: &RenderCtx, node: &Node, absolute: bool, cx: &mut App) -> Stateful<Div> {
    let id = node.id;
    // Not `id.element_id()`: that is the component's own id, and two elements
    // sharing one makes gpui alias their element state — a `div` and a `Switch`
    // sharing state means the switch never appears at all.
    let mut root = div().id(ElementId::Name(SharedString::from(id.wrapper_element_id())));

    root = apply_box(root, &node.style, absolute, cx);

    if ctx.mode == Mode::Preview {
        return root;
    }

    let hover_tint = chrome::hover_tint(cx);

    if !absolute {
        root = root.relative();
    }

    // Anything that takes children is a drop target for the whole of its box;
    // the strips between its children refine that to an index.
    let accepts = catalog::get(&node.kind)
        .map(|spec| spec.takes_children())
        .unwrap_or(false);
    let spot = accepts.then(|| {
        let index = ctx
            .doc()
            .and_then(|doc| doc.node(id))
            .map(|node| node.children().len())
            .unwrap_or(0);
        DropSpot::at(id, DEFAULT_SLOT, index)
    });

    if !node.locked {
        let select = ctx.hooks.select.clone();
        root = root.on_mouse_down(MouseButton::Left, move |event, window, cx| {
            cx.stop_propagation();
            select(
                id,
                event.modifiers.shift || event.modifiers.platform,
                window,
                cx,
            );
        });

        // Right-click selects first, then opens the menu. Acting on something
        // you have not visibly selected is how a builder deletes the wrong node.
        let select = ctx.hooks.select.clone();
        let context = ctx.hooks.context.clone();
        let already = ctx.is_selected(id);
        root = root.on_mouse_down(MouseButton::Right, move |event, window, cx| {
            cx.stop_propagation();
            if !already {
                select(id, false, window, cx);
            }
            context(Some(id), event.position, window, cx);
        });

        if absolute {
            // Inside an absolute container a drag moves the node where it is.
            // Reparenting is what the outline is for.
            let grab = ctx.hooks.grab.clone();
            root = root
                .cursor_move()
                .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                    grab(id, None, event.position, cx);
                })
                .on_drag(
                    GrabDrag {
                        node: id,
                        handle: None,
                    },
                    |_, _, _, cx| cx.new(|_| Empty),
                );
        } else {
            let label = label_of(node);
            let place = ctx.hooks.place.clone();
            root = root.on_drag(
                DragPayload::Existing(id),
                move |_payload, _offset, _window, cx| {
                    place(cx);
                    cx.new(|_| DragGhost {
                        label: SharedString::from(label.clone()),
                    })
                },
            );
        }
    }

    // Where the node landed, for the knobs, the guides, and the readout. Only
    // in an editing mode: preview has no use for it and pays no cost.
    let store = ctx.store.clone();
    root = root.child(
        gpui::canvas(
            move |bounds, _window, cx| {
                store.update(cx, |store, _| store.set_bounds(id, bounds));
            },
            |_, _: (), _, _| {},
        )
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0(),
    );

    // One `on_hover` per element — gpui allows exactly one, and both the hover
    // outline and the drop target want to know.
    let hover = ctx.hooks.hover.clone();
    let over = ctx.hooks.over.clone();
    let hover_spot = spot.clone();
    let selectable = !node.locked;
    root = root.on_hover(move |is_over, _window, cx| {
        if selectable {
            hover(if *is_over { Some(id) } else { None }, cx);
        }
        if hover_spot.is_some() {
            if *is_over && cx.has_active_drag() {
                over(hover_spot.clone(), cx);
            } else if !*is_over {
                // Leaving clears the indicator. Without this the last target a
                // drag passed over stays lit after the drag is gone.
                over(None, cx);
            }
        }
    });

    if let Some(spot) = spot {
        let drop = ctx.hooks.drop.clone();
        root = root
            .drag_over::<DragPayload>(move |style, _, _, _| style.bg(hover_tint.alpha(0.12)))
            .on_drop::<DragPayload>(move |payload, window, cx| {
                cx.stop_propagation();
                drop(spot.clone(), payload.clone(), window, cx);
            });
    }

    root
}

/// The selection chrome, built after the content so it paints — and hit-tests
/// — above it. A knob under the component it belongs to is a knob you cannot
/// press, which turns every attempted resize into an accidental reparent.
fn overlays(ctx: &RenderCtx, node: &Node, cx: &mut App) -> Vec<AnyElement> {
    if ctx.mode == Mode::Preview {
        return Vec::new();
    }
    let id = node.id;
    let selected = ctx.is_selected(id);
    let hovered = ctx.hovered == Some(id);
    let radius = node.style.radius.max(2.0);
    let faint = ctx
        .outlines
        .then(|| guise::theme::theme(cx).border().alpha(0.55));

    let mut out: Vec<AnyElement> = Vec::new();
    if selected || hovered || faint.is_some() {
        let (color, width) = if selected {
            (chrome::accent(cx), 1.5)
        } else if hovered {
            (chrome::hover_tint(cx), 1.0)
        } else {
            (faint.unwrap_or_else(|| chrome::hover_tint(cx)), 1.0)
        };
        out.push(
            div()
                .absolute()
                .inset_0()
                .rounded(px(radius))
                .border(px(width))
                .border_color(color)
                .into_any_element(),
        );
    }
    // Knobs only when exactly one node is selected: eight of them around each
    // of five nodes is not a selection, it is a snowstorm.
    if selected && ctx.selected.len() == 1 && !node.locked {
        out.push(chrome::handles(id, &ctx.hooks, cx).into_any_element());
    }
    out
}

/// Container layout — the calls that decide how children sit.
fn apply_container(mut root: Stateful<Div>, style: &StyleProps) -> Stateful<Div> {
    match style.layout {
        LayoutMode::Flow => {
            root = root.flex();
            root = match style.direction {
                Direction::Row => root.flex_row(),
                Direction::Column => root.flex_col(),
            };
            if style.wrap {
                root = root.flex_wrap();
            }
            if let Some(gap) = style.gap.filter(|g| *g > 0.0) {
                root = root.gap(px(gap));
            }
            if let Some(align) = style.align {
                root = match align {
                    tailor_model::AlignToken::Start => root.items_start(),
                    tailor_model::AlignToken::Center => root.items_center(),
                    tailor_model::AlignToken::End => root.items_end(),
                    tailor_model::AlignToken::Stretch => root,
                };
            }
            if let Some(justify) = style.justify {
                root = match justify {
                    tailor_model::JustifyToken::Start => root.justify_start(),
                    tailor_model::JustifyToken::Center => root.justify_center(),
                    tailor_model::JustifyToken::End => root.justify_end(),
                    tailor_model::JustifyToken::Between => root.justify_between(),
                    tailor_model::JustifyToken::Around => root.justify_around(),
                };
            }
        }
        LayoutMode::Absolute => root = root.relative(),
    }
    root
}

/// The node's own box: size, spacing, paint, text.
fn apply_box(
    mut root: Stateful<Div>,
    style: &StyleProps,
    absolute: bool,
    cx: &App,
) -> Stateful<Div> {
    if absolute {
        root = root.absolute().left(px(style.x)).top(px(style.y));
    }
    root = match style.width {
        Dimension::Auto => root,
        Dimension::Px(v) => root.w(px(v)),
        Dimension::Full => root.w_full(),
        Dimension::Grow(_) => root.flex_grow(),
    };
    root = match style.height {
        Dimension::Auto => root,
        Dimension::Px(v) => root.h(px(v)),
        Dimension::Full => root.h_full(),
        Dimension::Grow(_) => root.flex_grow(),
    };
    if let Some(v) = style.min_width {
        root = root.min_w(px(v));
    }
    if let Some(v) = style.max_width {
        root = root.max_w(px(v));
    }
    if let Some(v) = style.min_height {
        root = root.min_h(px(v));
    }
    if let Some(v) = style.max_height {
        root = root.max_h(px(v));
    }
    let p = &style.padding;
    if !p.is_zero() {
        root = root
            .pt(px(p.top))
            .pr(px(p.right))
            .pb(px(p.bottom))
            .pl(px(p.left));
    }
    let m = &style.margin;
    if !m.is_zero() {
        root = root
            .mt(px(m.top))
            .mr(px(m.right))
            .mb(px(m.bottom))
            .ml(px(m.left));
    }
    if let Some(color) = &style.background {
        root = root.bg(read::resolve(color, cx));
    }
    if style.border_width > 0.0 {
        let color = style.border_color.clone().unwrap_or_default();
        root = root
            .border(px(style.border_width))
            .border_color(read::resolve(&color, cx));
    }
    if style.radius > 0.0 {
        root = root.rounded(px(style.radius));
    }
    root = match style.shadow {
        tailor_model::ShadowToken::None => root,
        tailor_model::ShadowToken::Xs => root.shadow_xs(),
        tailor_model::ShadowToken::Sm => root.shadow_sm(),
        tailor_model::ShadowToken::Md => root.shadow_md(),
        tailor_model::ShadowToken::Lg => root.shadow_lg(),
        tailor_model::ShadowToken::Xl => root.shadow_xl(),
    };
    if style.opacity < 1.0 {
        root = root.opacity(style.opacity);
    }
    if let Some(color) = &style.text_color {
        root = root.text_color(read::resolve(color, cx));
    }
    if let Some(size) = style.font_size {
        root = root.text_size(px(size));
    }
    if let Some(weight) = style.font_weight {
        root = root.font_weight(gpui::FontWeight(weight as f32));
    }
    if style.italic {
        root = root.italic();
    }
    if let Some(align) = style.text_align {
        root = match align {
            tailor_model::TextAlign::Left => root.text_left(),
            tailor_model::TextAlign::Center => root.text_center(),
            tailor_model::TextAlign::Right => root.text_right(),
        };
    }
    root = match style.overflow {
        tailor_model::Overflow::Visible => root,
        tailor_model::Overflow::Hidden => root.overflow_hidden(),
        tailor_model::Overflow::ScrollX => root.overflow_x_scroll(),
        tailor_model::Overflow::ScrollY => root.overflow_y_scroll(),
    };
    root
}

/// The children of a slot, with drop strips woven in while a drag is live.
pub fn slot_children(
    ctx: &RenderCtx,
    node: &Node,
    slot: &str,
    window: &mut Window,
    cx: &mut App,
) -> Vec<AnyElement> {
    let children: Vec<NodeId> = node.slot(slot).to_vec();
    let absolute = node.style.layout == LayoutMode::Absolute;
    let dragging = ctx.mode != Mode::Preview && ctx.placing;
    let horizontal = node.style.direction == Direction::Row && !absolute;

    let mut out: Vec<AnyElement> = Vec::new();

    if children.is_empty() {
        if ctx.mode != Mode::Preview {
            let label = if slot == DEFAULT_SLOT {
                format!("Drop into {}", label_of(node))
            } else {
                slot_label(node, slot)
            };
            out.push(chrome::empty_slot(label, cx).into_any_element());
        }
        return out;
    }

    for (index, child) in children.iter().enumerate() {
        if dragging && !absolute {
            out.push(strip(ctx, node.id, slot, index, horizontal, cx));
        }
        out.push(render_in(ctx, *child, absolute, window, cx));
    }
    if dragging && !absolute {
        out.push(strip(ctx, node.id, slot, children.len(), horizontal, cx));
    }
    out
}

/// One insertion point between two children.
fn strip(
    ctx: &RenderCtx,
    parent: NodeId,
    slot: &str,
    index: usize,
    horizontal: bool,
    cx: &mut App,
) -> AnyElement {
    let spot = DropSpot::at(parent, slot, index);
    let active = ctx.drop.as_ref() == Some(&spot);
    let drop = ctx.hooks.drop.clone();
    let over = ctx.hooks.over.clone();
    let for_drop = spot.clone();
    let for_hover = spot.clone();
    chrome::drop_strip(active, horizontal, cx)
        .id(ElementId::Name(SharedString::from(format!(
            "strip-{parent}-{slot}-{index}"
        ))))
        .on_drop::<DragPayload>(move |payload, window, cx| {
            cx.stop_propagation();
            drop(for_drop.clone(), payload.clone(), window, cx);
        })
        .on_hover(move |is_over, _window, _cx| {
            over(is_over.then(|| for_hover.clone()), _cx);
        })
        .into_any_element()
}

/// What the layers tree and the empty-slot placeholder call a node.
pub fn label_of(node: &Node) -> String {
    node.name
        .clone()
        .or_else(|| node.component_ref().map(|name| name.to_string()))
        .or_else(|| catalog::get(&node.kind).map(|spec| spec.title.to_string()))
        .unwrap_or_else(|| node.kind.clone())
}

fn slot_label(node: &Node, slot: &str) -> String {
    catalog::get(&node.kind)
        .and_then(|spec| spec.slot_spec(slot).map(|s| s.label.to_string()))
        .unwrap_or_else(|| slot.to_string())
}

/// Render another document inline — a placed project component.
pub fn render_component(
    ctx: &RenderCtx,
    name: &str,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    if ctx.depth >= MAX_DEPTH {
        return chrome::missing(format!("{name} nests too deeply")).into_any_element();
    }
    let Some(target) = ctx.project.doc_by_name(name) else {
        return chrome::missing(format!("no component named {name}")).into_any_element();
    };
    let nested = ctx.nested(&target.id);
    let root = target.root;
    render_in(&nested, root, false, window, cx)
}
