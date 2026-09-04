//! Compiling a design, and running what comes out.
//!
//! Tailor could always *generate* Rust. This is the half that closes the loop:
//! press Run and the design becomes a crate on disk, cargo builds it, the
//! binary launches, and anything the compiler complains about comes back
//! pointing at the component that caused it.
//!
//! That last part is the reason this crate knows about the document model at
//! all. Codegen already tags every node's expression with the line it landed
//! on — that map is what *Open in Editor* reads — so a diagnostic at
//! `src/ui/main_screen.rs:41` resolves to a `NodeId`, and a compiler error can
//! select a component on the canvas. See [`workspace::node_at`].
//!
//! Three pieces, and no gpui in any of them:
//!
//! * [`workspace`] — where a project builds. An export directory if you named
//!   one, a managed directory under Tailor's config dir if you did not, so Run
//!   works on a project you have never exported.
//! * [`session`] — the build itself: `cargo build --message-format=json`, then
//!   the linked binary, on background threads, streaming into a channel the app
//!   drains on a timer.
//! * [`toolchain`] — finding cargo, which is not a given inside a bundled app.

pub mod message;
pub mod session;
pub mod toolchain;
pub mod workspace;

pub use message::{Diagnostic, Message, Severity};
pub use session::{Event, Intent, Outcome, Phase, Profile, Session};
pub use workspace::{line_map, node_at, LineMap, Workspace};
