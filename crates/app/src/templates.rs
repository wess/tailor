//! Projects you can start from.
//!
//! A blank canvas is the worst first screen a builder can show: you learn
//! nothing from it. These four are complete, laid out, and generate — opening
//! one and hitting Export is a working crate.

use tailor_model::node::DEFAULT_SLOT;
use tailor_model::props::PropValue as V;
use tailor_model::style::{Dimension, Edges};
use tailor_model::{
  ActionDef, ColorSpec, ColorToken, DocKind, Document, NodeId, Project, SizeToken, StateVar,
  VarType,
};

pub struct Template {
  pub name: &'static str,
  pub blurb: &'static str,
  pub icon: &'static str,
  pub build: fn() -> Project,
}

pub const TEMPLATES: &[Template] = &[
  Template {
    name: "Empty",
    blurb: "One screen, nothing on it.",
    icon: "square-dashed",
    build: empty,
  },
  Template {
    name: "Sign in",
    blurb: "A centred form with fields, a button, and an action wired up.",
    icon: "log-in",
    build: sign_in,
  },
  Template {
    name: "Dashboard",
    blurb: "App shell, sidebar, stat cards, and a chart.",
    icon: "layout-dashboard",
    build: dashboard,
  },
  Template {
    name: "Settings",
    blurb: "A tabbed settings screen with rows of controls.",
    icon: "settings",
    build: settings,
  },
];

fn empty() -> Project {
  Project::new("Untitled")
}

/// Add a node of `kind` under `parent`, run `setup` on it, and return its id.
fn add(
  doc: &mut Document,
  parent: NodeId,
  kind: &str,
  setup: impl FnOnce(&mut tailor_model::Node),
) -> NodeId {
  let spec = tailor_model::catalog::get(kind);
  let mut node = match spec {
    Some(spec) => spec.build(doc.ids.next()),
    None => doc.create(kind),
  };
  setup(&mut node);
  doc.insert(parent, DEFAULT_SLOT, usize::MAX, node)
}

fn sign_in() -> Project {
  let mut project = Project::new("Sign in");
  project.docs[0].name = "SignInScreen".into();
  let doc = &mut project.docs[0];
  doc.state.push(StateVar::new("email", VarType::Text));
  doc.actions.push(ActionDef::new("submit"));
  doc.canvas.width = 640.0;
  doc.canvas.height = 560.0;

  let root = doc.root;
  doc.node_mut(root).unwrap().style.padding = Edges::all(48.0);
  doc.node_mut(root).unwrap().style.gap = Some(0.0);
  doc.node_mut(root).unwrap().style.align = Some(tailor_model::AlignToken::Center);
  doc.node_mut(root).unwrap().style.justify = Some(tailor_model::JustifyToken::Center);

  let card = add(doc, root, "card", |node| {
    node.name = Some("Form".into());
    node.set_prop("padding", V::Size(SizeToken::Xl));
    node.style.width = Dimension::Px(380.0);
  });
  let stack = add(doc, card, "frame", |node| {
    node.style.gap = Some(16.0);
  });

  add(doc, stack, "title", |node| {
    node.set_prop("content", V::Text("Welcome back".into()));
    node.set_prop("order", V::Int(2));
  });
  add(doc, stack, "text", |node| {
    node.set_prop("content", V::Text("Sign in to continue.".into()));
    node.set_prop("dimmed", V::Bool(true));
    node.set_prop("size", V::Size(SizeToken::Sm));
  });
  add(doc, stack, "textinput", |node| {
    node.name = Some("Email".into());
    node.set_prop("label", V::Text("Email".into()));
    node.set_prop("placeholder", V::Text("you@example.com".into()));
  });
  add(doc, stack, "passwordinput", |node| {
    node.name = Some("Password".into());
    node.set_prop("label", V::Text("Password".into()));
    node.set_prop("placeholder", V::Text("••••••••".into()));
  });
  add(doc, stack, "checkbox", |node| {
    node.set_prop("label", V::Text("Keep me signed in".into()));
    node.set_prop("size", V::Size(SizeToken::Sm));
  });
  add(doc, stack, "button", |node| {
    node.name = Some("Submit".into());
    node.set_prop("label", V::Text("Sign in".into()));
    node.set_prop("full_width", V::Bool(true));
    node.events.insert("click".into(), "submit".into());
  });

  project
}

fn dashboard() -> Project {
  let mut project = Project::new("Dashboard");
  project.docs[0].name = "DashboardScreen".into();
  project
    .docs
    .push(Document::new("statcard", "StatCard", DocKind::Component));

  // The reusable card first, so the screen can place it.
  {
    let card = project.doc_mut("statcard").unwrap();
    let root = card.root;
    card.node_mut(root).unwrap().style.padding = Edges::all(0.0);
    card.node_mut(root).unwrap().style.gap = Some(0.0);
    let surface = add(card, root, "card", |node| {
      node.set_prop("padding", V::Size(SizeToken::Lg));
      node.style.width = Dimension::Grow(1.0);
    });
    let stack = add(card, surface, "frame", |node| {
      node.style.gap = Some(4.0);
      node.style.padding = Edges::all(0.0);
    });
    add(card, stack, "text", |node| {
      node.set_prop("content", V::Text("Active users".into()));
      node.set_prop("size", V::Size(SizeToken::Sm));
      node.set_prop("dimmed", V::Bool(true));
    });
    add(card, stack, "title", |node| {
      node.set_prop("content", V::Text("12,480".into()));
      node.set_prop("order", V::Int(2));
    });
    add(card, stack, "sparkline", |node| {
      node.set_prop("width", V::Float(180.0));
      node.set_prop("height", V::Float(28.0));
    });
  }

  let doc = project.doc_mut("main").unwrap();
  doc.canvas.width = 1280.0;
  doc.canvas.height = 800.0;
  let root = doc.root;
  doc.node_mut(root).unwrap().style.padding = Edges::all(0.0);
  doc.node_mut(root).unwrap().style.gap = Some(0.0);
  doc.node_mut(root).unwrap().style.height = Dimension::Full;

  let shell = add(doc, root, "appshell", |node| {
    node.style.height = Dimension::Full;
  });
  let shell_node = doc.node_mut(shell).unwrap();
  let header_id = NodeId(0);
  let _ = header_id;
  let _ = shell_node;

  // Header
  let header = doc.create("group");
  let header_id = header.id;
  doc.nodes.insert(header_id, header);
  doc.node_mut(header_id).unwrap().style.padding = Edges::symmetric(16.0, 0.0);
  doc
    .node_mut(header_id)
    .unwrap()
    .set_prop("gap", V::Size(SizeToken::Md));
  doc
    .node_mut(shell)
    .unwrap()
    .slot_mut("header")
    .push(header_id);
  add(doc, header_id, "title", |node| {
    node.set_prop("content", V::Text("Overview".into()));
    node.set_prop("order", V::Int(4));
  });
  add(doc, header_id, "spacer", |_| {});
  add(doc, header_id, "badge", |node| {
    node.set_prop("label", V::Text("Live".into()));
    node.set_prop("color", V::Color(ColorSpec::Named(ColorToken::Green)));
  });

  // Navbar
  let nav = doc.create("frame");
  let nav_id = nav.id;
  doc.nodes.insert(nav_id, nav);
  doc.node_mut(nav_id).unwrap().style.padding = Edges::all(12.0);
  doc.node_mut(nav_id).unwrap().style.gap = Some(2.0);
  doc.node_mut(shell).unwrap().slot_mut("navbar").push(nav_id);
  for (label, icon, active) in [
    ("Overview", "layout-dashboard", true),
    ("Reports", "chart-column", false),
    ("Customers", "users", false),
    ("Settings", "settings", false),
  ] {
    add(doc, nav_id, "navlink", |node| {
      node.set_prop("label", V::Text(label.into()));
      node.set_prop("icon", V::Icon(icon.into()));
      node.set_prop("active", V::Bool(active));
    });
  }

  // Body
  let body = add(doc, shell, "frame", |node| {
    node.style.padding = Edges::all(24.0);
    node.style.gap = Some(20.0);
  });
  let stats = add(doc, body, "frame", |node| {
    node.name = Some("Stat row".into());
    node.style.direction = tailor_model::Direction::Row;
    node.style.gap = Some(16.0);
    node.style.padding = Edges::all(0.0);
  });
  for _ in 0..3 {
    add(doc, stats, "@StatCard", |_| {});
  }
  let chart_card = add(doc, body, "card", |node| {
    node.set_prop("padding", V::Size(SizeToken::Lg));
  });
  add(doc, chart_card, "barchart", |node| {
    node.set_prop("width", V::Float(760.0));
    node.set_prop("height", V::Float(220.0));
  });

  project
}

fn settings() -> Project {
  let mut project = Project::new("Settings");
  project.docs[0].name = "SettingsScreen".into();
  let doc = &mut project.docs[0];
  doc.canvas.width = 820.0;
  doc.canvas.height = 620.0;
  let root = doc.root;
  doc.node_mut(root).unwrap().style.padding = Edges::all(24.0);
  doc.node_mut(root).unwrap().style.gap = Some(16.0);

  add(doc, root, "title", |node| {
    node.set_prop("content", V::Text("Settings".into()));
    node.set_prop("order", V::Int(2));
  });

  let tabs = add(doc, root, "tabs", |node| {
    node.set_prop(
      "tabs",
      V::Items(vec![
        "General".into(),
        "Appearance".into(),
        "Advanced".into(),
      ]),
    );
  });

  let general = doc.create("frame");
  let general_id = general.id;
  doc.nodes.insert(general_id, general);
  doc.node_mut(general_id).unwrap().style.gap = Some(14.0);
  doc.node_mut(general_id).unwrap().style.padding = Edges::all(0.0);
  doc
    .node_mut(tabs)
    .unwrap()
    .slot_mut("tab:0")
    .push(general_id);

  for (label, kind) in [
    ("Show the sidebar", "switch"),
    ("Confirm before deleting", "switch"),
    ("Autosave", "switch"),
  ] {
    let row = add(doc, general_id, "frame", |node| {
      node.style.direction = tailor_model::Direction::Row;
      node.style.justify = Some(tailor_model::JustifyToken::Between);
      node.style.align = Some(tailor_model::AlignToken::Center);
      node.style.padding = Edges::all(0.0);
      node.style.gap = Some(12.0);
    });
    add(doc, row, "text", |node| {
      node.set_prop("content", V::Text(label.into()));
    });
    add(doc, row, kind, |node| {
      node.set_prop("checked", V::Bool(true));
    });
  }

  let appearance = doc.create("frame");
  let appearance_id = appearance.id;
  doc.nodes.insert(appearance_id, appearance);
  doc.node_mut(appearance_id).unwrap().style.gap = Some(14.0);
  doc.node_mut(appearance_id).unwrap().style.padding = Edges::all(0.0);
  doc
    .node_mut(tabs)
    .unwrap()
    .slot_mut("tab:1")
    .push(appearance_id);
  add(doc, appearance_id, "segmented", |node| {
    node.set_prop(
      "data",
      V::Items(vec!["Light".into(), "Dark".into(), "System".into()]),
    );
    node.set_prop("selected", V::Int(1));
  });
  add(doc, appearance_id, "slider", |node| {
    node.name = Some("Font size".into());
    node.set_prop("min", V::Float(10.0));
    node.set_prop("max", V::Float(20.0));
  });

  project
}
