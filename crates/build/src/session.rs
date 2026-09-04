//! Running a build, and then running what it built.
//!
//! Two phases, deliberately separate. `cargo build --message-format=json`
//! first, which gives structured diagnostics; then the linked binary directly,
//! which gives the app's own output. Doing it as one `cargo run
//! --message-format=json` would merge cargo's JSON and the app's stdout into
//! one stream, and there is no way to tell them apart afterwards — a program
//! that prints a line of JSON would become a compiler error.
//!
//! It also matches what the panel shows. A build has diagnostics and ends; an
//! app has output and keeps going until you stop it. They are different things
//! and the console labels them as such.
//!
//! ## Threads
//!
//! One thread drives the session, and two more read each child's pipes — a
//! reader has to block, and a blocking read on the executor gpui draws with is
//! a frozen window. Everything they produce goes down one channel, which the
//! app drains on a timer. The channel is unbounded on purpose: a build that
//! prints faster than the UI reads should not stall the compiler.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use crate::message::{self, Diagnostic, Message};
use crate::toolchain;
use crate::workspace::Workspace;

/// Which half of the session a line came from — the compiler, or the program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
  Build,
  Run,
}

impl Phase {
  pub fn label(self) -> &'static str {
    match self {
      Phase::Build => "Build",
      Phase::Run => "Run",
    }
  }
}

/// How a phase ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
  Succeeded,
  /// The compiler rejected it, or the program exited non-zero.
  Failed(String),
  /// Stop was pressed.
  Cancelled,
}

/// Everything the app hears from a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
  /// A line for the console, tagged with what produced it.
  Line(Phase, String),
  Diagnostic(Diagnostic),
  Finished(Phase, Outcome),
}

/// What a session was asked to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
  /// Compile only — the Build command, and what a save-triggered check runs.
  Build,
  /// Compile, then launch. The Run button.
  Run,
}

/// Which build cargo makes.
///
/// Debug is what Run uses: it compiles in a second where release takes a
/// minute, and a designer pressing Run wants to see the change, not the
/// fastest possible binary. Release is what you check before shipping — and on
/// a gpui app the difference is visible, because `lto = "fat"` and
/// `codegen-units = 1` are in the manifest Tailor generates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Profile {
  #[default]
  Debug,
  Release,
}

impl Profile {
  pub fn label(self) -> &'static str {
    match self {
      Profile::Debug => "Debug",
      Profile::Release => "Release",
    }
  }

  /// The flag, if it needs one. Debug is cargo's default and naming it would
  /// be noise in the console.
  fn flag(self) -> Option<&'static str> {
    match self {
      Profile::Debug => None,
      Profile::Release => Some("--release"),
    }
  }
}

/// A build in flight. Dropping it leaves the child running; call
/// [`Session::stop`] to end it.
pub struct Session {
  events: Receiver<Event>,
  /// The process currently running, so Stop has something to kill. Shared
  /// with the driver thread, which swaps it as the phases change.
  child: Arc<Mutex<Option<Child>>>,
  /// Set by Stop, read by the driver between phases: killing the compiler
  /// should not then launch the app it half-built.
  cancelled: Arc<Mutex<bool>>,
  pub intent: Intent,
}

impl Session {
  /// Start compiling `workspace`, and launch the result if asked to.
  ///
  /// Returns immediately; everything after this arrives through [`poll`].
  ///
  /// [`poll`]: Session::poll
  pub fn start(workspace: &Workspace, intent: Intent, profile: Profile) -> Result<Session, String> {
    let cargo = toolchain::cargo().ok_or_else(|| toolchain::MISSING.to_string())?;
    let manifest = workspace.manifest();
    if !manifest.exists() {
      return Err(format!(
        "{} has no Cargo.toml — export the project first",
        workspace.root.display()
      ));
    }

    let (tx, events) = mpsc::channel();
    let child: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(None));
    let cancelled = Arc::new(Mutex::new(false));

    let root = workspace.root.clone();
    let handle = Arc::clone(&child);
    let stopped = Arc::clone(&cancelled);
    std::thread::Builder::new()
      .name("tailor-build".into())
      .spawn(move || drive(&cargo, &root, intent, profile, tx, handle, stopped))
      .map_err(|err| format!("could not start the build: {err}"))?;

    Ok(Session {
      events,
      child,
      cancelled,
      intent,
    })
  }

  /// Everything that has happened since the last call. Never blocks — this is
  /// called from a frame.
  pub fn poll(&self) -> Vec<Event> {
    let mut out = Vec::new();
    // `try_recv` also stops on Disconnected, which is the driver thread
    // having finished — the events it already sent are still in the queue.
    while let Ok(event) = self.events.try_recv() {
      out.push(event);
    }
    out
  }

  /// Kill whatever is running. Safe to call when nothing is.
  pub fn stop(&self) {
    *self.cancelled.lock().unwrap() = true;
    if let Some(child) = self.child.lock().unwrap().as_mut() {
      let _ = child.kill();
    }
  }
}

/// The session thread: build, then maybe run.
fn drive(
  cargo: &Path,
  root: &Path,
  intent: Intent,
  profile: Profile,
  tx: Sender<Event>,
  child: Arc<Mutex<Option<Child>>>,
  cancelled: Arc<Mutex<bool>>,
) {
  let executable = match build(cargo, root, profile, &tx, &child, &cancelled) {
    Ok(path) => path,
    // `build` has already sent the Finished event that says why.
    Err(()) => return,
  };

  if intent == Intent::Build || *cancelled.lock().unwrap() {
    return;
  }

  let Some(executable) = executable else {
    let _ = tx.send(Event::Finished(
      Phase::Run,
      Outcome::Failed("the build produced no binary to run".into()),
    ));
    return;
  };
  run(&executable, root, &tx, &child, &cancelled);
}

/// `cargo build`, streaming diagnostics. `Ok(Some(path))` when it linked
/// something runnable.
fn build(
  cargo: &Path,
  root: &Path,
  profile: Profile,
  tx: &Sender<Event>,
  child: &Arc<Mutex<Option<Child>>>,
  cancelled: &Arc<Mutex<bool>>,
) -> Result<Option<PathBuf>, ()> {
  let mut command = Command::new(cargo);
  command
    .arg("build")
    .args(profile.flag())
    .arg("--message-format=json")
    // Colour would arrive as escape codes in `rendered`, and the console
    // prints text.
    .arg("--color=never")
    .current_dir(root)
    .env("PATH", toolchain::path_with_toolchain(cargo))
    // Cargo's own progress, which is the only thing worth showing while it
    // works, is on stderr and is a terminal animation unless told otherwise.
    .env("CARGO_TERM_PROGRESS_WHEN", "never")
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());

  let mut spawned = match command.spawn() {
    Ok(spawned) => spawned,
    Err(err) => {
      let _ = tx.send(Event::Finished(
        Phase::Build,
        Outcome::Failed(format!("could not run cargo: {err}")),
      ));
      return Err(());
    }
  };

  let stdout = spawned.stdout.take();
  let stderr = spawned.stderr.take();
  *child.lock().unwrap() = Some(spawned);

  // stderr is cargo's progress: pass it straight through.
  let progress = stderr.map(|stderr| {
    let tx = tx.clone();
    std::thread::spawn(move || {
      for line in BufReader::new(stderr).lines().map_while(Result::ok) {
        let _ = tx.send(Event::Line(Phase::Build, line));
      }
    })
  });

  // stdout is the JSON, read on this thread so the phase ends when it does.
  let mut executable = None;
  let mut errors = 0usize;
  if let Some(stdout) = stdout {
    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
      match message::parse(&line) {
        Message::Diagnostic(diagnostic) => {
          if diagnostic.severity == message::Severity::Error {
            errors += 1;
          }
          let _ = tx.send(Event::Line(Phase::Build, diagnostic.rendered.clone()));
          let _ = tx.send(Event::Diagnostic(diagnostic));
        }
        Message::Executable(path) => executable = Some(path),
        Message::Finished { .. } => {}
        Message::Other(text) if !text.is_empty() => {
          let _ = tx.send(Event::Line(Phase::Build, text));
        }
        Message::Other(_) => {}
      }
    }
  }
  if let Some(progress) = progress {
    let _ = progress.join();
  }

  let status = child
    .lock()
    .unwrap()
    .as_mut()
    .and_then(|child| child.wait().ok());
  *child.lock().unwrap() = None;

  if *cancelled.lock().unwrap() {
    let _ = tx.send(Event::Finished(Phase::Build, Outcome::Cancelled));
    return Err(());
  }

  match status {
    Some(status) if status.success() => {
      let _ = tx.send(Event::Finished(Phase::Build, Outcome::Succeeded));
      Ok(executable)
    }
    _ => {
      let _ = tx.send(Event::Finished(
        Phase::Build,
        Outcome::Failed(match errors {
          0 => "the build failed".into(),
          1 => "1 error".into(),
          n => format!("{n} errors"),
        }),
      ));
      Err(())
    }
  }
}

/// The built binary, with its output going where the build's did.
fn run(
  executable: &Path,
  root: &Path,
  tx: &Sender<Event>,
  child: &Arc<Mutex<Option<Child>>>,
  cancelled: &Arc<Mutex<bool>>,
) {
  let mut command = Command::new(executable);
  command
    .current_dir(root)
    // A panic that says "run with RUST_BACKTRACE=1" is a dead end when the
    // console is the only place you can see it. This is that place.
    .env("RUST_BACKTRACE", "1")
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());

  let mut spawned = match command.spawn() {
    Ok(spawned) => spawned,
    Err(err) => {
      let _ = tx.send(Event::Finished(
        Phase::Run,
        Outcome::Failed(format!("could not launch {}: {err}", executable.display())),
      ));
      return;
    }
  };
  let _ = tx.send(Event::Line(
    Phase::Run,
    format!("Running {}", executable.display()),
  ));

  let stdout = spawned.stdout.take();
  let stderr = spawned.stderr.take();
  *child.lock().unwrap() = Some(spawned);

  let mut readers = Vec::new();
  for pipe in [stdout.map(Pipe::Out), stderr.map(Pipe::Err)] {
    let Some(pipe) = pipe else { continue };
    let tx = tx.clone();
    readers.push(std::thread::spawn(move || match pipe {
      Pipe::Out(stream) => forward(stream, tx),
      Pipe::Err(stream) => forward(stream, tx),
    }));
  }
  for reader in readers {
    let _ = reader.join();
  }

  let status = child
    .lock()
    .unwrap()
    .as_mut()
    .and_then(|child| child.wait().ok());
  *child.lock().unwrap() = None;

  let outcome = if *cancelled.lock().unwrap() {
    Outcome::Cancelled
  } else {
    match status {
      Some(status) if status.success() => Outcome::Succeeded,
      Some(status) => Outcome::Failed(match status.code() {
        Some(code) => format!("exited with code {code}"),
        // A signal, which on a GUI app usually means it was killed.
        None => "stopped".into(),
      }),
      None => Outcome::Failed("the process went away".into()),
    }
  };
  let _ = tx.send(Event::Finished(Phase::Run, outcome));
}

enum Pipe {
  Out(std::process::ChildStdout),
  Err(std::process::ChildStderr),
}

fn forward(stream: impl std::io::Read, tx: Sender<Event>) {
  for line in BufReader::new(stream).lines().map_while(Result::ok) {
    if tx.send(Event::Line(Phase::Run, line)).is_err() {
      break;
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_workspace_with_no_manifest_says_so_rather_than_running_cargo() {
    let workspace = Workspace {
      root: std::env::temp_dir().join("tailor-build-nothing-here"),
      managed: true,
    };
    let Err(err) = Session::start(&workspace, Intent::Build, Profile::Debug) else {
      panic!("started a build with no manifest");
    };
    assert!(err.contains("no Cargo.toml"), "{err}");
  }

  #[test]
  fn a_phase_says_what_it_is() {
    assert_eq!(Phase::Build.label(), "Build");
    assert_eq!(Phase::Run.label(), "Run");
  }

  #[test]
  fn only_release_needs_a_flag() {
    assert_eq!(Profile::Debug.flag(), None, "debug is cargo's default");
    assert_eq!(Profile::Release.flag(), Some("--release"));
    assert_eq!(Profile::default(), Profile::Debug);
  }
}
