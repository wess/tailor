//! How the canvas hears about what happened inside a rendered component.
//!
//! Every callback is an `Rc<dyn Fn>` the app builds from a *weak* handle to its
//! canvas view. Weak matters: a live guise component can outlive a single
//! render, and a strong handle in one of these closures would make the view own
//! the tree that owns the closure that owns the view.

use std::rc::Rc;

use gpui::{App, Pixels, Point, Window};
use tailor_model::NodeId;

/// Where a drop would land.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DropSpot {
    /// The container that would receive the node.
    pub parent: NodeId,
    pub slot: String,
    /// The index within that slot.
    pub index: usize,
    /// An absolute-layout parent drops at a point rather than at an index.
    pub point: Option<(i32, i32)>,
}

impl DropSpot {
    pub fn at(parent: NodeId, slot: impl Into<String>, index: usize) -> Self {
        DropSpot {
            parent,
            slot: slot.into(),
            index,
            point: None,
        }
    }
}

/// One of the eight knobs around a selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handle {
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}

impl Handle {
    pub const ALL: &'static [Handle] = &[
        Handle::NorthWest,
        Handle::North,
        Handle::NorthEast,
        Handle::East,
        Handle::SouthEast,
        Handle::South,
        Handle::SouthWest,
        Handle::West,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Handle::North => "n",
            Handle::South => "s",
            Handle::East => "e",
            Handle::West => "w",
            Handle::NorthEast => "ne",
            Handle::NorthWest => "nw",
            Handle::SouthEast => "se",
            Handle::SouthWest => "sw",
        }
    }

    /// Does dragging this knob change the width?
    pub fn horizontal(self) -> bool {
        !matches!(self, Handle::North | Handle::South)
    }

    /// Does dragging it change the height?
    pub fn vertical(self) -> bool {
        !matches!(self, Handle::East | Handle::West)
    }

    /// Knobs on the leading edges move the node's origin as well as its size.
    pub fn moves_left(self) -> bool {
        matches!(self, Handle::West | Handle::NorthWest | Handle::SouthWest)
    }

    pub fn moves_top(self) -> bool {
        matches!(self, Handle::North | Handle::NorthWest | Handle::NorthEast)
    }
}

/// Dragging on the canvas itself: a resize knob when there is a handle, the
/// node's own body when there is not. The workbench records where the drag
/// started when the mouse goes down, so this only has to say what was grabbed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrabDrag {
    pub node: NodeId,
    pub handle: Option<Handle>,
}

/// What is being dragged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DragPayload {
    /// A new component from the palette.
    New(String),
    /// A node already in the document.
    Existing(NodeId),
    /// A component from this project, placed by name.
    Component(String),
}

impl DragPayload {
    pub fn label(&self) -> String {
        match self {
            DragPayload::New(kind) => tailor_model::catalog::get(kind)
                .map(|spec| spec.title.to_string())
                .unwrap_or_else(|| kind.clone()),
            DragPayload::Existing(id) => format!("Node {id}"),
            DragPayload::Component(name) => name.clone(),
        }
    }
}

type Select = Rc<dyn Fn(NodeId, bool, &mut Window, &mut App)>;
type Hover = Rc<dyn Fn(Option<NodeId>, &mut App)>;
type Drop = Rc<dyn Fn(DropSpot, DragPayload, &mut Window, &mut App)>;
type Over = Rc<dyn Fn(Option<DropSpot>, &mut App)>;
type Reveal = Rc<dyn Fn(NodeId, usize, &mut App)>;
type Context = Rc<dyn Fn(Option<NodeId>, Point<Pixels>, &mut Window, &mut App)>;
type Grab = Rc<dyn Fn(NodeId, Option<Handle>, Point<Pixels>, &mut App)>;
type Place = Rc<dyn Fn(&mut App)>;

/// The canvas's ears. `inert` is the version a nested component gets: it draws,
/// but nothing inside it is yours to select.
#[derive(Clone)]
pub struct Hooks {
    pub select: Select,
    pub hover: Hover,
    pub drop: Drop,
    pub over: Over,
    /// Show a different page of a tabbed or sectioned container.
    pub reveal: Reveal,
    /// Right-click: open the node's menu at a window-coordinate point. `None`
    /// is the canvas itself, which still has paste and select-all to offer.
    pub context: Context,
    /// A resize knob or an absolutely placed node was pressed. `Some(handle)`
    /// starts a resize, `None` starts a move; either way the workbench records
    /// where the pointer was so the drag can work in deltas.
    pub grab: Grab,
    /// A drag that will *place* a node has begun — from the library, the
    /// outline, or a node's own body. Only these put drop strips between
    /// children; a resize must not, or every container in the document changes
    /// size under the pointer you are dragging with.
    pub place: Place,
    pub live: bool,
}

impl Hooks {
    pub fn inert() -> Self {
        Hooks {
            select: Rc::new(|_, _, _, _| {}),
            hover: Rc::new(|_, _| {}),
            drop: Rc::new(|_, _, _, _| {}),
            over: Rc::new(|_, _| {}),
            reveal: Rc::new(|_, _, _| {}),
            context: Rc::new(|_, _, _, _| {}),
            grab: Rc::new(|_, _, _, _| {}),
            place: Rc::new(|_| {}),
            live: false,
        }
    }
}

impl std::fmt::Debug for Hooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Hooks").field("live", &self.live).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_drop_spot_defaults_to_an_index() {
        let spot = DropSpot::at(NodeId(1), "children", 2);
        assert_eq!(spot.index, 2);
        assert!(spot.point.is_none());
    }

    #[test]
    fn payload_labels_read_from_the_catalog() {
        assert_eq!(DragPayload::New("button".into()).label(), "Button");
        assert_eq!(DragPayload::New("nope".into()).label(), "nope");
        assert_eq!(
            DragPayload::Component("StatCard".into()).label(),
            "StatCard"
        );
    }
}
