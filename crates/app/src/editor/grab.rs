//! Direct manipulation on the canvas: dragging a node to move it, dragging a
//! knob to resize it, and the alignment guides that make either land square.
//!
//! Two things make this feel like Interface Builder rather than like a form
//! that happens to move. The whole drag is one undo step, not sixty. And the
//! node snaps to the grid *and* to its siblings' edges, with a guide drawn
//! wherever it caught, so a layout comes out aligned without anyone typing a
//! number.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, Bounds, Context, DragMoveEvent, Pixels, Point};
use tailor_model::catalog;
use tailor_model::props::{PropType, PropValue};
use tailor_model::style::{Dimension, LayoutMode};
use tailor_model::NodeId;
use tailor_render::{GrabDrag, Handle};

use super::Workbench;

/// How close an edge has to be to snap to a sibling's.
const SNAP: f32 = 6.0;
/// Nothing may be dragged smaller than this.
const MIN_SIZE: f32 = 8.0;

/// A drag in flight, and everything about the node as it was when it started.
/// Deltas from the start rather than from the last frame: a frame the pointer
/// outran does not accumulate error.
#[derive(Debug, Clone, Copy)]
pub struct Grab {
    pub node: NodeId,
    pub handle: Option<Handle>,
    pub from: Point<Pixels>,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    /// Whether the parent places its children, or the flow does.
    pub absolute: bool,
    /// The component sizes itself through props rather than through the box
    /// around it, so that is where a resize should land.
    pub sizes_itself: (bool, bool),
    /// Whether the pointer ever actually moved. A press that resizes nothing
    /// should not leave an undo step behind.
    pub moved: bool,
}

/// The value of a component's own pixel-sized prop, when it has one.
fn pixel_prop(node: &tailor_model::Node, key: &str) -> Option<f32> {
    let spec = catalog::get(&node.kind)?;
    let prop = spec.prop(key)?;
    if prop.ty != PropType::Float {
        return None;
    }
    node.prop(key)
        .cloned()
        .or_else(|| Some(prop.default_value()))
        .and_then(|value| value.as_f64())
        .map(|value| value as f32)
}

/// A line drawn where something snapped, in window coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Guide {
    pub vertical: bool,
    /// The x of a vertical guide, or the y of a horizontal one.
    pub at: f32,
    pub from: f32,
    pub to: f32,
}

impl Workbench {
    /// The mouse went down on a node or one of its knobs.
    pub fn begin_grab(
        &mut self,
        node: NodeId,
        handle: Option<Handle>,
        from: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let Some(doc) = self.doc() else { return };
        let Some(style) = doc.node(node).map(|node| node.style.clone()) else {
            return;
        };
        let absolute = doc
            .parent_of(node)
            .map(|(parent, _, _)| doc.layout_of(parent) == LayoutMode::Absolute)
            .unwrap_or(false);

        // A component that carries its own pixel width and height — an image, a
        // chart — is resized through those, the way Interface Builder resizes a
        // view's frame rather than wrapping it in something. Everything else is
        // resized through the box around it.
        let sized = doc
            .node(node)
            .map(|node| (pixel_prop(node, "width"), pixel_prop(node, "height")))
            .unwrap_or((None, None));

        // Sizes come from what was painted, not from the style: a node sized by
        // its content has no number to start from until it has been drawn once.
        let painted = self.store.read(cx).bounds(node);
        let width = sized
            .0
            .or_else(|| style.width.px())
            .or_else(|| painted.map(|b| f32::from(b.size.width)))
            .unwrap_or(120.0);
        let height = sized
            .1
            .or_else(|| style.height.px())
            .or_else(|| painted.map(|b| f32::from(b.size.height)))
            .unwrap_or(40.0);

        let before = self.project.clone();
        self.history
            .begin(if handle.is_some() { "Resize" } else { "Move" }, &before);
        self.dirty = true;
        self.grab = Some(Grab {
            node,
            handle,
            from,
            x: style.x,
            y: style.y,
            width,
            height,
            absolute,
            sizes_itself: (sized.0.is_some(), sized.1.is_some()),
            moved: false,
        });
        cx.notify();
    }

    /// A drag frame, straight from gpui.
    pub fn on_grab_move(&mut self, event: &DragMoveEvent<GrabDrag>, cx: &mut Context<Self>) {
        let Some(grab) = self.grab else { return };
        if event.drag(cx).node != grab.node {
            return;
        }
        self.apply_grab(event.event.position, cx);
    }

    /// The same frame, from a pointer position. Split out so the arithmetic is
    /// reachable without fabricating a gpui drag event.
    pub fn apply_grab(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(grab) = self.grab else { return };
        let delta_x = f32::from(position.x - grab.from.x);
        let delta_y = f32::from(position.y - grab.from.y);
        if let Some(state) = self.grab.as_mut() {
            state.moved = true;
        }

        let (mut x, mut y, mut width, mut height) = (grab.x, grab.y, grab.width, grab.height);
        match grab.handle {
            // No knob: the body is being dragged, so only the origin moves.
            None => {
                x += delta_x;
                y += delta_y;
            }
            Some(handle) => {
                if handle.horizontal() {
                    if handle.moves_left() {
                        width = (grab.width - delta_x).max(MIN_SIZE);
                        if grab.absolute {
                            x = grab.x + (grab.width - width);
                        }
                    } else {
                        width = (grab.width + delta_x).max(MIN_SIZE);
                    }
                }
                if handle.vertical() {
                    if handle.moves_top() {
                        height = (grab.height - delta_y).max(MIN_SIZE);
                        if grab.absolute {
                            y = grab.y + (grab.height - height);
                        }
                    } else {
                        height = (grab.height + delta_y).max(MIN_SIZE);
                    }
                }
            }
        }

        if self.settings.snap {
            let step = self.settings.grid.max(1.0);
            let round = |value: f32| (value / step).round() * step;
            if grab.handle.is_none() {
                x = round(x);
                y = round(y);
            } else {
                width = round(width).max(MIN_SIZE);
                height = round(height).max(MIN_SIZE);
            }
        }

        let guides = self.snap_to_siblings(&grab, &mut x, &mut y, width, height, cx);
        self.guides = guides;

        let moving = grab.handle.is_none();
        if let Some(node) = self.doc_mut().and_then(|doc| doc.node_mut(grab.node)) {
            if moving {
                if grab.absolute {
                    node.style.x = x;
                    node.style.y = y;
                }
            } else {
                if let Some(handle) = grab.handle {
                    if handle.horizontal() {
                        if grab.sizes_itself.0 {
                            node.set_prop("width", PropValue::Float(width.round() as f64));
                        } else {
                            node.style.width = Dimension::Px(width.round());
                        }
                    }
                    if handle.vertical() {
                        if grab.sizes_itself.1 {
                            node.set_prop("height", PropValue::Float(height.round() as f64));
                        } else {
                            node.style.height = Dimension::Px(height.round());
                        }
                    }
                }
                if grab.absolute {
                    node.style.x = x;
                    node.style.y = y;
                }
            }
        }
        // No `refresh` mid-drag: regenerating the file on every frame is work
        // nobody is reading. The drag's end does it once.
        cx.notify();
    }

    /// Pull the dragged node's edges onto its siblings' when they are close,
    /// and report where it caught so the canvas can draw the line.
    fn snap_to_siblings(
        &self,
        grab: &Grab,
        x: &mut f32,
        y: &mut f32,
        width: f32,
        height: f32,
        cx: &Context<Self>,
    ) -> Vec<Guide> {
        if !self.settings.snap_objects || !grab.absolute {
            return Vec::new();
        }
        let Some(doc) = self.doc() else {
            return Vec::new();
        };
        let Some((parent, _, _)) = doc.parent_of(grab.node) else {
            return Vec::new();
        };
        let store = self.store.read(cx);
        let Some(origin) = store.bounds(parent).map(|b| b.origin) else {
            return Vec::new();
        };
        let siblings: Vec<Bounds<Pixels>> = store.sibling_bounds(doc, parent, grab.node);

        let parent_size = store.bounds(parent).map(|b| b.size);
        let left = f32::from(origin.x);
        let top = f32::from(origin.y);

        // Everything is compared in the parent's coordinate space, which is
        // what x and y are already in. Three candidates per axis per sibling:
        // leading edges, trailing edges, centres.
        let mut best_x: Option<(f32, f32)> = None;
        let mut best_y: Option<(f32, f32)> = None;
        for sibling in &siblings {
            let sx = f32::from(sibling.origin.x) - left;
            let sy = f32::from(sibling.origin.y) - top;
            let sw = f32::from(sibling.size.width);
            let sh = f32::from(sibling.size.height);

            for target in [sx, sx + sw - width, sx + sw / 2.0 - width / 2.0] {
                let distance = (*x - target).abs();
                if distance <= SNAP && best_x.map(|(best, _)| distance < best).unwrap_or(true) {
                    best_x = Some((distance, target));
                }
            }
            for target in [sy, sy + sh - height, sy + sh / 2.0 - height / 2.0] {
                let distance = (*y - target).abs();
                if distance <= SNAP && best_y.map(|(best, _)| distance < best).unwrap_or(true) {
                    best_y = Some((distance, target));
                }
            }
        }

        let mut guides = Vec::new();
        if let Some((_, target)) = best_x {
            *x = target;
            let height = parent_size
                .map(|size| f32::from(size.height))
                .unwrap_or(0.0);
            guides.push(Guide {
                vertical: true,
                at: left + target,
                from: top,
                to: top + height,
            });
        }
        if let Some((_, target)) = best_y {
            *y = target;
            let width = parent_size.map(|size| f32::from(size.width)).unwrap_or(0.0);
            guides.push(Guide {
                vertical: false,
                at: top + target,
                from: left,
                to: left + width,
            });
        }
        guides
    }

    /// The drag ended: one undo step, one regeneration.
    pub fn end_grab(&mut self, cx: &mut Context<Self>) {
        let Some(grab) = self.grab.take() else { return };
        self.history.end();
        self.guides.clear();
        if grab.moved {
            self.refresh(cx);
        } else {
            // A press with no drag behind it: take the snapshot back out.
            self.history.rollback(&mut self.project);
            cx.notify();
        }
    }

    /// What the readout says while a drag is in flight.
    pub fn grab_readout(&self, cx: &Context<Self>) -> Option<(String, Point<Pixels>)> {
        let grab = self.grab?;
        let node = self.doc()?.node(grab.node)?;
        let bounds = self.store.read(cx).bounds(grab.node)?;
        let label = match grab.handle {
            Some(_) => format!(
                "{} × {}",
                f32::from(bounds.size.width).round(),
                f32::from(bounds.size.height).round()
            ),
            None => format!("{}, {}", node.style.x.round(), node.style.y.round()),
        };
        Some((label, bounds.origin))
    }

    /// The guides, drawn over everything in window coordinates.
    pub(super) fn render_guides(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        if self.guides.is_empty() {
            return Vec::new();
        }
        let accent = tailor_render::chrome::accent(cx);
        self.guides
            .iter()
            .map(|guide| {
                let base = div().absolute().bg(accent);
                if guide.vertical {
                    base.left(px(guide.at))
                        .top(px(guide.from))
                        .w(px(1.))
                        .h(px((guide.to - guide.from).max(1.0)))
                } else {
                    base.top(px(guide.at))
                        .left(px(guide.from))
                        .h(px(1.))
                        .w(px((guide.to - guide.from).max(1.0)))
                }
                .into_any_element()
            })
            .collect()
    }

    /// The size or position readout that follows a drag.
    pub(super) fn render_readout(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let (label, origin) = self.grab_readout(cx)?;
        Some(
            div()
                .absolute()
                .left(origin.x)
                .top(origin.y - px(22.))
                .child(tailor_render::chrome::measure(label, cx))
                .into_any_element(),
        )
    }

    /// Arrow keys: move an absolutely placed node, or reorder a flow child.
    pub fn nudge(&mut self, dx: f32, dy: f32, cx: &mut Context<Self>) {
        let Some(id) = self.selection.first().copied() else {
            return;
        };
        let absolute = self
            .doc()
            .and_then(|doc| doc.parent_of(id))
            .map(|(parent, _, _)| {
                self.doc().map(|doc| doc.layout_of(parent)) == Some(LayoutMode::Absolute)
            })
            .unwrap_or(false);

        if !absolute {
            // In a flow container there is no x to nudge, so up and down mean
            // what they mean in the outline.
            if dy < 0.0 {
                self.shift_selection(-1, cx);
            } else if dy > 0.0 {
                self.shift_selection(1, cx);
            }
            return;
        }
        self.edit_style(id, "Nudge", cx, move |style| {
            style.x += dx;
            style.y += dy;
        });
    }
}
