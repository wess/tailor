//! Run, Build and Stop — the loop that makes a design a running app.
//!
//! Pressing Run writes the project out as a crate, compiles it, and launches
//! it. Nothing about that needs configuring first: a project with no export
//! directory gets a managed one (see [`tailor_build::Workspace`]), the way
//! Xcode's DerivedData means Run works on a project you just created.
//!
//! The half worth the trouble is what comes *back*. `cargo build
//! --message-format=json` reports every diagnostic with a file and a line, and
//! codegen already tags each node's expression with the line it landed on — so
//! a type error in generated Rust resolves to the component that generated it,
//! and clicking it in Problems selects that component on the canvas. That is
//! the loop Interface Builder never closed.
//!
//! The session runs on its own threads and is drained on a timer rather than
//! pushed from a channel. A poll is what the workbench already does for the
//! file watcher, it needs no executor plumbing, and 25ms is well under a frame.

use std::sync::Arc;
use std::time::Duration;

use gpui::{Context, Task};
use tailor_build::session::{Event, Intent, Outcome, Phase, Profile, Session};
use tailor_build::{LineMap, Workspace};
use tailor_model::lint::{Problem, Severity as LintSeverity};

use super::Workbench;

/// How often the console picks up what the build has said. Comfortably inside
/// a frame, so output reads as live without spinning.
const POLL: Duration = Duration::from_millis(25);

/// A console that grows without bound is a memory leak with a scrollbar. Old
/// lines go first; the tail is the part anyone reads.
const MAX_LINES: usize = 5_000;

/// Which half of the bottom pane is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Bottom {
  #[default]
  Problems,
  Console,
}

/// Where a build has got to. What the toolbar button and its label read from.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Status {
  #[default]
  Idle,
  Building,
  /// Built, and the app is up.
  Running,
  /// Built, and that was all that was asked for.
  Built,
  Failed(String),
  Stopped,
}

impl Status {
  /// Whether something is in flight — the button says Stop, and a second Run
  /// would have to replace it.
  pub fn busy(&self) -> bool {
    matches!(self, Status::Building | Status::Running)
  }

  pub fn label(&self) -> String {
    match self {
      Status::Idle => "Ready".into(),
      Status::Building => "Building…".into(),
      Status::Running => "Running".into(),
      Status::Built => "Build succeeded".into(),
      Status::Failed(why) => format!("Build failed — {why}"),
      Status::Stopped => "Stopped".into(),
    }
  }

  /// The same, saying which build it was. A release build takes a minute
  /// where a debug one takes a second, and knowing which you asked for is the
  /// difference between waiting and wondering.
  pub fn labelled(&self, profile: tailor_build::Profile) -> String {
    match self {
      Status::Building if profile == tailor_build::Profile::Release => "Building release…".into(),
      other => other.label(),
    }
  }
}

/// One line in the console, tagged with what produced it.
pub struct ConsoleLine {
  pub phase: Phase,
  pub text: String,
}

/// A compiler complaint, as the Problems panel wants it.
///
/// The `Problem` is what gets rendered; the file and line beside it are what
/// makes the row *go* somewhere — the code pane scrolls to the line, and the
/// canvas selects whatever generated it.
pub struct BuildProblem {
  pub problem: Problem,
  pub file: String,
  pub line: usize,
}

/// Everything the workbench knows about building and running.
#[derive(Default)]
pub struct Build {
  pub session: Option<Session>,
  pub status: Status,
  pub console: Vec<ConsoleLine>,
  /// Compiler diagnostics as problems, resolved back to the nodes that
  /// generated them. Kept apart from the lint pass's list, which is recomputed
  /// on every edit and would otherwise wipe these on the next keystroke.
  pub problems: Vec<BuildProblem>,
  /// The same messages unflattened, for the editor's gutter — it wants
  /// columns, which a `Problem` does not carry.
  pub diagnostics: Vec<tailor_build::Diagnostic>,
  /// Where each node ended up, captured at the moment of the build. Read to
  /// map a diagnostic back to a node — and captured then, not now, because the
  /// design may have moved on while the compiler was working.
  pub lines: LineMap,
  pub workspace: Option<Workspace>,
  /// Debug or release. Remembered for the session rather than the project: it
  /// is a thing you are doing right now, not a property of the design.
  pub profile: Profile,
  /// The poll. Held so that dropping it stops the draining.
  poll: Option<Task<()>>,
}

impl Build {
  fn push(&mut self, phase: Phase, text: String) {
    self.console.push(ConsoleLine { phase, text });
    if self.console.len() > MAX_LINES {
      let excess = self.console.len() - MAX_LINES;
      self.console.drain(..excess);
    }
  }
}

impl Workbench {
  /// ⌘R — build the project and launch it.
  ///
  /// Pressing it while something is running restarts, the way Xcode's play
  /// button does. Stop is its own button beside it rather than a state this
  /// one flips into, because "run it again" is the commonest thing you want
  /// and it should not cost two clicks.
  pub fn run_project(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
    self.start_build(Intent::Run, cx);
  }

  /// Product → Debug / Release.
  pub fn use_debug(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
    self.set_profile(Profile::Debug, cx);
  }

  pub fn use_release(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
    self.set_profile(Profile::Release, cx);
  }

  pub fn set_profile(&mut self, profile: Profile, cx: &mut Context<Self>) {
    if self.build.profile == profile {
      return;
    }
    self.build.profile = profile;
    self.toasts.info(format!("{} builds", profile.label()), cx);
    cx.notify();
  }

  /// ⌘B — compile it and stop there.
  pub fn build_project(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
    self.start_build(Intent::Build, cx);
  }

  /// ⌘. — kill whatever is running.
  pub fn stop_project(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
    let Some(session) = self.build.session.as_ref() else {
      return;
    };
    session.stop();
    self.build.push(Phase::Build, "— stopped —".into());
    cx.notify();
  }

  /// Throw away the build directory. The escape hatch for the one failure a
  /// generated crate can have that is not in the design: a stale `target/`.
  pub fn clean_build(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
    if self.build.status.busy() {
      self.toasts.info("Stop the build first", cx);
      return;
    }
    let workspace = Workspace::resolve(self.path.as_deref(), &self.project);
    if workspace.clean() {
      self.toasts.done("Build directory cleaned", cx);
    } else {
      self.toasts.info("Nothing to clean", cx);
    }
  }

  /// Reveal the build directory in Finder — the "where did it go" answer for
  /// a managed workspace, which by design has a path nobody would guess.
  pub fn reveal_build(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
    let workspace = Workspace::resolve(self.path.as_deref(), &self.project);
    if !workspace.root.exists() {
      self.toasts.info("Nothing built yet", cx);
      return;
    }
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open")
      .arg(&workspace.root)
      .spawn();
    #[cfg(not(target_os = "macos"))]
    let _ = &workspace.root;
  }

  fn start_build(&mut self, intent: Intent, cx: &mut Context<Self>) {
    // A second Run replaces the first rather than racing it.
    if let Some(session) = self.build.session.take() {
      session.stop();
    }

    let workspace = Workspace::resolve(self.path.as_deref(), &self.project);
    self.build.console.clear();
    self.build.problems.clear();
    self.build.diagnostics.clear();
    self.build.status = Status::Building;
    self.build.workspace = Some(workspace.clone());
    // The bottom pane opens on the console: a build you cannot see is a
    // progress bar with no bar.
    self.bottom = Bottom::Console;
    if !self.settings.is_open(tailor_store::Panel::Problems) {
      self.settings.set_open(tailor_store::Panel::Problems, true);
    }

    let project = Arc::clone(&self.project);
    self.build.lines = tailor_build::line_map(&project);

    let report = workspace.sync(&project);
    if !report.ok() {
      let why = report
        .failed
        .first()
        .map(|(path, err)| format!("{}: {err}", path.display()))
        .unwrap_or_else(|| "could not write the project".into());
      self.build.push(Phase::Build, why.clone());
      self.build.status = Status::Failed(why);
      cx.notify();
      return;
    }
    // The reverse jump — `tailordev --reveal <file>:<line>` — resolves a file
    // through this index. A build writes the same tree an export does, so it
    // should leave the same trail: without this, --reveal only works on
    // projects you have explicitly exported.
    if let Some(path) = self.path.clone() {
      tailor_store::ExportIndex::record(&workspace.root, &path);
    }
    self.build.push(
      Phase::Build,
      format!(
        "{} · {} → {}",
        self.project.name,
        self.build.profile.label().to_lowercase(),
        workspace.root.display()
      ),
    );

    match Session::start(&workspace, intent, self.build.profile) {
      Ok(session) => {
        self.build.session = Some(session);
        self.drain_soon(cx);
      }
      Err(why) => {
        self.build.push(Phase::Build, why.clone());
        self.build.status = Status::Failed(why);
      }
    }
    cx.notify();
  }

  /// Keep draining the session until it has nothing left to say.
  fn drain_soon(&mut self, cx: &mut Context<Self>) {
    self.build.poll = Some(cx.spawn(async move |this, cx| loop {
      cx.background_executor().timer(POLL).await;
      let Ok(more) = this.update(cx, |this, cx| this.drain_build(cx)) else {
        // The workbench is gone, and so is the reason to poll.
        break;
      };
      if !more {
        break;
      }
    }));
  }

  /// Take everything the session has produced. Returns whether to keep going.
  fn drain_build(&mut self, cx: &mut Context<Self>) -> bool {
    let Some(session) = self.build.session.as_ref() else {
      return false;
    };
    // Read out before the loop borrows `self` mutably; it does not change
    // while a session is alive.
    let intent = session.intent;
    let events = session.poll();
    if events.is_empty() {
      return true;
    }

    let mut finished = false;
    for event in events {
      finished |= self.apply_build_event(intent, event);
    }
    if finished {
      self.build.session = None;
    }
    // The gutter is part of the file, so it is redrawn with it rather than
    // waiting for the next analysis.
    self.sync_code_view(cx);
    cx.notify();
    !finished
  }

  /// Fold one event into the console, the status and the problem list.
  /// Returns whether it ended the session.
  ///
  /// Separate from the draining so it can be tested without a compiler: what
  /// is interesting here is the state machine, and `tailor-build` already
  /// tests the half that talks to cargo.
  pub(crate) fn apply_build_event(&mut self, intent: Intent, event: Event) -> bool {
    match event {
      Event::Line(phase, text) => {
        if text.is_empty() {
          self.build.push(phase, String::new());
        } else {
          // A rendered diagnostic arrives as one event and several lines.
          for line in text.lines() {
            self.build.push(phase, line.to_string());
          }
        }
        false
      }
      Event::Diagnostic(diagnostic) => {
        self.record_diagnostic(diagnostic);
        false
      }
      Event::Finished(phase, outcome) => {
        let (status, finished) = match (phase, &outcome) {
          // A build that succeeded is only half of a Run.
          (Phase::Build, Outcome::Succeeded) if intent == Intent::Run => (Status::Running, false),
          (Phase::Build, Outcome::Succeeded) => (Status::Built, true),
          (Phase::Run, Outcome::Succeeded) => (Status::Built, true),
          (_, Outcome::Cancelled) => (Status::Stopped, true),
          (_, Outcome::Failed(why)) => (Status::Failed(why.clone()), true),
        };
        self.build.status = status;
        if finished {
          let ending = match &outcome {
            Outcome::Succeeded => "finished".to_string(),
            Outcome::Cancelled => "stopped".to_string(),
            Outcome::Failed(why) => why.clone(),
          };
          self.build.push(
            phase,
            format!("— {} {ending} —", phase.label().to_lowercase()),
          );
        }
        finished
      }
    }
  }

  /// Turn a compiler diagnostic into a problem pointing at a component.
  ///
  /// The line map was captured when the build started, so this resolves
  /// against the code that was actually compiled rather than whatever the
  /// design has become since.
  fn record_diagnostic(&mut self, diagnostic: tailor_build::Diagnostic) {
    let severity = match diagnostic.severity {
      tailor_build::Severity::Error => LintSeverity::Error,
      tailor_build::Severity::Warning => LintSeverity::Warning,
      _ => LintSeverity::Info,
    };
    // Notes and helps are already inside the rendered error above them.
    if severity == LintSeverity::Info {
      return;
    }

    self.build.diagnostics.push(diagnostic.clone());
    let node = tailor_build::node_at(&self.build.lines, &diagnostic.file, diagnostic.line);
    // Which document the file belongs to, so clicking the problem can open it.
    let doc_id = node
      .and_then(|node| {
        self
          .project
          .docs
          .iter()
          .find(|doc| doc.nodes.contains_key(&node))
          .map(|doc| doc.id.clone())
      })
      .unwrap_or_default();

    let code = diagnostic
      .code
      .as_deref()
      .map(|code| format!("[{code}] "))
      .unwrap_or_default();
    self.build.problems.push(BuildProblem {
      problem: Problem {
        severity,
        doc_id,
        node,
        message: format!("{code}{}", diagnostic.message),
        fix: diagnostic.location(),
      },
      file: diagnostic.file.clone(),
      line: diagnostic.line,
    });
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_busy_status_is_the_one_stop_applies_to() {
    assert!(Status::Building.busy());
    assert!(Status::Running.busy());
    assert!(!Status::Idle.busy());
    assert!(!Status::Built.busy());
    assert!(!Status::Failed("x".into()).busy());
    assert!(!Status::Stopped.busy());
  }

  #[test]
  fn the_console_keeps_the_tail_rather_than_growing_forever() {
    let mut build = Build::default();
    for n in 0..MAX_LINES + 10 {
      build.push(Phase::Build, n.to_string());
    }
    assert_eq!(build.console.len(), MAX_LINES);
    // The oldest went, and the newest is still there.
    assert_eq!(build.console[0].text, "10");
    assert_eq!(
      build.console.last().unwrap().text,
      (MAX_LINES + 9).to_string()
    );
  }

  #[test]
  fn a_failure_says_why_in_the_status_line() {
    assert_eq!(
      Status::Failed("2 errors".into()).label(),
      "Build failed — 2 errors"
    );
    assert_eq!(Status::Running.label(), "Running");
  }
}
