# Component libraries

Tailor draws with [guise](https://github.com/wess/guise), and it *targets*
guise — but those are two different facts, and only the second one is
interesting. Tailor is written in guise the way an IDE is written in Qt. What it
targets is a plug-in.

A **library** is everything Tailor needs to know about a set of components: what
they are called, what props each one takes, what Rust each one generates, and
how to build a live one for the canvas. That is a trait — three, actually — and
guise is the first implementation of them rather than a special case.

```
crates/guise/         # tailor-guise        — what guise is, and what it generates
crates/guiserender/   # tailor-guiserender  — what one of its components looks like
```

Neither is privileged. `tailor-guise` uses only the public API of
`tailor-model` and `tailor-codegen`; `tailor-guiserender` uses only
`tailor-render`'s. A second library is two more crates of the same shape, and
one more line in each binary's `main`.

## The three seams

| Trait | Crate it plugs into | Needs gpui | What it answers |
| --- | --- | --- | --- |
| `Library` | `tailor-model` | no | What components exist, what props they take, what theme presets ship, what a token's type is called |
| `Generator` | `tailor-codegen` | no | The Rust for the shapes a chained constructor cannot express |
| `Renderer` | `tailor-render` | **yes** | The live component the canvas draws, and the entity behind a stateful one |

The gpui column is the reason there are three and not one.
[`tailor-mcp`](mcp.md) lists components, sets props and exports Rust with no
window anywhere near it. If describing a library required a renderer, the MCP
server would have to link gpui to answer "what props does a Button take". So the
description and the generator are one crate, and the drawing is another.

Registration is explicit, from `main`, before anything opens a project:

```rust
// crates/app/src/main.rs — a window, so everything.
tailor_guiserender::register();

// crates/mcp/src/main.rs — no window, so the description and the generator.
tailor_guise::register();
```

`tailor_guiserender::register()` calls `tailor_guise::register()` for you: a
renderer with no catalog behind it is not a thing. Both are idempotent.

## What a project stores

A `.tailor` file names its library by id:

```json
{ "format": 1, "name": "Demo", "library": "guise", "docs": [ … ] }
```

The field is skipped when empty, and empty resolves to whatever registered
first — so every project written before Tailor could target more than one
library opens unchanged, on guise, with no migration.

An id this build does not ship is **not** an error. `library::resolve` falls
back to the default rather than refusing to open the file, because a project you
cannot look at is worse than one drawn with the wrong catalog, and the
[Problems panel](state.md#problems) is where a person will see what happened.

## Writing a provider

### 1. The catalog

One `comp!` per component, in tables grouped by category. This is the bulk of
the work and it is all declarative:

```rust
use tailor_model::catalog::{ComponentSpec, Ctor, CHILDREN};
use tailor_model::comp;
use tailor_model::props::{size, text, Emit};

pub static SPECS: &[ComponentSpec] = &[
  comp!("button", "Button", "Button", Controls, "square",
    "A labelled button.",
    Ctor::Arg("label"),
    props: &[
      text("label", "Label", Emit::None),
      size("size", "Size", Emit::Method("size"), SizeToken::Md),
    ],
    events: &[CLICK],
    required: &[("label", "Set one in the Attributes inspector.")],
    aliases: &["cta", "submit"],
  ),
];
```

Seven positional facts — kind, title, Rust type, category, icon, blurb,
constructor — then any field that differs from the default. The interesting part
of a row is what it *sets*.

`Ctor` is how the constructor is called: `Unit` for `T::new()`, `Arg("label")`
for `T::new(<label>)`, `Id` for a component that needs a stable element id,
`Entity` for one that owns state and is built with `cx.new`, and `Special` for
one the generator writes by hand. `PropSpec::emit` decides what each prop
prints: `Emit::Method("size")` gives `.size(..)`, `Emit::Flag("fill")` gives
`.fill()` when the bool is true, `Emit::None` means the constructor already took
it.

`docs`, `example` and `aliases` are the agent-facing fields. A person picks a
component from a row of icons and finds out the rest by dropping one on the
canvas; an agent gets one shot from a text description, so the difference
between the right component and a plausible wrong one has to be written down.

### 2. `impl Library`

Facts, not behaviour. Everything below the divider in the trait — `get`,
`in_category`, `search`, `shadows` — has a default, and every library searches
its catalog identically.

```rust
impl Library for Guise {
  fn id(&self) -> &'static str { "guise" }
  fn label(&self) -> &'static str { "guise" }
  fn krate(&self) -> &'static str { "guise-ui" }
  fn version_req(&self) -> &'static str { "1.6" }
  fn prelude(&self) -> &'static [&'static str] {
    &["use gpui::prelude::*;", "use guise::prelude::*;"]
  }
  fn components(&self) -> &'static [&'static ComponentSpec] { catalog::registry() }
  fn presets(&self) -> &'static [ThemePreset] { theme::PRESETS }
  fn token_paths(&self) -> TokenPaths { /* Size, Variant, ColorName, … */ }
  fn boxes(&self) -> &'static [&'static str] { &["frame", "canvas", "surface", "spacer"] }
  fn drawn(&self) -> &'static [&'static str] { &["tabs", "accordion", /* … */] }
  fn reserved(&self) -> &'static [&'static str] { &["Theme", "Size", /* … */] }
}
```

Four of these are worth a word:

- **`token_paths`** — a document stores `md`, not `Size::Md`. The variant name
  is Tailor's; the type in front of it is yours.
- **`boxes`** — kinds whose expression *is* a styled box, so a node's layout
  goes onto it rather than into a wrapper. A wrapper is a new flex item, and a
  child that was `w_full` would start measuring against it.
- **`drawn`** — containers whose regions are `'static` closures. A designer
  cannot drop into a closure, so the canvas draws these from the theme instead
  (which is also what makes their slots real drop targets) while generated code
  uses the real component. The linter warns before an export fails to compile.
- **`reserved`** — a generated file glob-imports your prelude, so a document
  called `Button` would shadow the real one and every `Button::new` in the file
  would resolve to the wrong type. Component names are covered automatically;
  this is for the rest of the prelude.

### 3. `impl Generator`

Two required methods. `special` is the escape hatch for components whose Rust is
not one chained call, and `theme_rs` writes the theme file, which is entirely
your vocabulary. Everything else has a default:

```rust
impl Generator for Guise {
  fn library(&self) -> &'static dyn Library { crate::library() }

  fn special(&self, em: &mut Emitter, node: &Node) -> Option<Vec<String>> {
    Some(match node.kind.as_str() {
      "frame" => vec!["div()".into()],
      "divider" => vec!["Divider::new()".into()],
      _ => return None,
    })
  }

  fn theme_rs(&self, project: &Project) -> Generated { /* … */ }
}
```

`Emitter` is the generator's working state, and a `special` arm drives it:
`em.prop_value(node, key)` reads a prop with its catalog default,
`em.emit(child, placement)` builds a child expression, `em.push_scope()` /
`em.wrap_closure(..)` build a `'static` region closure with everything it
touches cloned in ahead of it.

The optional hooks cover the rest of what guise needed:
`custom_props` for list props that become one call per item, `closure_slots` and
`slot_size_prop` for regions that take a closure, `slots` for a component that
takes its regions some other way entirely, `entity_bind` / `controlled_bind` for
two-way binding, and `motion` for the animation vocabulary.

A library with none of those irregularities implements two methods and stops.

### 4. `impl Renderer`

One big `match` over your catalog, returning the real component:

```rust
impl Renderer for GuiseRenderer {
  fn library(&self) -> &'static dyn Library { crate::library() }

  fn element(&self, ctx: &RenderCtx, node: &Node, window: &mut Window, cx: &mut App)
    -> AnyElement { nodes::element(ctx, node, window, cx) }

  fn preview(&self, node: &Node, doc: &Document, cx: &mut Context<PreviewStore>)
    -> Option<Rc<dyn Any>> { preview::build(node, doc, cx) }
}
```

`element` returns the component and nothing else — the style box, the selection
outline, the drop strips and the entrance replay are already around whatever it
returns.

`preview` is the stateful half. A component that owns a focus handle or a text
buffer cannot be built inside `render`, so `PreviewStore` keeps one entity per
node, rebuilt when the node's props change. What is in the box is yours; the
store holds an `Rc<dyn Any>` and hands it back for `element` to downcast. Return
`None` for a kind the canvas draws itself.

### 5. The surface ratchet

The one part that is not a trait, and the reason the guise catalog has not
silently fallen behind the library.

`libraries/guise.surface` is a checked-in record of every component the pinned
crate defines and every theme preset it offers. It is generated, never
hand-edited:

```sh
cargo run -p tailor-surface        # reads the version Cargo.lock resolves
```

Two tests read it. `coverage` fails unless every component in the surface is
either catalogued or in `EXCLUDED` with a reason; `presets_match_guise` fails
when the preset table drifts. The file's first line carries the version it was
generated from, checked against `Cargo.lock`, so bumping the dependency is two
steps in one commit — change the version, regenerate the file. CI regenerates it
independently and fails on any diff, which catches the case the version line
cannot: a patch release that added a component.

A second provider wants the same thing and gets it the same way. Point
`tailor-surface` at the crate, check the result in, read it from your tests.

## gpui-kit: a library that is described but not drawn

[gpui-kit](https://github.com/longbridge/gpui-kit) is the second provider, and
it exists partly to show where the seams are. `tailor-gpuikit` is a catalog and
a generator; there is no renderer crate.

The reason is gpui. gpui-kit is built on the `gpui-pre` snapshots of Zed's
crates, and Tailor is built on crates.io `gpui 0.2.2`. They are two unrelated
sets of types, so an element built by one cannot be handed to the other's
window, and linking both into one binary is not an option. A library on a
different gpui than Tailor's can be catalogued and generated for — neither
touches gpui — but not instantiated.

Tailor does not stub that with an error. `tailor_render::schematic` draws any
library that has no `Renderer` as labelled cards: containers keep their slots as
real drop targets, so laying a screen out works the same way. What it cannot show
is how a component looks. **Run** and **Export** build the real thing, against
gpui-kit's own gpui, and that is where to check a design.

What a provider on a different gpui needs, beyond the two traits:

- `Library::dependencies()` — the `[dependencies]` lines of the generated
  `Cargo.toml`. gpui-kit re-exports gpui, so it renames itself
  (`gpui = { package = "gpui-kit", version = "0.7" }`) and generated `gpui::`
  paths resolve against the copy it was built with.
- `Generator::application_import`, `application` and `open_window` — gpui-kit
  moved `Application` behind a platform crate and wraps every window in its own
  root, which is what its dialogs mount on.
- `Library::state_style()` — `Signal` (guise's `Signal<T>`, read with `.get(cx)`)
  or `Field`. gpui-kit has no signal type, so a state variable is a plain field
  on the screen; an action assigns it and calls `cx.notify()`.
- `Library::fallback_icon()` and `has_icon()` — gpui-kit ships a subset of
  Lucide, and the picker offers all of it. A name the library lacks lints as a
  warning and generates the fallback rather than a name that does not compile.
- `Library::parents(kind)` — the parts that only exist inside something (a table
  cell in a row, a menu item in a menu). Elsewhere they generate code that does
  not compile, so the linter says so where the design is.

Every library must define a `frame` kind: a document's root is one.

### Choosing one

A project stores its library id (`"library": "gpuikit"`). New projects pick one
on the start screen — the choice is remembered in settings — or through
`create_project`'s `library` argument over MCP. It is fixed at creation:
component kinds do not map across libraries, so there is no switching an open
project.

### State entities

gpui-kit's stateful components are two things each: a state entity that owns the
buffer, selection or value, and an element drawn over it. That is
`Ctor::Stateful("InputState")` (or `StatefulCx` when the state needs no window).
The screen keeps the state as a field, builds it in `new`, and each frame builds
the element over a borrow of it:

```rust
let input = cx.new(|cx| InputState::new(window, cx).placeholder("Email"));
// ... in render:
Input::new(&self.input).cleanable(true)
```

A prop lands on whichever half owns it. `Emit::State("placeholder")` is the
state's and is applied when it is built; `Emit::Method` and `Emit::Flag` are the
element's and are applied every frame. A screen that owns window-built state
takes a `window` in `new`, and `main` passes it in — as does anything that places
it, so the need passes up through every component that contains one.

Three generator hooks cover the components whose state is not `State::new(window,
cx)`: `state` (a select's options, a slider's range), `element_over` and
`support` (top-level items, such as the delegate a table needs). Events are
bound to one variant of the state's event enum — `EventSpec::method` is the
pattern, `InputEvent::Change` — so focus and blur do not run a change handler.

Reading a value is an action's job: `self.input.read(cx).value()`, and completion
in the action editor offers the field.

### Overlays

Popovers, hover cards and dialogs take a trigger slot and content regions written
as closures, the same way guise's tabs do. Sheets and notifications have no
trigger API in gpui-kit, so they are a button that opens one from its click
handler. Menus are an *Items* slot of `menuitem`, `menuseparator`, `menulabel`
and `submenu` nodes on a dropdown button or context menu. On the canvas they are
cards whose slots are drop targets; to see one open, Run.

### What is still out

Every exclusion is a rule with a written reason in
`crates/gpuikit/tests/coverage.rs`, held against the surface, so the list cannot
go stale quietly. What remains falls into three groups:

- **Internals and parts** — test probes, drag and dock scaffolding, and the
  sub-parts a parent's generator already writes (`TableCell` is written by
  `Table`, `MenuItem` by a menu).
- **Window chrome** — `TitleBar`, `WindowBorder`, `WindowControls`. They need a
  borderless window, which is `main`'s to open, not a node's.
- **Not layout** — `AppMenu` reads the menus an application registers with
  `cx.set_menus` at start-up, and the editor's completion and hover popups are
  fed by a language server.

What Tailor cannot do is show any of it live on the canvas. That is the gpui
snapshot mismatch above, and it is structural: closing it means rendering
gpui-kit out of process and streaming its frames in.

### The surface

`libraries/gpuikit.surface` is regenerated with
`cargo run -p tailor-surface -- gpuikit`. Unlike guise, gpui-kit is not in
`Cargo.lock` and cannot be — that would pull its gpui snapshot in beside ours —
so the tool fetches `gpui-component` at an exact version from crates.io. The pin
is in `crates/surface/src/main.rs`; bump it, regenerate, and change the
provider's `version_req` in the same commit.

## What is still guise-shaped

Two things, and they are worth naming rather than glossing:

- **`tailor-render`'s own chrome** is drawn in guise. That is Tailor's UI, not
  what Tailor draws *with* — but it does mean a provider's components sit inside
  boxes styled by guise's theme.
- **Icon fallback and state.** An empty icon prop generates guise's
  `IconName::Circle`, which gpui-kit does not have (its `icon` kind works around
  it), and document state generates guise's `Signal<T>` — hence `signals()`.
- **`tailor-codegen`'s colour expressions** (`expr.rs`) still assume guise's
  three-way colour vocabulary: `impl Into<ColorValue>`, a bare `ColorName`, and
  guise's own `Color`, two of which resolve out of the theme and get hoisted
  into a `let` at the top of `render`. Token *names* already come from
  `token_paths`; the shape does not. It is the next thing that file owes the
  seam.

Neither blocks a second library that spells colours the same way. Both are on
the list.
