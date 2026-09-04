//! The build loop against a real cargo.
//!
//! Not a Tailor project — a two-line crate, so the test costs a second rather
//! than the minutes gpui takes to compile. What is being tested is the
//! plumbing: that cargo is found from wherever this is running, that a
//! compiler error comes back as a `Diagnostic` with a line on it, that a
//! successful build produces a binary and that the binary's own output arrives
//! tagged as `Run` rather than mixed in with the compiler's.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tailor_build::session::{Event, Intent, Outcome, Phase, Profile, Session};
use tailor_build::{Severity, Workspace};

/// A throwaway crate with `main.rs` as given.
fn crate_with(name: &str, main: &str) -> Workspace {
  let root = std::env::temp_dir().join(format!("tailor-build-it-{name}"));
  let _ = std::fs::remove_dir_all(&root);
  std::fs::create_dir_all(root.join("src")).unwrap();
  std::fs::write(
    root.join("Cargo.toml"),
    format!(
      "[package]\nname = \"{name}\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\
       [dependencies]\n\
       # Nothing. The point is to compile in a second, not to be realistic.\n"
    ),
  )
  .unwrap();
  std::fs::write(root.join("src/main.rs"), main).unwrap();
  // Share one target dir across the tests in this file so the second build
  // does not recompile the standard prelude glue from scratch.
  Workspace {
    root,
    managed: true,
  }
}

/// Drain a session until it has finished the phase we care about, or we give
/// up. Returns everything it said.
fn drain_until(session: &Session, last: Phase, limit: Duration) -> Vec<Event> {
  let start = Instant::now();
  let mut events = Vec::new();
  loop {
    events.extend(session.poll());
    let done = events
      .iter()
      .any(|event| matches!(event, Event::Finished(phase, _) if *phase == last));
    let failed_earlier = events.iter().any(|event| {
      matches!(
        event,
        Event::Finished(Phase::Build, Outcome::Failed(_) | Outcome::Cancelled)
      )
    });
    if done || failed_earlier || start.elapsed() > limit {
      // One last sweep: the finish event and the lines before it can land in
      // the same tick.
      std::thread::sleep(Duration::from_millis(50));
      events.extend(session.poll());
      return events;
    }
    std::thread::sleep(Duration::from_millis(20));
  }
}

fn finished(events: &[Event], phase: Phase) -> Option<&Outcome> {
  events.iter().find_map(|event| match event {
    Event::Finished(got, outcome) if *got == phase => Some(outcome),
    _ => None,
  })
}

fn lines(events: &[Event], phase: Phase) -> Vec<&str> {
  events
    .iter()
    .filter_map(|event| match event {
      Event::Line(got, line) if *got == phase => Some(line.as_str()),
      _ => None,
    })
    .collect()
}

#[test]
fn a_program_builds_runs_and_its_output_comes_back_tagged() {
  let workspace = crate_with(
    "tailorok",
    "fn main() { println!(\"hello from the design\"); eprintln!(\"and stderr\"); }",
  );
  let session =
    Session::start(&workspace, Intent::Run, Profile::Debug).expect("cargo is installed");
  let events = drain_until(&session, Phase::Run, Duration::from_secs(120));

  assert_eq!(
    finished(&events, Phase::Build),
    Some(&Outcome::Succeeded),
    "build did not succeed: {events:#?}"
  );
  assert_eq!(finished(&events, Phase::Run), Some(&Outcome::Succeeded));

  // The program's output is tagged Run, not Build — that separation is the
  // whole reason the session has two phases.
  let ran = lines(&events, Phase::Run);
  assert!(
    ran
      .iter()
      .any(|line| line.contains("hello from the design")),
    "{ran:#?}"
  );
  assert!(ran.iter().any(|line| line.contains("and stderr")));
  // And cargo's progress is tagged Build.
  let built = lines(&events, Phase::Build);
  assert!(
    built
      .iter()
      .any(|line| line.contains("Compiling") || line.contains("Finished")),
    "{built:#?}"
  );

  let _ = std::fs::remove_dir_all(&workspace.root);
}

#[test]
fn a_compiler_error_comes_back_with_a_file_and_a_line() {
  let workspace = crate_with("tailorbad", "fn main() {\n    let x: u32 = \"nope\";\n}\n");
  let session =
    Session::start(&workspace, Intent::Run, Profile::Debug).expect("cargo is installed");
  let events = drain_until(&session, Phase::Build, Duration::from_secs(120));

  let outcome = finished(&events, Phase::Build).expect("the build finished");
  assert!(
    matches!(outcome, Outcome::Failed(_)),
    "expected a failure, got {outcome:?}"
  );
  // The message says how many errors, because that is what the toolbar shows.
  if let Outcome::Failed(why) = outcome {
    assert!(why.contains("error"), "{why}");
  }

  let diagnostics: Vec<_> = events
    .iter()
    .filter_map(|event| match event {
      Event::Diagnostic(diagnostic) => Some(diagnostic),
      _ => None,
    })
    .collect();
  let mismatch = diagnostics
    .iter()
    .find(|d| d.severity == Severity::Error && d.file.ends_with("main.rs"))
    .unwrap_or_else(|| panic!("no error on main.rs in {diagnostics:#?}"));
  assert_eq!(mismatch.line, 2, "the error is on the second line");
  assert_eq!(mismatch.code.as_deref(), Some("E0308"));
  assert!(
    mismatch.rendered.contains("expected"),
    "{}",
    mismatch.rendered
  );

  // A failed build never launches anything.
  assert!(finished(&events, Phase::Run).is_none());

  let _ = std::fs::remove_dir_all(&workspace.root);
}

#[test]
fn stop_ends_a_running_program() {
  let workspace = crate_with(
    "tailorloop",
    "fn main() {\n    println!(\"up\");\n    loop { std::thread::sleep(std::time::Duration::from_millis(50)); }\n}\n",
  );
  let session =
    Session::start(&workspace, Intent::Run, Profile::Debug).expect("cargo is installed");

  // Wait for it to actually be running before stopping it — stopping a build
  // is a different path, and this test is about the program.
  let start = Instant::now();
  let mut events = Vec::new();
  while start.elapsed() < Duration::from_secs(120) {
    events.extend(session.poll());
    if lines(&events, Phase::Run).iter().any(|l| l.contains("up")) {
      break;
    }
    std::thread::sleep(Duration::from_millis(20));
  }
  assert!(
    lines(&events, Phase::Run).iter().any(|l| l.contains("up")),
    "the program never started: {events:#?}"
  );

  session.stop();
  let rest = drain_until(&session, Phase::Run, Duration::from_secs(20));
  events.extend(rest);
  assert_eq!(
    finished(&events, Phase::Run),
    Some(&Outcome::Cancelled),
    "stop should end the run as cancelled, not as a failure"
  );

  let _ = std::fs::remove_dir_all(&workspace.root);
}

#[test]
fn a_directory_with_no_manifest_fails_before_spawning_anything() {
  let root: PathBuf = std::env::temp_dir().join("tailor-build-it-empty");
  let _ = std::fs::remove_dir_all(&root);
  let workspace = Workspace {
    root,
    managed: true,
  };
  let Err(why) = Session::start(&workspace, Intent::Build, Profile::Debug) else {
    panic!("started a build with nothing to build");
  };
  assert!(why.contains("Cargo.toml"), "{why}");
}

#[test]
fn build_only_never_launches_the_binary() {
  let workspace = crate_with("tailorbuildonly", "fn main() { println!(\"ran\"); }");
  let session =
    Session::start(&workspace, Intent::Build, Profile::Debug).expect("cargo is installed");
  let events = drain_until(&session, Phase::Build, Duration::from_secs(120));

  assert_eq!(finished(&events, Phase::Build), Some(&Outcome::Succeeded));
  assert!(finished(&events, Phase::Run).is_none());
  assert!(
    !lines(&events, Phase::Run).iter().any(|l| l.contains("ran")),
    "Build ran the program"
  );

  let _ = std::fs::remove_dir_all(&workspace.root);
}

/// Belt and braces for the one thing that is different inside a `.app`: the
/// toolchain has to be findable without a shell's `PATH`.
#[test]
fn cargo_is_found_and_its_directory_leads_the_child_path() {
  let cargo = tailor_build::toolchain::cargo().expect("cargo runs these tests");
  assert!(cargo.is_file());
  let path = tailor_build::toolchain::path_with_toolchain(&cargo);
  let first: PathBuf = std::env::split_paths(&path).next().unwrap();
  assert_eq!(Some(first.as_path()), cargo.parent());
  assert!(!Path::new(&path).to_string_lossy().is_empty());
}
