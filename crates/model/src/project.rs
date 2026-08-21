//! A `.tailor` file: every screen and component you are building, plus the
//! theme they are designed against and the settings the generator reads.
//!
//! One file rather than one-per-screen, because a component placed inside a
//! screen has to resolve, and cross-file references would mean a project
//! index anyway. The whole thing is small — it is a tree of props, not assets.

use crate::doc::{DocKind, Document};
use crate::tokens::{ColorToken, SizeToken};
use serde::{Deserialize, Serialize};

/// Bumped when the on-disk shape changes in a way a reader must know about.
/// A file from the future is refused rather than half-read.
pub const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scheme {
    #[default]
    Dark,
    Light,
}

impl Scheme {
    pub const ALL: &'static [Scheme] = &[Scheme::Dark, Scheme::Light];

    pub fn label(self) -> &'static str {
        match self {
            Scheme::Dark => "dark",
            Scheme::Light => "light",
        }
    }
}

/// The guise theme a project designs against. The canvas installs it so what
/// you see is what the generated app gets, and `theme.rs` is generated from it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeSpec {
    #[serde(default)]
    pub scheme: Scheme,
    #[serde(default = "default_primary")]
    pub primary: ColorToken,
    #[serde(default = "default_radius")]
    pub radius: SizeToken,
    #[serde(default = "default_font")]
    pub font: String,
}

fn default_primary() -> ColorToken {
    ColorToken::Blue
}

fn default_radius() -> SizeToken {
    SizeToken::Md
}

fn default_font() -> String {
    ".SystemUIFont".into()
}

impl Default for ThemeSpec {
    fn default() -> Self {
        ThemeSpec {
            scheme: Scheme::default(),
            primary: default_primary(),
            radius: default_radius(),
            font: default_font(),
        }
    }
}

/// Which flavour of guise the generator writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Flavor {
    /// Plain builder calls and gpui `Styled` methods. Always compiles, reads
    /// like the rest of an app.
    #[default]
    Plain,
    /// guise's layout macros (`col!`, `row!`) and `style! { … }` blocks.
    Macros,
}

impl Flavor {
    pub const ALL: &'static [Flavor] = &[Flavor::Plain, Flavor::Macros];

    pub fn label(self) -> &'static str {
        match self {
            Flavor::Plain => "plain",
            Flavor::Macros => "macros",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenSettings {
    #[serde(default)]
    pub flavor: Flavor,
    /// The crate-relative module the generated files go into.
    #[serde(default = "default_module")]
    pub module: String,
    /// Emit a `main.rs` and a `theme.rs` alongside the components, so the
    /// export is a crate you can run rather than a folder you have to wire up.
    #[serde(default = "yes")]
    pub emit_app: bool,
}

fn default_module() -> String {
    "ui".into()
}

fn yes() -> bool {
    true
}

impl Default for GenSettings {
    fn default() -> Self {
        GenSettings {
            flavor: Flavor::default(),
            module: default_module(),
            emit_app: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    #[serde(default = "current_version")]
    pub format: u32,
    pub name: String,
    pub docs: Vec<Document>,
    #[serde(default)]
    pub theme: ThemeSpec,
    #[serde(default)]
    pub gen: GenSettings,
}

fn current_version() -> u32 {
    FORMAT_VERSION
}

#[derive(Debug)]
pub enum LoadError {
    /// The file parsed, but was written by a newer Tailor.
    Newer(u32),
    Json(serde_json::Error),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Newer(v) => {
                write!(f, "this project was saved by a newer Tailor (format {v})")
            }
            LoadError::Json(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for LoadError {}

impl Project {
    /// A new project with one empty screen.
    pub fn new(name: impl Into<String>) -> Self {
        Project {
            format: FORMAT_VERSION,
            name: name.into(),
            docs: vec![Document::new("main", "MainScreen", DocKind::Screen)],
            theme: ThemeSpec::default(),
            gen: GenSettings::default(),
        }
    }

    pub fn doc(&self, id: &str) -> Option<&Document> {
        self.docs.iter().find(|d| d.id == id)
    }

    pub fn doc_mut(&mut self, id: &str) -> Option<&mut Document> {
        self.docs.iter_mut().find(|d| d.id == id)
    }

    /// A document by the name a component reference uses.
    pub fn doc_by_name(&self, name: &str) -> Option<&Document> {
        self.docs.iter().find(|d| d.name == name)
    }

    /// The components (not screens) another document may place.
    pub fn placeable(&self, excluding: &str) -> Vec<&Document> {
        self.docs
            .iter()
            .filter(|d| d.kind == DocKind::Component && d.id != excluding)
            .collect()
    }

    pub fn unique_doc_name(&self, base: &str) -> String {
        crate::doc::unique(base, |candidate| {
            self.docs.iter().any(|d| d.name == candidate)
        })
    }

    pub fn unique_doc_id(&self, base: &str) -> String {
        crate::doc::unique(base, |candidate| {
            self.docs.iter().any(|d| d.id == candidate)
        })
    }

    /// Would placing the component named `placing` inside the document named
    /// `host` create a cycle? Both arguments are document *names*, which is
    /// what a `@Name` reference carries. The visited set is not paranoia: a
    /// hand-edited file can already contain the cycle this exists to prevent.
    pub fn would_recurse(&self, host: &str, placing: &str) -> bool {
        fn walk<'a>(
            project: &'a Project,
            host: &str,
            placing: &'a str,
            seen: &mut Vec<&'a str>,
        ) -> bool {
            if host == placing {
                return true;
            }
            if seen.contains(&placing) {
                return true;
            }
            seen.push(placing);
            let Some(doc) = project.doc_by_name(placing) else {
                return false;
            };
            doc.nodes.values().any(|node| match node.component_ref() {
                Some(inner) => walk(project, host, inner, seen),
                None => false,
            })
        }
        walk(self, host, placing, &mut Vec::new())
    }

    /// The first number in the project that JSON cannot write, named well
    /// enough to fix. serde turns an infinity or a NaN into `null` rather than
    /// failing, and the file that results does not load again — so a save has
    /// to look before it writes.
    pub fn non_finite(&self) -> Option<String> {
        for doc in &self.docs {
            for node in doc.nodes.values() {
                if node.style.has_non_finite() {
                    return Some(format!(
                        "{}: node {} has an unwritable size",
                        doc.name, node.id
                    ));
                }
                for (key, value) in &node.props {
                    if value.has_non_finite() {
                        return Some(format!(
                            "{}: node {}'s `{key}` is not a finite number",
                            doc.name, node.id
                        ));
                    }
                }
            }
        }
        None
    }

    /// Serialize. Fallible on purpose — see [`Project::non_finite`].
    pub fn to_json(&self) -> Result<String, String> {
        if let Some(found) = self.non_finite() {
            return Err(found);
        }
        serde_json::to_string_pretty(self).map_err(|err| err.to_string())
    }

    pub fn from_json(text: &str) -> Result<Self, LoadError> {
        let mut project: Project = serde_json::from_str(text).map_err(LoadError::Json)?;
        if project.format > FORMAT_VERSION {
            return Err(LoadError::Newer(project.format));
        }
        for doc in &mut project.docs {
            doc.repair();
        }
        if project.docs.is_empty() {
            project
                .docs
                .push(Document::new("main", "MainScreen", DocKind::Screen));
        }
        Ok(project)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::DEFAULT_SLOT;

    #[test]
    fn a_new_project_round_trips() {
        let project = Project::new("Demo");
        let parsed = Project::from_json(&project.to_json().unwrap()).unwrap();
        assert_eq!(parsed, project);
    }

    #[test]
    fn a_non_finite_number_is_refused_rather_than_panicking() {
        use crate::node::DEFAULT_SLOT;
        let mut project = Project::new("Demo");
        let doc = &mut project.docs[0];
        let root = doc.root;
        let node = doc.create("progress");
        let id = doc.insert(root, DEFAULT_SLOT, 0, node);
        doc.node_mut(id)
            .unwrap()
            .set_prop("value", crate::props::PropValue::Float(f64::NAN));
        let err = project.to_json().unwrap_err();
        assert!(err.contains("finite"), "{err}");
    }

    #[test]
    fn a_newer_format_is_refused() {
        let mut project = Project::new("Demo");
        project.format = FORMAT_VERSION + 1;
        let err = Project::from_json(&project.to_json().unwrap()).unwrap_err();
        assert!(matches!(err, LoadError::Newer(_)));
    }

    #[test]
    fn loading_repairs_a_hand_edited_file() {
        let mut project = Project::new("Demo");
        let doc = &mut project.docs[0];
        let root = doc.root;
        doc.node_mut(root)
            .unwrap()
            .slot_mut(DEFAULT_SLOT)
            .push(crate::id::NodeId(77));
        let parsed = Project::from_json(&project.to_json().unwrap()).unwrap();
        assert!(parsed.docs[0].children_of(parsed.docs[0].root).is_empty());
    }

    #[test]
    fn a_component_that_contains_its_host_is_a_cycle() {
        let mut project = Project::new("Demo");
        project
            .docs
            .push(Document::new("card", "Card", DocKind::Component));
        project
            .docs
            .push(Document::new("row", "Row", DocKind::Component));

        // Row places Card.
        let node = project.doc_mut("row").unwrap().create("@Card");
        let root = project.doc("row").unwrap().root;
        project
            .doc_mut("row")
            .unwrap()
            .insert(root, DEFAULT_SLOT, 0, node);

        assert!(project.would_recurse("Card", "Row"));
        assert!(!project.would_recurse("Row", "Card"));
        assert!(project.would_recurse("Card", "Card"));
    }

    #[test]
    fn placeable_lists_components_but_not_the_host() {
        let mut project = Project::new("Demo");
        project
            .docs
            .push(Document::new("card", "Card", DocKind::Component));
        let names: Vec<&str> = project
            .placeable("main")
            .iter()
            .map(|d| d.name.as_str())
            .collect();
        assert_eq!(names, ["Card"]);
        assert!(project.placeable("card").is_empty());
    }
}
