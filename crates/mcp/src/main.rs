//! Tailor as an MCP server: build and generate interfaces without opening the
//! app.
//!
//! It works on `.tailor` files and saves after every change, which is the whole
//! integration story — the app watches the file it has open, so a screen built
//! here appears on the canvas a moment later with nothing wired between the two
//! processes.
//!
//! ```sh
//! claude mcp add tailor -- tailor-mcp
//! ```

mod protocol;
mod session;
mod tools;

use std::io::{BufRead, Write};

use serde_json::{json, Value};

use protocol::{error, result, Request, INTERNAL_ERROR, METHOD_NOT_FOUND, PROTOCOL_VERSION};
use session::Session;

fn main() {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut session = Session::default();

    // Newline-delimited JSON, which is MCP's stdio framing. A line that does
    // not parse is skipped rather than fatal: a half-written line should not
    // take the server down mid-conversation.
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let Some(request) = Request::parse(&line) else {
            continue;
        };
        let Some(response) = handle(&mut session, &request) else {
            continue;
        };
        if writeln!(stdout, "{response}").is_err() {
            break;
        }
        let _ = stdout.flush();
    }
}

/// One message in, at most one message out. Notifications answer `None`.
fn handle(session: &mut Session, request: &Request) -> Option<Value> {
    if request.is_notification() {
        return None;
    }
    let id = request.id.clone()?;

    Some(match request.method.as_str() {
        "initialize" => result(
            &id,
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": "tailor", "version": env!("CARGO_PKG_VERSION") },
                "instructions": INSTRUCTIONS,
            }),
        ),
        "tools/list" => result(&id, tools::list()),
        "tools/call" => {
            let Some(name) = request.arg("name").and_then(|v| v.as_str()) else {
                return Some(error(&id, INTERNAL_ERROR, "a tool call needs a `name`"));
            };
            let args = tools::arguments(&request.params);
            result(&id, tools::call(session, name, &args))
        }
        "ping" => result(&id, json!({})),
        other => error(&id, METHOD_NOT_FOUND, format!("no method called {other}")),
    })
}

/// What the client is told the server is for. Worth the words: an agent that
/// reads this stops guessing at prop names.
const INSTRUCTIONS: &str = "\
Tailor builds gpui/guise interfaces. Open a .tailor project, place components \
into its node tree, wire state and actions, then generate or export idiomatic \
Rust.

Start with `overview` and `outline` to see what is there. Before setting props \
on a component you have not used, call `component` for its exact prop keys, \
types and defaults — `catalog` lists what can be placed. Nodes are addressed by \
the integer ids `outline` prints.

Every change saves the file immediately, so a running Tailor window picks it up.";

#[cfg(test)]
mod tests {
    use super::*;

    fn request(text: &str) -> Request {
        Request::parse(text).unwrap()
    }

    #[test]
    fn initialize_reports_the_protocol_and_the_server() {
        let mut session = Session::default();
        let response = handle(&mut session, &request(r#"{"id":1,"method":"initialize"}"#)).unwrap();
        assert_eq!(
            response["result"]["protocolVersion"],
            json!(PROTOCOL_VERSION)
        );
        assert_eq!(response["result"]["serverInfo"]["name"], json!("tailor"));
        assert!(response["result"]["instructions"]
            .as_str()
            .unwrap()
            .contains("outline"));
    }

    #[test]
    fn a_notification_gets_no_answer() {
        let mut session = Session::default();
        assert!(handle(
            &mut session,
            &request(r#"{"method":"notifications/initialized"}"#)
        )
        .is_none());
    }

    #[test]
    fn an_unknown_method_is_a_jsonrpc_error() {
        let mut session = Session::default();
        let response = handle(&mut session, &request(r#"{"id":2,"method":"dance"}"#)).unwrap();
        assert_eq!(response["error"]["code"], json!(METHOD_NOT_FOUND));
    }

    /// The whole shape of a session: create, place, wire, generate.
    #[test]
    fn a_screen_can_be_built_end_to_end() {
        let dir = std::env::temp_dir().join("tailor-mcp-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("demo.tailor");

        let mut session = Session::default();
        let created = tools::call(
            &mut session,
            "create_project",
            &json!({ "path": path.to_string_lossy(), "name": "Demo" }),
        );
        assert_eq!(created["isError"], json!(false));

        let outline = tools::call(&mut session, "outline", &json!({}));
        let text = outline["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("#1"), "{text}");

        let added = tools::call(
            &mut session,
            "add_node",
            &json!({
                "kind": "button",
                "parent": 1,
                "name": "Submit",
                "props": { "label": "Save", "variant": "outline", "size": "lg" },
                "style": { "width": 200, "padding": 8 }
            }),
        );
        assert_eq!(added["isError"], json!(false), "{added}");

        tools::call(&mut session, "add_action", &json!({ "name": "save" }));
        let wired = tools::call(
            &mut session,
            "connect_event",
            &json!({ "node": 2, "event": "click", "action": "save" }),
        );
        assert_eq!(wired["isError"], json!(false), "{wired}");

        let code = tools::call(&mut session, "generate_code", &json!({}));
        let source = code["content"][0]["text"].as_str().unwrap();
        assert!(
            source.contains("Button::new(\"node-2\", \"Save\")"),
            "{source}"
        );
        assert!(source.contains(".variant(Variant::Outline)"));
        assert!(source.contains("pub fn save(&mut self"));
        assert!(source.contains("cx.listener("));

        // Everything landed on disk as it went.
        let reloaded = tailor_store::open(&path).unwrap();
        assert_eq!(reloaded.docs[0].nodes.len(), 2);

        let problems = tools::call(&mut session, "problems", &json!({}));
        assert_eq!(problems["isError"], json!(false));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn creating_over_an_existing_project_is_refused() {
        let dir = std::env::temp_dir().join("tailor-mcp-overwrite");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("p.tailor");

        let mut session = Session::default();
        let args = json!({ "path": path.to_string_lossy() });
        assert_eq!(
            tools::call(&mut session, "create_project", &args)["isError"],
            json!(false)
        );

        let again = tools::call(&mut session, "create_project", &args);
        assert_eq!(again["isError"], json!(true));
        assert!(again["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("already exists"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_bad_prop_says_what_the_component_actually_takes() {
        let dir = std::env::temp_dir().join("tailor-mcp-props");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mut session = Session::default();
        tools::call(
            &mut session,
            "create_project",
            &json!({ "path": dir.join("p.tailor").to_string_lossy() }),
        );
        let refused = tools::call(
            &mut session,
            "add_node",
            &json!({ "kind": "button", "props": { "labl": "typo" } }),
        );
        assert_eq!(refused["isError"], json!(true));
        let message = refused["content"][0]["text"].as_str().unwrap();
        assert!(message.contains("no prop `labl`"), "{message}");
        assert!(message.contains("label"), "{message}");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
