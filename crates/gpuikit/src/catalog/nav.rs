//! Where you are, and how to get somewhere else.

use tailor_model::catalog::{ComponentSpec, Ctor};
use tailor_model::comp;
use tailor_model::props::{enums, int, items, Emit, PropValue};

use super::{CLICK_INDEX, SIZE};

pub static SPECS: &[ComponentSpec] = &[
  comp!(
      "breadcrumb", "Breadcrumb", "Breadcrumb", Navigation, "chevrons-right",
      "A trail of where you are.",
      Ctor::Unit,
      props: &[items("items", "Items", Emit::Custom, || {
        PropValue::Items(vec!["Home".into(), "Library".into(), "Data".into()])
      })],
      aliases: &["trail", "path"],
  ),
  comp!(
      "tabbar", "Tab bar", "TabBar", Navigation, "panel-top",
      "A bar of tabs. It draws the bar only; the panels are yours.",
      Ctor::Id,
      props: &[
          items("tabs", "Tabs", Emit::Custom, || {
            PropValue::Items(vec!["One".into(), "Two".into()])
          }),
          int("selected", "Selected", Emit::Method("selected_index"), || PropValue::Int(0)),
          enums("variant", "Variant", Emit::Method("with_variant"), "TabVariant",
            &["tab", "outline", "pill", "segmented", "underline"],
            || PropValue::Choice("tab".into())),
          SIZE,
      ],
      events: &[CLICK_INDEX],
      docs: "Unlike guise's Tabs, this has no panel regions: switch the content \
             yourself from the click handler.",
      aliases: &["tabs", "segmented control"],
  ),
  comp!(
      "stepper", "Stepper", "Stepper", Navigation, "list-ordered",
      "Numbered steps through a flow.",
      Ctor::Id,
      props: &[
          items("steps", "Steps", Emit::Custom, || {
            PropValue::Items(vec!["Account".into(), "Details".into(), "Done".into()])
          }),
          int("selected", "Selected", Emit::Method("selected_index"), || PropValue::Int(0)),
          tailor_model::props::boolean("vertical", "Vertical",
            Emit::Flag("vertical"), false),
          SIZE,
      ],
      events: &[CLICK_INDEX],
      aliases: &["wizard", "steps", "progress steps"],
  ),
];
