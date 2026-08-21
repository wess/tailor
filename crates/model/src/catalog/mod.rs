//! The component catalog — every guise component Tailor can place.
//!
//! One table, read by four consumers: the palette lists it, the inspector
//! builds a control per prop, the renderer builds the real component, and the
//! generator prints it. Adding a component is an entry here plus an arm in
//! `tailor-render`; everything else follows.
//!
//! Entries are declared with the `comp!` macro below, which starts from
//! [`base`] and overwrites only the fields that differ. That keeps a ninety-row
//! table scannable — the interesting part of each row is what it *sets*.

pub mod spec;

pub use spec::{Category, ComponentSpec, Ctor, DynamicSlots, SlotRef, SlotSpec, CHILDREN};

use std::sync::OnceLock;

/// A spec with every optional field empty. `comp!` fills in the rest.
pub const fn base(
    kind: &'static str,
    title: &'static str,
    rust: &'static str,
    category: Category,
    icon: &'static str,
    blurb: &'static str,
    ctor: Ctor,
) -> ComponentSpec {
    ComponentSpec {
        kind,
        title,
        rust,
        category,
        icon,
        blurb,
        ctor,
        props: &[],
        slots: &[],
        dynamic: None,
        events: &[],
        on_place: None,
        imports: &[],
    }
}

/// Declare a component: the seven positional facts, then any field that
/// differs from [`base`].
macro_rules! comp {
    (
        $kind:literal, $title:literal, $rust:literal, $cat:ident, $icon:literal, $blurb:literal,
        $ctor:expr $(, $field:ident: $value:expr )* $(,)?
    ) => {{
        #[allow(unused_mut)]
        let mut spec = $crate::catalog::base(
            $kind, $title, $rust, $crate::catalog::Category::$cat, $icon, $blurb, $ctor,
        );
        $( spec.$field = $value; )*
        spec
    }};
}

mod charts;
mod controls;
mod data;
mod feedback;
mod inputs;
mod layout;
mod media;
mod nav;
mod typography;

fn registry() -> &'static Vec<&'static ComponentSpec> {
    static REGISTRY: OnceLock<Vec<&'static ComponentSpec>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let groups: [&'static [ComponentSpec]; 9] = [
            layout::SPECS,
            typography::SPECS,
            controls::SPECS,
            inputs::SPECS,
            data::SPECS,
            feedback::SPECS,
            nav::SPECS,
            charts::SPECS,
            media::SPECS,
        ];
        groups.into_iter().flatten().collect()
    })
}

/// Every component, in palette order.
pub fn all() -> &'static [&'static ComponentSpec] {
    registry()
}

pub fn get(kind: &str) -> Option<&'static ComponentSpec> {
    registry().iter().copied().find(|spec| spec.kind == kind)
}

/// The specs in one category.
pub fn in_category(category: Category) -> Vec<&'static ComponentSpec> {
    registry()
        .iter()
        .copied()
        .filter(|spec| spec.category == category)
        .collect()
}

/// Palette search: matches the title, the kind, or the blurb, title first.
pub fn search(query: &str) -> Vec<&'static ComponentSpec> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return registry().to_vec();
    }
    let mut scored: Vec<(u8, &'static ComponentSpec)> = registry()
        .iter()
        .copied()
        .filter_map(|spec| {
            let title = spec.title.to_lowercase();
            if title == needle {
                Some((0, spec))
            } else if title.starts_with(&needle) {
                Some((1, spec))
            } else if title.contains(&needle) || spec.kind.contains(&needle) {
                Some((2, spec))
            } else if spec.blurb.to_lowercase().contains(&needle) {
                Some((3, spec))
            } else {
                None
            }
        })
        .collect();
    scored.sort_by_key(|(rank, spec)| (*rank, spec.title));
    scored.into_iter().map(|(_, spec)| spec).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::DEFAULT_SLOT;

    #[test]
    fn every_kind_is_unique() {
        let mut kinds: Vec<&str> = all().iter().map(|spec| spec.kind).collect();
        let count = kinds.len();
        kinds.sort_unstable();
        kinds.dedup();
        assert_eq!(kinds.len(), count, "two catalog entries share a kind");
    }

    #[test]
    fn every_prop_key_is_unique_within_its_component() {
        for spec in all() {
            let mut keys: Vec<&str> = spec.props.iter().map(|p| p.key).collect();
            let count = keys.len();
            keys.sort_unstable();
            keys.dedup();
            assert_eq!(keys.len(), count, "{} repeats a prop key", spec.kind);
        }
    }

    #[test]
    fn constructors_name_props_that_exist() {
        for spec in all() {
            let referenced: Vec<&str> = match spec.ctor {
                Ctor::IdAnd(key)
                | Ctor::Arg(key)
                | Ctor::EntityArg(key)
                | Ctor::EntityValue(key) => vec![key],
                _ => vec![],
            };
            for key in referenced {
                assert!(
                    spec.prop(key).is_some(),
                    "{} constructs from unknown prop {key}",
                    spec.kind
                );
            }
            if let Some(dynamic) = spec.dynamic {
                assert!(
                    spec.prop(dynamic.from_prop).is_some(),
                    "{} takes slots from unknown prop {}",
                    spec.kind,
                    dynamic.from_prop
                );
            }
        }
    }

    #[test]
    fn containers_declare_the_default_slot_first() {
        for spec in all() {
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
    fn search_ranks_the_exact_title_first() {
        let hits = search("card");
        assert_eq!(hits[0].kind, "card");
        assert!(search("zzzznotathing").is_empty());
        assert_eq!(search("  ").len(), all().len());
    }

    #[test]
    fn the_catalog_covers_the_library() {
        // A floor, not a target: if a category is accidentally dropped from the
        // registry this catches it, and the number only ever goes up.
        assert!(all().len() >= 85, "catalog has shrunk to {}", all().len());
    }
}
