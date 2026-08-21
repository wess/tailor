//! The checks behind the Problems panel.
//!
//! Android Studio's layout editor puts a lint list next to the canvas, and it
//! is the difference between "this looks fine" and "this will compile". Every
//! check here is something the canvas cannot show you: a binding to a variable
//! you renamed, an event pointing at an action you deleted, a container the
//! generator cannot fill.

use crate::catalog;
use crate::node::DEFAULT_SLOT;
use crate::{DocKind, Document, NodeId, Project};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// The generated file will not compile, or the design is broken.
    Error,
    /// It will build, but it is probably not what you meant.
    Warning,
    /// Worth knowing.
    Info,
}

impl Severity {
    pub fn label(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Severity::Error => "circle-x",
            Severity::Warning => "triangle-alert",
            Severity::Info => "info",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Problem {
    pub severity: Severity,
    /// The document this is about.
    pub doc_id: String,
    /// The node, when it is about one — clicking the row selects it.
    pub node: Option<NodeId>,
    pub message: String,
    /// What to do about it.
    pub fix: String,
}

/// Every problem in the project, most severe first.
pub fn check(project: &Project) -> Vec<Problem> {
    let mut out = Vec::new();
    let mut names: Vec<&str> = Vec::new();

    for doc in &project.docs {
        if names.contains(&doc.name.as_str()) {
            out.push(Problem {
                severity: Severity::Error,
                doc_id: doc.id.clone(),
                node: None,
                message: format!("Two documents are called {}", doc.name),
                fix: "Rename one — they generate to the same type.".into(),
            });
        }
        names.push(&doc.name);

        // A document called `Button` generates `pub struct Button` into a file
        // that glob-imports guise. The local item wins, and every `Button::new`
        // in it then resolves to the wrong type.
        let generated = crate::pascal_case(&doc.name);
        if shadows_guise(&generated) {
            out.push(Problem {
                severity: Severity::Error,
                doc_id: doc.id.clone(),
                node: None,
                message: format!("{generated} is also a guise component"),
                fix: "Rename it — the generated file imports guise, and the two \
                      names would collide."
                    .into(),
            });
        }

        if generated != doc.name.replace([' ', '-', '_'], "") {
            out.push(Problem {
                severity: Severity::Info,
                doc_id: doc.id.clone(),
                node: None,
                message: format!(
                    "{} generates as {}",
                    doc.name,
                    crate::pascal_case(&doc.name)
                ),
                fix: "Names become Rust types, so spaces and dashes are dropped.".into(),
            });
        }

        check_document(project, doc, &mut out);
    }

    out.sort_by_key(|problem| problem.severity);
    out
}

fn check_document(project: &Project, doc: &Document, out: &mut Vec<Problem>) {
    let mut report = |severity, node, message: String, fix: &str| {
        out.push(Problem {
            severity,
            doc_id: doc.id.clone(),
            node,
            message,
            fix: fix.to_string(),
        });
    };

    for id in std::iter::once(doc.root).chain(doc.descendants(doc.root)) {
        let Some(node) = doc.node(id) else { continue };
        let label = node
            .name
            .clone()
            .or_else(|| catalog::get(&node.kind).map(|spec| spec.title.to_string()))
            .unwrap_or_else(|| node.kind.clone());

        // A reference to a document that is gone, or that is a screen.
        if let Some(name) = node.component_ref() {
            match project.doc_by_name(name) {
                None => report(
                    Severity::Error,
                    Some(id),
                    format!("{label} places a component called {name}, which no longer exists"),
                    "Delete the node, or rename the component back.",
                ),
                Some(target) if target.kind == DocKind::Screen => report(
                    Severity::Warning,
                    Some(id),
                    format!("{name} is a screen, not a component"),
                    "Screens own their own state; mark it a component to place it.",
                ),
                Some(_) => {}
            }
            continue;
        }

        let Some(spec) = catalog::get(&node.kind) else {
            report(
                Severity::Error,
                Some(id),
                format!("{} is not a component Tailor knows", node.kind),
                "The file may have been written by a newer version.",
            );
            continue;
        };

        // Bindings to variables that are not there any more.
        for (key, value) in &node.props {
            if let Some(var) = value.as_binding() {
                if doc.var(var).is_none() {
                    report(
                        Severity::Error,
                        Some(id),
                        format!("{label}'s {key} reads a variable called {var}, which is gone"),
                        "Rebind it, or add the variable back in the State panel.",
                    );
                }
            }
        }

        // Events pointing at actions that are not there any more.
        for (event, action) in &node.events {
            if !action.is_empty() && !doc.actions.iter().any(|a| a.name == *action) {
                report(
                    Severity::Error,
                    Some(id),
                    format!("{label}'s {event} calls {action}, which is not an action"),
                    "Add the action, or clear the connection.",
                );
            }
        }

        // A container with nothing in it generates an empty element.
        if spec.takes_children() && node.slot(DEFAULT_SLOT).is_empty() && node.slots.is_empty() {
            report(
                Severity::Info,
                Some(id),
                format!("{label} is empty"),
                "Drop something into it, or delete it.",
            );
        }

        // The five containers whose regions are `'static` closures cannot hold
        // a component that lives in a struct field.
        if matches!(node.kind.as_str(), "tabs" | "accordion" | "splitpanel") {
            for child in doc.descendants(id) {
                let holds_state = doc
                    .node(child)
                    .and_then(|node| catalog::get(&node.kind))
                    .map(|spec| spec.ctor.is_entity())
                    .unwrap_or(false);
                if holds_state {
                    report(
                        Severity::Warning,
                        Some(child),
                        format!("{label} holds a component that owns state"),
                        "Its regions are static closures. Extract that part into \
                         its own component and place that instead.",
                    );
                    break;
                }
            }
        }

        // Components whose whole point is a value nobody set.
        let blank = |key: &str| {
            node.prop(key)
                .map(|value| value.is_empty())
                .unwrap_or_else(|| {
                    spec.default_prop(key)
                        .map(|value| value.is_empty())
                        .unwrap_or(false)
                })
        };
        match node.kind.as_str() {
            "button" | "badge" | "chip" | "anchor" | "navlink" if blank("label") => report(
                Severity::Warning,
                Some(id),
                format!("{label} has no label"),
                "Set one in the Attributes inspector.",
            ),
            "text" | "title" if blank("content") => report(
                Severity::Warning,
                Some(id),
                format!("{label} has no content"),
                "Type into Content, or bind it to a state variable.",
            ),
            "image" if blank("source") => report(
                Severity::Warning,
                Some(id),
                "Image has no source".into(),
                "Point it at a file path or a URL.",
            ),
            "icon" | "actionicon" if blank("icon") => report(
                Severity::Warning,
                Some(id),
                format!("{label} has no icon"),
                "Pick one from the icon picker.",
            ),
            _ => {}
        }
    }

    // Variables and actions that generate to the same field.
    let mut seen: Vec<String> = Vec::new();
    for var in &doc.state {
        let ident = crate::snake_case(&var.name);
        if seen.contains(&ident) {
            out.push(Problem {
                severity: Severity::Error,
                doc_id: doc.id.clone(),
                node: None,
                message: format!("Two state variables generate the field {ident}"),
                fix: "Rename one.".into(),
            });
        }
        seen.push(ident);
    }

    // A screen with no entry point into it is fine; a component that nothing
    // places is worth mentioning once.
    if doc.kind == DocKind::Component {
        let placed = project.docs.iter().any(|other| {
            other.id != doc.id
                && other
                    .nodes
                    .values()
                    .any(|node| node.component_ref() == Some(doc.name.as_str()))
        });
        if !placed {
            out.push(Problem {
                severity: Severity::Info,
                doc_id: doc.id.clone(),
                node: None,
                message: format!("{} is not placed anywhere", doc.name),
                fix: "It still generates — components are useful on their own.".into(),
            });
        }
    }
}

/// Whether a generated type name would collide with something the prelude
/// brings in. The catalog knows every component's Rust name; the rest is what
/// a generated file also names.
pub fn shadows_guise(name: &str) -> bool {
    const ALSO: &[&str] = &[
        "Signal",
        "Binding",
        "Theme",
        "Size",
        "Variant",
        "ColorName",
        "Color",
        "Align",
        "Justify",
        "Glyph",
        "IconName",
        "Window",
        "App",
    ];
    ALSO.contains(&name)
        || catalog::all()
            .iter()
            .any(|spec| !spec.rust.is_empty() && spec.rust == name)
}

/// Only the problems that belong to one document.
pub fn for_document<'a>(problems: &'a [Problem], doc_id: &str) -> Vec<&'a Problem> {
    problems
        .iter()
        .filter(|problem| problem.doc_id == doc_id)
        .collect()
}

/// How many of each severity, for the status bar.
pub fn counts(problems: &[Problem]) -> (usize, usize, usize) {
    let count = |severity| problems.iter().filter(|p| p.severity == severity).count();
    (
        count(Severity::Error),
        count(Severity::Warning),
        count(Severity::Info),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::props::PropValue as V;
    use crate::{ActionDef, StateVar, VarType};

    fn project() -> Project {
        Project::new("Demo")
    }

    #[test]
    fn a_binding_to_a_missing_variable_is_an_error() {
        let mut project = project();
        let doc = &mut project.docs[0];
        let root = doc.root;
        let node = doc.create("text");
        let id = doc.insert(root, DEFAULT_SLOT, 0, node);
        doc.node_mut(id)
            .unwrap()
            .set_prop("content", V::Binding("gone".into()));

        let problems = check(&project);
        assert!(problems
            .iter()
            .any(|p| p.severity == Severity::Error && p.message.contains("gone")));
    }

    #[test]
    fn a_binding_to_a_live_variable_is_fine() {
        let mut project = project();
        let doc = &mut project.docs[0];
        doc.state.push(StateVar::new("query", VarType::Text));
        let root = doc.root;
        let node = doc.create("text");
        let id = doc.insert(root, DEFAULT_SLOT, 0, node);
        doc.node_mut(id)
            .unwrap()
            .set_prop("content", V::Binding("query".into()));

        let problems = check(&project);
        assert!(!problems.iter().any(|p| p.severity == Severity::Error));
    }

    #[test]
    fn an_event_without_its_action_is_an_error() {
        let mut project = project();
        let doc = &mut project.docs[0];
        let root = doc.root;
        let node = doc.create("button");
        let id = doc.insert(root, DEFAULT_SLOT, 0, node);
        doc.node_mut(id)
            .unwrap()
            .events
            .insert("click".into(), "submit".into());

        assert!(check(&project).iter().any(|p| p.message.contains("submit")));

        project.docs[0].actions.push(ActionDef::new("submit"));
        assert!(!check(&project)
            .iter()
            .any(|p| p.message.contains("calls submit")));
    }

    #[test]
    fn a_field_inside_tabs_is_flagged() {
        let mut project = project();
        let doc = &mut project.docs[0];
        let root = doc.root;
        let tabs = doc.create("tabs");
        let tabs = doc.insert(root, DEFAULT_SLOT, 0, tabs);
        let field = doc.create("textinput");
        let field_id = field.id;
        doc.nodes.insert(field_id, field);
        doc.node_mut(tabs).unwrap().slot_mut("tab:0").push(field_id);

        let problems = check(&project);
        assert!(problems
            .iter()
            .any(|p| p.severity == Severity::Warning && p.message.contains("owns state")));
    }

    #[test]
    fn a_missing_component_reference_is_an_error() {
        let mut project = project();
        let doc = &mut project.docs[0];
        let root = doc.root;
        let node = doc.create("@Ghost");
        doc.insert(root, DEFAULT_SLOT, 0, node);

        assert!(check(&project)
            .iter()
            .any(|p| p.severity == Severity::Error && p.message.contains("Ghost")));
    }

    #[test]
    fn a_button_with_no_label_is_a_warning() {
        let mut project = project();
        let doc = &mut project.docs[0];
        let root = doc.root;
        let node = doc.create("button");
        doc.insert(root, DEFAULT_SLOT, 0, node);

        let problems = check(&project);
        assert!(problems.iter().any(|p| p.message.contains("no label")));
    }

    #[test]
    fn a_document_named_after_a_guise_component_is_an_error() {
        let mut project = project();
        project.docs[0].name = "Button".into();
        let problems = check(&project);
        assert!(problems
            .iter()
            .any(|p| p.severity == Severity::Error && p.message.contains("guise component")));

        project.docs[0].name = "ButtonRow".into();
        assert!(!check(&project)
            .iter()
            .any(|p| p.message.contains("guise component")));
    }

    #[test]
    fn duplicate_document_names_are_an_error() {
        let mut project = project();
        project
            .docs
            .push(Document::new("other", "MainScreen", DocKind::Component));
        assert!(check(&project)
            .iter()
            .any(|p| p.severity == Severity::Error && p.message.contains("Two documents")));
    }

    #[test]
    fn problems_are_sorted_and_countable() {
        let mut project = project();
        let doc = &mut project.docs[0];
        let root = doc.root;
        let node = doc.create("button");
        let id = doc.insert(root, DEFAULT_SLOT, 0, node);
        doc.node_mut(id)
            .unwrap()
            .events
            .insert("click".into(), "nope".into());

        let problems = check(&project);
        assert_eq!(problems[0].severity, Severity::Error);
        let (errors, warnings, _) = counts(&problems);
        assert_eq!(errors, 1);
        assert!(warnings >= 1);
    }
}
