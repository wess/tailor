//! The document outline — Interface Builder's document outline, Android
//! Studio's component tree.
//!
//! Rows are both drag sources and drop targets, so the tree is a second way to
//! restructure the layout when the canvas is too dense to aim at. Slots appear
//! as their own rows when a node has more than one, which is the only way to
//! see that a button has a left section or a panel has a footer.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, Context, ElementId, SharedString, Window};
use tailor_model::catalog;
use tailor_model::node::DEFAULT_SLOT;
use tailor_model::NodeId;
use tailor_render::chrome::DragGhost;
use tailor_render::{DragPayload, DropSpot};
use tailor_store::Panel;

use super::{icon, Workbench};
use crate::theme;

/// One row in the flattened tree.
struct Row {
    id: NodeId,
    depth: usize,
    label: String,
    glyph: &'static str,
    has_children: bool,
    expanded: bool,
    hidden: bool,
    locked: bool,
    /// A slot heading rather than a node — `None` for ordinary rows.
    slot: Option<(NodeId, String, String)>,
}

impl Workbench {
    pub(super) fn render_outline(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let chrome = theme::colors(cx);
        let rows = self.outline_rows();

        div()
            .w(px(self.settings.size(Panel::Outline)))
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .bg(chrome.surface)
            .child(self.panel_header(Panel::Outline, None, cx))
            .child(
                div()
                    .id("outline-list")
                    .flex()
                    .flex_col()
                    .flex_grow()
                    .overflow_y_scroll()
                    .py(px(4.))
                    .children(rows.into_iter().map(|row| self.outline_row(row, cx))),
            )
    }

    /// Depth-first, honouring the collapsed set.
    fn outline_rows(&self) -> Vec<Row> {
        let mut rows = Vec::new();
        let Some(doc) = self.doc() else { return rows };
        self.walk(doc, doc.root, 0, &mut rows);
        rows
    }

    fn walk(&self, doc: &tailor_model::Document, id: NodeId, depth: usize, rows: &mut Vec<Row>) {
        let Some(node) = doc.node(id) else { return };
        let spec = catalog::get(&node.kind);
        let slots = spec.map(|spec| spec.slots_of(node)).unwrap_or_default();
        let has_children = !node.all_children().is_empty();
        let expanded = !self.collapsed.contains(&id);

        rows.push(Row {
            id,
            depth,
            label: tailor_render::nodes::label_of(node),
            glyph: spec.map(|spec| spec.icon).unwrap_or("box"),
            has_children,
            expanded,
            hidden: node.hidden,
            locked: node.locked,
            slot: None,
        });
        if !expanded {
            return;
        }

        // One slot: children hang straight off the node. More than one: each
        // gets a heading, or you cannot tell a footer from a body.
        let named: Vec<_> = slots
            .iter()
            .filter(|slot| !node.slot(&slot.key).is_empty())
            .collect();
        let single = named.len() <= 1 && node.slots.keys().all(|key| key == DEFAULT_SLOT);

        if single {
            for child in node.slot(DEFAULT_SLOT) {
                self.walk(doc, *child, depth + 1, rows);
            }
            return;
        }
        for slot in slots {
            let children = node.slot(&slot.key);
            if children.is_empty() {
                continue;
            }
            rows.push(Row {
                id,
                depth: depth + 1,
                label: slot.label.clone(),
                glyph: "corner-down-right",
                has_children: true,
                expanded: true,
                hidden: false,
                locked: false,
                slot: Some((id, slot.key.clone(), slot.label.clone())),
            });
            for child in children {
                self.walk(doc, *child, depth + 2, rows);
            }
        }
    }

    fn outline_row(&self, row: Row, cx: &mut Context<Self>) -> AnyElement {
        let chrome = theme::colors(cx);
        let id = row.id;
        let indent = 8.0 + row.depth as f32 * 13.0;

        // A slot heading is a drop target, not a selectable node.
        if let Some((parent, slot, label)) = row.slot {
            let count = self
                .doc()
                .and_then(|doc| doc.node(parent))
                .map(|node| node.slot(&slot).len())
                .unwrap_or(0);
            let spot = DropSpot::at(parent, slot.clone(), count);
            let hooks_drop = self.drop_handler(spot, cx);
            return div()
                .id(ElementId::Name(SharedString::from(format!(
                    "slot-{parent}-{slot}"
                ))))
                .flex()
                .items_center()
                .gap(px(6.))
                .pl(px(indent))
                .pr(px(8.))
                .py(px(3.))
                .text_size(px(10.))
                .text_color(chrome.dimmed)
                .child(icon("corner-down-right"))
                .child(SharedString::from(label))
                .drag_over::<DragPayload>(move |style, _, _, _| style.bg(chrome.accent_soft))
                .on_drop::<DragPayload>(hooks_drop)
                .into_any_element();
        }

        let selected = self.selection.contains(&id);
        let renaming = self.renaming == Some(id);
        let label = row.label.clone();
        let drag_label = row.label.clone();
        let has_children = row.has_children;

        // Dropping onto a row puts the node inside it when it takes children,
        // and beside it when it does not.
        let spot = self
            .doc()
            .and_then(|doc| {
                let node = doc.node(id)?;
                let accepts = catalog::get(&node.kind)
                    .map(|spec| spec.takes_children())
                    .unwrap_or(false);
                if accepts {
                    Some(DropSpot::at(id, DEFAULT_SLOT, node.children().len()))
                } else {
                    doc.parent_of(id)
                        .map(|(parent, slot, index)| DropSpot::at(parent, slot, index + 1))
                }
            })
            .unwrap_or_else(|| DropSpot::at(id, DEFAULT_SLOT, 0));
        let on_drop = self.drop_handler(spot, cx);

        let mut root = div()
            .id(ElementId::Name(SharedString::from(format!("row-{id}"))))
            .flex()
            .items_center()
            .gap(px(5.))
            .pl(px(indent))
            .pr(px(6.))
            .py(px(3.))
            .rounded(px(4.))
            .mx(px(4.))
            .text_size(px(12.))
            .when(selected, |d| {
                d.bg(chrome.accent_soft).text_color(chrome.text)
            })
            .when(!selected, |d| d.text_color(chrome.text))
            .when(row.hidden, |d| d.text_color(chrome.dimmed))
            .hover(move |style| style.bg(chrome.raised));

        root = root.child(
            div()
                .id(ElementId::Name(SharedString::from(format!("twist-{id}"))))
                .w(px(12.))
                .text_color(chrome.dimmed)
                .when(has_children, |d| {
                    d.child(icon(if row.expanded {
                        "chevron-down"
                    } else {
                        "chevron-right"
                    }))
                })
                .on_click(cx.listener(move |this, _, _window, cx| {
                    if this.collapsed.contains(&id) {
                        this.collapsed.remove(&id);
                    } else {
                        this.collapsed.insert(id);
                    }
                    cx.notify();
                })),
        );
        root = root.child(div().text_color(chrome.dimmed).child(icon(row.glyph)));

        if renaming {
            if let Some(field) = self.rename_field.clone() {
                root = root.child(div().flex_grow().child(field));
                return root.into_any_element();
            }
        }
        root = root.child(div().flex_grow().child(SharedString::from(label)));

        let hidden = row.hidden;
        let locked = row.locked;
        root = root
            .child(
                div()
                    .id(ElementId::Name(SharedString::from(format!("lock-{id}"))))
                    .text_color(if locked { chrome.text } else { chrome.border })
                    .child(icon(if locked { "lock" } else { "lock-open" }))
                    .on_click(cx.listener(move |this, _, _window, cx| {
                        this.edit_node(id, "Lock", cx, |node| node.locked = !node.locked);
                    })),
            )
            .child(
                div()
                    .id(ElementId::Name(SharedString::from(format!("eye-{id}"))))
                    .text_color(if hidden { chrome.text } else { chrome.border })
                    .child(icon(if hidden { "eye-off" } else { "eye" }))
                    .on_click(cx.listener(move |this, _, _window, cx| {
                        this.edit_node(id, "Hide", cx, |node| node.hidden = !node.hidden);
                    })),
            );

        root = root
            .on_click(
                cx.listener(move |this, event: &gpui::ClickEvent, _window, cx| {
                    let additive = event.modifiers().shift || event.modifiers().platform;
                    this.select(id, additive, cx);
                }),
            )
            .on_mouse_down(
                gpui::MouseButton::Right,
                cx.listener(move |this, event: &gpui::MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    if !this.selection.contains(&id) {
                        this.select_only(id, cx);
                    }
                    this.open_context_menu(Some(id), event.position, window, cx);
                }),
            )
            .on_drag(DragPayload::Existing(id), {
                let weak = cx.entity().downgrade();
                move |_, _, _, cx| {
                    weak.update(cx, |this, cx| {
                        this.placing = true;
                        cx.notify();
                    })
                    .ok();
                    cx.new(|_| DragGhost {
                        label: SharedString::from(drag_label.clone()),
                    })
                }
            })
            .drag_over::<DragPayload>(move |style, _, _, _| style.bg(chrome.accent_soft))
            .on_drop::<DragPayload>(on_drop);

        root.into_any_element()
    }

    /// A drop handler bound to one spot.
    fn drop_handler(
        &self,
        spot: DropSpot,
        cx: &mut Context<Self>,
    ) -> impl Fn(&DragPayload, &mut Window, &mut gpui::App) + 'static {
        let weak = cx.entity().downgrade();
        move |payload: &DragPayload, _window: &mut Window, cx: &mut gpui::App| {
            let spot = spot.clone();
            let payload = payload.clone();
            weak.update(cx, |this, cx| this.accept_drop(spot, payload, cx))
                .ok();
        }
    }
}
