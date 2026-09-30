//! One node as a Rust expression.
//!
//! The generic path is the catalog: a constructor, a chained call per set prop,
//! a `.child(..)` per slot child, a handler per bound event. None of it names a
//! library — the constructor shape, the prop, the slot method all come off
//! [`tailor_model::library::Library`].
//!
//! The escape hatch is [`crate::generator::Generator`], which the library's own
//! provider implements: the components whose shape is not one chained call — a
//! `div` with no type of its own, a chart that pairs its numbers, a container
//! whose regions take closures.

use std::collections::{BTreeMap, BTreeSet};

use tailor_model::catalog::Ctor;
use tailor_model::library::Library;
use tailor_model::props::{Emit, PropValue};
use tailor_model::style::{LayoutMode, StyleProps};
use tailor_model::{Document, Flavor, Node, NodeId, Project};

use crate::expr::{self, Hoist};
use crate::generator::{self, Generator};
use crate::rust::{indent, string};
use crate::style::{self, Placement};

/// Whether the generated file has a `self` to hang handlers off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owner {
  /// A `Render` entity: events go through `cx.listener` into a method.
  Entity,
  /// A `RenderOnce` builder: no `self` at render time, so handlers are stubs.
  Plain,
}

/// The line a node's expression is tagged with while the file is being built,
/// so the finished text can be mapped back to the design. Stripped before
/// anything is written — see [`crate::file::document`].
pub const MARK: &str = "//__tailor:";

/// What a region closure calls its weak handle back to the view. `cx.listener`
/// borrows the context, and a region closure is `'static`, so a handler inside
/// one goes through this instead.
const VIEW: &str = "view";

/// Where the expression being built will sit. It decides how an entity-backed
/// node refers to itself: a field in `render`, a local in `new`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
  Render,
  Init,
}

pub struct Emitter<'a> {
  pub project: &'a Project,
  pub doc: &'a Document,
  /// The library this document is drawn with, and the generator for it. Every
  /// question about a component goes through one of the two.
  pub library: &'static dyn Library,
  pub generator: &'static dyn Generator,
  pub flavor: Flavor,
  /// Entity-backed nodes and the struct field each one lives in.
  pub fields: &'a BTreeMap<NodeId, String>,
  pub hoist: &'a mut Hoist,
  pub owner: Owner,
  pub imports: BTreeSet<String>,
  /// Things worth telling the user about the file that came out.
  pub notes: Vec<String>,
  /// `X::bind(&field, &signal, cx);` lines, emitted after the entity and the
  /// signal are both locals in `new`.
  pub binds: Vec<String>,
  phase: Phase,
  /// One frame per open closure. A closure is `'static`, so anything it
  /// touches has to be cloned in ahead of it; this records what to clone.
  captures: Vec<BTreeSet<String>>,
}

impl<'a> Emitter<'a> {
  pub fn new(
    project: &'a Project,
    doc: &'a Document,
    fields: &'a BTreeMap<NodeId, String>,
    hoist: &'a mut Hoist,
    owner: Owner,
  ) -> Self {
    Emitter {
      project,
      doc,
      library: project.library(),
      generator: generator::for_project(project),
      flavor: project.gen.flavor,
      fields,
      hoist,
      owner,
      imports: BTreeSet::new(),
      notes: Vec::new(),
      binds: Vec::new(),
      phase: Phase::Render,
      captures: Vec::new(),
    }
  }

  /// Build the `let <field> = cx.new(..)` line for an entity-backed node.
  /// Post-order over the document means the children it captures are already
  /// locals by the time this runs.
  pub fn emit_init(&mut self, id: NodeId) -> Vec<String> {
    let Some(node) = self.doc.node(id) else {
      return Vec::new();
    };
    let Some(field) = self.fields.get(&id).cloned() else {
      return Vec::new();
    };
    if let Some(target) = placed(self.project, node) {
      // A placed component that is an entity: `cx.new` over its own `new`,
      // handed the window when something inside it wants one.
      let ty = tailor_model::pascal_case(&target.name);
      self.imports.insert(format!("use super::{ty};"));
      return vec![if needs_window(self.project, target) {
        format!("let {field} = cx.new(|cx| {ty}::new(window, cx));")
      } else {
        format!("let {field} = cx.new({ty}::new);")
      }];
    }
    let Some(spec) = self.library.get(&node.kind) else {
      return Vec::new();
    };

    let previous = self.phase;
    self.phase = Phase::Init;
    self.captures.push(BTreeSet::new());

    let (ctor, takes_cx) = match spec.ctor {
      Ctor::EntityArg(key) => {
        let value = self.prop_value(node, key);
        let prop = spec
          .prop(key)
          .expect("catalog checks constructor props exist");
        let arg = expr::value(self.hoist, prop, &value, self.doc);
        (format!("{}::new(cx, {arg})", spec.rust), true)
      }
      Ctor::EntityValue(key) => {
        let value = self.prop_value(node, key);
        let prop = spec
          .prop(key)
          .expect("catalog checks constructor props exist");
        let arg = expr::value(self.hoist, prop, &value, self.doc);
        (format!("{}::new({arg})", spec.rust), false)
      }
      // The state, not the element: the element is rebuilt over it each frame.
      Ctor::Stateful(state) => (format!("{state}::new(window, cx)"), true),
      Ctor::StatefulCx(state) => (format!("{state}::new(cx)"), true),
      _ => (format!("{}::new(cx)", spec.rust), true),
    };
    let generator = self.generator;
    let mut body = match generator.state(self, node).filter(|_| spec.ctor.is_split()) {
      Some(lines) => lines,
      None => vec![ctor],
    };
    body.extend(indent(&self.prop_calls(node)));
    if !spec.ctor.is_split() {
      body.extend(indent(&self.slot_calls(node)));
    }
    // A state that takes no context (a slider's range) would leave `cx` unused,
    // and a generated warning reads as a defect.
    let param = if takes_cx && body.iter().any(|line| crate::file::mentions(line, "cx")) {
      "cx"
    } else {
      "_cx"
    };

    // Two-way binding is a call after construction, not a setter: the
    // entity and the signal both have to exist first.
    if let Some(key) = self.generator.entity_bind(&node.kind) {
      if let Some(var) = node.prop(key).and_then(|value| value.as_binding()) {
        self.binds.push(format!(
          "{}::bind(&{field}, &{}, cx);",
          spec.rust,
          tailor_model::snake_case(var)
        ));
      }
    }

    let captured = self.captures.pop().unwrap_or_default();
    self.phase = previous;

    let mut out = Vec::new();
    if captured.is_empty() {
      out.push(format!("let {field} = cx.new(|{param}| {{"));
      out.extend(indent(&body));
      out.push("});".into());
    } else {
      out.push(format!("let {field} = {{"));
      for name in &captured {
        out.push(format!("    let {name} = {name}.clone();"));
      }
      out.push(format!("    cx.new(move |{param}| {{"));
      out.extend(indent(&indent(&body)));
      out.push("    })".into());
      out.push("};".into());
    }
    out
  }

  /// The top-level items an entity-backed node needs beside its file's impls.
  pub fn emit_support(&mut self, id: NodeId) -> Vec<String> {
    let Some(node) = self.doc.node(id) else {
      return Vec::new();
    };
    let generator = self.generator;
    generator.support(self, node)
  }

  /// Subscriptions for entity-backed nodes with a bound event. guise entities
  /// emit rather than take a handler, so this is where their events land.
  pub fn emit_subscriptions(&self) -> Vec<String> {
    let mut out = Vec::new();
    for (id, field) in self.fields {
      let Some(node) = self.doc.node(*id) else {
        continue;
      };
      let spec = self.library.get(&node.kind);
      // A kind the generator wires itself has handlers on its element, not
      // events on its state; a subscription here would not typecheck.
      if self.generator.writes(&node.kind) {
        continue;
      }
      for (key, action) in &node.events {
        if action.is_empty() {
          continue;
        }
        let method = tailor_model::snake_case(action);
        // A state entity emits several events, and the handler is bound to one
        // of them. The catalog names it as a path — `InputEvent::Change` — so
        // the enum is the part before `::` and the whole thing is the pattern.
        // An enum with a type parameter writes it as `SelectEvent<_>::Confirm`:
        // the annotation needs the `<_>`, the pattern must not have it.
        let filter = spec
          .filter(|spec| spec.ctor.is_split())
          .and_then(|spec| spec.events.iter().find(|event| event.key == key))
          .and_then(|event| {
            event
              .method
              .split_once("::")
              .map(|(ty, _)| (ty, event.method))
          });
        match filter {
          Some((ty, pattern)) => {
            out.push(format!(
              "cx.subscribe(&{field}, |this, _entity, event: &{ty}, cx| {{"
            ));
            let pattern = pattern.replace("<_>", "");
            out.push(format!("    if matches!(event, {pattern}) {{"));
            out.push(format!("        this.{method}(cx);"));
            out.push("    }".into());
            out.push("}).detach();".into());
          }
          None => out.push(format!(
            "cx.subscribe(&{field}, |this, _entity, _event, cx| this.{method}(cx)).detach();"
          )),
        }
      }
    }
    out
  }

  /// The expression for a node, as lines. Line 0 is the constructor; the rest
  /// are chained calls, already indented one level relative to it.
  pub fn emit(&mut self, id: NodeId, placement: Placement) -> Vec<String> {
    let mut lines = self.emit_inner(id, placement);
    // Tag the expression with the node it came from. The tag is a comment
    // on its own line, so removing it later cannot disturb anything around
    // it — and removing it is how the line number is learned.
    lines.insert(0, format!("{MARK}{}", id.0));
    lines
  }

  fn emit_inner(&mut self, id: NodeId, placement: Placement) -> Vec<String> {
    let Some(node) = self.doc.node(id) else {
      return vec!["div()".into()];
    };
    if node.hidden {
      // Hidden is a canvas affordance, not a runtime condition — but a
      // node you have hidden is one you are not ready to ship, so it
      // leaves a marker rather than silently appearing in the output.
      let mut lines = vec!["div()".into()];
      lines.push(format!(
        "    // hidden in the designer: {}",
        self.label(node)
      ));
      return lines;
    }

    let mut lines = self.base(node);
    let container = self.library.boxes().contains(&node.kind.as_str());
    // The animation goes on the box the style system already emits, not
    // on a wrapper of its own: a wrapper is a new flex item, and a child
    // that was `w_full` would start measuring against it instead of the
    // row it was in. So a motion is one more reason to have a box.
    let motion = self.doc.motion_of(node.id);
    // A pinned child needs a box to pin, even if nothing else styles it.
    let styled = container || node.style.needs_wrapper() || placement.absolute || motion.is_some();
    // An entity was configured when it was built; here it is only a handle.
    // A stateful component is only half built there: its state was configured
    // in `new`, but the element over it still takes its own props.
    let configured_elsewhere = self.fields.contains_key(&node.id)
      && !self
        .library
        .get(&node.kind)
        .is_some_and(|spec| spec.ctor.is_split());

    if container {
      lines.extend(indent(&style::calls(
        &node.style,
        placement,
        true,
        self.flavor,
        self.hoist,
      )));
    }
    if !configured_elsewhere {
      lines.extend(indent(&self.prop_calls(node)));
      lines.extend(indent(&self.slot_calls(node)));
      lines.extend(indent(&self.event_calls(node)));
    }

    let mut lines = if styled && !container {
      self.wrap(lines, &node.style, placement)
    } else {
      lines
    };
    if let Some(motion) = motion {
      lines.extend(indent(&self.generator.motion(
        &node.id.element_id(),
        motion,
        placement.absolute,
        self.flavor,
      )));
    }
    lines
  }

  /// Put a styled `div` around a component that has box styling of its own.
  fn wrap(&mut self, inner: Vec<String>, style: &StyleProps, placement: Placement) -> Vec<String> {
    let mut lines = vec!["div()".into()];
    lines.extend(indent(&style::calls(
      style,
      placement,
      false,
      self.flavor,
      self.hoist,
    )));
    lines.extend(indent(&child_call(inner)));
    lines
  }

  fn label(&self, node: &Node) -> String {
    node
      .name
      .clone()
      .or_else(|| {
        self
          .library
          .get(&node.kind)
          .map(|spec| spec.title.to_string())
      })
      .unwrap_or_else(|| node.kind.clone())
  }

  /// The constructor line(s).
  fn base(&mut self, node: &Node) -> Vec<String> {
    if let Some(name) = node.component_ref() {
      let ty = tailor_model::pascal_case(name);
      // Generated documents are siblings in one module, so the placed
      // component is one `use super::` away.
      self.imports.insert(format!("use super::{ty};"));
      // A stateful one is an entity, built once and held in a field.
      if !self.fields.contains_key(&node.id) {
        return vec![format!("{ty}::new()")];
      }
    }
    // An entity lives in a field. How it names itself depends on where the
    // expression sits: a field in `render`, a local in `new`, and a cloned
    // local anywhere inside a closure.
    if let Some(field) = self.fields.get(&node.id).cloned() {
      // A stateful component's field is its state; what goes in the tree is
      // an element built over a borrow of it.
      let stateful = self
        .library
        .get(&node.kind)
        .filter(|spec| spec.ctor.is_split());
      if let Some(scope) = self.captures.last_mut() {
        scope.insert(field.clone());
        if stateful.is_none() {
          return vec![format!("{field}.clone()")];
        }
      }
      if let Some(spec) = stateful {
        let borrow = match (self.captures.is_empty(), self.phase) {
          (false, _) | (true, Phase::Init) => format!("&{field}"),
          (true, Phase::Render) => format!("&self.{field}"),
        };
        let generator = self.generator;
        if let Some(lines) = generator.element_over(self, node, &borrow) {
          return lines;
        }
        return vec![format!("{}::new({borrow})", spec.rust)];
      }
      return match (stateful, self.phase) {
        (Some(_), _) => unreachable!("handled above"),
        (None, Phase::Render) => vec![format!("self.{field}.clone()")],
        (None, Phase::Init) => vec![format!("{field}.clone()")],
      };
    }
    let Some(spec) = self.library.get(&node.kind) else {
      return vec!["div()".into()];
    };
    for import in spec.imports {
      self.imports.insert((*import).to_string());
    }
    if let Some(lines) = self.generator.special(self, node) {
      return lines;
    }
    let arg = |emitter: &mut Self, key: &str| -> String {
      let value = emitter.prop_value(node, key);
      let spec = spec
        .prop(key)
        .expect("catalog checks constructor props exist");
      expr::value(emitter.hoist, spec, &value, emitter.doc)
    };
    match spec.ctor {
      Ctor::Unit => vec![format!("{}::new()", spec.rust)],
      Ctor::Id => vec![format!(
        "{}::new({})",
        spec.rust,
        string(&node.id.element_id())
      )],
      Ctor::IdAnd(key) => {
        let value = arg(self, key);
        vec![format!(
          "{}::new({}, {value})",
          spec.rust,
          string(&node.id.element_id())
        )]
      }
      Ctor::Arg(key) => {
        let value = arg(self, key);
        vec![format!("{}::new({value})", spec.rust)]
      }
      Ctor::Args(keys) => {
        let values: Vec<String> = keys.iter().map(|key| arg(self, key)).collect();
        vec![format!("{}::new({})", spec.rust, values.join(", "))]
      }
      // Entities are constructed in `new()`, never here.
      Ctor::Entity
      | Ctor::EntityArg(_)
      | Ctor::EntityValue(_)
      | Ctor::Stateful(_)
      | Ctor::StatefulCx(_) => {
        vec![format!("{}::new(cx)", spec.rust)]
      }
      Ctor::Special => vec!["div()".into()],
    }
  }

  /// Open a closure scope. Anything an expression built inside it reaches for
  /// is recorded, because a `'static` closure has to clone it in first — see
  /// [`Emitter::wrap_closure`], which takes the scope back.
  pub fn push_scope(&mut self) {
    self.captures.push(BTreeSet::new());
  }

  pub fn pop_scope(&mut self) -> BTreeSet<String> {
    self.captures.pop().unwrap_or_default()
  }

  /// The innermost open closure's capture set, for a region that builds a
  /// second closure inside the first and has to tell the first what the second
  /// needs.
  pub fn scope_mut(&mut self) -> Option<&mut BTreeSet<String>> {
    self.captures.last_mut()
  }

  /// The weak handle a `'static` handler upgrades to reach the screen, spelled
  /// for where the expression sits — or `None` when there is no screen, which
  /// is a `RenderOnce` component with no `self` to call back into.
  ///
  /// Inside a closure region it is the `view` local the region cloned in, and
  /// the region is told to clone it; elsewhere it is made from the context.
  pub fn view_handle(&mut self) -> Option<String> {
    if self.owner != Owner::Entity {
      return None;
    }
    match self.captures.last_mut() {
      Some(scope) => {
        scope.insert(VIEW.to_string());
        Some(format!("{VIEW}.clone()"))
      }
      None => Some("cx.entity().downgrade()".to_string()),
    }
  }

  /// The value of a prop, falling back to the catalog default.
  pub fn prop_value(&self, node: &Node, key: &str) -> PropValue {
    node
      .prop(key)
      .cloned()
      .or_else(|| {
        self
          .library
          .get(&node.kind)
          .and_then(|spec| spec.default_prop(key))
      })
      .unwrap_or(PropValue::Text(String::new()))
  }

  fn prop_calls(&mut self, node: &Node) -> Vec<String> {
    let Some(spec) = self.library.get(&node.kind) else {
      return Vec::new();
    };
    let mut out = Vec::new();
    let split = spec.ctor.is_split();
    for prop in spec.props {
      let Some(value) = node.prop(prop.key) else {
        continue;
      };
      // The element's props wait for `render`, the state's for `new`.
      if split && self.phase == Phase::Init && !matches!(prop.emit, Emit::State(_)) {
        continue;
      }
      // A binding is never "the default" — it is a live read.
      let bound = value.as_binding().is_some();
      // The prop a two-way `X::bind` drives is set by that call, not by a
      // setter here; emitting both would fight over the same value.
      if bound && self.generator.entity_bind(&node.kind) == Some(prop.key) {
        continue;
      }
      // In `new` there is no `self` yet — a state variable is still a
      // local.
      let prefix = match self.phase {
        Phase::Render => "self.",
        Phase::Init => "",
      };
      // A controlled builder binds in the chain, and that binding replaces
      // the setter it drives.
      if bound && self.generator.controlled_bind(&node.kind) == Some(prop.key) {
        if let Some(var) = value.as_binding() {
          out.push(format!(
            ".bind({prefix}{}.binding())",
            tailor_model::snake_case(var)
          ));
          continue;
        }
      }
      match prop.emit {
        Emit::Method(method) => {
          if bound || (!expr::is_default(prop, value) && !value.is_empty()) {
            let arg = expr::value_with(self.hoist, prop, value, self.doc, prefix);
            out.push(format!(".{method}({arg})"));
          }
        }
        Emit::Flag(method) => {
          if value.as_bool() == Some(true) {
            out.push(format!(".{method}()"));
          }
        }
        Emit::State(method) => {
          if self.phase == Phase::Init
            && (bound || (!expr::is_default(prop, value) && !value.is_empty()))
          {
            let arg = expr::value_with(self.hoist, prop, value, self.doc, prefix);
            out.push(format!(".{method}({arg})"));
          }
        }
        Emit::Custom | Emit::None => {}
      }
    }
    out.extend(self.generator.custom_props(self, node));
    out
  }

  /// A list prop's lines.
  pub fn items(&self, node: &Node, key: &str) -> Option<Vec<String>> {
    match self.prop_value(node, key) {
      PropValue::Items(values) => Some(values),
      _ => None,
    }
  }

  /// A numeric list prop.
  pub fn numbers(&self, node: &Node, key: &str) -> Vec<f64> {
    match self.prop_value(node, key) {
      PropValue::Numbers(values) => values,
      _ => Vec::new(),
    }
  }

  fn slot_calls(&mut self, node: &Node) -> Vec<String> {
    let Some(spec) = self.library.get(&node.kind) else {
      return Vec::new();
    };
    let mut out = Vec::new();
    let absolute = node.style.layout == LayoutMode::Absolute;
    let placement = Placement { absolute };

    // A kind the generator writes whole: its regions are not `.child(..)` calls.
    if self.generator.writes(&node.kind) {
      let generator = self.generator;
      return generator.slots(self, node, placement).unwrap_or_default();
    }

    // The regions that take a closure rather than an element.
    let deferred = self.generator.closure_slots(&node.kind);

    for slot in spec.slots {
      let children = node.slot(slot.key);
      if children.is_empty() {
        continue;
      }
      if deferred.contains(&slot.key) {
        let head = match self.region_size(node, &node.kind.clone(), slot.key) {
          Some(value) => format!(".{}({}, ", slot.method, crate::rust::float(value)),
          None => format!(".{}(", slot.method),
        };
        out.extend(self.closure_region(head, children.first().copied()));
        continue;
      }
      for child in children {
        let inner = self.emit(*child, placement);
        out.extend(prefixed_call(slot.method, inner));
        if slot.single {
          break;
        }
      }
    }

    // A component that takes its regions some other way entirely.
    if let Some(lines) = self.generator.slots(self, node, placement) {
      out.extend(lines);
      return out;
    }

    if let Some(dynamic) = spec.dynamic {
      let labels = self.items(node, dynamic.from_prop).unwrap_or_default();
      let method = dynamic.method;
      for (index, label) in labels.iter().enumerate() {
        let key = format!("{}:{index}", dynamic.prefix);
        let children = node.slot(&key).to_vec();
        let head = format!(".{method}({}, ", string(label));
        if children.len() == 1 {
          out.extend(self.closure_region(head, children.first().copied()));
        } else {
          self.captures.push(BTreeSet::new());
          let mut wrapper = vec!["div().flex().flex_col().gap(px(8.))".into()];
          for child in &children {
            let child_lines = self.emit(*child, placement);
            wrapper.extend(indent(&child_call(child_lines)));
          }
          let captured = self.captures.pop().unwrap_or_default();
          out.extend(self.wrap_closure(head, captured, wrapper));
        }
      }
    }
    out
  }

  /// The size a closure region takes before its content, when the provider
  /// says it takes one.
  fn region_size(&self, node: &Node, kind: &str, slot: &str) -> Option<f32> {
    let key = self.generator.slot_size_prop(kind, slot)?;
    self.prop_value(node, key).as_f64().map(|v| v as f32)
  }

  /// Emit a region whose content is a `'static` closure, cloning in whatever
  /// the closure reaches for. `head` is everything before the closure —
  /// `.header(56., ` or `.tab("Overview", `.
  pub fn closure_region(&mut self, head: String, id: Option<NodeId>) -> Vec<String> {
    self.closure_region_with(head, "|_window, _cx|", id)
  }

  /// Run `build` as the inside of a `'static` closure and hand back what it
  /// reached for, which the caller then clones in ahead of the closure with
  /// [`Emitter::wrap_closure_with`]. For a closure whose body is not one
  /// element — a `match` over the row index — where `closure_region` does not
  /// fit.
  pub fn scoped<T>(&mut self, build: impl FnOnce(&mut Self) -> T) -> (T, BTreeSet<String>) {
    self.captures.push(BTreeSet::new());
    let out = build(self);
    let captured = self.captures.pop().unwrap_or_default();
    (out, captured)
  }

  /// The same, for a region whose closure takes other parameters than a window
  /// and a context — a settings item is handed its render options first.
  pub fn closure_region_with(
    &mut self,
    head: String,
    params: &str,
    id: Option<NodeId>,
  ) -> Vec<String> {
    self.captures.push(BTreeSet::new());
    let inner = match id {
      Some(id) => self.emit(id, Placement { absolute: false }),
      None => vec!["div()".into()],
    };
    let captured = self.captures.pop().unwrap_or_default();
    self.wrap_closure_with(head, params, captured, inner)
  }

  pub fn wrap_closure(
    &mut self,
    head: String,
    captured: BTreeSet<String>,
    inner: Vec<String>,
  ) -> Vec<String> {
    self.wrap_closure_with(head, "|_window, _cx|", captured, inner)
  }

  /// The same, for a region whose closure takes more than a window and a
  /// context — `SettingsView::content` is handed the active page and the
  /// search query first.
  pub fn wrap_closure_with(
    &mut self,
    head: String,
    params: &str,
    captured: BTreeSet<String>,
    inner: Vec<String>,
  ) -> Vec<String> {
    let mut out = Vec::new();
    if captured.is_empty() {
      out.push(format!("{head}{params} {{"));
      out.extend(indent(&inner));
      out.push("})".into());
      return out;
    }
    out.push(format!("{head}{{"));
    // Inside another closure `self` and `cx` are not the method's, so what this
    // one needs is cloned from a local the enclosing closure already holds —
    // and the enclosing closure is told it needs it.
    let nested = !self.captures.is_empty();
    for name in &captured {
      let source = if nested {
        format!("{name}.clone()")
      } else if name == VIEW {
        // Weak, not strong: a closure held by a live component tree
        // must not own the view that renders it.
        "cx.entity().downgrade()".to_string()
      } else {
        match self.phase {
          Phase::Render => format!("self.{name}.clone()"),
          Phase::Init => format!("{name}.clone()"),
        }
      };
      out.push(format!("    let {name} = {source};"));
      if let Some(scope) = self.captures.last_mut() {
        scope.insert(name.clone());
      }
    }
    out.push(format!("    move {params} {{"));
    out.extend(indent(&indent(&inner)));
    out.push("    }".into());
    out.push("})".into());
    out
  }

  fn event_calls(&mut self, node: &Node) -> Vec<String> {
    let Some(spec) = self.library.get(&node.kind) else {
      return Vec::new();
    };
    // Entity components emit rather than take a handler; `emit_subscriptions`
    // wires those in `new`.
    if spec.ctor.is_entity() {
      return Vec::new();
    }
    // The generator wires this kind's handlers with its own signatures.
    if self.generator.writes(&node.kind) {
      return Vec::new();
    }
    let mut out = Vec::new();
    for event in spec.events {
      let Some(action) = node.events.get(event.key) else {
        continue;
      };
      if action.is_empty() {
        continue;
      }
      let method = tailor_model::snake_case(action);
      let args: Vec<String> = event
        .args
        .iter()
        .map(|arg| format!("_{}", arg.trim_start_matches('_')))
        .collect();
      let params = if args.is_empty() {
        String::new()
      } else {
        format!("{}, ", args.join(", "))
      };
      match self.owner {
        // Inside a region closure there is no `cx` to listen with: the
        // closure is `'static` and outlives the borrow. A weak handle
        // is cloned in ahead of it and upgraded when the event fires —
        // the same shape a hand-written host uses.
        Owner::Entity if !self.captures.is_empty() => {
          if let Some(scope) = self.captures.last_mut() {
            scope.insert(VIEW.to_string());
          }
          // Cloned again per handler: the region closure is `Fn`, so
          // a `move` handler inside it would move the shared handle
          // out of the closure that owns it.
          out.push(format!(".{}({{", event.method));
          out.push(format!("    let {VIEW} = {VIEW}.clone();"));
          out.push(format!("    move |{params}_window, cx| {{"));
          out.push(format!(
            "        {VIEW}.update(cx, |this, cx| this.{method}(cx)).ok();"
          ));
          out.push("    }".into());
          out.push("})".into());
        }
        Owner::Entity => out.push(format!(
          ".{}(cx.listener(|this, {params}_window, cx| this.{method}(cx)))",
          event.method
        )),
        Owner::Plain => out.push(format!(
          ".{}(|{params}_window, _cx| {{ /* {action} */ }})",
          event.method
        )),
      }
    }
    out
  }
}

/// `.child(<expr>)`, inlined when the expression is short enough to read on
/// one line.
pub fn child_call(inner: Vec<String>) -> Vec<String> {
  prefixed_call("child", inner)
}

/// `.method(<expr>)`, inlined when the expression is short enough to read on
/// one line. [`child_call`] for the common method.
pub fn prefixed_call(method: &str, inner: Vec<String>) -> Vec<String> {
  // A node's tag rides in front of its expression. Take it off before
  // deciding whether that expression fits on one line, then put it back in
  // front of the call — the tag has to name the line the expression lands on,
  // and after collapsing that line is the `.child(..)` itself.
  let (mark, inner) = split_mark(inner);
  if inner.len() == 1 && inner[0].len() + method.len() + 8 <= 96 {
    // Collapsed: the call and the expression are the same line, so the tag
    // goes in front of it.
    let mut out = vec![format!(".{method}({})", inner[0])];
    if let Some(mark) = mark {
      out.insert(0, mark);
    }
    return out;
  }
  // Expanded: the tag goes *inside*, so it names the constructor rather than
  // the `.child(` that wraps it. Landing on `.child(` is landing next to the
  // component instead of on it.
  let mut out = vec![format!(".{method}(")];
  if let Some(mark) = mark {
    out.push(mark);
  }
  out.extend(indent(&inner));
  out.push(")".into());
  out
}

/// Split a leading node tag off a fragment, if it has one.
fn split_mark(mut lines: Vec<String>) -> (Option<String>, Vec<String>) {
  if lines
    .first()
    .map(|line| line.trim_start().starts_with(MARK))
    .unwrap_or(false)
  {
    let mark = lines.remove(0);
    return (Some(mark), lines);
  }
  (None, lines)
}

/// How far a chain of placed components is followed. A cycle is refused when
/// it is created, so this only catches a hand-edited file.
const DEPTH: usize = 8;

/// The document a node places, when it places one.
fn placed<'p>(project: &'p Project, node: &Node) -> Option<&'p Document> {
  let name = node.component_ref()?;
  project.docs.iter().find(|doc| doc.name == name)
}

/// Whether a document generates as a `Render` entity rather than a `RenderOnce`
/// builder: a screen, or anything holding state, an action, or an entity.
pub fn is_entity(project: &Project, doc: &Document) -> bool {
  entity_of(project, doc, 0)
}

fn entity_of(project: &Project, doc: &Document, depth: usize) -> bool {
  doc.kind == tailor_model::DocKind::Screen
    || !doc.state.is_empty()
    || !doc.actions.is_empty()
    || !fields_at(project, doc, depth).is_empty()
}

/// Whether building this document's entities takes a window, which its `new`
/// then has to be handed — and so does whoever constructs it. A placed
/// component that needs one passes the need up to whatever places it.
pub fn needs_window(project: &Project, doc: &Document) -> bool {
  window_at(project, doc, 0)
}

fn window_at(project: &Project, doc: &Document, depth: usize) -> bool {
  let library = project.library();
  fields_at(project, doc, depth).iter().any(|(id, _)| {
    let Some(node) = doc.node(*id) else {
      return false;
    };
    match placed(project, node) {
      Some(target) => depth < DEPTH && window_at(project, target, depth + 1),
      None => library
        .get(&node.kind)
        .is_some_and(|spec| spec.ctor.needs_window()),
    }
  })
}

/// Every entity-backed node in the document, **in the order they must be
/// built**: children before the parents that capture them. A `Tabs` whose panel
/// holds a `Slider` clones that slider into its content closure, so the slider
/// has to be a local by the time the tabs are constructed.
///
/// A placed component that is itself an entity counts: it is built once, in
/// `new`, and held in a field like any other.
///
/// A `Vec`, not a map: the order is the whole point, and a map keyed by id
/// would silently sort it back into creation order.
pub fn entity_fields(project: &Project, doc: &Document) -> Vec<(NodeId, String)> {
  fields_at(project, doc, 0)
}

fn fields_at(project: &Project, doc: &Document, depth: usize) -> Vec<(NodeId, String)> {
  let library = project.library();
  let mut ordered = Vec::new();
  collect(project, doc, doc.root, &mut ordered, depth);
  // State variables get their names first: the user typed those, and a node
  // called "Email" beside a variable called `email` is a real thing to do.
  let mut used: BTreeSet<String> = doc
    .state
    .iter()
    .map(|var| tailor_model::snake_case(&var.name))
    .collect();
  let mut fields = Vec::new();
  for id in ordered {
    let Some(node) = doc.node(id) else { continue };
    let base = node
      .name
      .clone()
      .or_else(|| node.component_ref().map(str::to_string))
      .or_else(|| library.get(&node.kind).map(|spec| spec.title.to_string()))
      .unwrap_or_else(|| node.kind.clone());
    let mut name = tailor_model::snake_case(&base);
    if used.contains(&name) {
      name = format!("{name}_field");
    }
    if used.contains(&name) {
      name = format!("{name}_{}", id.0);
    }
    used.insert(name.clone());
    fields.push((id, name));
  }
  fields
}

/// Post-order so a parent entity is built after the children it may capture.
fn collect(project: &Project, doc: &Document, id: NodeId, out: &mut Vec<NodeId>, depth: usize) {
  let Some(node) = doc.node(id) else { return };
  for child in node.all_children() {
    collect(project, doc, child, out, depth);
  }
  match placed(project, node) {
    Some(target) => {
      if depth < DEPTH && entity_of(project, target, depth + 1) {
        out.push(id);
      }
    }
    None => {
      if let Some(spec) = project.library().get(&node.kind) {
        if spec.ctor.is_entity() {
          out.push(id);
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn short_children_inline_and_long_ones_nest() {
    assert_eq!(
      child_call(vec!["Text::new(\"hi\")".into()]),
      [".child(Text::new(\"hi\"))"]
    );
    let nested = child_call(vec!["div()".into(), "    .flex()".into()]);
    assert_eq!(nested, [".child(", "    div()", "        .flex()", ")"]);
  }
}
