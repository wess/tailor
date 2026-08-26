//! Node identity.
//!
//! Ids are per-document counters, not global uuids: a document is loaded,
//! edited, and generated as a unit, so a `u32` that starts at 1 keeps the file
//! format readable and diffable. `0` is reserved for "no node", which is why
//! [`NodeId`] is not `Default`.

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(pub u32);

impl NodeId {
  /// A stable element id for gpui and for generated code (`node-12`).
  pub fn element_id(self) -> String {
    format!("node-{}", self.0)
  }

  /// The id for the box the canvas draws *around* a node. It has to differ
  /// from [`element_id`]: two elements sharing one makes gpui share their
  /// element state, and a `div` and a `Switch` sharing state means the
  /// switch never appears.
  pub fn wrapper_element_id(self) -> String {
    format!("wrap-{}", self.0)
  }
}

impl fmt::Display for NodeId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

/// Hands out the next free id for a document. Kept beside the arena so a
/// deleted node's id is never reused — undo restores nodes by id, and reuse
/// would let a redo resurrect the wrong subtree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdGen {
  next: u32,
}

impl Default for IdGen {
  fn default() -> Self {
    IdGen { next: 1 }
  }
}

impl IdGen {
  /// The name reads right for an allocator, and it is not an iterator.
  #[allow(clippy::should_implement_trait)]
  pub fn next(&mut self) -> NodeId {
    let id = NodeId(self.next);
    self.next += 1;
    id
  }

  /// Bump the counter past `id`. Used when loading a file, whose ids were
  /// assigned by an earlier session.
  pub fn seen(&mut self, id: NodeId) {
    self.next = self.next.max(id.0 + 1);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_node_and_its_wrapper_never_share_an_element_id() {
    for n in [1u32, 7, 4242] {
      let id = NodeId(n);
      assert_ne!(id.element_id(), id.wrapper_element_id());
    }
  }

  #[test]
  fn ids_start_at_one_and_never_repeat() {
    let mut gen = IdGen::default();
    assert_eq!(gen.next(), NodeId(1));
    assert_eq!(gen.next(), NodeId(2));
    gen.seen(NodeId(40));
    assert_eq!(gen.next(), NodeId(41));
    gen.seen(NodeId(3));
    assert_eq!(gen.next(), NodeId(42));
  }
}
