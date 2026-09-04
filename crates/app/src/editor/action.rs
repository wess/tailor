//! Writing the code behind an action.
//!
//! Interface Builder's whole trick is that a control is wired to a method and
//! you write the method. Tailor had the wiring — a button's `click` points at
//! an action, and the generated file calls it — and nowhere to write the body,
//! so every project came out a mockup with `// TODO` where the app should be.
//!
//! This is the other half. An action opens a real Rust buffer, and what you
//! type lands in the action's `body`, which lives in the `.tailor` document.
//! That last part is what makes it survive: the body is *design data*, so
//! regenerating rewrites the file around it rather than over it, and an export
//! you have edited by hand is never the thing being preserved.
//!
//! What is in scope is listed beside the editor, and offered as completions,
//! because the honest answer to "what can I write here" is a short list: the
//! document's state signals, its entity fields, and `cx`.

use gpui::prelude::*;
use gpui::{div, px, Context, Entity, SharedString, Window};
use guise::editor::{Editor, EditorEvent, Language};
use guise::prelude::*;
use tailor_model::NodeId;

use super::{icon, Workbench};
use crate::theme;

/// The action sheet, while it is open.
pub struct ActionEditor {
  /// Which action, by index into the document's list.
  pub index: usize,
  pub name: Entity<TextInput>,
  pub note: Entity<TextInput>,
  pub body: Entity<Editor>,
  /// Names the body can reach, with what each one is.
  pub scope: Vec<(String, String)>,
  /// The completion popup: what is on offer and which one is picked.
  pub suggestions: Vec<Completion>,
  pub picked: usize,
  /// The partial word the suggestions are for, so accepting one can replace
  /// exactly what was typed.
  pub prefix: String,
}

/// One thing the completer can offer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
  /// What gets inserted.
  pub text: String,
  /// What it is — "signal", "field", "keyword". Shown greyed beside it.
  pub kind: &'static str,
}

/// Rust's keywords and the handful of gpui/guise names that appear in almost
/// every handler. Not a language server — a short list of what is actually
/// reachable from inside one of these methods, which is most of the value at
/// none of the cost.
const COMMON: &[(&str, &str)] = &[
  ("let", "keyword"),
  ("if", "keyword"),
  ("else", "keyword"),
  ("match", "keyword"),
  ("for", "keyword"),
  ("while", "keyword"),
  ("return", "keyword"),
  ("true", "keyword"),
  ("false", "keyword"),
  ("Some", "keyword"),
  ("None", "keyword"),
  ("self", "keyword"),
  ("cx", "context"),
  ("cx.notify();", "context"),
  ("cx.emit(", "context"),
  ("cx.spawn(", "context"),
];

/// What `needle` could be completed to, best first.
///
/// Prefix matches beat contained ones, and what belongs to *this* document —
/// its signals and fields — beats the language's own words. Someone typing
/// `em` in a handler means `email`, not `emit`.
pub fn suggest(scope: &[(String, String)], needle: &str) -> Vec<Completion> {
  if needle.len() < 2 {
    return Vec::new();
  }
  let lower = needle.to_lowercase();
  let mut scored: Vec<(u8, Completion)> = Vec::new();

  for (name, kind) in scope {
    let rank = rank(&name.to_lowercase(), &lower, 0);
    if let Some(rank) = rank {
      scored.push((
        rank,
        Completion {
          text: name.clone(),
          kind: if kind == "signal" { "signal" } else { "field" },
        },
      ));
    }
  }
  for (name, kind) in COMMON {
    if let Some(rank) = rank(&name.to_lowercase(), &lower, 2) {
      scored.push((
        rank,
        Completion {
          text: (*name).to_string(),
          kind,
        },
      ));
    }
  }

  scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.text.cmp(&b.1.text)));
  scored.dedup_by(|a, b| a.1.text == b.1.text);
  scored.truncate(8);
  scored
    .into_iter()
    .map(|(_, completion)| completion)
    .collect()
}

/// Prefix beats contains; `bias` sinks a whole category below another.
fn rank(candidate: &str, needle: &str, bias: u8) -> Option<u8> {
  if candidate == needle {
    // An exact match is not a suggestion — there is nothing to complete.
    None
  } else if candidate.starts_with(needle) {
    Some(bias)
  } else if candidate.contains(needle) {
    Some(bias + 1)
  } else {
    None
  }
}

/// The word being typed, ending at `col` on `line`.
///
/// Rust identifiers, so letters, digits and underscores. Stops at a `.` too,
/// because after one you are completing a method and this completer does not
/// know types — offering `email` after `self.` would be wrong.
pub fn word_before(line: &str, col: usize) -> String {
  let chars: Vec<char> = line.chars().collect();
  let end = col.min(chars.len());
  let mut start = end;
  while start > 0 {
    let ch = chars[start - 1];
    if ch.is_alphanumeric() || ch == '_' {
      start -= 1;
    } else {
      break;
    }
  }
  if start > 0 && chars[start - 1] == '.' {
    return String::new();
  }
  chars[start..end].iter().collect()
}

impl Workbench {
  /// Open the editor for the action at `index`.
  pub fn open_action(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
    let Some(action) = self.doc().and_then(|doc| doc.actions.get(index)).cloned() else {
      return;
    };
    let scope = self.action_scope();

    let name = cx.new(|cx| {
      TextInput::new(cx)
        .value(&action.name)
        .label("Name")
        .size(Size::Sm)
    });
    let note = cx.new(|cx| {
      TextInput::new(cx)
        .value(&action.note)
        .label("What it does")
        .placeholder("Becomes the doc comment above the method")
        .size(Size::Sm)
    });
    let body = cx.new(|cx| {
      // The line tokenizer rather than tree-sitter, unlike the code pane: a
      // body is a *fragment*, and a grammar asked to parse statements as a
      // whole file spends most of its time in an error node.
      Editor::new(cx)
        .language(Language::Rust)
        .line_numbers(true)
        .font_size(12.0)
        .value(&action.body)
        .placeholder("// Runs when the event fires. `self` is the view; `cx` is its context.")
    });

    self.subs.push(cx.subscribe(
      &name,
      move |this: &mut Workbench, field, _: &TextInputEvent, cx| {
        let next = field.read(cx).text();
        this.rename_action(index, &next, cx);
      },
    ));
    self.subs.push(cx.subscribe(
      &note,
      move |this: &mut Workbench, field, _: &TextInputEvent, cx| {
        let next = field.read(cx).text();
        this.edit_doc("Action note", cx, move |doc| {
          if let Some(action) = doc.actions.get_mut(index) {
            action.note = next.clone();
          }
        });
      },
    ));
    self.subs.push(cx.subscribe_in(
      &body,
      window,
      move |this: &mut Workbench, editor, event: &EditorEvent, _window, cx| {
        let EditorEvent::Change(text) = event else {
          return;
        };
        let text = text.clone();
        this.edit_doc("Action body", cx, move |doc| {
          if let Some(action) = doc.actions.get_mut(index) {
            action.body = text.clone();
          }
        });
        this.refresh_suggestions(editor.clone(), cx);
      },
    ));

    body.read(cx).focus_handle().focus(window);
    self.action_editor = Some(ActionEditor {
      index,
      name,
      note,
      body,
      scope,
      suggestions: Vec::new(),
      picked: 0,
      prefix: String::new(),
    });
    cx.notify();
  }

  pub fn close_action(&mut self, cx: &mut Context<Self>) {
    if self.action_editor.take().is_some() {
      cx.notify();
    }
  }

  /// What a handler can reach: the document's signals, and the entity fields
  /// codegen will give it. Both are `self.`-prefixed in the generated method,
  /// which is why they are offered that way.
  fn action_scope(&self) -> Vec<(String, String)> {
    let Some(doc) = self.doc() else {
      return Vec::new();
    };
    let mut scope: Vec<(String, String)> = doc
      .state
      .iter()
      .map(|var| {
        (
          format!("self.{}", tailor_model::snake_case(&var.name)),
          "signal".to_string(),
        )
      })
      .collect();
    // The entity-backed nodes become struct fields, and they are the other
    // half of what a handler is for — reading a text field, opening a menu.
    for (_, field) in tailor_codegen::node::entity_fields(self.library(), doc) {
      scope.push((format!("self.{field}"), "field".to_string()));
    }
    scope
  }

  #[cfg(test)]
  pub(crate) fn rename_action_for_test(
    &mut self,
    index: usize,
    next: &str,
    cx: &mut Context<Self>,
  ) {
    self.rename_action(index, next, cx);
  }

  /// Recompute the completions against the buffer as it stands.
  #[cfg(test)]
  pub(crate) fn refresh_suggestions_for_test(&mut self, cx: &mut Context<Self>) {
    let Some(body) = self.action_editor.as_ref().map(|state| state.body.clone()) else {
      return;
    };
    self.refresh_suggestions(body, cx);
  }

  fn rename_action(&mut self, index: usize, next: &str, cx: &mut Context<Self>) {
    let next = next.trim().to_string();
    if next.is_empty() {
      return;
    }
    let previous = self
      .doc()
      .and_then(|doc| doc.actions.get(index))
      .map(|action| action.name.clone())
      .unwrap_or_default();
    if previous == next {
      return;
    }
    self.edit_doc("Rename action", cx, move |doc| {
      if let Some(action) = doc.actions.get_mut(index) {
        action.name = next.clone();
      }
      // Every event pointing at the old name follows it, or the rename would
      // silently unwire every control that used it.
      let ids: Vec<NodeId> = doc.nodes.keys().copied().collect();
      for id in ids {
        let Some(node) = doc.node_mut(id) else {
          continue;
        };
        for wired in node.events.values_mut() {
          if *wired == previous {
            *wired = next.clone();
          }
        }
      }
    });
  }

  /// Recompute the completions for wherever the caret is.
  fn refresh_suggestions(&mut self, editor: Entity<Editor>, cx: &mut Context<Self>) {
    let (line, col) = {
      let editor = editor.read(cx);
      let cursor = editor.model().cursor();
      let line = editor
        .model()
        .line(cursor.line)
        .map(str::to_string)
        .unwrap_or_default();
      (line, cursor.col)
    };
    let prefix = word_before(&line, col);
    let Some(state) = self.action_editor.as_mut() else {
      return;
    };
    state.suggestions = suggest(&state.scope, &prefix);
    state.prefix = prefix;
    state.picked = 0;
  }

  /// Put the picked completion in, replacing the partial word.
  pub fn accept_suggestion(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let Some(state) = self.action_editor.as_ref() else {
      return;
    };
    let Some(completion) = state.suggestions.get(state.picked).cloned() else {
      return;
    };
    let back = state.prefix.chars().count();
    let body = state.body.clone();

    body.update(cx, |editor, cx| {
      editor.edit(window, cx, |model| {
        for _ in 0..back {
          model.backspace();
        }
        model.insert(&completion.text);
      });
    });
    if let Some(state) = self.action_editor.as_mut() {
      state.suggestions.clear();
      state.prefix.clear();
    }
    cx.notify();
  }

  /// ↓ / ↑ inside the popup.
  pub fn step_suggestion(&mut self, down: bool, cx: &mut Context<Self>) {
    let Some(state) = self.action_editor.as_mut() else {
      return;
    };
    let count = state.suggestions.len();
    if count == 0 {
      return;
    }
    state.picked = if down {
      (state.picked + 1) % count
    } else {
      (state.picked + count - 1) % count
    };
    cx.notify();
  }

  pub(super) fn render_action_editor(
    &mut self,
    cx: &mut Context<Self>,
  ) -> Option<gpui::AnyElement> {
    let chrome = theme::colors(cx);
    let state = self.action_editor.as_ref()?;
    let action = self.doc()?.actions.get(state.index)?.clone();
    let signature = format!(
      "pub fn {}(&mut self, cx: &mut Context<Self>)",
      tailor_model::snake_case(&action.name)
    );
    let scope = state.scope.clone();
    let body = state.body.clone();
    let name = state.name.clone();
    let note = state.note.clone();
    let suggestions = state.suggestions.clone();
    let picked = state.picked;

    Some(
      div()
        .id("action-scrim")
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .bg(gpui::black().opacity(0.45))
        .occlude()
        .on_click(cx.listener(|this, _, _window, cx| this.close_action(cx)))
        .child(
          div()
            .id("action-sheet")
            .w(px(820.))
            .h(px(560.))
            .rounded(px(10.))
            .overflow_hidden()
            .border(px(1.))
            .border_color(chrome.border)
            .bg(chrome.body)
            .shadow_xl()
            .flex()
            .flex_col()
            .on_click(|_, _window, cx| cx.stop_propagation())
            // Header: what this is, and the signature it generates.
            .child(
              div()
                .flex()
                .items_center()
                .justify_between()
                .px(px(16.))
                .py(px(12.))
                .border_b(px(1.))
                .border_color(chrome.border)
                .child(
                  div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(div().text_color(chrome.accent).child(icon("zap")))
                    .child(div().text_size(px(13.)).child("Action")),
                )
                .child(
                  div()
                    .id("action-close")
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(24.))
                    .rounded(px(6.))
                    .text_color(chrome.dimmed)
                    .hover(move |style| style.bg(chrome.raised).text_color(chrome.text))
                    .child(icon("x"))
                    .on_click(cx.listener(|this, _, _window, cx| this.close_action(cx))),
                ),
            )
            .child(
              div()
                .flex()
                .gap(px(12.))
                .px(px(16.))
                .py(px(12.))
                .child(div().flex_grow().child(name))
                .child(div().flex_grow().child(note)),
            )
            .child(
              div()
                .px(px(16.))
                .pb(px(6.))
                .font_family("ui-monospace")
                .text_size(px(11.))
                .text_color(chrome.dimmed)
                .child(SharedString::from(signature)),
            )
            // The buffer, with the completion popup over it.
            .child(
              div()
                .relative()
                .flex_grow()
                .mx(px(16.))
                .mb(px(10.))
                .overflow_hidden()
                .rounded(px(8.))
                .border(px(1.))
                .border_color(chrome.border)
                .child(body)
                .children(popup(&suggestions, picked, chrome, cx)),
            )
            // What is reachable. Short enough to show rather than describe.
            .child(
              div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(6.))
                .px(px(16.))
                .pb(px(14.))
                .child(
                  div()
                    .text_size(px(10.))
                    .text_color(chrome.dimmed)
                    .child("In scope"),
                )
                .children(scope.into_iter().map(|(name, kind)| {
                  div()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .px(px(6.))
                    .py(px(2.))
                    .rounded(px(4.))
                    .bg(chrome.raised)
                    .font_family("ui-monospace")
                    .text_size(px(10.))
                    .child(SharedString::from(name))
                    .child(
                      div()
                        .text_color(chrome.dimmed)
                        .child(SharedString::from(kind)),
                    )
                }))
                .when(true, |d| {
                  d.child(
                    div()
                      .text_size(px(10.))
                      .text_color(chrome.dimmed)
                      .child("· ⌃Space accepts a completion, ⌃N / ⌃P walk them"),
                  )
                }),
            ),
        )
        .into_any_element(),
    )
  }
}

/// The completion list, anchored under the caret.
///
/// Positioned at the top of the buffer rather than at the caret: gpui hands an
/// element no bounds until it has painted, and a popup that jumps into place
/// one frame late is worse than one that sits still.
fn popup(
  suggestions: &[Completion],
  picked: usize,
  chrome: theme::Chrome,
  cx: &mut Context<Workbench>,
) -> Option<gpui::AnyElement> {
  if suggestions.is_empty() {
    return None;
  }
  Some(
    div()
      .absolute()
      .right(px(8.))
      .top(px(8.))
      .w(px(260.))
      .rounded(px(6.))
      .border(px(1.))
      .border_color(chrome.border)
      .bg(chrome.surface)
      .shadow_lg()
      .py(px(3.))
      .occlude()
      .children(suggestions.iter().enumerate().map(|(index, completion)| {
        let text = completion.text.clone();
        div()
          .id(SharedString::from(format!("suggest-{index}")))
          .flex()
          .items_center()
          .justify_between()
          .gap(px(8.))
          .px(px(9.))
          .py(px(3.))
          .when(index == picked, |d| d.bg(chrome.accent_soft))
          .child(
            div()
              .font_family("ui-monospace")
              .text_size(px(11.))
              .text_color(if index == picked {
                chrome.accent
              } else {
                chrome.text
              })
              .child(SharedString::from(text)),
          )
          .child(
            div()
              .text_size(px(9.))
              .text_color(chrome.dimmed)
              .child(completion.kind),
          )
          .on_click(cx.listener(move |this, _, window, cx| {
            if let Some(state) = this.action_editor.as_mut() {
              state.picked = index;
            }
            this.accept_suggestion(window, cx);
          }))
      }))
      .into_any_element(),
  )
}

impl Workbench {
  /// ⌃Space, and ⌃N / ⌃P beside it. Thin wrappers, because a `no_json` action
  /// carries nothing and the direction has to live in the name.
  pub fn accept_completion(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.accept_suggestion(window, cx);
  }

  pub fn next_completion(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
    self.step_suggestion(true, cx);
  }

  pub fn previous_completion(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
    self.step_suggestion(false, cx);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn scope() -> Vec<(String, String)> {
    vec![
      ("self.email".into(), "signal".into()),
      ("self.password".into(), "signal".into()),
      ("self.email_field".into(), "field".into()),
    ]
  }

  #[test]
  fn the_documents_own_names_rank_above_the_languages() {
    let hits = suggest(&scope(), "ema");
    assert_eq!(hits[0].text, "self.email");
    assert_eq!(hits[0].kind, "signal");
  }

  #[test]
  fn a_short_needle_suggests_nothing_rather_than_everything() {
    assert!(suggest(&scope(), "").is_empty());
    assert!(suggest(&scope(), "e").is_empty());
  }

  #[test]
  fn keywords_are_offered_but_only_after_what_is_in_scope() {
    let hits = suggest(&scope(), "ma");
    assert!(hits.iter().any(|c| c.text == "match"));

    // `cx.notify();` is offered whole — the thing you actually want to type.
    let hits = suggest(&scope(), "notif");
    assert_eq!(hits[0].text, "cx.notify();");
  }

  #[test]
  fn an_exact_match_is_not_a_suggestion() {
    // Nothing to complete once the word is complete.
    let hits = suggest(&scope(), "match");
    assert!(!hits.iter().any(|c| c.text == "match"), "{hits:?}");
  }

  #[test]
  fn the_word_before_the_caret_stops_at_the_boundaries() {
    assert_eq!(word_before("let x = ema", 11), "ema");
    // Column 5 is just past `x`; column 7 is past the `=` and its space, so
    // there is no word there to complete.
    assert_eq!(word_before("let x = ema", 5), "x");
    assert_eq!(word_before("let x = ema", 7), "");
    assert_eq!(word_before("", 0), "");
    assert_eq!(word_before("value_2", 7), "value_2");
    // A caret past the end of the line is not a panic.
    assert_eq!(word_before("ab", 99), "ab");
  }

  #[test]
  fn nothing_is_offered_after_a_dot() {
    // This completer does not know types, and guessing a method would be
    // worse than staying quiet.
    assert_eq!(word_before("self.ema", 8), "");
    assert!(suggest(&scope(), &word_before("self.ema", 8)).is_empty());
  }

  #[test]
  fn suggestions_are_capped_so_the_popup_stays_a_popup() {
    let wide: Vec<(String, String)> = (0..40)
      .map(|n| (format!("self.value_{n}"), "signal".to_string()))
      .collect();
    assert!(suggest(&wide, "val").len() <= 8);
  }
}
