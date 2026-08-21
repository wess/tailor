//! A placed component.
//!
//! Children live in *slots*, not in one list. Most components have only the
//! default slot, but the ones you actually build a screen out of — `AppShell`,
//! `Panel`, `Tabs`, `Button` with its sections — have named regions, and giving
//! every node the same slot map means the tree, the canvas drop targets, the
//! layer list, and the generator all speak one mechanism instead of four.

use crate::id::NodeId;
use crate::props::{PropValue, Props};
use crate::style::StyleProps;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The slot every container uses unless the catalog says otherwise.
pub const DEFAULT_SLOT: &str = "children";

/// A node whose `kind` starts with this refers to another document in the same
/// project — a component you built and are now placing inside a screen.
pub const COMPONENT_PREFIX: char = '@';

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    /// A catalog key (`button`, `stack`) or `@Name` for a project component.
    pub kind: String,
    /// What the layers tree calls it. Absent means "use the component's title".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub props: Props,
    #[serde(default, skip_serializing_if = "StyleProps::is_default")]
    pub style: StyleProps,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub slots: BTreeMap<String, Vec<NodeId>>,
    /// Event key (`click`) to the name of the action it dispatches.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub events: BTreeMap<String, String>,
    /// Hidden nodes stay in the document and in generated code, but the canvas
    /// skips them. It is a design affordance, not a runtime `if`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hidden: bool,
    /// Locked nodes cannot be selected or dragged on the canvas.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub locked: bool,
}

impl Node {
    pub fn new(id: NodeId, kind: impl Into<String>) -> Self {
        Node {
            id,
            kind: kind.into(),
            name: None,
            props: Props::new(),
            style: StyleProps::default(),
            slots: BTreeMap::new(),
            events: BTreeMap::new(),
            hidden: false,
            locked: false,
        }
    }

    /// The project component this node places, if it is a reference.
    pub fn component_ref(&self) -> Option<&str> {
        self.kind.strip_prefix(COMPONENT_PREFIX)
    }

    pub fn children(&self) -> &[NodeId] {
        self.slot(DEFAULT_SLOT)
    }

    pub fn slot(&self, key: &str) -> &[NodeId] {
        self.slots.get(key).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn slot_mut(&mut self, key: &str) -> &mut Vec<NodeId> {
        self.slots.entry(key.to_string()).or_default()
    }

    /// Every child, across every slot, in slot-then-index order.
    pub fn all_children(&self) -> Vec<NodeId> {
        self.slots
            .values()
            .flat_map(|ids| ids.iter().copied())
            .collect()
    }

    /// Which slot holds `child`, and at what index.
    pub fn locate(&self, child: NodeId) -> Option<(String, usize)> {
        for (slot, ids) in &self.slots {
            if let Some(index) = ids.iter().position(|id| *id == child) {
                return Some((slot.clone(), index));
            }
        }
        None
    }

    pub fn detach(&mut self, child: NodeId) -> Option<(String, usize)> {
        let found = self.locate(child)?;
        self.slots.get_mut(&found.0)?.remove(found.1);
        Some(found)
    }

    pub fn prop(&self, key: &str) -> Option<&PropValue> {
        self.props.get(key)
    }

    pub fn set_prop(&mut self, key: impl Into<String>, value: PropValue) {
        self.props.insert(key.into(), value);
    }

    /// Read a prop, falling back to the catalog default when it was never set.
    pub fn prop_or(&self, key: &str, fallback: PropValue) -> PropValue {
        self.props.get(key).cloned().unwrap_or(fallback)
    }
}

/// One event a component can raise, and what its handler is handed.
#[derive(Debug, Clone, Copy)]
pub struct EventSpec {
    pub key: &'static str,
    pub label: &'static str,
    /// The builder method that takes the handler.
    pub method: &'static str,
    /// The handler's parameter list in generated code, minus the trailing
    /// `_window, cx` that every guise handler takes.
    pub args: &'static [&'static str],
}

pub const CLICK: EventSpec = EventSpec {
    key: "click",
    label: "On click",
    method: "on_click",
    args: &["_event"],
};

pub const CHANGE_BOOL: EventSpec = EventSpec {
    key: "change",
    label: "On change",
    method: "on_change",
    args: &["value"],
};

pub const CHANGE_INDEX: EventSpec = EventSpec {
    key: "change",
    label: "On change",
    method: "on_change",
    args: &["index"],
};

pub const CHANGE_VALUE: EventSpec = EventSpec {
    key: "change",
    label: "On change",
    method: "on_change",
    args: &["value"],
};

pub const CLOSE: EventSpec = EventSpec {
    key: "close",
    label: "On close",
    method: "on_close",
    args: &[],
};

pub const TOGGLE: EventSpec = EventSpec {
    key: "toggle",
    label: "On toggle",
    method: "on_toggle",
    args: &["open"],
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_node_serializes_to_id_and_kind_only() {
        let node = Node::new(NodeId(1), "button");
        let json = serde_json::to_string(&node).unwrap();
        assert_eq!(json, r#"{"id":1,"kind":"button"}"#);
    }

    #[test]
    fn slots_locate_and_detach_children() {
        let mut node = Node::new(NodeId(1), "panel");
        node.slot_mut(DEFAULT_SLOT).extend([NodeId(2), NodeId(3)]);
        node.slot_mut("footer").push(NodeId(4));

        assert_eq!(node.locate(NodeId(3)), Some((DEFAULT_SLOT.to_string(), 1)));
        assert_eq!(node.locate(NodeId(4)), Some(("footer".to_string(), 0)));
        assert_eq!(node.locate(NodeId(9)), None);
        assert_eq!(node.all_children(), vec![NodeId(2), NodeId(3), NodeId(4)]);

        assert_eq!(node.detach(NodeId(2)), Some((DEFAULT_SLOT.to_string(), 0)));
        assert_eq!(node.children(), [NodeId(3)]);
        assert_eq!(node.detach(NodeId(2)), None);
    }

    #[test]
    fn component_references_are_recognised_by_their_prefix() {
        assert_eq!(
            Node::new(NodeId(1), "@Sidebar").component_ref(),
            Some("Sidebar")
        );
        assert_eq!(Node::new(NodeId(1), "stack").component_ref(), None);
    }
}
