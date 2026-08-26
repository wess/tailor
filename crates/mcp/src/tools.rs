//! The tool table, and the dispatch from a `tools/call` to the session.
//!
//! The descriptions matter more than usual here: they are the only
//! documentation the caller gets, and an agent that has to guess at prop names
//! will write four wrong calls before a right one. `catalog` and `component`
//! exist so it never has to guess.

use serde_json::{json, Map, Value};

use crate::protocol::{json_content, text_content, tool_error};
use crate::session::{Edit, Placement, Session};

/// Every tool, with its JSON Schema.
/// What the `motion` object accepts. Spelled out once: an agent that has to
/// guess the vocabulary writes `"ease": "easeOut"` and gets an error instead
/// of an animation.
const MOTION_DOC: &str = "Entrance animation. Keys: `enter` (fade, slideup, slidedown, \
     slideleft, slideright, or null to turn it off), `ease` (linear, out-quad, out-cubic, \
     out-quint, out-expo, out-circ, out-back, out-elastic, out-bounce, in-quad, in-cubic, \
     in-expo, in-out-quad, in-out-cubic, in-out-sine, spring), `duration` and `delay` in ms, \
     `distance` in px for the slides, `stagger` in ms (non-zero animates this node's children \
     one after another instead of the node itself), `repeat` (once, forever) and `alternate`.";

pub fn list() -> Value {
  let string = |description: &str| json!({ "type": "string", "description": description });
  let integer = |description: &str| json!({ "type": "integer", "description": description });
  let doc = || {
    json!({
        "type": "string",
        "description": "Document id or name. Defaults to the one last opened or added."
    })
  };

  json!({ "tools": [
      tool("open_project",
          "Open a .tailor project. Every other tool works on what this opened.",
          json!({ "path": string("Path to the .tailor file") }), &["path"]),
      tool("create_project",
          "Create an empty .tailor project at a path and open it.",
          json!({
              "path": string("Where to write it"),
              "name": string("Project name; defaults to the file name")
          }), &["path"]),
      tool("overview",
          "What is in the open project: documents, state, actions, theme, problem counts.",
          json!({}), &[]),
      tool("outline",
          "The node tree of a document, as indented text with node ids.",
          json!({ "document": doc() }), &[]),
      tool("catalog",
          "The components you can place. Search or filter before adding anything — the kind \
           strings here are what add_node takes.",
          json!({
              "query": string("Free text; matches titles and blurbs"),
              "category": string("Layout, Typography, Controls, Inputs, Data, Feedback, Navigation, Charts, Media")
          }), &[]),
      tool("component",
          "Everything about one component: its props, their types and defaults, its slots and \
           its events. Call this before setting props you have not set before.",
          json!({ "kind": string("A catalog kind, such as `button`") }), &["kind"]),
      tool("add_node",
          "Place a component. Returns the new node's id. `kind` is a catalog kind, or `@Name` \
           to place one of this project's own components.",
          json!({
              "document": doc(),
              "kind": string("Catalog kind, or @ComponentName"),
              "parent": integer("Parent node id; defaults to the document root"),
              "slot": string("Named slot, such as `footer`. Defaults to the children slot."),
              "index": integer("Where among the siblings; defaults to last"),
              "name": string("What to call it in the outline"),
              "props": json!({ "type": "object", "description": "Component props by key" }),
              "style": json!({ "type": "object", "description": "Box and layout style" }),
              "motion": json!({ "type": "object", "description": MOTION_DOC })
          }), &["kind"]),
      tool("set_node",
          "Change a node: its props, its style, its entrance animation, its name, whether it \
           is hidden or locked. Props, style and motion are merged, not replaced.",
          json!({
              "document": doc(),
              "node": integer("Node id"),
              "name": string("New outline name"),
              "props": json!({ "type": "object" }),
              "style": json!({ "type": "object" }),
              "motion": json!({ "type": "object", "description": MOTION_DOC }),
              "hidden": json!({ "type": "boolean" }),
              "locked": json!({ "type": "boolean" })
          }), &["node"]),
      tool("move_node",
          "Move a node under a new parent, or to a new position among its siblings.",
          json!({
              "document": doc(),
              "node": integer("Node id"),
              "parent": integer("New parent node id"),
              "slot": string("Named slot"),
              "index": integer("Position among the siblings")
          }), &["node", "parent"]),
      tool("remove_node",
          "Delete a node and everything under it.",
          json!({ "document": doc(), "node": integer("Node id") }), &["node"]),
      tool("add_document",
          "Add a screen or a component to the project.",
          json!({
              "name": string("Name; becomes the generated Rust type"),
              "kind": string("`screen` (a Render entity) or `component` (a RenderOnce builder)")
          }), &["name"]),
      tool("add_state",
          "Add a state variable to a document. It becomes a Signal<T> field on the generated type.",
          json!({
              "document": doc(),
              "name": string("Variable name"),
              "type": string("text, bool, int, float or items"),
              "initial": string("Starting value, as text")
          }), &["name"]),
      tool("add_action",
          "Add an action to a document. It becomes a method on the generated type.",
          json!({ "document": doc(), "name": string("Action name") }), &["name"]),
      tool("bind_prop",
          "Make a prop read a state variable instead of a literal.",
          json!({
              "document": doc(),
              "node": integer("Node id"),
              "prop": string("Prop key"),
              "variable": string("State variable name")
          }), &["node", "prop", "variable"]),
      tool("connect_event",
          "Wire a component's event to an action.",
          json!({
              "document": doc(),
              "node": integer("Node id"),
              "event": string("Event key, such as `click`"),
              "action": string("Action name")
          }), &["node", "event", "action"]),
      tool("set_theme",
          "Set the project theme the design is drawn against.",
          json!({
              "scheme": string("dark or light"),
              "primary": string("A palette name, such as `violet`"),
              "radius": string("xs, sm, md, lg or xl"),
              "font": string("Font family")
          }), &[]),
      tool("generate_code",
          "The guise Rust for one document, without writing anything.",
          json!({ "document": doc() }), &[]),
      tool("export_code",
          "Write the whole project as a runnable crate: one file per document, a mod.rs, a \
           main.rs, a theme.rs and a Cargo.toml.",
          json!({ "directory": string("Where to write it") }), &["directory"]),
      tool("problems",
          "The lint pass: bindings to variables that are gone, events pointing at missing \
           actions, components that will not generate where they are.",
          json!({}), &[]),
  ]})
}

fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
  json!({
      "name": name,
      "description": description,
      "inputSchema": {
          "type": "object",
          "properties": properties,
          "required": required,
      }
  })
}

/// Run one tool call.
pub fn call(session: &mut Session, name: &str, args: &Value) -> Value {
  let text = |key: &str| args.get(key).and_then(|v| v.as_str());
  let number = |key: &str| args.get(key).and_then(|v| v.as_u64());
  let object = |key: &str| args.get(key).and_then(|v| v.as_object());
  let doc = text("document");

  let outcome = match name {
    "open_project" => match text("path") {
      Some(path) => session.open(std::path::Path::new(path)),
      None => Err("`path` is required".into()),
    },
    "create_project" => match text("path") {
      Some(path) => {
        let fallback = std::path::Path::new(path)
          .file_stem()
          .map(|stem| stem.to_string_lossy().to_string())
          .unwrap_or_else(|| "Untitled".into());
        let name = text("name").map(|n| n.to_string()).unwrap_or(fallback);
        session.create(std::path::Path::new(path), &name)
      }
      None => Err("`path` is required".into()),
    },
    "overview" => session.overview(),
    "outline" => session.outline(doc),
    "catalog" => session.catalog(text("query"), text("category")),
    "component" => match text("kind") {
      Some(kind) => session.component(kind),
      None => Err("`kind` is required".into()),
    },
    "add_node" => match text("kind") {
      Some(kind) => session.add_node(
        doc,
        Placement {
          parent: number("parent").map(|v| v as u32),
          kind,
          slot: text("slot"),
          index: number("index").map(|v| v as usize),
          name: text("name"),
          props: object("props"),
          style: object("style"),
          motion: object("motion"),
        },
      ),
      None => Err("`kind` is required".into()),
    },
    "set_node" => match number("node") {
      Some(node) => session.set_node(
        doc,
        node as u32,
        Edit {
          name: text("name"),
          props: object("props"),
          style: object("style"),
          motion: object("motion"),
          hidden: args.get("hidden").and_then(|v| v.as_bool()),
          locked: args.get("locked").and_then(|v| v.as_bool()),
        },
      ),
      None => Err("`node` is required".into()),
    },
    "move_node" => match (number("node"), number("parent")) {
      (Some(node), Some(parent)) => session.move_node(
        doc,
        node as u32,
        parent as u32,
        text("slot"),
        number("index").map(|v| v as usize),
      ),
      _ => Err("`node` and `parent` are required".into()),
    },
    "remove_node" => match number("node") {
      Some(node) => session.remove_node(doc, node as u32),
      None => Err("`node` is required".into()),
    },
    "add_document" => match text("name") {
      Some(name) => session.add_document(name, text("kind").unwrap_or("screen")),
      None => Err("`name` is required".into()),
    },
    "add_state" => match text("name") {
      Some(name) => session.add_state(doc, name, text("type").unwrap_or("text"), text("initial")),
      None => Err("`name` is required".into()),
    },
    "add_action" => match text("name") {
      Some(name) => session.add_action(doc, name),
      None => Err("`name` is required".into()),
    },
    "bind_prop" => match (number("node"), text("prop"), text("variable")) {
      (Some(node), Some(prop), Some(var)) => session.bind(doc, node as u32, prop, var),
      _ => Err("`node`, `prop` and `variable` are required".into()),
    },
    "connect_event" => match (number("node"), text("event"), text("action")) {
      (Some(node), Some(event), Some(action)) => session.connect(doc, node as u32, event, action),
      _ => Err("`node`, `event` and `action` are required".into()),
    },
    "set_theme" => session.set_theme(
      text("scheme"),
      text("primary"),
      text("radius"),
      text("font"),
    ),
    "generate_code" => session.code(doc),
    "export_code" => match text("directory") {
      Some(dir) => session.export(std::path::Path::new(dir)),
      None => Err("`directory` is required".into()),
    },
    "problems" => session.problems(),
    other => Err(format!("no tool called {other}")),
  };

  match outcome {
    // The generated source is the answer, so hand it over as text rather
    // than as a JSON string with the newlines escaped.
    Ok(value) if name == "generate_code" => {
      let source = value
        .get("source")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
      text_content(source)
    }
    Ok(value) => json_content(value),
    Err(message) => tool_error(message),
  }
}

/// The arguments object of a `tools/call`, defaulted when absent.
pub fn arguments(params: &Value) -> Value {
  params
    .get("arguments")
    .cloned()
    .unwrap_or(Value::Object(Map::new()))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn every_tool_has_a_schema_and_a_description() {
    let tools = list();
    let tools = tools["tools"].as_array().unwrap();
    assert!(tools.len() >= 18);
    for tool in tools {
      assert!(tool["name"].as_str().is_some_and(|n| !n.is_empty()));
      assert!(tool["description"].as_str().is_some_and(|d| d.len() > 20));
      assert_eq!(tool["inputSchema"]["type"], json!("object"));
    }
  }

  #[test]
  fn the_node_tools_advertise_the_motion_object() {
    let tools = list();
    let tools = tools["tools"].as_array().unwrap();
    for name in ["add_node", "set_node"] {
      let tool = tools
        .iter()
        .find(|t| t["name"] == json!(name))
        .unwrap_or_else(|| panic!("no {name} tool"));
      let motion = &tool["inputSchema"]["properties"]["motion"];
      assert_eq!(motion["type"], json!("object"), "{name}");
      // The vocabulary has to be in the description: an agent that
      // guesses `easeOut` gets an error instead of an animation.
      let doc = motion["description"].as_str().unwrap();
      assert!(doc.contains("slideup") && doc.contains("out-back"));
    }
  }

  #[test]
  fn an_unknown_tool_is_an_error_not_a_panic() {
    let mut session = Session::default();
    let value = call(&mut session, "fly", &json!({}));
    assert_eq!(value["isError"], json!(true));
  }

  #[test]
  fn tools_refuse_to_run_without_a_project() {
    let mut session = Session::default();
    let value = call(&mut session, "outline", &json!({}));
    assert_eq!(value["isError"], json!(true));
    assert!(value["content"][0]["text"]
      .as_str()
      .unwrap()
      .contains("no project"));
  }
}
