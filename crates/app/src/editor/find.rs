//! Find in the code pane.
//!
//! A hundred lines of generated Rust is more than fits on a screen, and the
//! thing you are looking for is usually a name you already know — the prop you
//! just set, the component you just placed. So: ⌘F, type, and every hit is
//! highlighted with the caret on the first one; Enter walks them.
//!
//! Case-insensitive unless you type a capital, which is the convention every
//! editor settled on and nobody has to be told about. Match positions are
//! computed over `char` indices rather than bytes, because that is what the
//! editor's [`Pos`] counts in and a non-ASCII string literal in a design would
//! otherwise shift every highlight after it.

use gpui::prelude::*;
use gpui::{div, px, Context, Entity, SharedString, Window};
use guise::editor::Pos;
use guise::prelude::*;

use super::Workbench;
use crate::theme;

/// The find bar's state. `None` on [`CodePane`](super::code::CodePane) means
/// the bar is closed.
pub struct Find {
  pub field: Entity<TextInput>,
  /// Where every hit starts, in editor coordinates.
  pub hits: Vec<Pos>,
  /// Which hit the caret is on.
  pub at: usize,
  /// How long the needle was, so a highlight can span it.
  pub width: usize,
}

/// Every occurrence of `needle` in `text`, as `(line, column)` pairs counted in
/// characters.
///
/// Case-insensitive while the needle is all lowercase — "smart case", which is
/// what you want when the thing you are looking for is `flex_col` and what you
/// do not want when it is `Button`.
pub fn hits(text: &str, needle: &str) -> Vec<Pos> {
  if needle.is_empty() {
    return Vec::new();
  }
  let fold = needle.chars().all(|c| !c.is_uppercase());
  let needle: Vec<char> = if fold {
    needle.to_lowercase().chars().collect()
  } else {
    needle.chars().collect()
  };

  let mut out = Vec::new();
  for (line, source) in text.lines().enumerate() {
    let chars: Vec<char> = if fold {
      source.to_lowercase().chars().collect()
    } else {
      source.chars().collect()
    };
    if chars.len() < needle.len() {
      continue;
    }
    // Overlapping matches are not interesting — `aa` in `aaa` is two hits at
    // 0 and 1 to a regex engine and one-and-a-bit to a person reading.
    let mut col = 0;
    while col + needle.len() <= chars.len() {
      if chars[col..col + needle.len()] == needle[..] {
        out.push(Pos::new(line, col));
        col += needle.len();
      } else {
        col += 1;
      }
    }
  }
  out
}

impl Workbench {
  /// ⌘F — open the find bar, or refocus it when it is already up.
  pub fn find_in_code(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    // Finding in a pane you cannot see is not finding. The code pane is what
    // Split mode *is*, so that is the switch to throw.
    self.show_code(cx);
    if let Some(find) = self.code.find.as_ref() {
      find.field.read(cx).focus_handle().focus(window);
      return;
    }

    let field = cx.new(|cx| {
      TextInput::new(cx)
        .placeholder("Find in this file")
        .size(Size::Sm)
    });
    let sub = cx.subscribe_in(
      &field,
      window,
      |this: &mut Workbench, _, event: &TextInputEvent, window, cx| match event {
        // Enter walks the hits rather than submitting anything — there is
        // nothing to submit, and it is what every find bar does.
        TextInputEvent::Submit(_) => this.find_next(true, window, cx),
        TextInputEvent::Change(_) => this.refresh_find(cx),
      },
    );
    self.subs.push(sub);
    field.read(cx).focus_handle().focus(window);
    self.code.find = Some(Find {
      field,
      hits: Vec::new(),
      at: 0,
      width: 0,
    });
    cx.notify();
  }

  /// Escape — put the bar away and drop the highlights with it.
  pub fn close_find(&mut self, cx: &mut Context<Self>) {
    if self.code.find.take().is_none() {
      return;
    }
    self
      .code_view
      .update(cx, |editor, cx| editor.set_highlights(Vec::new(), cx));
    cx.notify();
  }

  /// Recompute the hits for whatever is in the field, and highlight them.
  pub(super) fn refresh_find(&mut self, cx: &mut Context<Self>) {
    let Some(find) = self.code.find.as_ref() else {
      return;
    };
    let needle = find.field.read(cx).text();
    let text = self.code_view.read(cx).text();
    let found = hits(&text, &needle);
    let width = needle.chars().count();

    let Some(find) = self.code.find.as_mut() else {
      return;
    };
    find.hits = found;
    find.width = width;
    find.at = 0;

    self.paint_find(cx);
    cx.notify();
  }

  /// Draw the current hit set. The active one is brighter, the way every
  /// editor distinguishes "a match" from "the match you are on".
  fn paint_find(&mut self, cx: &mut Context<Self>) {
    let chrome = theme::colors(cx);
    let Some(find) = self.code.find.as_ref() else {
      return;
    };
    let width = find.width;
    let at = find.at;
    let spans: Vec<(Pos, Pos, gpui::Hsla)> = find
      .hits
      .iter()
      .enumerate()
      .map(|(index, start)| {
        let end = Pos::new(start.line, start.col + width);
        let mut colour = chrome.accent_soft;
        if index == at {
          colour = chrome.accent;
          colour.a = 0.45;
        }
        (*start, end, colour)
      })
      .collect();
    self
      .code_view
      .update(cx, |editor, cx| editor.set_highlights(spans, cx));
  }

  /// Enter, or ⌘G — walk to the next hit, wrapping at the end.
  pub fn find_next(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
    let Some(find) = self.code.find.as_mut() else {
      return;
    };
    if find.hits.is_empty() {
      return;
    }
    let count = find.hits.len();
    find.at = if forward {
      (find.at + 1) % count
    } else {
      (find.at + count - 1) % count
    };
    let target = find.hits[find.at];
    self.code_view.update(cx, |editor, cx| {
      editor.edit(window, cx, |model| {
        model.move_to(target.line, target.col, false)
      });
    });
    self.paint_find(cx);
    cx.notify();
  }

  /// The bar itself: the field, the tally, and the two arrows.
  pub(super) fn render_find(&self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
    let chrome = theme::colors(cx);
    let find = self.code.find.as_ref()?;
    let tally = if find.hits.is_empty() {
      let empty = find.field.read(cx).text().is_empty();
      if empty {
        String::new()
      } else {
        "no matches".into()
      }
    } else {
      format!("{} of {}", find.at + 1, find.hits.len())
    };

    let step = |id: &'static str, glyph: &'static str, forward: bool, cx: &mut Context<Self>| {
      div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .size(px(20.))
        .rounded(px(4.))
        .text_color(chrome.dimmed)
        .hover(move |style| style.bg(chrome.raised).text_color(chrome.text))
        .child(super::icon(glyph))
        .on_click(cx.listener(move |this, _, window, cx| this.find_next(forward, window, cx)))
    };

    Some(
      div()
        .flex()
        .items_center()
        .gap(px(6.))
        .px(px(8.))
        .pb(px(4.))
        .child(div().flex_grow().child(find.field.clone()))
        .child(
          div()
            .flex_none()
            .text_size(px(10.))
            .text_color(chrome.dimmed)
            .child(SharedString::from(tally)),
        )
        .child(step("find-prev", "chevron-up", false, cx))
        .child(step("find-next", "chevron-down", true, cx))
        .child(
          div()
            .id("find-close")
            .flex()
            .items_center()
            .justify_center()
            .size(px(20.))
            .rounded(px(4.))
            .text_color(chrome.dimmed)
            .hover(move |style| style.bg(chrome.raised).text_color(chrome.text))
            .child(super::icon("x"))
            .on_click(cx.listener(|this, _, _window, cx| this.close_find(cx))),
        )
        .into_any_element(),
    )
  }
}

impl Workbench {
  /// ⌘G, and the menu item beside it. Two thin wrappers because a `no_json`
  /// action carries nothing, so the direction has to be in the name.
  pub fn find_forward(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.find_next(true, window, cx);
  }

  pub fn find_backward(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.find_next(false, window, cx);
  }
}

impl Workbench {
  /// Escape. Puts away whatever is on top: the action sheet if it is open,
  /// otherwise the find bar. One key, one meaning, and never both at once.
  pub fn dismiss(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
    // Topmost first: Open Quickly sits over the action sheet, which sits over
    // the find bar.
    if self.quickly.is_some() {
      self.close_quickly(cx);
      return;
    }
    if self.action_editor.is_some() {
      self.close_action(cx);
      return;
    }
    self.close_find(cx);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn every_occurrence_is_found_line_by_line() {
    let text = "let x = 1;\nlet y = x + x;\n";
    let found = hits(text, "x");
    assert_eq!(found, vec![Pos::new(0, 4), Pos::new(1, 8), Pos::new(1, 12)]);
  }

  #[test]
  fn a_lowercase_needle_ignores_case_and_a_capital_does_not() {
    let text = "Button::new(\"button\")";
    assert_eq!(hits(text, "button").len(), 2, "smart case folds");
    assert_eq!(hits(text, "Button"), vec![Pos::new(0, 0)]);
  }

  #[test]
  fn matches_do_not_overlap() {
    // Two `aa` in `aaaa`, not three.
    assert_eq!(hits("aaaa", "aa"), vec![Pos::new(0, 0), Pos::new(0, 2)]);
  }

  #[test]
  fn columns_are_characters_so_a_wide_glyph_does_not_shift_the_rest() {
    // Four bytes before `end`, but two characters.
    let found = hits("é→end", "end");
    assert_eq!(found, vec![Pos::new(0, 2)]);
  }

  #[test]
  fn an_empty_needle_finds_nothing_rather_than_everything() {
    assert!(hits("anything at all", "").is_empty());
  }

  #[test]
  fn a_needle_longer_than_the_line_is_not_a_panic() {
    assert!(hits("ab\n", "abcdef").is_empty());
  }
}
