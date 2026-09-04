//! The shapes guise wants that a chained constructor cannot express.
//!
//! Everything ordinary about generating a guise component comes off the
//! catalog: `Button::new(label)`, then a `.size(..)` per prop the designer set.
//! What is left over is this file — about thirty components whose Rust is not
//! one call, plus the two vocabularies that are guise's rather than Tailor's:
//! how a theme is built, and how an animation is spelled.
//!
//! It is deliberately the *whole* of what a provider has to write beyond a
//! catalog. A library with no irregular components implements [`special`] as
//! `None` and [`theme_rs`], and stops.
//!
//! [`special`]: Generator::special
//! [`theme_rs`]: Generator::theme_rs

use std::collections::BTreeMap;

use tailor_codegen::file::Generated;
use tailor_codegen::node::{child_call, Emitter};
use tailor_codegen::rust::{comment, float, indent, string, Source};
use tailor_codegen::style::Placement;
use tailor_codegen::Generator;
use tailor_model::library::Library;
use tailor_model::motion::Resolved as ResolvedMotion;
use tailor_model::props::PropValue;
use tailor_model::tokens::{EaseToken, EnterToken, LoopToken};
use tailor_model::{Flavor, Node, Project, Scheme};

/// guise's half of the generator.
pub struct GuiseGenerator;

/// The entity components guise gives a two-way `X::bind(&entity, &signal, cx)`,
/// and the prop that binding drives. Binding any *other* prop on these is a
/// one-shot read of the signal at construction, which is what the fallback in
/// `prop_calls` emits.
///
/// The controlled builders (`Checkbox`, `Switch`, `Radio`, `Chip`, `Rating`)
/// take `.bind(signal.binding())` in the builder chain instead, and are handled
/// where their props are emitted — they have no `new` to bind after.
const ENTITY_BINDS: &[(&str, &str)] = &[
  ("textinput", "value"),
  ("textarea", "value"),
  ("passwordinput", "value"),
  ("numberinput", "value"),
  ("pininput", "value"),
  ("colorinput", "value"),
  ("tagsinput", "tags"),
  ("autocomplete", "value"),
  ("markdowneditor", "value"),
  ("select", "selected"),
  ("segmented", "selected"),
  ("slider", "value"),
  ("rangeslider", "value"),
];

/// The controlled builders, which take `.bind(signal.binding())` in the chain.
/// They hold no state of their own, so there is no entity to bind afterwards —
/// the binding is a setter like any other, and it is what makes them two-way.
/// Without it a bound checkbox reads its signal and never writes back, which
/// looks like a binding right up until you click it.
const CONTROLLED_BINDS: &[(&str, &str)] = &[
  ("checkbox", "checked"),
  ("switch", "checked"),
  ("chip", "checked"),
  ("rating", "value"),
];

impl Generator for GuiseGenerator {
  fn library(&self) -> &'static dyn Library {
    crate::library()
  }

  fn special(&self, em: &mut Emitter, node: &Node) -> Option<Vec<String>> {
    let lines = match node.kind.as_str() {
      "frame" | "canvas" => vec!["div()".into()],
      "surface" => {
        let mut lines = vec!["div()".into()];
        if let PropValue::Color(color) = em.prop_value(node, "fill") {
          let local = tailor_codegen::expr::hsla(em.hoist, &color);
          lines.push(format!("    .bg({local})"));
        }
        lines
      }
      "spacer" => vec!["div()".into(), "    .flex_grow()".into()],
      "space" => {
        let axis = em.prop_value(node, "axis");
        let size = em
          .prop_value(node, "size")
          .as_size()
          .unwrap_or(tailor_model::SizeToken::Md);
        let method = if axis.as_str() == Some("x") { "x" } else { "y" };
        vec![format!("Space::{method}({})", size_path(size))]
      }
      "divider" => {
        let vertical = em.prop_value(node, "orientation").as_str() == Some("vertical");
        if vertical {
          vec!["Divider::vertical()".into()]
        } else {
          vec!["Divider::new()".into()]
        }
      }
      "indicator" => {
        let child = node.slot("child").first().copied();
        let inner = match child {
          Some(id) => em.emit(id, Placement { absolute: false }),
          None => vec!["div()".into()],
        };
        let mut lines = vec!["Indicator::new(".into()];
        lines.extend(indent(&inner));
        lines.push(")".into());
        lines
      }
      "expanded" => {
        let child = node.children().first().copied();
        let inner = match child {
          Some(id) => em.emit(id, Placement { absolute: false }),
          None => vec!["div()".into()],
        };
        let mut lines = vec!["Expanded::new(".into()];
        lines.extend(indent(&inner));
        lines.push(")".into());
        lines
      }
      "tooltip" => {
        let label = em.prop_value(node, "label");
        let label = label.as_str().unwrap_or("");
        let child = node.slot("child").first().copied();
        let inner = match child {
          Some(id) => em.emit(id, Placement { absolute: false }),
          None => vec!["div()".into()],
        };
        let mut lines = vec!["div()".into()];
        lines.extend(indent(&child_call(inner)));
        lines.push(format!("    .tooltip(tooltip({}))", string(label)));
        lines
      }
      "appshell" => vec!["AppShell::new()".into()],
      "virtuallist" => {
        // The rows a designer typed *are* the row builder: the real component
        // takes a closure over an index, so the list becomes a const it reads.
        let rows = em.items(node, "rows").unwrap_or_default();
        let literals: Vec<String> = rows.iter().map(|row| string(row)).collect();
        vec![
          format!(
            "VirtualList::new({}, {}, |index, _window, _cx| {{",
            string(&node.id.element_id()),
            rows.len()
          ),
          format!(
            "    const ROWS: [&str; {}] = [{}];",
            rows.len(),
            literals.join(", ")
          ),
          "    div().px(px(10.)).py(px(6.)).child(ROWS[index])".into(),
          "})".into(),
        ]
      }
      "aicost" => {
        let usage = format!(
          "AIUsage::new({}, {})",
          em.prop_value(node, "input").as_i64().unwrap_or(0),
          em.prop_value(node, "output").as_i64().unwrap_or(0)
        );
        let price = |key: &str| -> String {
          let value = em.prop_value(node, key);
          let text = value.as_str().unwrap_or("").trim().trim_start_matches('$');
          format!("{:?}", text.parse::<f64>().unwrap_or(0.0))
        };
        let pricing = format!(
          "AIPricing::new({}, {})",
          price("input_price"),
          price("output_price")
        );
        vec![format!("AICost::new({usage}, {pricing})")]
      }
      "aisources" => {
        let sources: Vec<String> = em
          .items(node, "sources")
          .unwrap_or_default()
          .iter()
          .map(|line| {
            let (title, location) = split_source(line);
            format!("AISource::new({}, {})", string(title), string(location))
          })
          .collect();
        vec![format!("AISources::new([{}])", sources.join(", "))]
      }
      "kbdgroup" => {
        let keys = em.items(node, "keys").unwrap_or_default();
        let mut lines = vec![
          "div()".into(),
          "    .flex()".into(),
          "    .gap(px(4.))".into(),
        ];
        for key in keys {
          lines.push(format!("    .child(Kbd::new({}))", string(&key)));
        }
        lines
      }
      "barchart" => {
        // `entries` is a second constructor, not a builder call: it
        // takes the labels and the values together.
        let values = em.numbers(node, "values");
        let labels = em.items(node, "labels").unwrap_or_default();
        if !labels.is_empty() && labels.len() == values.len() {
          let pairs: Vec<String> = labels
            .iter()
            .zip(values.iter())
            .map(|(label, value)| format!("({}, {})", string(label), float(*value as f32)))
            .collect();
          vec![format!("BarChart::entries([{}])", pairs.join(", "))]
        } else {
          vec![format!(
            "BarChart::new({})",
            tailor_codegen::expr::numbers(&values)
          )]
        }
      }
      "scatterchart" => {
        let values = em.numbers(node, "values");
        let pairs: Vec<String> = values
          .chunks(2)
          .filter(|pair| pair.len() == 2)
          .map(|pair| format!("({}, {})", float(pair[0] as f32), float(pair[1] as f32)))
          .collect();
        vec![format!("ScatterChart::new([{}])", pairs.join(", "))]
      }
      _ => return None,
    };
    Some(lines)
  }
  fn custom_props(&self, em: &mut Emitter, node: &Node) -> Vec<String> {
    let mut out = Vec::new();
    match node.kind.as_str() {
      "table" => {
        if let Some(head) = em.items(node, "head") {
          if !head.is_empty() {
            out.push(format!(".head({})", tailor_codegen::expr::items(&head)));
          }
        }
        for row in em.items(node, "rows").unwrap_or_default() {
          let cells: Vec<String> = row.split('|').map(|cell| cell.trim().to_string()).collect();
          out.push(format!(".row({})", tailor_codegen::expr::items(&cells)));
        }
      }
      "timeline" => {
        for item in em.items(node, "items").unwrap_or_default() {
          match item.split_once('|') {
            Some((title, description)) => out.push(format!(
              ".item_desc({}, {})",
              string(title.trim()),
              string(description.trim())
            )),
            None => out.push(format!(".item({})", string(item.trim()))),
          }
        }
      }
      "stepper" => {
        for item in em.items(node, "steps").unwrap_or_default() {
          match item.split_once('|') {
            Some((label, description)) => out.push(format!(
              ".step_desc({}, {})",
              string(label.trim()),
              string(description.trim())
            )),
            None => out.push(format!(".step({})", string(item.trim()))),
          }
        }
      }
      "navigationmenu" => {
        for item in em.items(node, "items").unwrap_or_default() {
          let (id, label) = item.split_once(':').unwrap_or((&item, &item));
          out.push(format!(
            ".item({}, {})",
            string(id.trim()),
            string(label.trim())
          ));
        }
      }
      "menu" | "contextmenu" => {
        for line in em.items(node, "items").unwrap_or_default() {
          let line = line.trim();
          if line == "-" {
            out.push(".divider()".into());
          } else if let Some(label) = line.strip_prefix('#') {
            out.push(format!(".section({})", string(label.trim())));
          } else {
            // The handler is the caller's to fill in; an empty one keeps the
            // export compiling and says plainly where the work goes.
            out.push(format!(".item({}, |_window, _cx| {{}})", string(line)));
          }
        }
      }
      "aichatview" => {
        // One turn per line, alternating the way a transcript reads.
        let turns: Vec<String> = em
          .items(node, "turns")
          .unwrap_or_default()
          .iter()
          .enumerate()
          .map(|(index, body)| {
            let role = if index % 2 == 0 { "User" } else { "Assistant" };
            format!("AITurn::new(AIRole::{role}, {})", string(body))
          })
          .collect();
        if !turns.is_empty() {
          out.push(format!(".turns([{}])", turns.join(", ")));
        }
      }
      "aimodelpicker" => {
        let models: Vec<String> = em
          .items(node, "models")
          .unwrap_or_default()
          .iter()
          .map(|label| {
            format!(
              "AIModel::new({}, {})",
              string(&tailor_model::snake_case(label)),
              string(label)
            )
          })
          .collect();
        if !models.is_empty() {
          out.push(format!(".models([{}])", models.join(", ")));
        }
      }
      "treeview" => {
        let nodes = tree_nodes(&em.items(node, "nodes").unwrap_or_default());
        if !nodes.is_empty() {
          out.push(format!(".nodes(vec![{}])", nodes.join(", ")));
        }
      }
      _ => {}
    }
    out
  }

  fn closure_slots(&self, kind: &str) -> &'static [&'static str] {
    match kind {
      "appshell" => &["header", "navbar", "aside", "footer"],
      "splitpanel" => &["first", "second"],
      _ => &[],
    }
  }

  /// AppShell's regions take their size before their content.
  fn slot_size_prop(&self, kind: &str, slot: &str) -> Option<&'static str> {
    if kind != "appshell" {
      return None;
    }
    Some(match slot {
      "header" => "header_height",
      "navbar" => "navbar_width",
      "aside" => "aside_width",
      "footer" => "footer_height",
      _ => return None,
    })
  }

  fn slots(&self, em: &mut Emitter, node: &Node, placement: Placement) -> Option<Vec<String>> {
    (node.kind == "settingsview").then(|| self.settings_pages(em, node, placement))
  }

  fn entity_bind(&self, kind: &str) -> Option<&'static str> {
    ENTITY_BINDS
      .iter()
      .find(|(k, _)| *k == kind)
      .map(|(_, prop)| *prop)
  }

  fn controlled_bind(&self, kind: &str) -> Option<&'static str> {
    CONTROLLED_BINDS
      .iter()
      .find(|(k, _)| *k == kind)
      .map(|(_, prop)| *prop)
  }

  fn motion(
    &self,
    element_id: &str,
    motion: ResolvedMotion,
    pinned: bool,
    flavor: Flavor,
  ) -> Vec<String> {
    animate_calls(element_id, motion, pinned, flavor)
  }

  fn theme_rs(&self, project: &Project) -> Generated {
    theme_rs(project)
  }
}

impl GuiseGenerator {
  /// `SettingsView` declares its pages and then fills them from one closure
  /// keyed by page id — not one closure per region like `Tabs`. So the pages
  /// are printed first and the slots become the arms of a `match`.
  fn settings_pages(&self, em: &mut Emitter, node: &Node, placement: Placement) -> Vec<String> {
    let labels = em.items(node, "pages").unwrap_or_default();
    if labels.is_empty() {
      return Vec::new();
    }
    let ids: Vec<String> = labels
      .iter()
      .map(|label| tailor_model::snake_case(label))
      .collect();

    let mut out = Vec::new();
    for (id, label) in ids.iter().zip(&labels) {
      out.push(format!(".page({}, {})", string(id), string(label)));
    }

    em.push_scope();
    let mut body = vec!["match page {".into()];
    for (index, id) in ids.iter().enumerate() {
      let children = node.slot(&format!("page:{index}")).to_vec();
      let mut region = vec!["div().flex().flex_col().gap(px(12.))".into()];
      for child in &children {
        let child_lines = em.emit(*child, placement);
        region.extend(indent(&child_call(child_lines)));
      }
      // The last arm is the catch-all: `match` on a `&str` needs one, and an
      // unreachable arm would be worse than reusing the final page.
      let head = if index + 1 == ids.len() {
        "    _ => ".to_string()
      } else {
        format!("    {} => ", string(id))
      };
      let mut arm = indent(&indent(&region));
      arm[0] = format!("{head}{}", arm[0].trim_start());
      let last = arm.len() - 1;
      arm[last] = format!("{}.into_any_element(),", arm[last]);
      body.extend(arm);
    }
    body.push("}".into());
    let captured = em.pop_scope();
    out.extend(em.wrap_closure_with(
      ".content(".into(),
      "|page, _query, _window, _cx|",
      captured,
      body,
    ));
    out
  }
}

/// The `.animate(..)` call that plays a node's entrance.
///
/// It lands on the node's own box — `Motioned::animate` puts the sampled
/// values straight onto the element's style, so the generated tree has
/// exactly the same shape whether a node animates or not.
fn animate_calls(
  element_id: &str,
  motion: ResolvedMotion,
  pinned: bool,
  flavor: Flavor,
) -> Vec<String> {
  let clip = match flavor {
    Flavor::Plain => motion_builder(motion, pinned),
    Flavor::Macros => motion_block(motion, pinned),
  };

  let mut out = vec![".animate(".to_string()];
  out.push(format!("    {},", string(element_id)));
  out.extend(indent(&clip).iter().enumerate().map(|(i, line)| {
    if i + 1 == clip.len() {
      format!("{line},")
    } else {
      line.clone()
    }
  }));
  out.push(")".into());
  out
}

/// The motion as a chained builder.
fn motion_builder(motion: ResolvedMotion, pinned: bool) -> Vec<String> {
  let mut clip = if motion.enter == EnterToken::Fade {
    // Distance means nothing to a fade, and printing it invites the
    // reader to wonder what it does.
    vec![format!("Motion::enter({})", enter_path(motion.enter))]
  } else {
    vec![format!(
      "Motion::enter_from({}, {})",
      enter_path(motion.enter),
      float(motion.distance)
    )]
  };
  clip.push(format!("    .duration({})", float(motion.duration)));
  if motion.delay > 0.0 {
    clip.push(format!("    .delay({})", float(motion.delay)));
  }
  clip.push(format!("    .ease({})", ease_path(motion.ease)));
  if motion.repeat == LoopToken::Forever {
    clip.push("    .repeat_forever()".into());
  }
  if motion.alternate {
    clip.push("    .alternate(true)".into());
  }
  if pinned {
    // A pinned node *is* its inset, so the offsets move to margins or the
    // animation would drag it off its pin.
    clip.push("    .as_margins()".into());
  }
  clip
}

/// The same motion as a `motion!` block — the macro flavour's half, matching
/// what `style!` does for the box.
fn motion_block(motion: ResolvedMotion, pinned: bool) -> Vec<String> {
  let mut clip = vec!["motion! {".to_string()];
  if motion.enter == EnterToken::Fade {
    clip.push(format!("    enter: {};", motion.enter.word()));
  } else {
    clip.push(format!(
      "    enter: {} {};",
      motion.enter.word(),
      float(motion.distance)
    ));
  }
  clip.push(format!("    duration: {};", float(motion.duration)));
  if motion.delay > 0.0 {
    clip.push(format!("    delay: {};", float(motion.delay)));
  }
  clip.push(format!("    ease: {};", motion.ease.words()));
  if motion.repeat == LoopToken::Forever {
    clip.push("    repeat: forever;".into());
  }
  if motion.alternate {
    clip.push("    alternate;".into());
  }
  if pinned {
    clip.push("    margins;".into());
  }
  clip.push("}".into());
  clip
}

/// Turn indented lines into nested `TreeNode` constructors.
fn tree_nodes(lines: &[String]) -> Vec<String> {
  fn depth(line: &str) -> usize {
    (line.len() - line.trim_start().len()) / 2
  }
  fn build(lines: &[String], index: &mut usize, level: usize) -> Vec<String> {
    let mut out = Vec::new();
    while *index < lines.len() {
      let line = &lines[*index];
      if line.trim().is_empty() {
        *index += 1;
        continue;
      }
      let this = depth(line);
      if this < level {
        break;
      }
      let label = line.trim().to_string();
      *index += 1;
      let children = build(lines, index, level + 1);
      let id = tailor_model::snake_case(&label);
      let mut node = format!("TreeNode::new({}, {})", string(&id), string(&label));
      if !children.is_empty() {
        node = format!("{node}.children([{}])", children.join(", "));
      }
      out.push(node);
    }
    out
  }
  let mut index = 0;
  build(lines, &mut index, 0)
}

/// `title — location`, the way the sources prop is typed. An em dash because
/// that is what reads on the canvas; a plain hyphen works too.
fn split_source(line: &str) -> (&str, &str) {
  match line
    .split_once('\u{2014}')
    .or_else(|| line.split_once(" - "))
  {
    Some((title, location)) => (title.trim(), location.trim()),
    None => (line.trim(), ""),
  }
}

/// `theme.rs` — the theme the design was laid out against.
fn theme_rs(project: &Project) -> Generated {
  let theme = &project.theme;
  let mut source = Source::new();
  source.block(comment(
    "//! ",
    "The theme this interface was designed against. Every component reads its \
         colours and sizes from here, so changing one value re-themes the whole app.",
    76,
  ));
  source.line("");
  source.line("use guise::prelude::*;");
  source.line("");
  source.open("pub fn build() -> Theme {");
  // The same precedence the canvas resolves: a theme file, then a preset, then
  // plain light/dark. A preset or a file owns the colours, so the primary token
  // is only printed when neither is doing the work.
  if !theme.json.is_empty() {
    source.line("let mut theme = Theme::from_json(include_str!(\"theme.json\"))");
    source.line("    .expect(\"theme.json ships beside this file and Tailor checked it\");");
  } else if let Some(preset) = theme.preset_entry(crate::library()) {
    source.line(format!("let mut theme = Theme::{}();", preset.rust));
  } else {
    source.line(match theme.scheme {
      Scheme::Dark => "let mut theme = Theme::dark();",
      Scheme::Light => "let mut theme = Theme::light();",
    });
  }
  if !theme.is_overridden(crate::library()) {
    source.line(format!(
      "theme.primary_color = {};",
      color_path(theme.primary)
    ));
  }
  source.line(format!(
    "theme.default_radius = {};",
    size_path(theme.radius)
  ));
  source.line(format!("theme.font_family = {:?}.into();", theme.font));
  source.line("theme");
  source.close("}");

  Generated {
    path: "theme.rs".into(),
    source: source.finish(),
    notes: Vec::new(),
    lines: BTreeMap::new(),
  }
}

/// A token as the Rust path guise spells it. The variant name is Tailor's; the
/// type it hangs off is the library's, and comes from
/// [`Library::token_paths`].
fn size_path(size: tailor_model::SizeToken) -> String {
  format!("{}::{}", crate::Guise.token_paths().size, size.variant())
}

fn color_path(color: tailor_model::ColorToken) -> String {
  format!("{}::{}", crate::Guise.token_paths().color, color.variant())
}

/// The guise curve an easing token names. `tailor-render` maps the same token
/// onto the live `Easing`, so the canvas and the export ease identically.
fn ease_path(ease: EaseToken) -> &'static str {
  match ease {
    EaseToken::Linear => "Easing::Linear",
    EaseToken::OutQuad => "Easing::Out(Curve::Quad)",
    EaseToken::OutCubic => "Easing::Out(Curve::Cubic)",
    EaseToken::OutQuint => "Easing::Out(Curve::Quint)",
    EaseToken::OutExpo => "Easing::Out(Curve::Expo)",
    EaseToken::OutCirc => "Easing::Out(Curve::Circ)",
    EaseToken::OutBack => "Easing::Out(Curve::Back)",
    EaseToken::OutElastic => "Easing::Out(Curve::Elastic)",
    EaseToken::OutBounce => "Easing::Out(Curve::Bounce)",
    EaseToken::InQuad => "Easing::In(Curve::Quad)",
    EaseToken::InCubic => "Easing::In(Curve::Cubic)",
    EaseToken::InExpo => "Easing::In(Curve::Expo)",
    EaseToken::InOutQuad => "Easing::InOut(Curve::Quad)",
    EaseToken::InOutCubic => "Easing::InOut(Curve::Cubic)",
    EaseToken::InOutSine => "Easing::InOut(Curve::Sine)",
    EaseToken::Spring => "Easing::Spring(Spring::default())",
  }
}

/// The transition kind an entrance names.
fn enter_path(enter: EnterToken) -> String {
  format!("TransitionKind::{}", enter.variant())
}
