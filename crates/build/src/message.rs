//! Cargo's JSON output, as something the app can act on.
//!
//! `cargo build --message-format=json` prints one JSON object per line. Three
//! of them matter: a **compiler message** (a diagnostic to show), a **compiler
//! artifact** (which carries the path of the binary that was just linked), and
//! **build finished** (whether it worked). Everything else is skipped rather
//! than treated as an error — cargo adds message kinds between releases, and a
//! build should not fail because one of them is new.
//!
//! A line that is not JSON at all is not an error either. Cargo writes its
//! progress to stderr, but a build script can print anything it likes to
//! stdout, and that lands in the middle of the stream.

use std::path::PathBuf;

use serde_json::Value;

/// How bad a compiler message is. Ordered so `max` picks the worst, which is
/// what decides whether a build "has errors".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
  Note,
  Help,
  Warning,
  Error,
}

impl Severity {
  fn parse(level: &str) -> Option<Severity> {
    Some(match level {
      "error" | "error: internal compiler error" => Severity::Error,
      "warning" => Severity::Warning,
      "help" => Severity::Help,
      "note" | "failure-note" => Severity::Note,
      _ => return None,
    })
  }

  pub fn label(self) -> &'static str {
    match self {
      Severity::Error => "error",
      Severity::Warning => "warning",
      Severity::Help => "help",
      Severity::Note => "note",
    }
  }
}

/// One compiler message, anchored where the app can use it.
///
/// The location is the *primary* span — the one rustc underlines. A message
/// with no primary span (a bare `note`, a link error) still comes through, with
/// `file` empty, because a build that failed for a reason nobody can see is
/// worse than one line in the console with no line number on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
  /// Relative to the workspace root, as cargo reports it.
  pub file: String,
  /// 1-based, as rustc counts.
  pub line: usize,
  /// 1-based, and half-open at the end, as rustc counts.
  pub col: usize,
  pub col_end: usize,
  pub severity: Severity,
  /// The one-line summary — "cannot find value `x` in this scope".
  pub message: String,
  /// rustc's full rendering, carets and all. What the console shows.
  pub rendered: String,
  /// `E0425`, when there is one.
  pub code: Option<String>,
}

impl Diagnostic {
  /// `src/ui/main.rs:12:5`, or just the message when there is no span.
  pub fn location(&self) -> String {
    if self.file.is_empty() {
      return self.severity.label().to_string();
    }
    format!("{}:{}:{}", self.file, self.line, self.col)
  }
}

/// What one line of cargo's output turned out to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
  Diagnostic(Diagnostic),
  /// A binary cargo just linked. The last one wins — a workspace can build
  /// several, and the one belonging to the package being run comes last.
  Executable(PathBuf),
  Finished {
    success: bool,
  },
  /// Not JSON, or JSON of a kind that says nothing. Shown in the console
  /// verbatim.
  Other(String),
}

/// Parse one line of `cargo --message-format=json`.
pub fn parse(line: &str) -> Message {
  let line = line.trim_end();
  let Ok(value) = serde_json::from_str::<Value>(line) else {
    return Message::Other(line.to_string());
  };
  match value.get("reason").and_then(Value::as_str) {
    Some("compiler-message") => match value.get("message").and_then(diagnostic) {
      Some(diagnostic) => Message::Diagnostic(diagnostic),
      // A message whose level is `help` attached to nothing, or a shape we
      // do not understand: keep the rendering rather than dropping it.
      None => Message::Other(
        value
          .get("message")
          .and_then(|m| m.get("rendered"))
          .and_then(Value::as_str)
          .unwrap_or(line)
          .trim_end()
          .to_string(),
      ),
    },
    Some("compiler-artifact") => match executable(&value) {
      Some(path) => Message::Executable(path),
      None => Message::Other(String::new()),
    },
    Some("build-finished") => Message::Finished {
      success: value
        .get("success")
        .and_then(Value::as_bool)
        .unwrap_or(false),
    },
    // `build-script-executed`, and whatever cargo adds next.
    _ => Message::Other(String::new()),
  }
}

fn diagnostic(message: &Value) -> Option<Diagnostic> {
  let severity = Severity::parse(message.get("level")?.as_str()?)?;
  let text = message
    .get("message")
    .and_then(Value::as_str)
    .unwrap_or_default()
    .to_string();
  let rendered = message
    .get("rendered")
    .and_then(Value::as_str)
    .unwrap_or(&text)
    .trim_end()
    .to_string();
  let code = message
    .get("code")
    .and_then(|c| c.get("code"))
    .and_then(Value::as_str)
    .map(str::to_string);

  // The primary span, or the first one. rustc marks exactly one primary per
  // message, but a macro expansion can leave none.
  let spans = message
    .get("spans")
    .and_then(Value::as_array)
    .cloned()
    .unwrap_or_default();
  let span = spans
    .iter()
    .find(|s| s.get("is_primary").and_then(Value::as_bool) == Some(true))
    .or_else(|| spans.first());

  let (file, line, col, col_end) = match span {
    Some(span) => (
      span
        .get("file_name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string(),
      span.get("line_start").and_then(Value::as_u64).unwrap_or(1) as usize,
      span
        .get("column_start")
        .and_then(Value::as_u64)
        .unwrap_or(1) as usize,
      span.get("column_end").and_then(Value::as_u64).unwrap_or(1) as usize,
    ),
    None => (String::new(), 1, 1, 1),
  };

  Some(Diagnostic {
    file,
    line,
    col,
    col_end,
    severity,
    message: text,
    rendered,
    code,
  })
}

/// The executable a `compiler-artifact` message linked, if it linked one.
///
/// Only for a `bin` target: a test binary and a build script are artifacts too,
/// and running either instead of the app would be a confusing way to fail.
fn executable(value: &Value) -> Option<PathBuf> {
  let kinds = value.get("target")?.get("kind")?.as_array()?;
  if !kinds.iter().any(|k| k.as_str() == Some("bin")) {
    return None;
  }
  Some(PathBuf::from(value.get("executable")?.as_str()?))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn an_error_carries_its_primary_span() {
    let line = r#"{"reason":"compiler-message","message":{
      "rendered":"error[E0425]: cannot find value `x`\n --> src/main.rs:3:13\n",
      "level":"error","message":"cannot find value `x` in this scope",
      "code":{"code":"E0425"},
      "spans":[
        {"file_name":"src/lib.rs","line_start":9,"column_start":1,"column_end":2,"is_primary":false},
        {"file_name":"src/main.rs","line_start":3,"column_start":13,"column_end":14,"is_primary":true}
      ]}}"#;
    let Message::Diagnostic(d) = parse(line) else {
      panic!("not a diagnostic");
    };
    // The primary span wins even when it is not the first.
    assert_eq!(d.file, "src/main.rs");
    assert_eq!((d.line, d.col, d.col_end), (3, 13, 14));
    assert_eq!(d.severity, Severity::Error);
    assert_eq!(d.code.as_deref(), Some("E0425"));
    assert_eq!(d.message, "cannot find value `x` in this scope");
    assert!(d.rendered.contains("--> src/main.rs:3:13"));
    assert_eq!(d.location(), "src/main.rs:3:13");
  }

  #[test]
  fn a_message_with_no_span_still_comes_through() {
    let line = r#"{"reason":"compiler-message","message":{
      "level":"error","message":"linking with `cc` failed","spans":[]}}"#;
    let Message::Diagnostic(d) = parse(line) else {
      panic!("not a diagnostic");
    };
    assert!(d.file.is_empty());
    assert_eq!(d.message, "linking with `cc` failed");
    // No file means no line number to show — say the severity instead.
    assert_eq!(d.location(), "error");
    // `rendered` falls back to the message rather than being empty.
    assert_eq!(d.rendered, "linking with `cc` failed");
  }

  #[test]
  fn only_a_bin_artifact_is_something_to_run() {
    let bin = r#"{"reason":"compiler-artifact","target":{"kind":["bin"],"name":"demo"},
      "executable":"/tmp/demo/target/debug/demo"}"#;
    assert_eq!(
      parse(bin),
      Message::Executable(PathBuf::from("/tmp/demo/target/debug/demo"))
    );

    // A test binary is an executable artifact too, and running it would be a
    // confusing way to fail.
    let test = r#"{"reason":"compiler-artifact","target":{"kind":["test"],"name":"it"},
      "executable":"/tmp/demo/target/debug/deps/it-abc"}"#;
    assert_eq!(parse(test), Message::Other(String::new()));

    // A library links nothing.
    let lib = r#"{"reason":"compiler-artifact","target":{"kind":["lib"],"name":"demo"},
      "executable":null}"#;
    assert_eq!(parse(lib), Message::Other(String::new()));
  }

  #[test]
  fn the_finish_line_says_whether_it_worked() {
    assert_eq!(
      parse(r#"{"reason":"build-finished","success":true}"#),
      Message::Finished { success: true }
    );
    assert_eq!(
      parse(r#"{"reason":"build-finished","success":false}"#),
      Message::Finished { success: false }
    );
  }

  #[test]
  fn anything_else_is_kept_rather_than_dropped() {
    // A build script printing to stdout, in the middle of the JSON stream.
    assert_eq!(
      parse("cargo:rerun-if-changed=build.rs"),
      Message::Other("cargo:rerun-if-changed=build.rs".into())
    );
    // A message kind added by a newer cargo says nothing and costs nothing.
    assert_eq!(
      parse(r#"{"reason":"something-new","x":1}"#),
      Message::Other(String::new())
    );
  }

  #[test]
  fn severity_orders_worst_last_so_max_picks_the_error() {
    let worst = [Severity::Note, Severity::Error, Severity::Warning]
      .into_iter()
      .max();
    assert_eq!(worst, Some(Severity::Error));
    assert!(Severity::Error > Severity::Warning);
    assert!(Severity::Warning > Severity::Help);
  }
}
