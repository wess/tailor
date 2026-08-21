//! A single screen or component: its node arena, its state, and its artboard.
//!
//! The tree is an arena keyed by [`NodeId`] rather than nested structs, because
//! every operation the builder performs — select, reparent, drag between two
//! branches, delete a subtree — is a lookup by id, and nesting would turn each
//! of those into a recursive search with a borrow problem at the end of it.

use crate::id::{IdGen, NodeId};
use crate::node::{Node, DEFAULT_SLOT};
use crate::state::{ActionDef, StateVar};
use crate::style::LayoutMode;
use crate::tokens::ColorSpec;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// What a document becomes when it is generated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocKind {
    /// A `Render` entity that owns its state — a window's root, a page.
    #[default]
    Screen,
    /// A stateless `RenderOnce` builder other documents can place.
    Component,
}

impl DocKind {
    pub const ALL: &'static [DocKind] = &[DocKind::Screen, DocKind::Component];

    pub fn label(self) -> &'static str {
        match self {
            DocKind::Screen => "screen",
            DocKind::Component => "component",
        }
    }
}

/// The artboard a document is designed against. Not part of the generated
/// component — a screen renders at whatever size its window gives it — but the
/// size you were looking at when you laid it out is worth keeping.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Canvas {
    pub width: f32,
    pub height: f32,
    /// The preset this size came from, so the toolbar can show it selected.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub preset: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<ColorSpec>,
}

impl Default for Canvas {
    fn default() -> Self {
        Canvas {
            width: 960.0,
            height: 640.0,
            preset: "desktop".into(),
            background: None,
        }
    }
}

/// The device presets the canvas toolbar offers.
pub const PRESETS: &[(&str, f32, f32)] = &[
    ("desktop", 1280.0, 800.0),
    ("laptop", 960.0, 640.0),
    ("tablet", 768.0, 1024.0),
    ("phone", 390.0, 844.0),
    ("panel", 420.0, 560.0),
    ("square", 600.0, 600.0),
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    /// Stable across renames — component references and the open-tab list use
    /// it, so renaming a component does not orphan every place it is used.
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub kind: DocKind,
    pub root: NodeId,
    pub nodes: BTreeMap<NodeId, Node>,
    #[serde(default)]
    pub ids: IdGen,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub state: Vec<StateVar>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<ActionDef>,
    #[serde(default)]
    pub canvas: Canvas,
}

impl Document {
    /// A document with one empty root frame — the thing a new tab opens onto.
    pub fn new(id: impl Into<String>, name: impl Into<String>, kind: DocKind) -> Self {
        let mut ids = IdGen::default();
        let root = ids.next();
        let mut nodes = BTreeMap::new();
        let mut frame = Node::new(root, "frame");
        frame.style.padding = crate::style::Edges::all(24.0);
        frame.style.gap = Some(16.0);
        nodes.insert(root, frame);
        Document {
            id: id.into(),
            name: name.into(),
            kind,
            root,
            nodes,
            ids,
            state: Vec::new(),
            actions: Vec::new(),
            canvas: Canvas::default(),
        }
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(&id)
    }

    /// The parent of `id`, plus the slot and index it sits at.
    pub fn parent_of(&self, id: NodeId) -> Option<(NodeId, String, usize)> {
        self.nodes
            .values()
            .find_map(|node| node.locate(id).map(|(slot, index)| (node.id, slot, index)))
    }

    /// Add a node to the arena and hook it into a parent's slot. `index` is
    /// clamped, so "drop at the end" can pass `usize::MAX`.
    ///
    /// A parent that is not in the arena adds nothing: the id comes back, but
    /// `node(id)` will be `None`. Every caller here validates the parent first;
    /// the check is so that a bad one leaves no orphan behind.
    pub fn insert(&mut self, parent: NodeId, slot: &str, index: usize, node: Node) -> NodeId {
        let id = node.id;
        // No parent, no node: a node in the arena that nothing points at is a
        // leak the caller would then try to select.
        if !self.nodes.contains_key(&parent) {
            return id;
        }
        self.ids.seen(id);
        self.nodes.insert(id, node);
        if let Some(parent) = self.nodes.get_mut(&parent) {
            let list = parent.slot_mut(slot);
            let at = index.min(list.len());
            list.insert(at, id);
        }
        id
    }

    /// Create a node of `kind` and place it. The caller supplies the props.
    pub fn create(&mut self, kind: &str) -> Node {
        Node::new(self.ids.next(), kind)
    }

    /// Detach `id` and everything under it. Returns the removed nodes so a
    /// caller can re-insert them; the root is always the first element.
    pub fn remove(&mut self, id: NodeId) -> Vec<Node> {
        if id == self.root {
            return Vec::new();
        }
        if let Some((parent, _, _)) = self.parent_of(id) {
            if let Some(parent) = self.nodes.get_mut(&parent) {
                parent.detach(id);
            }
        }
        let mut removed = Vec::new();
        let mut stack = vec![id];
        while let Some(current) = stack.pop() {
            if let Some(node) = self.nodes.remove(&current) {
                stack.extend(node.all_children());
                removed.push(node);
            }
        }
        removed
    }

    /// Move `id` under `parent`'s `slot` at `index`. Refuses to move a node
    /// into its own subtree — the check the whole drag interaction rests on.
    pub fn move_node(&mut self, id: NodeId, parent: NodeId, slot: &str, index: usize) -> bool {
        if id == self.root || id == parent || self.is_ancestor(id, parent) {
            return false;
        }
        let Some((old_parent, old_slot, old_index)) = self.parent_of(id) else {
            return false;
        };
        if let Some(node) = self.nodes.get_mut(&old_parent) {
            node.detach(id);
        }
        // Removing from earlier in the same list shifts every later index down.
        let mut at = index;
        if old_parent == parent && old_slot == slot && old_index < index {
            at = index.saturating_sub(1);
        }
        if let Some(node) = self.nodes.get_mut(&parent) {
            let list = node.slot_mut(slot);
            let at = at.min(list.len());
            list.insert(at, id);
            true
        } else {
            // The target vanished; put it back where it was rather than leaking.
            if let Some(node) = self.nodes.get_mut(&old_parent) {
                let list = node.slot_mut(&old_slot);
                let at = old_index.min(list.len());
                list.insert(at, id);
            }
            false
        }
    }

    /// Copy `id`'s subtree, fresh ids throughout, placed right after it.
    pub fn duplicate(&mut self, id: NodeId) -> Option<NodeId> {
        let (parent, slot, index) = self.parent_of(id)?;
        let (root, rest) = self.clone_subtree(id)?;
        let copy = root.id;
        for node in rest {
            self.ids.seen(node.id);
            self.nodes.insert(node.id, node);
        }
        self.insert(parent, &slot, index + 1, root);
        Some(copy)
    }

    /// The copied root plus every copied descendant, with new ids and rewritten
    /// slot lists. Returned rather than inserted so `duplicate` controls where
    /// the root lands.
    fn clone_subtree(&mut self, id: NodeId) -> Option<(Node, Vec<Node>)> {
        self.clone_subtree_at(id, 0)
    }

    fn clone_subtree_at(&mut self, id: NodeId, depth: usize) -> Option<(Node, Vec<Node>)> {
        if depth > MAX_DEPTH {
            return None;
        }
        let source = self.nodes.get(&id)?.clone();
        let mut root = source.clone();
        root.id = self.ids.next();
        let mut rest = Vec::new();
        for (slot, children) in source.slots.iter() {
            let mut copied = Vec::new();
            for child in children {
                if let Some((child_root, mut child_rest)) = self.clone_subtree_at(*child, depth + 1)
                {
                    copied.push(child_root.id);
                    rest.push(child_root);
                    rest.append(&mut child_rest);
                }
            }
            root.slots.insert(slot.clone(), copied);
        }
        Some((root, rest))
    }

    /// Every node under `id`, `id` itself excluded, in depth-first order.
    ///
    /// Cycle-safe. `repair` makes a loaded document a true tree, but this is
    /// the function every other traversal is built on, and a hand-edited file
    /// that closes a loop should come back with a short answer rather than hang
    /// the app that opened it.
    pub fn descendants(&self, id: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut seen: BTreeSet<NodeId> = BTreeSet::from([id]);
        let mut stack: Vec<NodeId> = self
            .nodes
            .get(&id)
            .map(|n| n.all_children())
            .unwrap_or_default();
        stack.reverse();
        while let Some(current) = stack.pop() {
            if !seen.insert(current) {
                continue;
            }
            out.push(current);
            if let Some(node) = self.nodes.get(&current) {
                let mut children = node.all_children();
                children.reverse();
                stack.extend(children);
            }
        }
        out
    }

    pub fn is_ancestor(&self, ancestor: NodeId, of: NodeId) -> bool {
        let mut current = of;
        let mut seen = BTreeSet::from([of]);
        while let Some((parent, _, _)) = self.parent_of(current) {
            if parent == ancestor {
                return true;
            }
            if !seen.insert(parent) {
                return false;
            }
            current = parent;
        }
        false
    }

    /// From the root down to `id`, inclusive — the layers tree's breadcrumb.
    pub fn path_to(&self, id: NodeId) -> Vec<NodeId> {
        let mut path = vec![id];
        let mut seen = BTreeSet::from([id]);
        let mut current = id;
        while let Some((parent, _, _)) = self.parent_of(current) {
            if !seen.insert(parent) {
                break;
            }
            path.push(parent);
            current = parent;
        }
        path.reverse();
        path
    }

    /// Nodes reachable from the root. Anything else is a leak and the loader
    /// drops it.
    pub fn reachable(&self) -> Vec<NodeId> {
        let mut out = vec![self.root];
        out.extend(self.descendants(self.root));
        out
    }

    /// Make a loaded document a well-formed tree: a root that exists, one
    /// parent per node, no loops, nothing deeper than [`MAX_DEPTH`], and no
    /// nodes that nothing points at.
    ///
    /// Every traversal downstream — the canvas, the generator, the lint pass —
    /// assumes all of that. A `.tailor` file is text somebody can edit, so it is
    /// assumed once, here, rather than defended against in twenty places.
    pub fn repair(&mut self) {
        // A root that is not in the arena would make the document unopenable.
        if !self.nodes.contains_key(&self.root) {
            let root = self.ids.next();
            self.root = root;
            self.nodes.insert(root, Node::new(root, "frame"));
        }

        // Walk from the root keeping the first link to each node and cutting
        // every other one. That drops second parents and closes loops in the
        // same pass, and bounds the depth while it is there.
        let mut kept: BTreeSet<NodeId> = BTreeSet::from([self.root]);
        let mut queue: Vec<(NodeId, usize)> = vec![(self.root, 0)];
        while let Some((id, depth)) = queue.pop() {
            let Some(node) = self.nodes.get(&id) else {
                continue;
            };
            let slots: Vec<String> = node.slots.keys().cloned().collect();
            for slot in slots {
                let Some(children) = self.nodes.get(&id).map(|node| node.slot(&slot).to_vec())
                else {
                    continue;
                };
                let mut keep = Vec::with_capacity(children.len());
                for child in children {
                    if depth >= MAX_DEPTH || !self.nodes.contains_key(&child) {
                        continue;
                    }
                    if kept.insert(child) {
                        keep.push(child);
                        queue.push((child, depth + 1));
                    }
                }
                if let Some(node) = self.nodes.get_mut(&id) {
                    *node.slot_mut(&slot) = keep;
                }
            }
        }

        self.nodes.retain(|id, _| kept.contains(id));
        for node in self.nodes.values_mut() {
            node.slots.retain(|_, children| !children.is_empty());
        }

        let mut ids = IdGen::default();
        for id in self.nodes.keys() {
            ids.seen(*id);
        }
        self.ids = ids;
    }

    /// Whether `parent` lays its children out absolutely — what the canvas asks
    /// before it decides between a drop indicator and an x/y drop.
    pub fn layout_of(&self, parent: NodeId) -> LayoutMode {
        self.nodes
            .get(&parent)
            .map(|n| n.style.layout)
            .unwrap_or_default()
    }

    /// The default slot's children, for the common case.
    pub fn children_of(&self, id: NodeId) -> &[NodeId] {
        self.nodes
            .get(&id)
            .map(|n| n.slot(DEFAULT_SLOT))
            .unwrap_or(&[])
    }

    pub fn var(&self, name: &str) -> Option<&StateVar> {
        self.state.iter().find(|v| v.name == name)
    }

    /// A name not already taken by a state variable.
    pub fn unique_var_name(&self, base: &str) -> String {
        unique(base, |candidate| {
            self.state.iter().any(|v| v.name == candidate)
        })
    }

    pub fn unique_action_name(&self, base: &str) -> String {
        unique(base, |candidate| {
            self.actions.iter().any(|a| a.name == candidate)
        })
    }
}

/// How deep a document may nest. Far past anything a real interface needs, and
/// close enough to bound the recursion in the generator and the renderer.
pub const MAX_DEPTH: usize = 64;

/// `base`, or `base2`, `base3`, … until `taken` says it is free.
pub fn unique(base: &str, taken: impl Fn(&str) -> bool) -> String {
    if !taken(base) {
        return base.to_string();
    }
    for n in 2..1000 {
        let candidate = format!("{base}{n}");
        if !taken(&candidate) {
            return candidate;
        }
    }
    format!("{base}_x")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc_with_two_children() -> (Document, NodeId, NodeId) {
        let mut doc = Document::new("d1", "Screen", DocKind::Screen);
        let a = doc.create("button");
        let a = doc.insert(doc.root, DEFAULT_SLOT, 0, a);
        let b = doc.create("text");
        let b = doc.insert(doc.root, DEFAULT_SLOT, 1, b);
        (doc, a, b)
    }

    #[test]
    fn insert_places_and_clamps() {
        let (mut doc, a, b) = doc_with_two_children();
        assert_eq!(doc.children_of(doc.root), [a, b]);
        let c = doc.create("badge");
        let c = doc.insert(doc.root, DEFAULT_SLOT, usize::MAX, c);
        assert_eq!(doc.children_of(doc.root), [a, b, c]);
    }

    #[test]
    fn remove_takes_the_whole_subtree() {
        let mut doc = Document::new("d1", "Screen", DocKind::Screen);
        let outer = doc.create("stack");
        let outer = doc.insert(doc.root, DEFAULT_SLOT, 0, outer);
        let inner = doc.create("stack");
        let inner = doc.insert(outer, DEFAULT_SLOT, 0, inner);
        let leaf = doc.create("text");
        let leaf = doc.insert(inner, DEFAULT_SLOT, 0, leaf);

        let removed = doc.remove(outer);
        assert_eq!(removed.len(), 3);
        assert_eq!(removed[0].id, outer);
        assert!(doc.node(leaf).is_none());
        assert!(doc.children_of(doc.root).is_empty());
    }

    #[test]
    fn the_root_cannot_be_removed_or_moved() {
        let (mut doc, a, _) = doc_with_two_children();
        assert!(doc.remove(doc.root).is_empty());
        assert!(!doc.move_node(doc.root, a, DEFAULT_SLOT, 0));
    }

    #[test]
    fn a_node_cannot_be_moved_into_itself() {
        let mut doc = Document::new("d1", "Screen", DocKind::Screen);
        let outer = doc.create("stack");
        let outer = doc.insert(doc.root, DEFAULT_SLOT, 0, outer);
        let inner = doc.create("stack");
        let inner = doc.insert(outer, DEFAULT_SLOT, 0, inner);

        assert!(!doc.move_node(outer, inner, DEFAULT_SLOT, 0));
        assert!(!doc.move_node(outer, outer, DEFAULT_SLOT, 0));
        assert!(doc.move_node(inner, doc.root, DEFAULT_SLOT, 0));
        assert_eq!(doc.children_of(doc.root), [inner, outer]);
    }

    #[test]
    fn moving_later_in_the_same_list_accounts_for_the_gap() {
        let (mut doc, a, b) = doc_with_two_children();
        let c = doc.create("badge");
        let c = doc.insert(doc.root, DEFAULT_SLOT, 2, c);
        // Drop `a` at the slot after `c`: it should land last, not second.
        assert!(doc.move_node(a, doc.root, DEFAULT_SLOT, 3));
        assert_eq!(doc.children_of(doc.root), [b, c, a]);
    }

    #[test]
    fn duplicate_copies_the_subtree_with_new_ids() {
        let mut doc = Document::new("d1", "Screen", DocKind::Screen);
        let outer = doc.create("stack");
        let outer = doc.insert(doc.root, DEFAULT_SLOT, 0, outer);
        let leaf = doc.create("text");
        doc.insert(outer, DEFAULT_SLOT, 0, leaf);

        let copy = doc.duplicate(outer).unwrap();
        assert_ne!(copy, outer);
        assert_eq!(doc.children_of(doc.root), [outer, copy]);
        assert_eq!(doc.children_of(copy).len(), 1);
        assert_ne!(doc.children_of(copy)[0], doc.children_of(outer)[0]);
    }

    #[test]
    fn descendants_and_paths_agree() {
        let mut doc = Document::new("d1", "Screen", DocKind::Screen);
        let outer = doc.create("stack");
        let outer = doc.insert(doc.root, DEFAULT_SLOT, 0, outer);
        let leaf = doc.create("text");
        let leaf = doc.insert(outer, DEFAULT_SLOT, 0, leaf);

        assert_eq!(doc.descendants(doc.root), vec![outer, leaf]);
        assert_eq!(doc.path_to(leaf), vec![doc.root, outer, leaf]);
        assert!(doc.is_ancestor(doc.root, leaf));
        assert!(!doc.is_ancestor(leaf, outer));
    }

    #[test]
    fn repair_drops_orphans_and_dangling_references() {
        let (mut doc, a, _) = doc_with_two_children();
        doc.nodes.insert(NodeId(99), Node::new(NodeId(99), "text"));
        doc.node_mut(a)
            .unwrap()
            .slot_mut(DEFAULT_SLOT)
            .push(NodeId(404));
        doc.repair();
        assert!(doc.node(NodeId(99)).is_none());
        assert!(doc.node(a).unwrap().children().is_empty());
    }

    #[test]
    fn repair_breaks_a_cycle_rather_than_hanging() {
        let (mut doc, a, b) = doc_with_two_children();
        // a contains b, and b contains a.
        doc.node_mut(a).unwrap().slot_mut(DEFAULT_SLOT).push(b);
        doc.node_mut(b).unwrap().slot_mut(DEFAULT_SLOT).push(a);
        doc.repair();

        // Whatever it kept, it is a tree: every node is reachable once, and
        // walking it terminates.
        let reachable = doc.reachable();
        let mut unique = reachable.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(reachable.len(), unique.len());
        assert!(!doc.is_ancestor(a, a));
        assert_eq!(doc.path_to(b).first(), Some(&doc.root));
    }

    #[test]
    fn a_node_with_two_parents_keeps_one() {
        let (mut doc, a, b) = doc_with_two_children();
        let shared = doc.create("text");
        let shared = doc.insert(a, DEFAULT_SLOT, 0, shared);
        doc.node_mut(b).unwrap().slot_mut(DEFAULT_SLOT).push(shared);
        assert_eq!(doc.node(b).unwrap().children(), [shared]);

        doc.repair();
        let parents = [a, b]
            .iter()
            .filter(|id| doc.children_of(**id).contains(&shared))
            .count();
        assert_eq!(parents, 1);
    }

    #[test]
    fn repair_gives_a_document_back_its_root() {
        let mut doc = Document::new("d1", "Screen", DocKind::Screen);
        doc.nodes.clear();
        doc.repair();
        assert!(doc.node(doc.root).is_some());
    }

    #[test]
    fn nesting_is_bounded() {
        let mut doc = Document::new("d1", "Screen", DocKind::Screen);
        let mut parent = doc.root;
        for _ in 0..(MAX_DEPTH + 40) {
            let child = doc.create("frame");
            parent = doc.insert(parent, DEFAULT_SLOT, 0, child);
        }
        doc.repair();
        // The deepest surviving node is at most MAX_DEPTH links from the root.
        let deepest = doc
            .reachable()
            .into_iter()
            .map(|id| doc.path_to(id).len())
            .max()
            .unwrap_or(0);
        assert!(deepest <= MAX_DEPTH + 1, "{deepest} deep");
    }

    #[test]
    fn a_node_with_no_parent_is_not_inserted() {
        let mut doc = Document::new("d1", "Screen", DocKind::Screen);
        let orphan = doc.create("text");
        let id = doc.insert(NodeId(4242), DEFAULT_SLOT, 0, orphan);
        assert!(doc.node(id).is_none());
    }

    #[test]
    fn unique_names_walk_forward() {
        let taken = ["query", "query2"];
        assert_eq!(unique("query", |c| taken.contains(&c)), "query3");
        assert_eq!(unique("other", |c| taken.contains(&c)), "other");
    }
}
