//! The catalog's own invariants: the things every provider's table has to
//! satisfy, and the few that are gpui-kit's.

use tailor_model::catalog::Ctor;
use tailor_model::library::Library;
use tailor_model::node::DEFAULT_SLOT;

fn library() -> &'static dyn Library {
  tailor_gpuikit::register();
  tailor_gpuikit::library()
}

#[test]
fn every_kind_is_unique() {
  let mut kinds: Vec<&str> = library().components().iter().map(|s| s.kind).collect();
  let count = kinds.len();
  kinds.sort_unstable();
  kinds.dedup();
  assert_eq!(kinds.len(), count, "two catalog entries share a kind");
}

#[test]
fn every_prop_key_is_unique_within_its_component() {
  for spec in library().components() {
    let mut keys: Vec<&str> = spec.props.iter().map(|p| p.key).collect();
    let count = keys.len();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), count, "{} repeats a prop key", spec.kind);
  }
}

#[test]
fn constructors_and_requirements_name_props_that_exist() {
  for spec in library().components() {
    let referenced: Vec<&str> = match spec.ctor {
      Ctor::IdAnd(key) | Ctor::Arg(key) | Ctor::EntityArg(key) | Ctor::EntityValue(key) => {
        vec![key]
      }
      Ctor::Args(keys) => keys.to_vec(),
      _ => vec![],
    };
    for key in referenced {
      assert!(
        spec.prop(key).is_some(),
        "{} constructs from unknown prop {key}",
        spec.kind
      );
    }
    for (key, _) in spec.required {
      assert!(
        spec.prop(key).is_some(),
        "{} requires unknown prop {key}",
        spec.kind
      );
    }
  }
}

#[test]
fn containers_declare_the_default_slot_first() {
  for spec in library().components() {
    if let Some(index) = spec.slots.iter().position(|s| s.key == DEFAULT_SLOT) {
      assert_eq!(
        index, 0,
        "{} lists its children slot out of order",
        spec.kind
      );
    }
  }
}

#[test]
fn every_choice_default_is_one_of_its_choices() {
  for spec in library().components() {
    for prop in spec.props.iter().filter(|p| !p.choices.is_empty()) {
      let default = prop.default_value();
      let value = default.as_str().unwrap_or("");
      assert!(
        prop.choices.contains(&value),
        "{}.{} defaults to {value:?}, which is not a choice",
        spec.kind,
        prop.key
      );
    }
  }
}

#[test]
fn an_alias_finds_its_component_and_is_not_anothers_name() {
  let titles: Vec<String> = library()
    .components()
    .iter()
    .map(|s| s.title.to_lowercase())
    .collect();
  for spec in library().components() {
    for alias in spec.aliases {
      let hits = library().search(alias);
      assert!(
        hits.iter().any(|hit| hit.kind == spec.kind),
        "{} lists the alias {alias:?}, which finds it nothing",
        spec.kind
      );
      assert!(
        !titles.contains(&alias.to_lowercase()) || spec.title.eq_ignore_ascii_case(alias),
        "{} claims the alias {alias:?}, which is another component's name",
        spec.kind
      );
    }
  }
}

#[test]
fn a_document_root_can_be_built() {
  // Every document's root is a `frame`, so a library without one cannot open
  // a project at all.
  let frame = library().get("frame").expect("the catalog defines frame");
  assert!(frame.takes_children());
  assert!(library().boxes().contains(&"frame"));
}

#[test]
fn every_box_is_a_component() {
  for kind in library().boxes() {
    assert!(
      library().get(kind).is_some(),
      "{kind} is listed as a box but is not in the catalog"
    );
  }
}

#[test]
fn the_catalog_has_not_shrunk() {
  // A floor, not a target: it only ever goes up.
  assert!(library().components().len() >= 30);
}

#[test]
fn registering_twice_keeps_one() {
  tailor_gpuikit::register();
  tailor_gpuikit::register();
  let ids: Vec<&str> = tailor_model::library::all()
    .iter()
    .map(|l| l.id())
    .collect();
  assert_eq!(ids.iter().filter(|id| **id == "gpuikit").count(), 1);
  assert_eq!(
    tailor_model::library::resolve("gpuikit").label(),
    "gpui-kit"
  );
}

#[test]
fn shadowing_covers_types_the_prelude_exports() {
  let library = library();
  assert!(library.shadows("Button"));
  assert!(library.shadows("Theme"));
  assert!(!library.shadows("Dashboard"));
  assert!(!library.shadows(""));
}
