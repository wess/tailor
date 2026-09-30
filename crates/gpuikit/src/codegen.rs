//! The shapes gpui-kit wants that a chained constructor cannot express.
//!
//! Most of a gpui-kit component is `Type::new(id)` and a setter per prop, which
//! the catalog already says. What is left is the boxes (gpui-kit lays out with
//! gpui's own `div()`), the few builders whose content is a nested builder
//! rather than a string (`Tag`, `Empty`), and the lists that become one child
//! per item (`Breadcrumb`, `TabBar`, `Stepper`, `RadioGroup`).
//!
//! The rest of what a host has to say is the window: gpui-kit splits the
//! platform out of gpui and wraps every window in its own root, so `main` is
//! told a different entry point than plain gpui's.

use std::collections::BTreeMap;

use tailor_codegen::file::Generated;
use tailor_codegen::node::Emitter;
use tailor_codegen::rust::{comment, string, Source};
use tailor_codegen::{Generator, OpenWindow};
use tailor_model::library::Library;
use tailor_model::{Node, Project, Scheme, SizeToken};

/// gpui-kit's half of the generator.
pub struct GpuikitGenerator;

impl Generator for GpuikitGenerator {
  fn library(&self) -> &'static dyn Library {
    crate::library()
  }

  fn special(&self, em: &mut Emitter, node: &Node) -> Option<Vec<String>> {
    let text = |em: &Emitter, key: &str| -> String {
      em.prop_value(node, key).as_str().unwrap_or("").to_string()
    };
    let lines = match node.kind.as_str() {
      "frame" => vec!["div()".into()],
      "hstack" => vec!["h_flex()".into()],
      "vstack" => vec!["v_flex()".into()],
      // `flex_1`, not `flex_grow`: gpui-kit's styling extension shadows gpui's
      // no-argument `flex_grow()` with one that wants a factor.
      "spacer" => vec!["div()".into(), "    .flex_1()".into()],
      // An empty icon prop would print the library-agnostic fallback name,
      // which gpui-kit's icon set does not have.
      "icon" => {
        let name = text(em, "icon");
        let name = if name.is_empty() { "info" } else { &name };
        vec![format!(
          "Icon::new({})",
          tailor_codegen::expr::icon_path(name)
        )]
      }
      "separator" => {
        let vertical = em.prop_value(node, "orientation").as_str() == Some("vertical");
        vec![if vertical {
          "Separator::vertical()".into()
        } else {
          "Separator::horizontal()".into()
        }]
      }
      "kbd" => {
        let keys = text(em, "keys");
        let keys = if keys.is_empty() {
          "cmd-k".into()
        } else {
          keys
        };
        vec![format!(
          "Kbd::new(gpui::Keystroke::parse({}).expect(\"a valid keystroke\"))",
          string(&keys)
        )]
      }
      "link" => vec![
        format!("Link::new({})", string(&node.id.element_id())),
        format!("    .child({})", string(&text(em, "label"))),
      ],
      "tag" => {
        let variant = text(em, "variant");
        let ctor = match variant.as_str() {
          "" | "default" => "Tag::new()".to_string(),
          other => format!("Tag::{other}()"),
        };
        vec![ctor, format!("    .child({})", string(&text(em, "label")))]
      }
      "empty" => {
        let (title, description) = (text(em, "title"), text(em, "description"));
        let mut header = Vec::new();
        if !title.is_empty() {
          header.push(format!(
            "        .title(EmptyTitle::new().child({}))",
            string(&title)
          ));
        }
        if !description.is_empty() {
          header.push(format!(
            "        .description(EmptyDescription::new().child({}))",
            string(&description)
          ));
        }
        let mut lines = vec!["Empty::new()".into()];
        if !header.is_empty() {
          lines.push("    .header(".into());
          lines.push("      EmptyHeader::new()".into());
          lines.extend(header);
          lines.push("    )".into());
        }
        lines
      }
      _ => return None,
    };
    Some(lines)
  }

  fn custom_props(&self, em: &mut Emitter, node: &Node) -> Vec<String> {
    let each = |em: &Emitter, key: &str| em.items(node, key).unwrap_or_default();
    match node.kind.as_str() {
      "breadcrumb" => each(em, "items")
        .iter()
        .map(|label| format!(".child(BreadcrumbItem::new({}))", string(label)))
        .collect(),
      "tabbar" => each(em, "tabs")
        .iter()
        .map(|label| format!(".child(Tab::new().label({}))", string(label)))
        .collect(),
      "stepper" => each(em, "steps")
        .iter()
        .map(|label| format!(".item(StepperItem::new().child({}))", string(label)))
        .collect(),
      "radiogroup" => {
        let id = node.id.element_id();
        let mut out: Vec<String> = each(em, "options")
          .iter()
          .enumerate()
          .map(|(index, label)| {
            format!(
              ".child(Radio::new({}).label({}))",
              string(&format!("{id}-{index}")),
              string(label)
            )
          })
          .collect();
        if let Some(selected) = em.prop_value(node, "selected").as_i64() {
          if selected >= 0 {
            out.push(format!(".selected_index(Some({selected}))"));
          }
        }
        out
      }
      _ => Vec::new(),
    }
  }

  fn application_import(&self) -> &'static str {
    // No `Application`: the snapshot gpui-kit ships keeps it behind a
    // platform crate, which the facade reaches as `gpui::application()`.
    "use gpui::{px, size, Bounds, TitlebarOptions, WindowBounds, WindowOptions};"
  }

  fn application(&self) -> &'static str {
    "gpui::application()"
  }

  fn open_window(&self) -> OpenWindow {
    // The facade wraps the view in a root, which is what dialogs, sheets and
    // notifications mount on. A bare `cx.open_window` would draw fine and then
    // fail the first time something tried to open one.
    OpenWindow::Facade
  }

  fn theme_init(&self) -> &'static str {
    "theme::init(cx);"
  }

  fn theme_rs(&self, project: &Project) -> Generated {
    theme_rs(project)
  }
}

/// `theme.rs` — initialises gpui-kit and applies the scheme the design was laid
/// out in. The gpui-kit theme file format is its own, so a theme file the
/// project carries is reported rather than half-applied.
fn theme_rs(project: &Project) -> Generated {
  let theme = &project.theme;
  let mut source = Source::new();
  source.block(comment(
    "//! ",
    "The theme this interface was designed against. gpui-kit must be initialised \
     before anything draws, so this is also where that happens.",
    76,
  ));
  source.line("");
  source.line("use gpui::component::{Theme, ThemeMode};");
  source.line("use gpui::{px, App};");
  source.line("");
  source.open("pub fn init(cx: &mut App) {");
  source.line("gpui::init(cx);");
  source.line(match theme.scheme {
    Scheme::Dark => "Theme::change(ThemeMode::Dark, None, cx);",
    Scheme::Light => "Theme::change(ThemeMode::Light, None, cx);",
  });
  source.line("let theme = Theme::global_mut(cx);");
  source.line(format!("theme.radius = px({});", radius(theme.radius)));
  source.line(format!("theme.font_family = {:?}.into();", theme.font));
  source.close("}");

  let mut notes = Vec::new();
  if !theme.json.trim().is_empty() {
    notes.push(
      "This project's theme file is guise's format, which gpui-kit does not read; \
       the scheme, radius and font were applied instead."
        .to_string(),
    );
  }
  Generated {
    path: "theme.rs".into(),
    source: source.finish(),
    notes,
    lines: BTreeMap::new(),
    scaffold: false,
  }
}

/// Tailor's `xs..xl` as gpui-kit's corner radius, in pixels.
fn radius(size: SizeToken) -> &'static str {
  match size {
    SizeToken::Xs => "2.",
    SizeToken::Sm => "4.",
    SizeToken::Md => "6.",
    SizeToken::Lg => "8.",
    SizeToken::Xl => "12.",
  }
}
