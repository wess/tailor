//! The right-click menu, and the one command that only exists here.
//!
//! Everything in the menu is a command the menus and the keyboard already
//! reach; what the menu adds is that it acts on what is under the pointer.
//! Right-click selects first and opens second, so you can always see what the
//! next item is about to do.

use gpui::prelude::*;
use gpui::{App, ClipboardItem, Context, Entity, Pixels, Point, WeakEntity, Window};
use guise::prelude::*;
use tailor_model::catalog;
use tailor_model::node::DEFAULT_SLOT;
use tailor_model::{DocKind, Document, Node, NodeId};
use tailor_render::DropSpot;

use super::Workbench;

/// Wrap a workbench command as a menu handler. The menu outlives a single
/// render, so the handle has to be weak.
fn action(
  weak: &WeakEntity<Workbench>,
  f: impl Fn(&mut Workbench, &mut Window, &mut Context<Workbench>) + 'static,
) -> impl Fn(&mut Window, &mut App) + 'static {
  let weak = weak.clone();
  move |window, cx| {
    weak.update(cx, |this, cx| f(this, window, cx)).ok();
  }
}

impl Workbench {
  /// Open the menu for a node, or for the canvas when `target` is `None`.
  pub fn open_context_menu(
    &mut self,
    target: Option<NodeId>,
    position: Point<Pixels>,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let menu = match target {
      Some(id) => self.node_menu(id, cx),
      None => self.canvas_menu(cx),
    };
    menu.update(cx, |menu, cx| menu.show(position, window, cx));
    self.menu = Some(menu);
    cx.notify();
  }

  fn canvas_menu(&mut self, cx: &mut Context<Self>) -> Entity<ContextMenu> {
    let weak = cx.entity().downgrade();
    cx.new(|cx| {
      ContextMenu::new(cx)
        .width(210.0)
        .item_icon(
          IconName::ClipboardPaste,
          "Paste",
          action(&weak, |this, window, cx| this.paste(window, cx)),
        )
        .divider()
        .item_icon(
          IconName::BoxSelect,
          "Select all",
          action(&weak, |this, window, cx| this.select_all(window, cx)),
        )
        .item(
          "Add a screen",
          action(&weak, |this, window, cx| this.new_screen(window, cx)),
        )
        .item(
          "Add a component",
          action(&weak, |this, window, cx| this.new_component(window, cx)),
        )
    })
  }

  fn node_menu(&mut self, id: NodeId, cx: &mut Context<Self>) -> Entity<ContextMenu> {
    let weak = cx.entity().downgrade();
    let node = self.doc().and_then(|doc| doc.node(id)).cloned();
    let is_root = self.doc().map(|doc| doc.root == id).unwrap_or(false);
    let locked = node.as_ref().map(|node| node.locked).unwrap_or(false);
    let hidden = node.as_ref().map(|node| node.hidden).unwrap_or(false);
    let container = node
      .as_ref()
      .and_then(|node| catalog::get(&node.kind))
      .map(|spec| spec.takes_children())
      .unwrap_or(false);
    let has_children = node
      .as_ref()
      .map(|node| !node.children().is_empty())
      .unwrap_or(false);
    let count = self.selection.len();

    cx.new(move |cx| {
      let mut menu = ContextMenu::new(cx).width(230.0);

      menu = menu
        .item_icon(
          IconName::PenLine,
          "Rename…",
          action(&weak, |this, window, cx| this.begin_rename(window, cx)),
        )
        .item_icon(
          IconName::Copy,
          "Duplicate",
          action(&weak, |this, window, cx| this.duplicate(window, cx)),
        )
        .divider()
        .item(
          "Cut",
          action(&weak, |this, window, cx| this.cut(window, cx)),
        )
        .item(
          "Copy",
          action(&weak, |this, window, cx| this.copy(window, cx)),
        )
        .item(
          "Paste",
          action(&weak, |this, window, cx| this.paste(window, cx)),
        );

      menu = menu
        .divider()
        .section("Arrange")
        .item_icon(
          IconName::Group,
          "Embed in frame",
          action(&weak, |this, window, cx| this.embed_frame(window, cx)),
        )
        .item(
          "Embed in stack",
          action(&weak, |this, window, cx| this.embed_stack(window, cx)),
        )
        .item(
          "Embed in card",
          action(&weak, |this, window, cx| this.embed_card(window, cx)),
        )
        .item(
          "Embed in scroll area",
          action(&weak, |this, window, cx| this.embed_scroll(window, cx)),
        );

      if container && has_children {
        menu = menu.item_icon(
          IconName::Ungroup,
          "Unwrap",
          action(&weak, |this, window, cx| this.unwrap_selection(window, cx)),
        );
      }
      if !is_root {
        menu = menu
          .item(
            "Move up",
            action(&weak, |this, window, cx| this.move_up(window, cx)),
          )
          .item(
            "Move down",
            action(&weak, |this, window, cx| this.move_down(window, cx)),
          );
      }

      menu = menu
        .divider()
        .item_icon(
          IconName::Package,
          if count > 1 {
            "Extract to a component…"
          } else {
            "Extract to a component"
          },
          action(&weak, |this, _window, cx| this.extract_component(cx)),
        )
        .item_icon(
          IconName::SquareCode,
          "Open in Editor",
          action(&weak, |this, window, cx| this.open_in_editor(window, cx)),
        );

      menu = menu
        .divider()
        .item_icon(
          if locked {
            IconName::LockOpen
          } else {
            IconName::Lock
          },
          if locked { "Unlock" } else { "Lock" },
          action(&weak, move |this, _window, cx| {
            this.edit_node(id, "Lock", cx, |node| node.locked = !node.locked);
          }),
        )
        .item_icon(
          if hidden {
            IconName::Eye
          } else {
            IconName::EyeOff
          },
          if hidden { "Show" } else { "Hide" },
          action(&weak, move |this, _window, cx| {
            this.edit_node(id, "Hide", cx, |node| node.hidden = !node.hidden);
          }),
        )
        .item(
          "Select parent",
          action(&weak, |this, window, cx| this.select_parent(window, cx)),
        );

      if !is_root {
        menu = menu.divider().danger_item(
          "Delete",
          action(&weak, |this, window, cx| this.delete_selection(window, cx)),
        );
      }
      menu
    })
  }

  /// Lift the selection into a new component document and leave a reference
  /// behind. The move that turns a screen you have been pushing around into
  /// something reusable, and the one command that has no menu-bar twin
  /// because it only makes sense pointed at something.
  pub fn extract_component(&mut self, cx: &mut Context<Self>) {
    let ids = self.selection.clone();
    let Some(first) = ids.first().copied() else {
      return;
    };
    if self.doc().map(|doc| doc.root == first).unwrap_or(true) {
      self
        .toasts
        .info("A screen's root is already a component", cx);
      return;
    }
    // A selection spread across two parents has no single place to put the
    // reference back.
    let Some((parent, slot, index)) = self.doc().and_then(|doc| doc.parent_of(first)) else {
      return;
    };
    let same_parent = ids.iter().all(|id| {
      self
        .doc()
        .and_then(|doc| doc.parent_of(*id))
        .map(|(p, _, _)| p)
        == Some(parent)
    });
    if !same_parent {
      self
        .toasts
        .failed("Select nodes that share a parent to extract them", cx);
      return;
    }

    let base = self
      .doc()
      .and_then(|doc| doc.node(first))
      .map(tailor_render::nodes::label_of)
      .unwrap_or_else(|| "Component".into());
    let name = self
      .project
      .unique_doc_name(&tailor_model::pascal_case(&base));
    let doc_id = self.project.unique_doc_id(&tailor_model::snake_case(&name));

    self.commit(&format!("Extract {name}"));

    // Build the component from copies, then delete the originals: moving
    // between two documents means renumbering into the new arena anyway.
    let mut component = Document::new(doc_id.clone(), name.clone(), DocKind::Component);
    {
      let root = component.root;
      component.node_mut(root).unwrap().style = Default::default();
      if let Some(source) = self.project.doc(&self.doc_id) {
        for id in &ids {
          copy_into(source, *id, &mut component, root);
        }
      }
    }
    component.canvas.width = 480.0;
    component.canvas.height = 320.0;
    std::sync::Arc::make_mut(&mut self.project)
      .docs
      .push(component);

    if let Some(doc) = self.doc_mut() {
      for id in &ids {
        doc.remove(*id);
      }
      let node = Node::new(doc.ids.next(), format!("@{name}"));
      let placed = doc.insert(parent, &slot, index, node);
      self.selection = vec![placed];
    }
    self.fields.clear();
    self.areas.clear();
    self.refresh(cx);
    self.toasts.done(format!("{name} is now a component"), cx);
  }
}

/// The menu on a document tab. `close_document` deletes rather than hides —
/// every document in a project is always in the strip — so the destructive item
/// says so.
impl Workbench {
  pub fn open_doc_menu(
    &mut self,
    id: &str,
    position: Point<Pixels>,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let weak = cx.entity().downgrade();
    let id = id.to_string();
    let last = self.project.docs.len() <= 1;
    let kind = self
      .project
      .doc(&id)
      .map(|doc| doc.kind)
      .unwrap_or(DocKind::Screen);

    let menu = cx.new(move |cx| {
      let rename_id = id.clone();
      let duplicate_id = id.clone();
      let delete_id = id.clone();
      let mut menu = ContextMenu::new(cx)
        .width(210.0)
        .item_icon(
          IconName::PenLine,
          "Rename…",
          action(&weak, move |this, _window, cx| {
            this.begin_rename_document(&rename_id, cx)
          }),
        )
        .item_icon(
          IconName::Copy,
          "Duplicate",
          action(&weak, move |this, _window, cx| {
            this.duplicate_document(&duplicate_id, cx)
          }),
        )
        .divider()
        .item_icon(
          IconName::AppWindow,
          "Add a screen",
          action(&weak, |this, window, cx| this.new_screen(window, cx)),
        )
        .item_icon(
          IconName::Package,
          "Add a component",
          action(&weak, |this, window, cx| this.new_component(window, cx)),
        );

      if kind == DocKind::Screen {
        menu = menu.item_icon(
          IconName::MonitorPlay,
          "Open the live window",
          action(&weak, |this, window, cx| this.open_live_window(window, cx)),
        );
      }
      if !last {
        menu = menu.divider().danger_item(
          "Delete",
          action(&weak, move |this, _window, cx| {
            this.close_document(&delete_id, cx)
          }),
        );
      }
      menu
    });
    menu.update(cx, |menu, cx| menu.show(position, window, cx));
    self.menu = Some(menu);
    cx.notify();
  }
}

/// The menus on the side panels. Each one acts on the row under the pointer,
/// and each item is a command the panel already reaches some other way — a menu
/// that invents commands is a menu nobody finds twice.
impl Workbench {
  /// A catalog component in the Library.
  pub fn open_palette_menu(
    &mut self,
    kind: &str,
    position: Point<Pixels>,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let weak = cx.entity().downgrade();
    let kind = kind.to_string();
    let root = self.doc().map(|doc| doc.root);
    let menu = cx.new(move |cx| {
      let into_selection = kind.clone();
      let at_root = kind.clone();
      let mut menu = ContextMenu::new(cx).width(220.0).item_icon(
        IconName::CornerDownRight,
        "Insert into the selection",
        action(&weak, move |this, _window, cx| {
          this.place_into_selection(&into_selection, cx)
        }),
      );
      if let Some(root) = root {
        menu = menu.item_icon(
          IconName::Frame,
          "Insert at the top level",
          action(&weak, move |this, _window, cx| {
            let spot = DropSpot::at(root, DEFAULT_SLOT, usize::MAX);
            this.insert_kind(&at_root, spot, cx);
          }),
        );
      }
      menu
    });
    menu.update(cx, |menu, cx| menu.show(position, window, cx));
    self.menu = Some(menu);
    cx.notify();
  }

  /// One of this project's own components, in the Library.
  pub fn open_component_menu(
    &mut self,
    name: &str,
    doc_id: &str,
    position: Point<Pixels>,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let weak = cx.entity().downgrade();
    let reference = format!("@{name}");
    let doc_id = doc_id.to_string();
    let root = self.doc().map(|doc| doc.root);
    let last = self.project.docs.len() <= 1;
    let menu = cx.new(move |cx| {
      let (open_id, rename_id) = (doc_id.clone(), doc_id.clone());
      let (dup_id, delete_id) = (doc_id.clone(), doc_id.clone());
      let at_root = reference.clone();
      let mut menu = ContextMenu::new(cx).width(220.0).item_icon(
        IconName::CornerDownRight,
        "Insert into the selection",
        action(&weak, move |this, _window, cx| {
          this.place_into_selection(&reference, cx)
        }),
      );
      if let Some(root) = root {
        menu = menu.item_icon(
          IconName::Frame,
          "Insert at the top level",
          action(&weak, move |this, _window, cx| {
            let spot = DropSpot::at(root, DEFAULT_SLOT, usize::MAX);
            this.insert_kind(&at_root, spot, cx);
          }),
        );
      }
      menu = menu
        .divider()
        .item_icon(
          IconName::SquarePen,
          "Edit the component",
          action(&weak, move |this, _window, cx| {
            this.open_document(&open_id, cx)
          }),
        )
        .item_icon(
          IconName::PenLine,
          "Rename…",
          action(&weak, move |this, _window, cx| {
            this.begin_rename_document(&rename_id, cx)
          }),
        )
        .item_icon(
          IconName::Copy,
          "Duplicate",
          action(&weak, move |this, _window, cx| {
            this.duplicate_document(&dup_id, cx)
          }),
        );
      if !last {
        menu = menu.divider().danger_item(
          "Delete",
          action(&weak, move |this, _window, cx| {
            this.close_document(&delete_id, cx)
          }),
        );
      }
      menu
    });
    menu.update(cx, |menu, cx| menu.show(position, window, cx));
    self.menu = Some(menu);
    cx.notify();
  }

  /// A row in Problems.
  pub fn open_problem_menu(
    &mut self,
    doc_id: &str,
    node: Option<NodeId>,
    message: &str,
    position: Point<Pixels>,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let weak = cx.entity().downgrade();
    let doc_id = doc_id.to_string();
    let message = message.to_string();
    let menu = cx.new(move |cx| {
      let reveal_id = doc_id.clone();
      let mut menu = ContextMenu::new(cx).width(220.0).item_icon(
        IconName::Crosshair,
        "Reveal",
        action(&weak, move |this, _window, cx| {
          this.open_document(&reveal_id, cx);
          if let Some(node) = node {
            this.select_only(node, cx);
          }
        }),
      );
      menu = menu.item_icon(IconName::Copy, "Copy the message", move |_window, cx| {
        cx.write_to_clipboard(ClipboardItem::new_string(message.clone()));
      });
      menu
    });
    menu.update(cx, |menu, cx| menu.show(position, window, cx));
    self.menu = Some(menu);
    cx.notify();
  }

  /// The generated code, in Split mode.
  pub fn open_code_menu(
    &mut self,
    position: Point<Pixels>,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let weak = cx.entity().downgrade();
    let menu = cx.new(move |cx| {
      ContextMenu::new(cx)
        .width(220.0)
        .item_icon(
          IconName::Copy,
          "Copy the file",
          action(&weak, |this, _window, cx| this.copy_code(cx)),
        )
        .item_icon(
          IconName::FolderOutput,
          "Export the project…",
          action(&weak, |this, window, cx| this.export_code(window, cx)),
        )
        .divider()
        .item_icon(
          IconName::PanelRight,
          "Hide the code",
          action(&weak, |this, window, cx| this.mode_design(window, cx)),
        )
    });
    menu.update(cx, |menu, cx| menu.show(position, window, cx));
    self.menu = Some(menu);
    cx.notify();
  }
}

/// Copy a subtree from one document into another, renumbering as it goes.
fn copy_into(
  source: &Document,
  id: NodeId,
  target: &mut Document,
  parent: NodeId,
) -> Option<NodeId> {
  let original = source.node(id)?.clone();
  let mut node = original.clone();
  node.id = target.ids.next();
  node.slots.clear();
  let new_id = target.insert(parent, DEFAULT_SLOT, usize::MAX, node);

  for (slot, children) in &original.slots {
    for child in children {
      if let Some(child_id) = copy_into(source, *child, target, new_id) {
        if slot != DEFAULT_SLOT {
          if let Some(node) = target.node_mut(new_id) {
            node.detach(child_id);
            node.slot_mut(slot).push(child_id);
          }
        }
      }
    }
  }
  Some(new_id)
}
