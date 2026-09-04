//! Tailor's Rust generator.
//!
//! A `.tailor` document in, a guise component out — and the output is the point.
//! It is not a runtime format the app has to keep loading: it is a file you can
//! read, paste into a crate, and then own, with no dependency on Tailor left in
//! it. That is the difference between a mockup tool and a builder.
//!
//! The generator is deliberately pure. Every decision it makes comes from the
//! catalog the target library publishes, so the same table drives the palette,
//! the canvas, and this — and a component cannot render one way and generate
//! another. Which library that is comes off the project;
//! [`Generator`] is the library's half of this crate, and lives in its
//! provider (`tailor-guise` for guise).

pub mod app;
pub mod expr;
pub mod file;
pub mod generator;
pub mod node;
pub mod rust;
pub mod style;

pub use file::{document, module, Generated};
pub use generator::{register, Generator};

use tailor_model::{Document, Project};

/// Every file an export writes, relative to the chosen directory.
pub fn project_files(project: &Project) -> Vec<Generated> {
  let module_dir = tailor_model::snake_case(&project.gen.module);
  let mut out = Vec::new();

  for doc in &project.docs {
    let mut file = file::document(project, doc);
    file.path = format!("src/{module_dir}/{}", file.path);
    out.push(file);
  }
  let mut module_file = file::module(project);
  module_file.path = format!("src/{module_dir}/mod.rs");
  out.push(module_file);

  if project.gen.emit_app {
    let mut main = app::main_rs(project);
    main.path = "src/main.rs".into();
    out.push(main);

    let mut theme = generator::for_project(project).theme_rs(project);
    theme.path = "src/theme.rs".into();
    out.push(theme);

    // Beside `theme.rs`, because that is where its `include_str!` looks.
    if let Some(mut json) = app::theme_json(project) {
      json.path = "src/theme.json".into();
      out.push(json);
    }

    out.push(app::cargo_toml(project));
  }
  out
}

/// The single file for one document — what the code panel shows.
pub fn preview(project: &Project, doc: &Document) -> Generated {
  file::document(project, doc)
}
