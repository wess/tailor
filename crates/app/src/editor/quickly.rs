//! Open Quickly — ⌘⇧O.
//!
//! Xcode's answer to a project with more files than tabs. Tailor's version
//! reaches one thing Xcode's cannot: a *component on the canvas*. In a screen
//! of two hundred nodes the outline is a tree you scroll, and "the submit
//! button" is a name you already know.
//!
//! So it searches four things at once — documents, generated files, actions,
//! and the nodes in the open document — and picking one navigates rather than
//! filtering. There is no mode to choose first, because the whole point is not
//! having to say which kind of thing you are looking for.

use gpui::prelude::*;
use gpui::{div, px, Context, Entity, SharedString, Window};
use guise::prelude::*;
use tailor_model::NodeId;

use super::{icon, Workbench};
use crate::theme;

/// Where picking a result takes you.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Go {
  /// Open a document's tab.
  Document(String),
  /// Pin the code pane to a file.
  File(String),
  /// Open the action editor.
  Action(usize),
  /// Select a node on the canvas, opening its document first.
  Node(String, NodeId),
}

impl Go {
  fn icon(&self) -> &'static str {
    match self {
      Go::Document(_) => "file-text",
      Go::File(_) => "file-code-2",
      Go::Action(_) => "zap",
      Go::Node(..) => "box",
    }
  }

  fn kind(&self) -> &'static str {
    match self {
      Go::Document(_) => "document",
      Go::File(_) => "file",
      Go::Action(_) => "action",
      Go::Node(..) => "component",
    }
  }
}

/// One thing you can open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
  pub label: String,
  /// The greyed half — a path, a document name, a component's type.
  pub detail: String,
  pub go: Go,
}

/// The palette, while it is up.
pub struct Quickly {
  pub field: Entity<TextInput>,
  pub results: Vec<Target>,
  pub picked: usize,
}

/// How well `needle` matches `haystack`, lower being better, or `None` when it
/// does not match at all.
///
/// A subsequence match — typing `msc` finds `MainScreen` — scored so that the
/// obvious answer wins: a prefix beats a word-start run, which beats letters
/// scattered through the middle. Case-insensitive throughout; nobody holds
/// shift in a jump-to-file box.
pub fn score(haystack: &str, needle: &str) -> Option<u32> {
  if needle.is_empty() {
    return Some(1000);
  }
  let hay: Vec<char> = haystack.to_lowercase().chars().collect();
  let want: Vec<char> = needle.to_lowercase().chars().collect();

  let mut at = 0usize;
  let mut penalty = 0u32;
  let mut last: Option<usize> = None;
  for ch in &want {
    let found = hay[at..].iter().position(|c| c == ch)? + at;
    // A gap costs, and a gap that does not land on a word boundary costs
    // more — `msc` should prefer `MainScreen` to `dismiss_action_c`.
    if let Some(previous) = last {
      let gap = (found - previous - 1) as u32;
      let boundary =
        found > 0 && (hay[found - 1] == '_' || hay[found - 1] == '/' || hay[found - 1] == ' ');
      penalty += if boundary { gap.min(2) } else { gap * 2 };
    } else {
      // Where the match starts matters most: a hit at position 0 is what
      // someone typing three letters means.
      penalty += found as u32 * 3;
    }
    last = Some(found);
    at = found + 1;
  }
  // Shorter names win ties, so `submit` beats `submit_and_close`.
  Some(penalty + haystack.chars().count() as u32 / 4)
}

/// Rank `targets` against `needle`, best first.
pub fn rank(targets: Vec<Target>, needle: &str) -> Vec<Target> {
  let mut scored: Vec<(u32, Target)> = targets
    .into_iter()
    .filter_map(|target| {
      // The detail is searchable too — a path is how you find a file.
      let best = score(&target.label, needle)
        .into_iter()
        .chain(score(&target.detail, needle).map(|s| s + 40))
        .min()?;
      Some((best, target))
    })
    .collect();
  scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.label.cmp(&b.1.label)));
  scored.truncate(20);
  scored.into_iter().map(|(_, target)| target).collect()
}

impl Workbench {
  /// Everything the palette can reach, before filtering.
  fn targets(&self) -> Vec<Target> {
    let mut out = Vec::new();
    for doc in &self.project.docs {
      out.push(Target {
        label: doc.name.clone(),
        detail: doc.kind.label().to_string(),
        go: Go::Document(doc.id.clone()),
      });
    }
    for (path, _) in &self.code.files {
      let name = path.rsplit('/').next().unwrap_or(path).to_string();
      out.push(Target {
        label: name,
        detail: path.clone(),
        go: Go::File(path.clone()),
      });
    }
    if let Some(doc) = self.doc() {
      for (index, action) in doc.actions.iter().enumerate() {
        out.push(Target {
          label: action.name.clone(),
          detail: if action.note.is_empty() {
            format!("action in {}", doc.name)
          } else {
            action.note.clone()
          },
          go: Go::Action(index),
        });
      }
      // Nodes in the open document only. Every node in every document would
      // bury the four things you were actually looking for.
      let library = self.library();
      for id in std::iter::once(doc.root).chain(doc.descendants(doc.root)) {
        let Some(node) = doc.node(id) else { continue };
        // An unnamed node is findable by its component's name, which is what
        // the outline shows it as.
        let label = tailor_render::nodes::label_of(library, node);
        out.push(Target {
          label,
          detail: node.kind.clone(),
          go: Go::Node(doc.id.clone(), id),
        });
      }
    }
    out
  }

  /// ⌘⇧O.
  pub fn open_quickly(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if let Some(quickly) = self.quickly.as_ref() {
      quickly.field.read(cx).focus_handle().focus(window);
      return;
    }
    let field = cx.new(|cx| {
      TextInput::new(cx)
        .placeholder("Open a screen, a file, an action, a component…")
        .size(Size::Md)
    });
    let sub = cx.subscribe_in(
      &field,
      window,
      |this: &mut Workbench, _, event: &TextInputEvent, window, cx| match event {
        TextInputEvent::Change(_) => this.refresh_quickly(cx),
        TextInputEvent::Submit(_) => this.accept_quickly(window, cx),
      },
    );
    self.subs.push(sub);
    field.read(cx).focus_handle().focus(window);
    self.quickly = Some(Quickly {
      field,
      results: Vec::new(),
      picked: 0,
    });
    self.refresh_quickly(cx);
    cx.notify();
  }

  pub fn close_quickly(&mut self, cx: &mut Context<Self>) {
    if self.quickly.take().is_some() {
      cx.notify();
    }
  }

  pub fn refresh_quickly(&mut self, cx: &mut Context<Self>) {
    let Some(quickly) = self.quickly.as_ref() else {
      return;
    };
    let needle = quickly.field.read(cx).text();
    let results = rank(self.targets(), &needle);
    if let Some(quickly) = self.quickly.as_mut() {
      quickly.results = results;
      quickly.picked = 0;
    }
    cx.notify();
  }

  pub(super) fn step_quickly(&mut self, down: bool, cx: &mut Context<Self>) {
    let Some(quickly) = self.quickly.as_mut() else {
      return;
    };
    let count = quickly.results.len();
    if count == 0 {
      return;
    }
    quickly.picked = if down {
      (quickly.picked + 1) % count
    } else {
      (quickly.picked + count - 1) % count
    };
    cx.notify();
  }

  /// Go where the picked result points, and put the palette away.
  pub fn accept_quickly(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let Some(go) = self
      .quickly
      .as_ref()
      .and_then(|quickly| quickly.results.get(quickly.picked))
      .map(|target| target.go.clone())
    else {
      return;
    };
    self.close_quickly(cx);
    match go {
      Go::Document(id) => self.open_document(&id, cx),
      Go::File(path) => self.reveal_in_code(&path, None, window, cx),
      Go::Action(index) => self.open_action(index, window, cx),
      Go::Node(doc, id) => {
        self.open_document(&doc, cx);
        self.select_only(id, cx);
      }
    }
  }

  pub(super) fn render_quickly(&mut self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
    let chrome = theme::colors(cx);
    let quickly = self.quickly.as_ref()?;
    let field = quickly.field.clone();
    let picked = quickly.picked;
    let results = quickly.results.clone();

    Some(
      div()
        .id("quickly-scrim")
        .absolute()
        .inset_0()
        .flex()
        .flex_col()
        .items_center()
        .bg(gpui::black().opacity(0.35))
        .occlude()
        .on_click(cx.listener(|this, _, _window, cx| this.close_quickly(cx)))
        .child(
          div()
            .id("quickly")
            // Near the top, not centred: a jump-to box is a thing you type
            // into and dismiss, and the middle of the screen is where you
            // were looking.
            .mt(px(120.))
            .w(px(620.))
            .max_h(px(460.))
            .rounded(px(10.))
            .overflow_hidden()
            .border(px(1.))
            .border_color(chrome.border)
            .bg(chrome.body)
            .shadow_xl()
            .flex()
            .flex_col()
            .on_click(|_, _window, cx| cx.stop_propagation())
            .child(div().p(px(10.)).child(field))
            .child(
              div()
                .id("quickly-results")
                .flex()
                .flex_col()
                .overflow_y_scroll()
                .pb(px(6.))
                .children(results.into_iter().enumerate().map(|(index, target)| {
                  let go = target.go.clone();
                  div()
                    .id(SharedString::from(format!("quick-{index}")))
                    .flex()
                    .items_center()
                    .gap(px(9.))
                    .px(px(12.))
                    .py(px(6.))
                    .when(index == picked, |d| d.bg(chrome.accent_soft))
                    .child(
                      div()
                        .text_color(if index == picked {
                          chrome.accent
                        } else {
                          chrome.dimmed
                        })
                        .child(icon(target.go.icon())),
                    )
                    .child(
                      div()
                        .flex_grow()
                        .text_size(px(12.))
                        .child(SharedString::from(target.label)),
                    )
                    .child(
                      div()
                        .text_size(px(10.))
                        .text_color(chrome.dimmed)
                        .child(SharedString::from(target.detail)),
                    )
                    .child(
                      div()
                        .text_size(px(9.))
                        .text_color(chrome.dimmed)
                        .child(target.go.kind()),
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                      if let Some(quickly) = this.quickly.as_mut() {
                        quickly.picked = index;
                      }
                      let _ = &go;
                      this.accept_quickly(window, cx);
                    }))
                })),
            ),
        )
        .into_any_element(),
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn target(label: &str, detail: &str) -> Target {
    Target {
      label: label.into(),
      detail: detail.into(),
      go: Go::File(detail.into()),
    }
  }

  #[test]
  fn a_subsequence_matches_and_a_missing_letter_does_not() {
    assert!(score("MainScreen", "msc").is_some());
    assert!(score("MainScreen", "main").is_some());
    assert!(score("MainScreen", "xyz").is_none());
    // Order matters — it is a subsequence, not a bag of letters.
    assert!(score("MainScreen", "nm").is_none());
  }

  #[test]
  fn the_obvious_answer_wins() {
    let hits = rank(
      vec![
        target("dismiss_action_c", "src/a.rs"),
        target("MainScreen", "src/main_screen.rs"),
      ],
      "msc",
    );
    assert_eq!(hits[0].label, "MainScreen");
  }

  #[test]
  fn a_prefix_beats_a_match_further_in() {
    let hits = rank(
      vec![target("stat_card.rs", "x"), target("card.rs", "y")],
      "card",
    );
    assert_eq!(hits[0].label, "card.rs");
  }

  #[test]
  fn a_shorter_name_wins_a_tie() {
    let hits = rank(
      vec![target("submit_and_close", "x"), target("submit", "y")],
      "submit",
    );
    assert_eq!(hits[0].label, "submit");
  }

  #[test]
  fn the_detail_is_searchable_but_ranks_below_the_name() {
    let hits = rank(
      vec![
        target("mod.rs", "src/ui/mod.rs"),
        target("theme.rs", "src/theme.rs"),
      ],
      "theme",
    );
    assert_eq!(hits[0].label, "theme.rs");

    // A path-only match still comes back.
    let hits = rank(vec![target("mod.rs", "src/ui/mod.rs")], "ui");
    assert_eq!(hits.len(), 1);
  }

  #[test]
  fn an_empty_needle_lists_everything_it_can_show() {
    let all: Vec<Target> = (0..40).map(|n| target(&format!("f{n}"), "x")).collect();
    let hits = rank(all, "");
    assert_eq!(hits.len(), 20, "the list is capped so it stays a list");
  }
}
