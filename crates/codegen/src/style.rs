//! `StyleProps` as gpui `Styled` calls.
//!
//! Two flavours: plain chained calls, and a `style! { … }` block for the keys
//! the macro covers with the rest chained after it. Both produce the same
//! layout — the choice is which one reads more like the codebase you are
//! pasting into.

use tailor_model::style::{Dimension, Direction, Edges, LayoutMode, StyleProps};
use tailor_model::{AlignToken, Flavor, JustifyToken};

use crate::expr::{hsla, Hoist};
use crate::rust::{float, px};

/// What the node needs from its parent to place itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
  /// The parent lays out absolutely, so this node pins itself.
  pub absolute: bool,
}

/// The calls that make a `div` behave as this node's container: direction,
/// gap, alignment, and — in absolute mode — the `relative()` its children pin
/// against.
pub fn container_calls(style: &StyleProps) -> Vec<String> {
  let mut out = Vec::new();
  match style.layout {
    LayoutMode::Flow => {
      out.push(".flex()".into());
      out.push(
        match style.direction {
          Direction::Row => ".flex_row()",
          Direction::Column => ".flex_col()",
        }
        .into(),
      );
      if style.wrap {
        out.push(".flex_wrap()".into());
      }
      if let Some(gap) = style.gap.filter(|g| *g > 0.0) {
        out.push(format!(".gap({})", px(gap)));
      }
      if let Some(align) = style.align {
        out.push(
          match align {
            AlignToken::Start => ".items_start()",
            AlignToken::Center => ".items_center()",
            AlignToken::End => ".items_end()",
            // 0.2.2's `Styled` has no stretch; it is the flex
            // default, so leaving it off is the same layout.
            AlignToken::Stretch => "",
          }
          .into(),
        );
      }
      if let Some(justify) = style.justify {
        out.push(
          match justify {
            JustifyToken::Start => ".justify_start()",
            JustifyToken::Center => ".justify_center()",
            JustifyToken::End => ".justify_end()",
            JustifyToken::Between => ".justify_between()",
            JustifyToken::Around => ".justify_around()",
          }
          .into(),
        );
      }
    }
    // Absolute children pin against the nearest positioned ancestor.
    LayoutMode::Absolute => out.push(".relative()".into()),
  }
  out.retain(|call| !call.is_empty());
  out
}

/// The calls that draw and size the node's own box.
pub fn box_calls(style: &StyleProps, placement: Placement, hoist: &mut Hoist) -> Vec<String> {
  let mut out = Vec::new();

  if placement.absolute {
    out.push(".absolute()".into());
    out.push(format!(".left({})", px(style.x)));
    out.push(format!(".top({})", px(style.y)));
  }

  match style.width {
    Dimension::Auto => {}
    Dimension::Px(v) => out.push(format!(".w({})", px(v))),
    Dimension::Full => out.push(".w_full()".into()),
    Dimension::Grow(factor) => out.push(grow_call(factor)),
  }
  match style.height {
    Dimension::Auto => {}
    Dimension::Px(v) => out.push(format!(".h({})", px(v))),
    Dimension::Full => out.push(".h_full()".into()),
    Dimension::Grow(factor) => {
      // One `flex_grow` covers both axes; only add it once.
      let call = grow_call(factor);
      if !out.contains(&call) {
        out.push(call);
      }
    }
  }
  if let Some(v) = style.min_width {
    out.push(format!(".min_w({})", px(v)));
  }
  if let Some(v) = style.max_width {
    out.push(format!(".max_w({})", px(v)));
  }
  if let Some(v) = style.min_height {
    out.push(format!(".min_h({})", px(v)));
  }
  if let Some(v) = style.max_height {
    out.push(format!(".max_h({})", px(v)));
  }

  out.extend(edge_calls(&style.padding, "p"));
  out.extend(edge_calls(&style.margin, "m"));

  if let Some(color) = &style.background {
    let local = hsla(hoist, color);
    out.push(format!(".bg({local})"));
  }
  if style.border_width > 0.0 {
    out.push(format!(".border({})", px(style.border_width)));
    let color = style.border_color.clone().unwrap_or_default();
    let local = hsla(hoist, &color);
    out.push(format!(".border_color({local})"));
  }
  if style.radius > 0.0 {
    out.push(format!(".rounded({})", px(style.radius)));
  }
  if let Some(method) = style.shadow.method() {
    out.push(format!(".{method}()"));
  }
  if style.opacity < 1.0 {
    out.push(format!(".opacity({})", float(style.opacity)));
  }
  if let Some(color) = &style.text_color {
    let local = hsla(hoist, color);
    out.push(format!(".text_color({local})"));
  }
  if let Some(size) = style.font_size {
    out.push(format!(".text_size({})", px(size)));
  }
  if let Some(weight) = style.font_weight {
    out.push(format!(".font_weight(FontWeight({weight}))"));
  }
  if style.italic {
    out.push(".italic()".into());
  }
  if let Some(align) = style.text_align {
    out.push(format!(".{}()", align.method()));
  }
  if let Some(method) = style.overflow.method() {
    out.push(format!(".{method}()"));
  }
  out
}

/// `flex_grow` takes no factor in the 0.2.2 `Styled`, so anything other than 1
/// is the same call — the factor is a design-time nicety the canvas honours.
fn grow_call(_factor: f32) -> String {
  ".flex_grow()".into()
}

/// Collapse four sides into the fewest calls that say the same thing.
fn edge_calls(edges: &Edges, prefix: &str) -> Vec<String> {
  if edges.is_zero() {
    return Vec::new();
  }
  if let Some(all) = edges.uniform() {
    return vec![format!(".{prefix}({})", px(all))];
  }
  if let Some((horizontal, vertical)) = edges.axes() {
    let mut out = Vec::new();
    if horizontal != 0.0 {
      out.push(format!(".{prefix}x({})", px(horizontal)));
    }
    if vertical != 0.0 {
      out.push(format!(".{prefix}y({})", px(vertical)));
    }
    return out;
  }
  let mut out = Vec::new();
  for (value, suffix) in [
    (edges.top, "t"),
    (edges.right, "r"),
    (edges.bottom, "b"),
    (edges.left, "l"),
  ] {
    if value != 0.0 {
      out.push(format!(".{prefix}{suffix}({})", px(value)));
    }
  }
  out
}

/// The `style! { … }` flavour: one block for the keys the macro covers, then
/// the leftovers chained the plain way.
pub fn macro_calls(
  style: &StyleProps,
  placement: Placement,
  container: bool,
  hoist: &mut Hoist,
) -> Vec<String> {
  let mut decls: Vec<String> = Vec::new();
  let mut rest: Vec<String> = Vec::new();

  if container && style.layout == LayoutMode::Flow {
    decls.push("display: flex;".into());
    decls.push(
      match style.direction {
        Direction::Row => "direction: row;",
        Direction::Column => "direction: column;",
      }
      .into(),
    );
    if style.wrap {
      rest.push(".flex_wrap()".into());
    }
    if let Some(gap) = style.gap.filter(|g| *g > 0.0) {
      decls.push(format!("gap: {};", float(gap)));
    }
    if let Some(align) = style.align {
      decls.push(format!("align: {};", align.label()));
    }
    if let Some(justify) = style.justify {
      decls.push(format!("justify: {};", justify.label()));
    }
  } else if container {
    decls.push("position: relative;".into());
  }

  if placement.absolute {
    decls.push("position: absolute;".into());
    rest.push(format!(".left({})", px(style.x)));
    rest.push(format!(".top({})", px(style.y)));
  }

  match style.width {
    Dimension::Auto => {}
    Dimension::Px(v) => decls.push(format!("width: {};", float(v))),
    Dimension::Full => decls.push("width: full;".into()),
    Dimension::Grow(_) => rest.push(".flex_grow()".into()),
  }
  match style.height {
    Dimension::Auto => {}
    Dimension::Px(v) => decls.push(format!("height: {};", float(v))),
    Dimension::Full => decls.push("height: full;".into()),
    Dimension::Grow(_) => {
      if !rest.iter().any(|c| c == ".flex_grow()") {
        rest.push(".flex_grow()".into());
      }
    }
  }
  if let Some(all) = style.padding.uniform().filter(|v| *v != 0.0) {
    decls.push(format!("padding: {};", float(all)));
  } else {
    rest.extend(edge_calls(&style.padding, "p"));
  }
  if let Some(all) = style.margin.uniform().filter(|v| *v != 0.0) {
    decls.push(format!("margin: {};", float(all)));
  } else {
    rest.extend(edge_calls(&style.margin, "m"));
  }
  if let Some(color) = &style.background {
    let local = hsla(hoist, color);
    decls.push(format!("background: {local};"));
  }
  if style.border_width > 0.0 {
    let color = style.border_color.clone().unwrap_or_default();
    let local = hsla(hoist, &color);
    rest.push(format!(".border({})", px(style.border_width)));
    rest.push(format!(".border_color({local})"));
  }
  if style.radius > 0.0 {
    decls.push(format!("radius: {};", float(style.radius)));
  }
  if style.opacity < 1.0 {
    decls.push(format!("opacity: {};", float(style.opacity)));
  }
  if let Some(color) = &style.text_color {
    let local = hsla(hoist, color);
    decls.push(format!("color: {local};"));
  }
  if let Some(size) = style.font_size {
    decls.push(format!("size: {};", float(size)));
  }

  // Everything the macro has no key for.
  let leftovers = StyleProps {
    min_width: style.min_width,
    max_width: style.max_width,
    min_height: style.min_height,
    max_height: style.max_height,
    shadow: style.shadow,
    italic: style.italic,
    text_align: style.text_align,
    overflow: style.overflow,
    font_weight: style.font_weight,
    ..StyleProps::default()
  };
  rest.extend(box_calls(&leftovers, Placement { absolute: false }, hoist));

  let mut out = Vec::new();
  if !decls.is_empty() {
    out.push(".apply(style! {".into());
    for decl in decls {
      out.push(format!("    {decl}"));
    }
    out.push("})".into());
  }
  out.extend(rest);
  out
}

/// Pick the flavour and emit.
pub fn calls(
  style: &StyleProps,
  placement: Placement,
  container: bool,
  flavor: Flavor,
  hoist: &mut Hoist,
) -> Vec<String> {
  match flavor {
    Flavor::Plain => {
      let mut out = Vec::new();
      if container {
        out.extend(container_calls(style));
      }
      out.extend(box_calls(style, placement, hoist));
      out
    }
    Flavor::Macros => macro_calls(style, placement, container, hoist),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tailor_model::style::ShadowToken;
  use tailor_model::tokens::{ColorSpec, ColorToken};

  fn flow() -> StyleProps {
    StyleProps::default()
  }

  #[test]
  fn a_flow_container_emits_its_axis_and_gap() {
    let mut style = flow();
    style.direction = Direction::Row;
    style.gap = Some(12.0);
    style.align = Some(AlignToken::Center);
    assert_eq!(
      container_calls(&style),
      [".flex()", ".flex_row()", ".gap(px(12.))", ".items_center()"]
    );
  }

  #[test]
  fn stretch_is_the_flex_default_so_it_emits_nothing() {
    let mut style = flow();
    style.align = Some(AlignToken::Stretch);
    assert_eq!(container_calls(&style), [".flex()", ".flex_col()"]);
  }

  #[test]
  fn an_absolute_container_only_positions_itself() {
    let mut style = flow();
    style.layout = LayoutMode::Absolute;
    assert_eq!(container_calls(&style), [".relative()"]);
  }

  #[test]
  fn edges_collapse_to_the_fewest_calls() {
    let mut hoist = Hoist::default();
    let mut style = flow();
    style.padding = Edges::all(8.0);
    let calls = box_calls(&style, Placement { absolute: false }, &mut hoist);
    assert_eq!(calls, [".p(px(8.))"]);

    style.padding = Edges::symmetric(16.0, 8.0);
    let calls = box_calls(&style, Placement { absolute: false }, &mut hoist);
    assert_eq!(calls, [".px(px(16.))", ".py(px(8.))"]);

    style.padding = Edges {
      top: 1.0,
      right: 2.0,
      bottom: 3.0,
      left: 4.0,
    };
    let calls = box_calls(&style, Placement { absolute: false }, &mut hoist);
    assert_eq!(
      calls,
      [".pt(px(1.))", ".pr(px(2.))", ".pb(px(3.))", ".pl(px(4.))"]
    );
  }

  #[test]
  fn an_absolutely_placed_child_pins_itself() {
    let mut hoist = Hoist::default();
    let mut style = flow();
    style.x = 24.0;
    style.y = 40.0;
    let calls = box_calls(&style, Placement { absolute: true }, &mut hoist);
    assert_eq!(calls, [".absolute()", ".left(px(24.))", ".top(px(40.))"]);
  }

  #[test]
  fn paint_props_hoist_their_colours() {
    let mut hoist = Hoist::default();
    let mut style = flow();
    style.background = Some(ColorSpec::Named(ColorToken::Dark));
    style.border_width = 1.0;
    style.border_color = Some(ColorSpec::Custom("#222".into()));
    style.radius = 6.0;
    style.shadow = ShadowToken::Md;
    let calls = box_calls(&style, Placement { absolute: false }, &mut hoist);
    assert_eq!(
      calls,
      [
        ".bg(dark_6)",
        ".border(px(1.))",
        ".border_color(hex_222)",
        ".rounded(px(6.))",
        ".shadow_md()"
      ]
    );
    assert_eq!(hoist.lines().len(), 2);
  }

  #[test]
  fn the_macro_flavour_opens_a_style_block() {
    let mut hoist = Hoist::default();
    let mut style = flow();
    style.gap = Some(8.0);
    style.padding = Edges::all(16.0);
    style.shadow = ShadowToken::Sm;
    let calls = macro_calls(&style, Placement { absolute: false }, true, &mut hoist);
    assert_eq!(calls[0], ".apply(style! {");
    assert!(calls.iter().any(|c| c.trim() == "gap: 8.;"));
    assert!(calls.iter().any(|c| c.trim() == "padding: 16.;"));
    assert_eq!(calls.last().unwrap(), ".shadow_sm()");
  }
}
