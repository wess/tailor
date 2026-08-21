//! Tailor's on-disk side: project files, the recents list, and the editor's
//! settings.
//!
//! Thin on purpose. The document model owns the format and the validation; this
//! crate only decides where the bytes go and how failures are reported, so the
//! app never touches `std::fs` directly.

pub mod export;
pub mod paths;
pub mod project;
pub mod recents;
pub mod settings;

pub use export::{export, ExportReport};
pub use paths::{config_dir, default_project_dir, with_extension, EXTENSION};
pub use project::{open, save, SaveError};
pub use recents::{Recent, Recents};
pub use settings::{CanvasMode, Panel, Settings};
