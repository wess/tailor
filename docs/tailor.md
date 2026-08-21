# Tailor

Tailor is a visual interface builder for gpui and guise, and it ships in this
repository. You lay out a screen by dragging real components onto a canvas, wire
the state and the actions, and export idiomatic Rust that has no dependency on
Tailor left in it.

```sh
cargo run -p tailor-app                     # open Tailor
cargo run -p tailor-app -- demo.tailor      # open a project
cargo build -p tailor-app                   # the binary is `tailordev`
```

The binary is named `tailordev` so a development build never collides with an
installed `tailor`.

## Installing

Every release attaches **`Tailor.dmg`** — drag it to Applications. The bundle
carries the app as `tailor` and the MCP server as `tailor-mcp` beside it, and it
is signed and notarized when the release was built with a Developer ID; an
unsigned build warns on first launch and opens from the right-click menu.

To build the bundle yourself, on macOS:

```sh
scripts/bundle.sh   # dist/Tailor.app  (CODESIGN_IDENTITY to sign it)
scripts/dmg.sh      # dist/Tailor.dmg
scripts/icon.sh     # regenerate assets/icon.icns from scripts/icon.swift
```

Tailor's version is the workspace version, so it ships on the same tag as the
library — one repository, one version, one set of release notes.

## What it is, and what it is not

The canvas is not a drawing of your interface — it is your interface. A `Button`
on the canvas is a `guise::Button`, reading the same theme, laid out by the same
flexbox. That is the whole design: there is no second rendering path to keep in
step with the real one, so a component cannot look right in the builder and
wrong in the app.

What it is not is a runtime. The `.tailor` file is a design document, not
something your app loads. The output is a Rust file you own.

## The workspace

```
crates/tailor/
├── model/     # the document: catalog, node tree, tokens, state, file format
├── codegen/   # document -> idiomatic guise Rust
├── store/     # project files, recents, editor settings, export
├── render/    # document -> live guise components (the canvas)
├── app/       # the gpui workbench
└── mcp/       # an MCP server over the same document model
```

`model`, `codegen`, and `store` are free of gpui and carry the tests: the
reparent rules, the cycle checks, the undo stack, the generated output, and the
file round-trip are all plain-data logic, and that is where a builder actually
goes wrong.

## The window

Five regions, laid out the way Interface Builder and Android Studio's layout
editor lay theirs out.

| Region | What it is |
| --- | --- |
| **Library** (left) | Every component you can place, searchable, grouped by category, plus the components you built in this project. Drag one onto the canvas, or click it to drop it into the selection. |
| **Outline** | The node tree. Rows are drag sources and drop targets, so it is a second way to restructure a layout when the canvas is too dense to aim at. Named slots — a panel's footer, a button's left section — appear as their own rows. |
| **Canvas** (centre) | The artboard, at the device size you picked, with the document rendered inside it. |
| **Inspector** (right) | Five tabs: Attributes, Size, Style, Connections, Identity. |
| **Problems** (bottom) | What will not generate, and what probably was not meant. |

Every panel resizes — grab the divider beside it — and every panel folds away
from the chevron in its header, leaving a rail you click to bring it back. Sizes
and open/closed state are written to the settings file, so the layout you left
is the layout you come back to. The inspector's sections fold individually and
stay folded across selections. ⌥⌘1 through ⌥⌘4 toggle the four panels from the
keyboard.

### Canvas modes

- **Design** — real components; a click selects rather than activates.
- **Blueprint** — outlines and names only. Useful when the content is dense
  enough to hide the structure.
- **Split** — design on the left, the generated Rust on the right, regenerated
  on every edit.
- **Preview** — components go live and the canvas stops intercepting clicks.

There is no magnification. gpui 0.2.2 has no transform for an arbitrary element
tree, so rather than fake a zoom that would only scale some of the pixels,
Tailor gives you device presets, a rotate button, and a canvas that scrolls.

### The live window

`View → Open Live Window` (⌘⇧L) opens a second OS window showing the document at
its real device size, with no canvas chrome and every component interactive. It
updates on the same edit that updates the canvas. Leave it on a second display
and watch the app take shape while you work.

`View → Developer Tools` (⌥⌘I) opens guise's own inspector along the bottom of
that window — the Elements tree, the box model, the resolved styles with the
source line each one came from, plus Layers, Timelines and the rest. It opens
the live window first if it is closed, and ⌥⌘I again closes it.

It lives there rather than on the canvas because the live window renders the
document *alone*: the tree you get is your interface and nothing else, where the
same inspector on the workbench would show your design nested inside Tailor's
own panels. It is closed until you ask for it — the recorder behind the Elements
tree only runs while an inspector is alive, so a closed one costs the document
nothing per frame — and the inspector's dock buttons move it to the right edge.
**General → Inspect the live window** makes it open that way every time.

Right-clicking the design gives you **Inspect element**, which does what the
same item does in a browser: it selects the deepest component under the pointer,
scrolls the Elements tree to it, and draws a box around it in the window so you
can see what you got. It opens the inspector first if it was closed — the tree
is recorded by the frames that follow, so the pick waits for one rather than
answering from an empty tree.

The inspector takes its room from the *window*, not from the design: opening it
makes the window taller (or wider, docked right) and leaves the document at its
device size. Squeezing the design to make space would defeat the one thing this
window is for.

## Selecting and resizing

Selecting a node puts an outline and eight knobs around it, on the edges rather
than inside them. Drag a knob to resize; drag an absolutely placed node's body
to move it. Both work in deltas from where the drag started, so a frame the
pointer outran does not drift, and both are a single undo step no matter how
many frames they took.

A component that carries its own pixel width and height — an image, a chart —
is resized through those props, the way Interface Builder resizes a view's
frame. Everything else is resized through the box around it, which is exactly
what the generated code will say.

With snapping on, a drag catches on the grid and on its siblings' edges and
centres, and draws a guide wherever it caught. Arrow keys nudge by one, shift-
arrow by the grid; in a flow container, where there is no x to nudge, up and
down reorder instead.

## Right-click

Right-clicking a component on the canvas — or a row in the outline — selects it
and opens a menu on it. Selecting first is deliberate: acting on something you
have not visibly selected is how a builder deletes the wrong node.

- Rename…, Duplicate, Cut / Copy / Paste
- Embed in frame / stack / card / scroll area, Unwrap
- Move up, Move down
- **Extract to a component** — lift the selection into a new component document
  and leave a reference behind. The move that turns a screen you have been
  pushing around into something reusable. The selection has to share one parent,
  because that is where the reference goes back.
- Lock / Unlock, Show / Hide, Select parent
- Delete

Right-clicking the canvas itself offers Paste, Select all, and a new screen or
component.

Every other surface has one too, and each acts on the row under the pointer
rather than on whatever happened to be selected:

| Surface | What it offers |
| --- | --- |
| A document tab | Rename…, Duplicate, add a screen or component, open the live window (screens only), Delete |
| A component in the Library | Insert into the selection, insert at the top level |
| One of *your* components | the same, plus Edit, Rename…, Duplicate, Delete |
| A row in Problems | Reveal — open the document and select the node — and copy the message |
| The generated code | Copy the file, export the project, hide the code pane |
| A project in Recent, on the start screen | Open, reveal in Finder, copy the path, remove from the list |
| The live window | Inspect element, show or hide the inspector |

Nothing in any of them is a command that exists only there. **Rename…** on a tab
opens the document and puts the cursor in the name field the inspector already
had; the menu is a shortcut to the app, not a second way to drive it.

`View → Show Layout Bounds` (⇧⌘B) outlines every node rather than only the
selected one, for when you need to see the boxes rather than the content.

## Layout: two modes, per container

Every container chooses how it arranges its children, in the Size inspector:

- **Flow** — gpui's flexbox. Direction, gap, alignment, wrap. This is what maps
  one-to-one onto real guise code, and it is the default.
- **Free form** — children carry an x and a y. The container generates as
  `relative()` and its children as `absolute().left(..).top(..)`.

Both generate real code. Flow is what you want for anything that has to reflow;
free form is what you want for overlays, badges, and pinned decoration. Flip the
selected container with ⇧⌘G, and if you work mostly free form, turn on **New
frames are free form** in Settings so every frame you drop starts that way.

## Settings

⌘, opens preferences, built out of guise's own `SettingsView`, `SettingsSection`
and `SettingsRow` — the settings screen the library ships is the settings screen
its builder uses.

| Page | What is on it |
| --- | --- |
| **General** | Autosave, whether the live window opens with the inspector, what new projects generate as, the start screen's scheme |
| **Canvas** | Mode, show the grid, grid spacing, snap to grid, snap to objects, nudge distance, new frames are free form, show layout bounds |
| **Panels** | Panel sizes, and a reset for the whole layout |
| **About** | Version, and where the settings file lives |

The canvas options are also on the Arrange menu, where a layout program puts
them: **Flow / Free Form** (⇧⌘G) flips the selected container, and **Snap to
Grid**, **Snap to Objects** and **New Frames Are Free Form** toggle the
preferences without opening anything.

Snapping is two independent things, because wanting one without the other is the
normal case: the grid catches a drag on the spacing you chose, and objects catch
it on a sibling's edge or centre and draw a guide where it caught.

## Slots

Children live in *slots*, not in one list. Most components have only the default
`children` slot, but the ones you actually build a screen out of have named
regions:

| Component | Slots |
| --- | --- |
| `Panel` | children, icon, action, footer |
| `Button` | left section, right section |
| `AppShell` | children, header, navbar, aside, footer |
| `SplitPanel` | first, second |
| `StatusBar` | left, center, right |
| `Tabs` / `Accordion` | one per tab or section, from the labels you typed |

Drop into a slot from the canvas or from the outline. An empty slot draws a
dashed placeholder so there is something to aim at.

## Five components the canvas draws itself

`Tabs`, `Accordion`, `SplitPanel`, `AppShell`, and `Carousel` take their regions
as `'static` closures, which a designer cannot reach into. Tailor draws those
five from the theme instead. That is not a downgrade: it is what lets you click
a tab to reveal the slot behind it and drop into it. Generated code uses the
real component.

## State and actions

A screen you can only look at is a mockup. Two tables in the Connections
inspector make it a component you can wire up:

- **State variables** — a name, a type (text, bool, int, float, items) and a
  starting value. Each becomes a `Signal<T>` field on the generated screen. Any
  text, number, or boolean prop can be *bound* to one instead of holding a
  literal; the generator emits `self.<var>.get(cx)`, and the canvas shows the
  variable's starting value so you can see what the first frame will look like.
- **Actions** — a name and an optional body. Each becomes a method. Connect a
  component's event to one and the generator wires `cx.listener` (for builder
  components) or `cx.subscribe` (for entity components) to it.

## What gets generated

The document decides the shape, not a setting. Anything that owns state — a text
field, a picker, a state variable, an action — has to be a `Render` entity, and
everything else can be the `RenderOnce` builder you would have written by hand.

A screen with a field, a bound prop, and a connected button comes out as:

```rust
//! MainScreen — generated by Tailor from Demo. Edit the design and
//! regenerate, or take this file and own it; it has no dependency on Tailor.

use gpui::prelude::*;
use gpui::{Entity, Window, div, px};
use guise::prelude::*;

pub struct MainScreen {
    email: Entity<TextInput>,
    pub query: Signal<String>,
}

impl MainScreen {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let email = cx.new(|cx| {
            TextInput::new(cx)
                .placeholder("you@example.com")
        });
        MainScreen {
            email,
            query: Signal::new(cx, "".to_string()),
        }
    }

    pub fn submit(&mut self, cx: &mut Context<Self>) {
        // TODO
        let _ = cx;
    }
}

impl Render for MainScreen {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let violet_6 = theme(cx).color(ColorName::Violet, 6).hsla();
        div()
            .flex()
            .flex_col()
            .gap(px(16.))
            .p(px(24.))
            .child(Title::new("Sign in"))
            .child(self.email.clone())
            .child(
                div()
                    .w_full()
                    .bg(violet_6)
                    .child(
                        Button::new("node-4", "Continue")
                            .on_click(cx.listener(|this, _event, _window, cx| this.submit(cx)))
                    )
            )
    }
}
```

Note the hoisted colour. guise's own convention is that a `theme(cx)` read must
not be held across a `cx.listener`, so every resolved colour is lifted into a
`let` at the top of `render` rather than resolved inline.

### Two flavours

- **plain** — builder calls and gpui `Styled` methods. Always compiles, reads
  like the rest of an app.
- **macros** — the same layout through `style! { … }` blocks.

Switch between them in the code panel or the Generator section of the inspector.

### Export

`File → Export Code…` (⌘E) writes a directory:

```
src/ui/main_screen.rs     # one file per screen and component
src/ui/mod.rs             # the module that ties them together
src/main.rs               # a window on the first screen
src/theme.rs              # the theme you designed against
Cargo.toml                # gpui + guise-ui, and the release profile
```

Every file is written whole; nothing is merged. An export is a snapshot of the
design, and quietly merging into a file someone has since edited by hand is how
a builder eats your work.

## Components you build

A document can be a **screen** (a `Render` entity) or a **component** (a
`RenderOnce` builder). Components appear at the top of the Library and can be
placed inside any other document — a `@Name` node that renders inline on the
canvas and generates as `Name::new()`. A component that would contain itself is
refused when you drop it, and again when the file is loaded.

## Editing

| Command | Shortcut |
| --- | --- |
| Undo / redo | ⌘Z / ⇧⌘Z |
| Duplicate | ⌘D |
| Delete | ⌫ |
| Select parent | Esc |
| Rename | ↵ |
| Embed in frame | ⇧⌘E |
| Unwrap | ⇧⌘U |
| Design / Blueprint / Split / Preview | ⌘1 … ⌘4 |
| Live window | ⇧⌘L |
| Developer tools | ⌥⌘I |
| Library / Outline / Inspector / Problems | ⌥⌘1 … ⌥⌘4 |
| Show grid / Snap / Layout bounds | ⌘' / ⇧⌘' / ⇧⌘B |
| Nudge / nudge by the grid | Arrows / ⇧Arrows |
| Save / Save as / Export | ⌘S / ⇧⌘S / ⌘E |

**Embed in…** wraps the selection in a new container in place, and **Unwrap**
lifts a container's children into its parent and deletes it. Between them they
are the fastest way to restructure a layout without dragging anything.

**Align** and **Distribute** do arithmetic on x/y inside an absolute container.
Inside a flow container there is nothing to move, so they set the *container's*
alignment instead — which is what you meant.

Undo is whole-project snapshots rather than an inverse-operation log. A project
is a few hundred nodes of plain data, so a clone costs microseconds and every
operation is correct by construction. Typing into a field collapses into one
undo step rather than one per keystroke.

## Problems

The lint pass runs on every edit and reports what the canvas cannot show you:

- a prop bound to a variable you renamed or deleted
- an event pointing at an action that is gone
- a component reference to a document that no longer exists
- a stateful component inside `Tabs`, `Accordion`, or `SplitPanel` — their
  regions are `'static` closures, so extract that part into its own component
- two documents that generate the same type name
- a button with no label, an image with no source, an empty container

Clicking a row opens the document and selects the node.

## What runs where

Everything an edit causes that is not drawing happens off the main thread, on
gpui's background executor — the same arrangement Zed uses for its own derived
state.

- **The project is shared, not copied.** It lives behind an `Arc`, so an undo
  snapshot and the canvas's view of it are refcount bumps rather than deep
  copies. Editing goes through `Arc::make_mut`, which pays for exactly one copy
  per edit — the one undo needs anyway — instead of one per commit *and* one per
  frame. An idle or hovering frame now copies nothing at all.
- **Regenerating the Rust and running the lint pass** happen on a background
  thread against that shared project, debounced by 120 ms, and are applied on
  the main thread only if no newer edit has landed. Every refresh bumps a
  revision; a result carrying an old one is dropped. The `Task` handle lives in
  the workbench, so replacing it cancels work nobody is waiting for.
- **Autosave** is debounced by 600 ms and both serializes and writes in the
  background, so a burst of typing costs one file write rather than one per
  keystroke.
- **Export** generates every document and writes the crate in the background;
  the window keeps drawing while it runs.
- **The file watcher** stats, reads, and parses off the main thread too.

Measured on a debug build of a 3,744-node project, one keystroke used to cost
about 7.9 ms on the main thread — half a frame, before drawing anything. It now
costs about 2.4 ms, which is the copy undo requires and nothing else.

The exception is the entity cache: a text field or a picker on the canvas is a
gpui entity, and entities can only be built on the main thread. It is also the
cheap part.

## The file format

`.tailor` is JSON, and it is meant to be read in a diff. Defaults are dropped on
save, so a node that was placed and never styled writes just its id and its
kind:

```json
{ "id": 4, "kind": "button", "props": { "label": { "t": "text", "v": "Save" } } }
```

The `format` field is a version. A file from a newer Tailor is refused rather
than half-read; a hand-edited file is repaired on load (unreachable nodes
dropped, dangling slot references cleaned up, the id counter re-pointed).

## The theme

Tailor wears the project's theme. guise reads its colours from an app-wide
global at the moment a component paints, not at the moment you build it, so
there is no way to scope a second theme to the canvas without it leaking.
Rather than fight that, switching the project to light switches the editor to
light — which is also the most honest preview a builder can give you. The panels
keep a neutral graphite surface ramp so they never read as part of the design.

## Adding a component to the catalog

The catalog in `tailor-model` is read by four consumers: the Library lists it,
the inspector builds a control per prop, the renderer builds the real component,
and the generator prints it.

1. Add an entry to the right file under `crates/tailor/model/src/catalog/`,
   using the `comp!` macro: seven positional facts, then any field that differs
   from the defaults.
2. Add an arm to `crates/tailor/render/src/nodes/build.rs`.
3. That is all — unless the constructor is not one chained call, in which case
   add an arm to `Emitter::special` in `crates/tailor/codegen/src/node.rs` too.

`PropSpec::emit` is what keeps the three in step: `Emit::Method("size")`
generates `.size(..)`, `Emit::Flag("fill")` generates `.fill()` when the bool is
true, `Emit::None` means the constructor already consumed it.

## The MCP server

`tailor-mcp` is an MCP server over the same document model, so an agent can
build and generate interfaces without opening the app.

```sh
cargo build -p tailor-mcp
claude mcp add tailor -- /path/to/tailor-mcp
```

It works on `.tailor` files and saves after every change. Tailor watches the
file it has open, so a screen built over MCP appears on the canvas a moment
later, with nothing wired between the two processes. Unsaved work always wins:
if the file changes on disk while there are edits in the window that are not in
it, the reload is refused and said out loud rather than one of the two being
quietly picked.

| Tool | What it does |
| --- | --- |
| `open_project` / `create_project` | Open or make a `.tailor` file |
| `overview` | Documents, state, actions, theme, problem counts |
| `outline` | The node tree, as indented text with node ids |
| `catalog` / `component` | What can be placed, and one component's exact props |
| `add_node` / `set_node` / `move_node` / `remove_node` | Edit the tree |
| `add_document` | Add a screen or a component |
| `add_state` / `add_action` | The state and actions half of a document |
| `bind_prop` / `connect_event` | Wire a prop to a variable, an event to an action |
| `set_theme` | Scheme, primary colour, radius, font |
| `generate_code` | The guise Rust for one document |
| `export_code` | Write the whole crate |
| `problems` | The lint pass |

The server reads and writes wherever it is told, the same as the app it pairs
with — it is a document tool, not a sandbox. It will not create a project over
an existing file, and an export only ever writes below the directory you name.

Props are plain JSON, resolved through the catalog: `{"variant": "outline",
"size": "lg", "color": "grape", "full_width": true}`. A wrong key answers with
what the component actually takes, so `component` is worth calling once rather
than guessing four times. `{"bind": "query"}` in place of a value binds the prop
to a state variable.

## Scaffolding from the shell

```sh
tailordev --template dashboard out.tailor   # empty | sign in | dashboard | settings
```

Writes a project and exits, so a script can scaffold one without opening a
window.
