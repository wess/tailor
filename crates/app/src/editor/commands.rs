//! Every command the menus, the toolbar, and the canvas dispatch.
//!
//! One rule runs through all of them: take a history snapshot *before* the
//! mutation, then mutate, then `refresh`. Undo is whole-project snapshots, so
//! nothing here has to write an inverse — which is what lets a command like
//! "embed in card" be six lines instead of sixty.

use std::path::PathBuf;
use std::sync::Arc;

use gpui::prelude::*;
use gpui::{ClipboardItem, Context, Window};
use guise::prelude::*;
use tailor_model::node::DEFAULT_SLOT;
use tailor_model::props::PropValue;
use tailor_model::style::{LayoutMode, StyleProps};
use tailor_model::{DocKind, Document, Node, NodeId};
use tailor_render::{DragPayload, DropSpot};
use tailor_store::{CanvasMode, Panel};

use super::{Inspector, Workbench};

impl Workbench {
    // --- selection ------------------------------------------------------

    pub fn select(&mut self, id: NodeId, additive: bool, cx: &mut Context<Self>) {
        if additive {
            if let Some(index) = self.selection.iter().position(|s| *s == id) {
                self.selection.remove(index);
            } else {
                self.selection.push(id);
            }
        } else if self.selection != [id] {
            self.selection = vec![id];
        } else {
            return;
        }
        self.fields.clear();
        self.areas.clear();
        self.reveal_selection();
        cx.notify();
    }

    /// Dismiss the right-click menu. Any command that changes the tree runs
    /// through `refresh`, and a menu still open over a node that just moved is
    /// pointing at the wrong thing.
    pub fn close_menu(&mut self) {
        self.menu = None;
    }

    pub fn select_only(&mut self, id: NodeId, cx: &mut Context<Self>) {
        self.selection = vec![id];
        self.fields.clear();
        self.areas.clear();
        self.reveal_selection();
        cx.notify();
    }

    pub fn clear_selection(&mut self, cx: &mut Context<Self>) {
        if !self.selection.is_empty() {
            self.selection.clear();
            self.fields.clear();
            self.areas.clear();
            cx.notify();
        }
    }

    /// Expand the outline down to whatever is selected, and switch a tabbed
    /// ancestor to the page the selection is on. Selecting something you cannot
    /// see is worse than not selecting it.
    fn reveal_selection(&mut self) {
        let Some(id) = self.selection.first().copied() else {
            return;
        };
        let Some(doc) = self.doc() else { return };
        for ancestor in doc.path_to(id) {
            self.collapsed.remove(&ancestor);
        }
    }

    pub fn select_all(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(doc) = self.doc() else { return };
        let root = doc.root;
        self.selection = doc.children_of(root).to_vec();
        cx.notify();
    }

    pub fn select_parent(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.renaming.is_some() {
            self.renaming = None;
            self.rename_field = None;
            cx.notify();
            return;
        }
        let Some(id) = self.selection.first().copied() else {
            return;
        };
        let Some(doc) = self.doc() else { return };
        match doc.parent_of(id) {
            Some((parent, _, _)) => self.select_only(parent, cx),
            None => self.clear_selection(cx),
        }
    }

    // --- inserting and moving --------------------------------------------

    /// Handle a drop from the palette, the outline, or the canvas.
    pub fn accept_drop(&mut self, spot: DropSpot, payload: DragPayload, cx: &mut Context<Self>) {
        self.drop = None;
        self.placing = false;
        // A resize in flight is not a drop, however gpui routed the event.
        if self.grab.is_some() {
            return;
        }
        match payload {
            DragPayload::New(kind) => self.insert_kind(&kind, spot, cx),
            DragPayload::Component(name) => self.insert_kind(&format!("@{name}"), spot, cx),
            DragPayload::Existing(id) => self.move_to(id, spot, cx),
        }
    }

    /// Place a new node of `kind`.
    pub fn insert_kind(&mut self, kind: &str, spot: DropSpot, cx: &mut Context<Self>) {
        // A component cannot contain itself, however you got there.
        if let Some(name) = kind.strip_prefix('@') {
            let host = self.doc().map(|doc| doc.name.clone()).unwrap_or_default();
            if self.project.would_recurse(&host, name) {
                self.toasts
                    .failed(format!("{name} would contain {host}"), cx);
                return;
            }
        }
        let label = tailor_model::catalog::get(kind)
            .map(|spec| spec.title.to_string())
            .unwrap_or_else(|| kind.trim_start_matches('@').to_string());
        self.commit(&format!("Add {label}"));

        // "Free form" is a placement preference, the way it is in any layout
        // program: new frames lay their children out at explicit x/y. Read
        // before the document is borrowed.
        let free_form = self.settings.free_form && matches!(kind, "frame" | "surface");

        let Some(doc) = self.doc_mut() else { return };
        let node = match tailor_model::catalog::get(kind) {
            Some(spec) => spec.build(doc.ids.next()),
            None => Node::new(doc.ids.next(), kind),
        };
        let mut node = node;
        if free_form {
            node.style.layout = LayoutMode::Absolute;
        }
        if let Some((x, y)) = spot.point {
            node.style.x = x as f32;
            node.style.y = y as f32;
        }
        let id = doc.insert(spot.parent, &spot.slot, spot.index, node);
        self.select_only(id, cx);
        self.refresh(cx);
    }

    /// Move an existing node to a new home.
    pub fn move_to(&mut self, id: NodeId, spot: DropSpot, cx: &mut Context<Self>) {
        self.commit("Move");
        let mut moved = false;
        if let Some(doc) = self.doc_mut() {
            moved = doc.move_node(id, spot.parent, &spot.slot, spot.index);
            if moved {
                if let Some((x, y)) = spot.point {
                    if let Some(node) = doc.node_mut(id) {
                        node.style.x = x as f32;
                        node.style.y = y as f32;
                    }
                }
            }
        }
        if moved {
            self.select_only(id, cx);
            self.refresh(cx);
        } else {
            // Nothing changed, so the snapshot would be a phantom undo step.
            self.history.rollback(&mut self.project);
            self.toasts
                .failed("A node cannot be moved inside itself", cx);
        }
    }

    /// Nudge the selection within its parent.
    pub fn move_up(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.shift_selection(-1, cx);
    }

    pub fn move_down(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.shift_selection(1, cx);
    }

    pub(super) fn shift_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        let Some(id) = self.selection.first().copied() else {
            return;
        };
        let Some((parent, slot, index)) = self.doc().and_then(|doc| doc.parent_of(id)) else {
            return;
        };
        let count = self
            .doc()
            .and_then(|doc| doc.node(parent))
            .map(|n| n.slot(&slot).len())
            .unwrap_or(0);
        let target = index as isize + delta;
        if target < 0 || target >= count as isize {
            return;
        }
        self.commit(if delta < 0 { "Move up" } else { "Move down" });
        // `move_node` accounts for the gap the removal leaves behind, so a
        // forward move addresses the slot *after* the one it is passing.
        let index = if delta > 0 {
            target as usize + 1
        } else {
            target as usize
        };
        if let Some(doc) = self.doc_mut() {
            doc.move_node(id, parent, &slot, index);
        }
        self.refresh(cx);
    }

    // --- clipboard --------------------------------------------------------

    pub fn copy(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(json) = self.selection_json() else {
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(json));
        self.toasts.info("Copied", cx);
    }

    pub fn cut(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(json) = self.selection_json() else {
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(json));
        self.delete_selection(window, cx);
    }

    pub fn paste(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            return;
        };
        let Ok(clip) = serde_json::from_str::<Clip>(&text) else {
            self.toasts
                .failed("The clipboard does not hold Tailor nodes", cx);
            return;
        };
        let target = self
            .selection
            .first()
            .copied()
            .filter(|id| {
                self.doc()
                    .and_then(|doc| doc.node(*id))
                    .and_then(|node| tailor_model::catalog::get(&node.kind))
                    .map(|spec| spec.takes_children())
                    .unwrap_or(false)
            })
            .or_else(|| self.doc().map(|doc| doc.root));
        let Some(parent) = target else { return };

        self.commit("Paste");
        let mut pasted = Vec::new();
        if let Some(doc) = self.doc_mut() {
            for tree in &clip.trees {
                if let Some(id) = graft(doc, tree, parent) {
                    pasted.push(id);
                }
            }
        }
        if pasted.is_empty() {
            self.history.rollback(&mut self.project);
            return;
        }
        self.selection = pasted;
        self.fields.clear();
        self.areas.clear();
        self.refresh(cx);
    }

    pub fn duplicate(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.selection.is_empty() {
            return;
        }
        self.commit("Duplicate");
        let ids = self.selection.clone();
        let mut copies = Vec::new();
        if let Some(doc) = self.doc_mut() {
            for id in ids {
                if let Some(copy) = doc.duplicate(id) {
                    copies.push(copy);
                }
            }
        }
        if copies.is_empty() {
            self.history.rollback(&mut self.project);
            return;
        }
        self.selection = copies;
        self.fields.clear();
        self.areas.clear();
        self.refresh(cx);
    }

    pub fn delete_selection(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.selection.is_empty() || self.renaming.is_some() {
            return;
        }
        self.commit("Delete");
        let ids = self.selection.clone();
        let mut next = None;
        if let Some(doc) = self.doc_mut() {
            for id in &ids {
                if next.is_none() {
                    next = doc.parent_of(*id).map(|(parent, _, _)| parent);
                }
                doc.remove(*id);
            }
        }
        self.selection = next.into_iter().collect();
        self.fields.clear();
        self.areas.clear();
        self.refresh(cx);
    }

    fn selection_json(&self) -> Option<String> {
        let doc = self.doc()?;
        let mut trees = Vec::new();
        for id in &self.selection {
            trees.push(harvest(doc, *id)?);
        }
        (!trees.is_empty())
            .then(|| serde_json::to_string(&Clip { trees }).ok())
            .flatten()
    }

    // --- structure --------------------------------------------------------

    pub fn embed_frame(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.embed("frame", cx);
    }

    pub fn embed_stack(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.embed("stack", cx);
    }

    pub fn embed_card(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.embed("card", cx);
    }

    pub fn embed_scroll(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.embed("scrollarea", cx);
    }

    /// Wrap the selection in a new container, in place. Interface Builder's
    /// "Embed In", and the fastest way to restructure a layout without
    /// dragging anything.
    pub fn embed(&mut self, kind: &str, cx: &mut Context<Self>) {
        if self.selection.is_empty() {
            return;
        }
        let label = tailor_model::catalog::get(kind)
            .map(|spec| spec.title)
            .unwrap_or(kind);
        self.commit(&format!("Embed in {label}"));

        let ids = self.selection.clone();
        let mut wrapper_id = None;
        if let Some(doc) = self.doc_mut() {
            let Some((parent, slot, index)) = doc.parent_of(ids[0]) else {
                return;
            };
            let wrapper = match tailor_model::catalog::get(kind) {
                Some(spec) => spec.build(doc.ids.next()),
                None => Node::new(doc.ids.next(), kind),
            };
            let id = doc.insert(parent, &slot, index, wrapper);
            for (offset, child) in ids.iter().enumerate() {
                doc.move_node(*child, id, DEFAULT_SLOT, offset);
            }
            wrapper_id = Some(id);
        }
        if let Some(id) = wrapper_id {
            self.select_only(id, cx);
            self.refresh(cx);
        }
    }

    /// The opposite: lift a container's children into its parent and drop the
    /// container.
    pub fn unwrap_selection(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.selection.first().copied() else {
            return;
        };
        let Some((parent, slot, index)) = self.doc().and_then(|doc| doc.parent_of(id)) else {
            return;
        };
        let children = self
            .doc()
            .map(|doc| doc.children_of(id).to_vec())
            .unwrap_or_default();
        if children.is_empty() {
            self.toasts.info("Nothing inside that to lift out", cx);
            return;
        }
        self.commit("Unwrap");
        if let Some(doc) = self.doc_mut() {
            for (offset, child) in children.iter().enumerate() {
                doc.move_node(*child, parent, &slot, index + offset);
            }
            doc.remove(id);
        }
        self.selection = children;
        self.fields.clear();
        self.areas.clear();
        self.refresh(cx);
    }

    // --- alignment --------------------------------------------------------

    pub fn align_left(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        self.align(Edge::Left, cx);
    }

    pub fn align_center_h(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        self.align(Edge::CenterH, cx);
    }

    pub fn align_right(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        self.align(Edge::Right, cx);
    }

    pub fn align_top(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        self.align(Edge::Top, cx);
    }

    pub fn align_middle(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        self.align(Edge::Middle, cx);
    }

    pub fn align_bottom(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        self.align(Edge::Bottom, cx);
    }

    /// Align the selection. In an absolute container that is arithmetic on
    /// x/y; in a flow container there is nothing to move, so it sets the
    /// parent's alignment instead — which is what you actually meant.
    fn align(&mut self, edge: Edge, cx: &mut Context<Self>) {
        let Some(first) = self.selection.first().copied() else {
            return;
        };
        let Some((parent, _, _)) = self.doc().and_then(|doc| doc.parent_of(first)) else {
            return;
        };
        let absolute = self.doc().map(|doc| doc.layout_of(parent)) == Some(LayoutMode::Absolute);
        self.commit("Align");

        if !absolute {
            if let Some(node) = self.doc_mut().and_then(|doc| doc.node_mut(parent)) {
                let row = node.style.direction == tailor_model::Direction::Row;
                apply_flow_alignment(&mut node.style, edge, row);
            }
            self.refresh(cx);
            return;
        }

        let ids = self.selection.clone();
        let boxes: Vec<(NodeId, f32, f32, f32, f32)> = ids
            .iter()
            .filter_map(|id| {
                let node = self.doc()?.node(*id)?;
                let width = node.style.width.px().unwrap_or(0.0);
                let height = node.style.height.px().unwrap_or(0.0);
                Some((*id, node.style.x, node.style.y, width, height))
            })
            .collect();
        if boxes.len() < 2 {
            self.history.rollback(&mut self.project);
            self.toasts
                .info("Select two or more nodes to align them", cx);
            return;
        }
        let target = edge.target(&boxes);
        for (id, _, _, width, height) in &boxes {
            let Some(node) = self.doc_mut().and_then(|doc| doc.node_mut(*id)) else {
                continue;
            };
            match edge {
                Edge::Left => node.style.x = target,
                Edge::CenterH => node.style.x = target - width / 2.0,
                Edge::Right => node.style.x = target - width,
                Edge::Top => node.style.y = target,
                Edge::Middle => node.style.y = target - height / 2.0,
                Edge::Bottom => node.style.y = target - height,
            }
        }
        self.refresh(cx);
    }

    pub fn distribute_h(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        self.distribute(true, cx);
    }

    pub fn distribute_v(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        self.distribute(false, cx);
    }

    /// Even gaps between three or more absolutely placed nodes.
    fn distribute(&mut self, horizontal: bool, cx: &mut Context<Self>) {
        let ids = self.selection.clone();
        if ids.len() < 3 {
            self.toasts
                .info("Select three or more nodes to distribute them", cx);
            return;
        }
        let mut boxes: Vec<(NodeId, f32)> = ids
            .iter()
            .filter_map(|id| {
                let node = self.doc()?.node(*id)?;
                Some((
                    *id,
                    if horizontal {
                        node.style.x
                    } else {
                        node.style.y
                    },
                ))
            })
            .collect();
        boxes.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let (Some(first), Some(last)) = (boxes.first().map(|b| b.1), boxes.last().map(|b| b.1))
        else {
            return;
        };
        let step = (last - first) / (boxes.len() - 1) as f32;

        self.commit("Distribute");
        for (index, (id, _)) in boxes.iter().enumerate() {
            let Some(node) = self.doc_mut().and_then(|doc| doc.node_mut(*id)) else {
                continue;
            };
            let value = first + step * index as f32;
            if horizontal {
                node.style.x = value;
            } else {
                node.style.y = value;
            }
        }
        self.refresh(cx);
    }

    // --- props and style ---------------------------------------------------

    pub fn set_prop(&mut self, id: NodeId, key: &str, value: PropValue, cx: &mut Context<Self>) {
        let current = self
            .doc()
            .and_then(|doc| doc.node(id))
            .and_then(|node| node.prop(key).cloned());
        if current.as_ref() == Some(&value) {
            return;
        }
        let before = self.project.clone();
        self.history.commit_run(format!("Set {key}"), &before);
        self.dirty = true;
        if let Some(node) = self.doc_mut().and_then(|doc| doc.node_mut(id)) {
            node.set_prop(key, value);
        }
        self.refresh(cx);
    }

    /// Edit a node's style. The closure keeps the borrow short, which matters:
    /// `refresh` needs the whole workbench back.
    pub fn edit_style(
        &mut self,
        id: NodeId,
        label: &str,
        cx: &mut Context<Self>,
        f: impl FnOnce(&mut StyleProps),
    ) {
        let before = self.project.clone();
        self.history.commit_run(label, &before);
        self.dirty = true;
        if let Some(node) = self.doc_mut().and_then(|doc| doc.node_mut(id)) {
            f(&mut node.style);
        }
        self.refresh(cx);
    }

    /// Edit a node itself — name, events, lock, hidden.
    pub fn edit_node(
        &mut self,
        id: NodeId,
        label: &str,
        cx: &mut Context<Self>,
        f: impl FnOnce(&mut Node),
    ) {
        self.commit(label);
        if let Some(node) = self.doc_mut().and_then(|doc| doc.node_mut(id)) {
            f(node);
        }
        self.refresh(cx);
    }

    /// Edit the open document — state variables, actions, canvas, name.
    pub fn edit_doc(&mut self, label: &str, cx: &mut Context<Self>, f: impl FnOnce(&mut Document)) {
        self.commit(label);
        if let Some(doc) = self.doc_mut() {
            f(doc);
        }
        self.refresh(cx);
    }

    // --- history -----------------------------------------------------------

    pub fn undo(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        match self.history.undo(&mut self.project) {
            Some(label) => {
                self.after_history(cx);
                self.toasts.info(format!("Undo {label}"), cx);
            }
            None => self.toasts.info("Nothing to undo", cx),
        }
    }

    pub fn redo(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        match self.history.redo(&mut self.project) {
            Some(label) => {
                self.after_history(cx);
                self.toasts.info(format!("Redo {label}"), cx);
            }
            None => self.toasts.info("Nothing to redo", cx),
        }
    }

    fn after_history(&mut self, cx: &mut Context<Self>) {
        // The document the tab was on may not exist in the restored project.
        if self.project.doc(&self.doc_id).is_none() {
            self.doc_id = self
                .project
                .docs
                .first()
                .map(|doc| doc.id.clone())
                .unwrap_or_default();
        }
        let live: Vec<NodeId> = self
            .selection
            .iter()
            .copied()
            .filter(|id| {
                self.doc()
                    .map(|doc| doc.node(*id).is_some())
                    .unwrap_or(false)
            })
            .collect();
        self.selection = live;
        self.fields.clear();
        self.areas.clear();
        self.dirty = true;
        self.refresh(cx);
    }

    // --- documents ----------------------------------------------------------

    pub fn new_screen(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.add_document(DocKind::Screen, cx);
    }

    pub fn new_component(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.add_document(DocKind::Component, cx);
    }

    pub fn add_document(&mut self, kind: DocKind, cx: &mut Context<Self>) {
        let base = if kind == DocKind::Screen {
            "Screen"
        } else {
            "Component"
        };
        self.commit(&format!("New {}", base.to_lowercase()));
        let name = self.project.unique_doc_name(base);
        let id = self.project.unique_doc_id(&tailor_model::snake_case(&name));
        Arc::make_mut(&mut self.project)
            .docs
            .push(Document::new(id.clone(), name, kind));
        self.open_document(&id, cx);
    }

    pub fn open_document(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.doc_id == id {
            return;
        }
        self.doc_id = id.to_string();
        self.selection.clear();
        self.fields.clear();
        self.areas.clear();
        self.collapsed.clear();
        self.store.update(cx, |store, _| store.clear());
        self.refresh(cx);
    }

    /// Copy a document, its tree and all, under a fresh name. A component's
    /// name is how screens reference it, so the copy taking a new one is what
    /// keeps the references pointing at the original.
    pub fn duplicate_document(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(source) = self.project.doc(id).cloned() else {
            return;
        };
        self.commit("Duplicate document");
        let name = self.project.unique_doc_name(&source.name);
        let new_id = self.project.unique_doc_id(&tailor_model::snake_case(&name));
        let mut copy = source;
        copy.id = new_id.clone();
        copy.name = name;
        Arc::make_mut(&mut self.project).docs.push(copy);
        self.open_document(&new_id, cx);
    }

    /// Open a document and put the cursor in its name field. The field belongs
    /// to the inspector, which has to render once before it exists — hence the
    /// pending key rather than a focus call here.
    pub fn begin_rename_document(&mut self, id: &str, cx: &mut Context<Self>) {
        self.open_document(id, cx);
        self.selection.clear();
        if !self.settings.is_open(Panel::Inspector) {
            self.toggle_panel(Panel::Inspector, cx);
        }
        self.focus_field = Some((format!("doc/{id}/name"), 0));
        cx.notify();
    }

    pub fn close_document(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.project.docs.len() <= 1 {
            self.toasts
                .info("A project needs at least one document", cx);
            return;
        }
        self.commit("Delete document");
        Arc::make_mut(&mut self.project)
            .docs
            .retain(|doc| doc.id != id);
        if self.doc_id == id {
            let next = self
                .project
                .docs
                .first()
                .map(|doc| doc.id.clone())
                .unwrap_or_default();
            self.doc_id = next;
            self.selection.clear();
            self.store.update(cx, |store, _| store.clear());
        }
        self.refresh(cx);
    }

    // --- view ----------------------------------------------------------------

    pub fn mode_design(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        self.set_mode(CanvasMode::Design, cx);
    }

    pub fn mode_blueprint(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        self.set_mode(CanvasMode::Blueprint, cx);
    }

    pub fn mode_split(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        self.set_mode(CanvasMode::Split, cx);
    }

    pub fn mode_preview(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        self.set_mode(CanvasMode::Preview, cx);
    }

    pub fn set_mode(&mut self, mode: CanvasMode, cx: &mut Context<Self>) {
        self.settings.canvas_mode = mode;
        self.save_settings();
        if mode == CanvasMode::Preview {
            self.hovered = None;
        }
        cx.notify();
    }

    pub fn toggle_palette(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        self.toggle_panel(Panel::Palette, cx);
    }

    pub fn toggle_outline(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        self.toggle_panel(Panel::Outline, cx);
    }

    pub fn toggle_inspector(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        self.toggle_panel(Panel::Inspector, cx);
    }

    pub fn toggle_problems(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        self.toggle_panel(Panel::Problems, cx);
    }

    /// Arrow keys. The big step follows the grid when snapping is on, so
    /// shift-arrow lands where a drag would.
    fn step(&self) -> f32 {
        if self.settings.snap {
            self.settings.grid.max(1.0)
        } else {
            10.0
        }
    }

    pub fn nudge_left(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        let step = self.settings.nudge;
        self.nudge(-step, 0.0, cx);
    }

    pub fn nudge_right(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        let step = self.settings.nudge;
        self.nudge(step, 0.0, cx);
    }

    pub fn nudge_up(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        let step = self.settings.nudge;
        self.nudge(0.0, -step, cx);
    }

    pub fn nudge_down(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        let step = self.settings.nudge;
        self.nudge(0.0, step, cx);
    }

    pub fn nudge_left_big(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        let step = self.step();
        self.nudge(-step, 0.0, cx);
    }

    pub fn nudge_right_big(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        let step = self.step();
        self.nudge(step, 0.0, cx);
    }

    pub fn nudge_up_big(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        let step = self.step();
        self.nudge(0.0, -step, cx);
    }

    pub fn nudge_down_big(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        let step = self.step();
        self.nudge(0.0, step, cx);
    }

    /// Snap to siblings' edges and centres.
    pub fn toggle_snap_objects(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        self.settings.snap_objects = !self.settings.snap_objects;
        self.save_settings();
        cx.notify();
    }

    /// Whether a new frame lays its children out at explicit x/y.
    pub fn toggle_free_form(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        self.settings.free_form = !self.settings.free_form;
        self.save_settings();
        let mode = if self.settings.free_form {
            "free form"
        } else {
            "flow"
        };
        self.toasts.info(format!("New frames are {mode}"), cx);
    }

    /// Flip the selected container between flow and free form — the command a
    /// layout program puts on the frame itself, not only in preferences.
    pub fn toggle_selection_layout(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.selection.first().copied() else {
            return;
        };
        let container = self
            .doc()
            .and_then(|doc| doc.node(id))
            .and_then(|node| tailor_model::catalog::get(&node.kind))
            .map(|spec| spec.takes_children())
            .unwrap_or(false);
        if !container {
            self.toasts.info("That does not hold children", cx);
            return;
        }
        self.edit_style(id, "Layout", cx, |style| {
            style.layout = match style.layout {
                LayoutMode::Flow => LayoutMode::Absolute,
                LayoutMode::Absolute => LayoutMode::Flow,
            };
        });
    }

    /// Show every node's box, not only the selected one.
    pub fn toggle_outlines(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        self.settings.show_outlines = !self.settings.show_outlines;
        self.save_settings();
        cx.notify();
    }

    pub fn toggle_grid(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        self.settings.show_grid = !self.settings.show_grid;
        self.save_settings();
        cx.notify();
    }

    pub fn toggle_snap(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        self.settings.snap = !self.settings.snap;
        self.save_settings();
        cx.notify();
    }

    pub fn toggle_orientation(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        self.landscape = !self.landscape;
        let landscape = self.landscape;
        self.edit_doc("Rotate", cx, move |doc| {
            let (width, height) = (doc.canvas.width, doc.canvas.height);
            let portrait = height >= width;
            if portrait == landscape {
                doc.canvas.width = height;
                doc.canvas.height = width;
            }
        });
    }

    pub fn set_preset(&mut self, preset: &str, cx: &mut Context<Self>) {
        let Some((_, width, height)) = tailor_model::PRESETS
            .iter()
            .find(|(name, _, _)| *name == preset)
            .copied()
        else {
            return;
        };
        let landscape = self.landscape;
        let preset = preset.to_string();
        self.edit_doc("Device", cx, move |doc| {
            doc.canvas.preset = preset;
            if landscape && height > width {
                doc.canvas.width = height;
                doc.canvas.height = width;
            } else {
                doc.canvas.width = width;
                doc.canvas.height = height;
            }
        });
    }

    pub fn set_inspector(&mut self, tab: Inspector, cx: &mut Context<Self>) {
        self.inspector = tab;
        cx.notify();
    }

    pub fn open_settings(&mut self, _w: &mut Window, cx: &mut Context<Self>) {
        self.toggle_settings(cx);
    }

    // --- renaming -------------------------------------------------------------

    pub fn begin_rename(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.selection.first().copied() else {
            return;
        };
        let current = self
            .doc()
            .and_then(|doc| doc.node(id))
            .map(tailor_render::nodes::label_of)
            .unwrap_or_default();
        let field = cx.new(|cx| TextInput::new(cx).value(&current).size(Size::Sm));
        let sub = cx.subscribe(
            &field,
            move |this: &mut Workbench, _, event: &TextInputEvent, cx| {
                if let TextInputEvent::Submit(value) = event {
                    this.finish_rename(Some(value.clone()), cx);
                }
            },
        );
        self.subs.push(sub);
        self.rename_field = Some(field);
        self.renaming = Some(id);
        cx.notify();
    }

    pub fn finish_rename(&mut self, value: Option<String>, cx: &mut Context<Self>) {
        let Some(id) = self.renaming.take() else {
            return;
        };
        self.rename_field = None;
        if let Some(value) = value {
            let trimmed = value.trim().to_string();
            self.edit_node(id, "Rename", cx, move |node| {
                node.name = (!trimmed.is_empty()).then_some(trimmed);
            });
        } else {
            cx.notify();
        }
    }

    // --- files ------------------------------------------------------------------

    pub fn save(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        match self.path.clone() {
            Some(path) => self.write(path, cx),
            None => self.save_as(_window, cx),
        }
    }

    pub fn save_as(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let directory = self
            .path
            .as_ref()
            .and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
            .unwrap_or_else(tailor_store::default_project_dir);
        let suggested = tailor_model::snake_case(&self.project.name);
        let receiver = cx.prompt_for_new_path(&directory, Some(&suggested));
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(path))) = receiver.await {
                this.update(cx, |this, cx| {
                    let path = tailor_store::with_extension(path);
                    this.write(path, cx);
                })
                .ok();
            }
        })
        .detach();
    }

    fn write(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        match tailor_store::save(&path, &self.project) {
            Ok(()) => {
                let mut recents = tailor_store::Recents::load();
                recents.touch(&path, &self.project.name);
                recents.save();
                self.path = Some(path);
                self.dirty = false;
                self.warned_about_file = false;
                self.mark_file_seen();
                self.toasts.done("Project saved", cx);
                cx.notify();
            }
            Err(err) => self.toasts.failed(format!("Could not save: {err}"), cx),
        }
    }

    pub fn export_code(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Export".into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = receiver.await else {
                return;
            };
            let Some(root) = paths.into_iter().next() else {
                return;
            };
            let Ok(project) = this.update(cx, |this, _| Arc::clone(&this.project)) else {
                return;
            };

            // Generating every document and writing a dozen files is the
            // longest thing Tailor does. It has no business on the main thread
            // — the window should still be drawing while it runs.
            let target = root.clone();
            let report = cx
                .background_executor()
                .spawn(async move { tailor_store::export(&target, &project) })
                .await;

            this.update(cx, |this, cx| {
                if report.ok() {
                    // Remember it, so *Open in Zed* knows which file on disk a
                    // node is in without asking again.
                    let directory = root.to_string_lossy().to_string();
                    if this.project.gen.export_dir.as_deref() != Some(directory.as_str()) {
                        Arc::make_mut(&mut this.project).gen.export_dir = Some(directory);
                        this.dirty = true;
                    }
                    // And the other direction: which project owns these files,
                    // so an editor can ask for the component behind a line.
                    if let Some(path) = this.path.clone() {
                        tailor_store::ExportIndex::record(&root, &path);
                    }
                    this.toasts.done(
                        format!("Exported {} to {}", report.summary(), root.display()),
                        cx,
                    );
                    for note in report.notes.iter().take(2) {
                        this.toasts.info(note.clone(), cx);
                    }
                    // `reveal_path` is the platform call; a file:// URL built by
                    // hand breaks on the first space in a path.
                    cx.reveal_path(&root);
                } else {
                    this.toasts
                        .failed(format!("Export failed: {}", report.summary()), cx);
                }
            })
            .ok();
        })
        .detach();
    }

    /// Open the selected node's generated line in Zed.
    ///
    /// Zed extensions cannot draw, so the editor cannot host Tailor — but its
    /// CLI takes `path:line:column`, which is the whole of the jump. The line
    /// comes from the map the generator builds while it writes the file.
    pub fn open_in_editor(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.selection.first().copied() else {
            self.toasts.info("Select a component first", cx);
            return;
        };
        let Some(directory) = self.project.gen.export_dir.clone() else {
            self.toasts.info(
                "Export the project first — that is what creates the file",
                cx,
            );
            return;
        };
        let Some(doc) = self.doc() else { return };

        let generated = tailor_codegen::document(&self.project, doc);
        let Some(line) = generated.lines.get(&id).copied() else {
            self.toasts
                .info("That node does not appear in the generated file", cx);
            return;
        };
        let path = PathBuf::from(&directory)
            .join("src")
            .join(&self.project.gen.module)
            .join(&generated.path);

        if !path.exists() {
            self.toasts.failed(
                format!("{} is not there — export again?", path.display()),
                cx,
            );
            return;
        }

        match open_in_zed(&path, line) {
            Ok(()) => self
                .toasts
                .info(format!("{}:{line} in Zed", generated.path), cx),
            Err(err) => self.toasts.failed(err, cx),
        }
    }

    pub fn copy_code(&mut self, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(self.generated.clone()));
        self.toasts.info("Copied the generated component", cx);
    }
}

/// Which way an align command pushes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Edge {
    Left,
    CenterH,
    Right,
    Top,
    Middle,
    Bottom,
}

impl Edge {
    /// The coordinate everything moves to.
    fn target(self, boxes: &[(NodeId, f32, f32, f32, f32)]) -> f32 {
        let value = |b: &(NodeId, f32, f32, f32, f32)| match self {
            Edge::Left => b.1,
            Edge::CenterH => b.1 + b.3 / 2.0,
            Edge::Right => b.1 + b.3,
            Edge::Top => b.2,
            Edge::Middle => b.2 + b.4 / 2.0,
            Edge::Bottom => b.2 + b.4,
        };
        match self {
            // Left and top pull to the nearest edge; right and bottom to the
            // furthest; the centres to the average of the two extremes.
            Edge::Left | Edge::Top => boxes.iter().map(value).fold(f32::MAX, f32::min),
            Edge::Right | Edge::Bottom => boxes.iter().map(value).fold(f32::MIN, f32::max),
            Edge::CenterH | Edge::Middle => {
                let min = boxes.iter().map(value).fold(f32::MAX, f32::min);
                let max = boxes.iter().map(value).fold(f32::MIN, f32::max);
                (min + max) / 2.0
            }
        }
    }
}

/// In a flow container, aligning the selection means aligning the container.
fn apply_flow_alignment(style: &mut StyleProps, edge: Edge, row: bool) {
    use tailor_model::{AlignToken, JustifyToken};
    let cross = |style: &mut StyleProps, value: AlignToken| style.align = Some(value);
    let main = |style: &mut StyleProps, value: JustifyToken| style.justify = Some(value);
    match (edge, row) {
        (Edge::Left, true) => main(style, JustifyToken::Start),
        (Edge::CenterH, true) => main(style, JustifyToken::Center),
        (Edge::Right, true) => main(style, JustifyToken::End),
        (Edge::Top, true) => cross(style, AlignToken::Start),
        (Edge::Middle, true) => cross(style, AlignToken::Center),
        (Edge::Bottom, true) => cross(style, AlignToken::End),
        (Edge::Left, false) => cross(style, AlignToken::Start),
        (Edge::CenterH, false) => cross(style, AlignToken::Center),
        (Edge::Right, false) => cross(style, AlignToken::End),
        (Edge::Top, false) => main(style, JustifyToken::Start),
        (Edge::Middle, false) => main(style, JustifyToken::Center),
        (Edge::Bottom, false) => main(style, JustifyToken::End),
    }
}

/// A copied subtree. Ids are kept so the shape survives; `graft` renumbers.
#[derive(serde::Serialize, serde::Deserialize)]
struct Clip {
    trees: Vec<Tree>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Tree {
    node: Node,
    children: Vec<Tree>,
}

fn harvest(doc: &Document, id: NodeId) -> Option<Tree> {
    let node = doc.node(id)?.clone();
    let children = node
        .slots
        .values()
        .flatten()
        .filter_map(|child| harvest(doc, *child))
        .collect();
    Some(Tree { node, children })
}

/// Paste a harvested tree under `parent`, renumbering as it goes.
fn graft(doc: &mut Document, tree: &Tree, parent: NodeId) -> Option<NodeId> {
    let mut node = tree.node.clone();
    let old_slots = node.slots.clone();
    node.id = doc.ids.next();
    node.slots.clear();
    let id = doc.insert(parent, DEFAULT_SLOT, usize::MAX, node);

    // Re-hang the children slot by slot, so a panel's footer stays its footer.
    let mut queue: Vec<&Tree> = tree.children.iter().collect();
    for (slot, old_children) in old_slots {
        for _ in old_children {
            let Some(child) = queue.first().copied() else {
                break;
            };
            queue.remove(0);
            if let Some(child_id) = graft(doc, child, id) {
                if slot != DEFAULT_SLOT {
                    if let Some(node) = doc.node_mut(id) {
                        node.detach(child_id);
                        node.slot_mut(&slot).push(child_id);
                    }
                }
            }
        }
    }
    Some(id)
}

/// Hand a file and a line to Zed.
///
/// The `zed` CLI takes `path:line:column`, so the jump is one spawn. It is not
/// always on a GUI app's `$PATH` — a bundle launched from Finder inherits a
/// minimal one — so the app's own copy is the fallback, and it is where the CLI
/// lives on every macOS install.
fn open_in_zed(path: &std::path::Path, line: usize) -> Result<(), String> {
    const BUNDLED_CLI: &str = "/Applications/Zed.app/Contents/MacOS/cli";

    let target = format!("{}:{line}:1", path.display());
    let mut candidates: Vec<&str> = vec!["zed"];
    if std::path::Path::new(BUNDLED_CLI).exists() {
        candidates.push(BUNDLED_CLI);
    }

    let mut last = String::from("could not find the `zed` CLI");
    for candidate in candidates {
        match std::process::Command::new(candidate).arg(&target).spawn() {
            Ok(_) => return Ok(()),
            Err(err) => last = format!("{candidate}: {err}"),
        }
    }
    Err(format!("Could not open Zed — {last}"))
}
