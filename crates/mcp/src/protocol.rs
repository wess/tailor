//! JSON-RPC 2.0 over stdio, which is all MCP's stdio transport is.
//!
//! Hand-rolled rather than pulled from a crate: the surface Tailor needs is
//! four message shapes and a newline-delimited framing, and the rest of this
//! repository is gpui and std.

use serde_json::{json, Value};

/// The protocol revision this server speaks.
pub const PROTOCOL_VERSION: &str = "2024-11-05";

#[derive(Debug, Clone)]
pub struct Request {
    /// Absent for a notification, which takes no response.
    pub id: Option<Value>,
    pub method: String,
    pub params: Value,
}

impl Request {
    pub fn parse(line: &str) -> Option<Request> {
        let value: Value = serde_json::from_str(line).ok()?;
        let method = value.get("method")?.as_str()?.to_string();
        Some(Request {
            id: value.get("id").cloned(),
            method,
            params: value.get("params").cloned().unwrap_or(Value::Null),
        })
    }

    pub fn is_notification(&self) -> bool {
        self.id.is_none()
    }

    pub fn arg(&self, key: &str) -> Option<&Value> {
        self.params.get(key)
    }
}

/// The JSON-RPC error codes this server returns.
pub const METHOD_NOT_FOUND: i64 = -32601;
pub const INTERNAL_ERROR: i64 = -32603;

pub fn result(id: &Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

pub fn error(id: &Value, code: i64, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message.into() }
    })
}

/// A tool result. MCP wants content blocks even when the answer is one line.
pub fn text_content(text: impl Into<String>) -> Value {
    json!({ "content": [{ "type": "text", "text": text.into() }], "isError": false })
}

/// A tool that failed. This is a *successful* JSON-RPC response carrying an
/// error flag — the call reached the tool, the tool said no.
pub fn tool_error(text: impl Into<String>) -> Value {
    json!({ "content": [{ "type": "text", "text": text.into() }], "isError": true })
}

/// Pretty JSON as a text block, which is what every structured answer here is.
pub fn json_content(value: Value) -> Value {
    text_content(serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_parses_and_knows_whether_it_wants_an_answer() {
        let call = Request::parse(r#"{"jsonrpc":"2.0","id":3,"method":"tools/list"}"#).unwrap();
        assert_eq!(call.method, "tools/list");
        assert!(!call.is_notification());

        let note =
            Request::parse(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).unwrap();
        assert!(note.is_notification());

        assert!(Request::parse("not json").is_none());
        assert!(Request::parse(r#"{"jsonrpc":"2.0","id":1}"#).is_none());
    }

    #[test]
    fn arguments_come_out_of_params() {
        let call = Request::parse(r#"{"id":1,"method":"tools/call","params":{"name":"outline"}}"#)
            .unwrap();
        assert_eq!(call.arg("name").and_then(|v| v.as_str()), Some("outline"));
        assert!(call.arg("missing").is_none());
    }

    #[test]
    fn a_tool_error_is_still_a_successful_response() {
        let value = tool_error("no project is open");
        assert_eq!(value["isError"], json!(true));
        assert_eq!(value["content"][0]["text"], json!("no project is open"));
    }
}
