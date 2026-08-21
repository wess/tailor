//! Renders a Tailor document as live guise components.
//!
//! The canvas is not a drawing of your interface — it is your interface. A
//! `Button` on the canvas is a `guise::Button`, reading the same theme, so what
//! you lay out is what the generated code produces.
//!
//! Five components are the exception, and deliberately: `Tabs`, `Accordion`,
//! `SplitPanel`, `AppShell`, and `Carousel` take their regions as `'static`
//! closures, which a designer cannot reach into. Those are drawn here from the
//! theme instead, which is also what lets you click a tab to reveal the slot
//! behind it and drop into it. Generated code uses the real component.
//!
//! Interaction never reaches back into the app directly: everything the canvas
//! needs to hear about arrives through [`Hooks`], which the app builds from a
//! weak handle. That is what keeps a component tree from owning the view that
//! renders it.

pub mod chrome;
pub mod hooks;
pub mod nodes;
pub mod read;
pub mod store;

pub use hooks::{DragPayload, DropSpot, GrabDrag, Handle, Hooks};
pub use store::PreviewStore;

use std::rc::Rc;
use std::sync::Arc;

use gpui::prelude::*;
use gpui::{AnyElement, App, Window};
use tailor_model::{Document, NodeId, Project};

/// How the canvas is drawing right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Real components; clicks select rather than activate.
    Design,
    /// Outlines and names — structure without content.
    Blueprint,
    /// Real components, live: clicks go to the component.
    Preview,
}

impl Mode {
    pub fn interactive(self) -> bool {
        self == Mode::Preview
    }
}

/// Everything a render pass needs. Cheap to clone: the project is shared, the
/// rest is a handful of ids.
#[derive(Clone)]
pub struct RenderCtx {
    pub project: Arc<Project>,
    pub doc_id: String,
    pub mode: Mode,
    pub selected: Rc<Vec<NodeId>>,
    pub hovered: Option<NodeId>,
    /// The node a drag is currently over, and where it would land.
    pub drop: Option<DropSpot>,
    /// The node being dragged, so it can draw itself as the source.
    pub dragging: Option<NodeId>,
    pub store: gpui::Entity<PreviewStore>,
    pub hooks: Hooks,
    /// Outline every node, not only the selected one — the "show layout
    /// bounds" toggle every layout editor has.
    pub outlines: bool,
    /// A placement drag is in flight, so containers should show where a drop
    /// would land. Tracked rather than read from gpui, which only knows that
    /// *some* drag is happening and would light the whole document up during a
    /// resize.
    pub placing: bool,
    /// Depth guard for component references — a screen that places a component
    /// that places the screen would otherwise recurse forever.
    pub depth: usize,
}

impl RenderCtx {
    pub fn doc(&self) -> Option<&Document> {
        self.project.doc(&self.doc_id)
    }

    pub fn is_selected(&self, id: NodeId) -> bool {
        self.selected.contains(&id)
    }

    /// A context for rendering another document inline — a placed component.
    pub fn nested(&self, doc_id: &str) -> RenderCtx {
        let mut ctx = self.clone();
        ctx.doc_id = doc_id.to_string();
        ctx.depth += 1;
        // A nested component is not the document you are editing, so nothing
        // inside it is selectable or droppable.
        ctx.selected = Rc::new(Vec::new());
        ctx.hovered = None;
        ctx.drop = None;
        ctx.hooks = Hooks::inert();
        ctx
    }
}

/// The maximum component-reference depth the canvas will draw. A cycle is
/// refused when it is created, so this only catches a hand-edited file.
pub const MAX_DEPTH: usize = 8;

/// Render a whole document — the artboard's content.
pub fn render_document(ctx: &RenderCtx, window: &mut Window, cx: &mut App) -> AnyElement {
    let Some(doc) = ctx.doc() else {
        return chrome::missing("This document is not in the project").into_any_element();
    };
    let root = doc.root;
    nodes::render(ctx, root, window, cx)
}

/// Render one node and everything under it.
pub fn render_node(ctx: &RenderCtx, id: NodeId, window: &mut Window, cx: &mut App) -> AnyElement {
    nodes::render(ctx, id, window, cx)
}
