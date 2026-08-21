# Tailor: components and slots

Tailor's Library is guise's component set, one entry per component, grouped the
way the docs group them. There are **101** of them.

## The catalog

| Category | Count | Some of what is in it |
| --- | --- | --- |
| Layout | 20 | Frame, Absolute frame, Stack, Group, Center, Grid, Container, Card, Paper, Panel, Scroll area, App shell, Split panel, Expanded |
| Typography | 9 | Text, Title, Anchor, Code, Kbd, Mark, Blockquote, Markdown, Spoiler |
| Controls | 12 | Button, Action icon, Close button, Copy button, Badge, Chip, Icon, Theme icon, Indicator, Rating, WebView |
| Inputs | 27 | Text input, Text area, Number, Password, PIN, Select, Combobox, Autocomplete, Checkbox, Switch, Radio, Segmented, Slider, Range slider, Colour, Tags, Date, Time, Calendar, File, Dropzone, Transfer, Field, Editor, Markdown editor |
| Data | 10 | Avatar, Avatar group, List, Table, Tabs, Accordion, Tab bar, Timeline, Tree view, Carousel |
| Feedback | 10 | Alert, Notification, Loader, Progress, Ring progress, Skeleton, Modal, Drawer, Tooltip, Loading overlay |
| Navigation | 6 | Breadcrumbs, Nav link, Stepper, Pagination, Status bar, Navigation menu |
| Charts | 6 | Sparkline, Line, Area, Bar, Pie, Scatter |
| Media | 1 | Image |

The Library searches across names and blurbs, and the category pills filter it.
Drag an entry onto the canvas, or click it to drop it into the selection —
click is faster once you know where a thing is going, because it lands inside
whatever is selected rather than wherever you let go.

Right-clicking an entry offers **Insert into the selection** and **Insert at the
top level**.

## Two kinds of component

The catalog marks which is which, and it decides the shape of the generated
file:

- **Stateless builders.** A `Button` is a value: you hand it a label, a variant
  and a click handler, and it draws. Most of the catalog is this.
- **Stateful entities.** A `TextInput` owns a buffer, a caret and an IME. It is
  a gpui entity, created with `cx.new`, and it emits events rather than taking
  handlers.

You do not pick between them; the component is one or the other. What it changes
for you is that a document holding *any* entity generates as a `Render` entity
with a constructor, rather than a `RenderOnce` builder. See
[what gets generated](tailorcodegen.md).

The entities are every text-ish input, every picker, the overlays that own
open-state, and the big surfaces — Editor, Markdown editor, Tabs, Accordion,
Pagination, Carousel, Table view, Tree view, Tab bar, Split panel.

## Slots

Children live in *slots*, not in one list. Most components have only the default
`children` slot, but the ones you build a screen out of have named regions:

| Component | Slots |
| --- | --- |
| `AppShell` | children, header, navbar, aside, footer |
| `Panel` | children, icon, action, footer |
| `Button` | left section, right section |
| `SplitPanel` | first, second |
| `StatusBar` | left, center, right |
| `Field` | children |
| `Tabs` / `Accordion` | one per tab or section, from the labels you typed |

Drop into a slot from the canvas or from the Outline. An empty slot draws a
dashed placeholder so there is something to aim at, and the Outline shows slots
as their own rows — which is the easier target when a region is a 64px strip.

Some slots are **single**: a shell has one header, not a list of them. Dropping
a second node into a single slot replaces what was there.

## The five containers Tailor draws itself

`Tabs`, `Accordion`, `SplitPanel`, `AppShell` and `Carousel` take their regions
as `'static` closures. A closure is opaque — a designer cannot drop a node into
one — so Tailor draws those five from the theme instead of instantiating them.

That is not a downgrade, it is the point: drawing them is what lets you click a
tab to reveal the slot behind it and drop into it. A real `Tabs` would show you
one panel and hide the rest behind a closure you cannot open.

**Generated code uses the real component.** The drawing is a canvas affordance
and never leaves the canvas.

One consequence, and the Problems panel will tell you about it: a *stateful*
component inside one of these five is a problem. Their regions are `'static`, so
a `TextInput` in an `AppShell` header cannot be a field of the screen — it would
have to be created inside the closure, on every frame. Extract that part into
its own component, which generates its own entity, and place that instead.

Event handlers inside those regions are fine — Tailor routes them through a weak
handle rather than a borrowed context, which is what a hand-written host does.

## Components you build

A document is either a **screen** (generates a `Render` entity) or a
**component** (generates a `RenderOnce` builder). **File → New Component**, or
the *Add a component* item on a tab's right-click menu.

Your components appear at the top of the Library under **This project**. Place
one and you get a `@Name` node: it renders inline on the canvas, so the screen
looks like the screen, and it generates as `Name::new()`.

A placed component is a **reference, not a copy**. Change the component and
every placement changes.

Right-clicking one in the Library offers Insert, **Edit the component** (opens
its document), Rename…, Duplicate and Delete.

### Extract to a component

Select some nodes on the canvas, right-click, **Extract to a component…**.
Tailor lifts them into a new component document and leaves a reference behind in
their place.

The selection has to share one parent, because that is where the reference goes
back.

### Cycles

A component that would contain itself is refused when you drop it, and again
when the file is loaded — a hand-edited `.tailor` with a loop in it comes back
repaired rather than hanging the app that opened it.

That check is on *names*, because a `@Name` reference carries a name. It is also
why duplicating a component gives the copy a new name rather than a new id
alone: two documents called the same thing would generate the same Rust type,
which the Problems panel reports as an error.

## Props

The inspector's **Attributes** tab is generated from the catalog, one row per
prop, with the control the prop's type asks for: a text field, a number, a size
or variant picker, a colour swatch, an icon picker with search, a list editor.

Defaults are shown greyed. Tailor does not restate a default in the generated
code — the file is what you would have written, not a dump of every prop — so a
prop you never touched costs nothing in the output.

Any text, number or boolean prop can be **bound** to a state variable instead of
holding a literal. See [state, bindings and actions](tailorstate.md).

## Adding a component to the catalog

If you are working on Tailor itself: the catalog is the single source of truth,
and adding a component is two edits that have to happen together.

1. One `comp!` entry in `crates/tailor/model/src/catalog/`: the kind, the Rust
   type, the category, the blurb, the constructor shape, the props with their
   types and defaults, the slots, the events.
2. One arm in `crates/tailor/render/src/nodes/build.rs`, which turns a node of
   that kind into a live guise component for the canvas.

`PropSpec::emit` decides what the generator prints for each prop — a method
call, a bare flag, or something custom. Editing the catalog without the renderer
(or the other way round) is how a canvas and an export drift apart, which is the
one failure this design exists to prevent.
