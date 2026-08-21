# Tutorial: building an app in Tailor

This walks the whole distance — from an empty project to a signed-off Rust
binary you own — by building **Roster**, a small team directory. It is the
companion to the [library tutorial](tutorial.md): that one writes guise by hand,
this one draws it and takes the Rust.

Every code block here is output Tailor actually produced for the project this
tutorial builds. The finished export compiles and runs.

## What you're building

A single screen with an app shell around it:

- a **navbar** with three links,
- a **header** with a title, a count badge, and an *Add person* button,
- a **filter row** — a search field, a role picker, an *Active only* switch,
- a **card** holding rows of people, where the row is a component you build once
  and place three times,
- two pieces of **state** the controls are bound to, and one **action** the
  button calls.

By the end you will have `src/ui/people.rs`, `src/ui/person_row.rs`, a `main.rs`
that opens a window, and a `Cargo.toml` — a project that runs on its own and
has no dependency on Tailor left in it.

## Before you start

Get Tailor: every [release](https://github.com/wess/guise/releases) attaches
`Tailor.dmg`. From a checkout, `cargo run -p tailor-app`.

You do not need to know guise to follow this. You do need a Rust toolchain at
the end, to build what comes out.

## 1. A new project

**File → New Project** (⌘N). Tailor opens with one screen, called `MainScreen`.

Rename it: right-click its tab and choose **Rename…**, which opens the document
and puts the cursor in the Name field of the inspector. Call it `People`.

The name matters more than it looks. It is the Rust type this document
generates — the inspector says so, under the field: *Generates as `People`*. A
document called "people screen" would generate `PeopleScreen`; one called
"2fa" would generate `X2fa`, because a type cannot start with a digit.

While you are in the inspector, set the canvas size. The Size section has
device presets across the toolbar — pick **desktop**, or type 1280 × 800.

## 2. The shell

Open the **Library** (⌥⌘1 if it is folded away) and find **App shell** under
Layout. Drag it onto the canvas.

It lands as a child of the document's root Frame, and it looks like almost
nothing: a header strip, a navbar column, and an empty middle. Those are its
*slots* — named regions that take their own children:

| Slot | What goes in it |
| --- | --- |
| `header` | one node, across the top |
| `navbar` | one node, down the left |
| `aside` | one node, down the right |
| `footer` | one node, across the bottom |
| `children` | the content, in the middle |

The Outline shows them as their own rows, so you can always see which region a
node is in even when the canvas is dense.

Select the shell and set two things in the inspector's Attributes tab: **Navbar
width** 220, **Header height** 64.

### Make the root fill the window

Select the document's root Frame — the row at the very top of the Outline — and
in the **Size** tab set width and height to **Full**, gap 0, padding 0.

This is worth doing deliberately. A new document's root is a plain Frame with a
comfortable default gap and padding, which is right for a card or a form and
wrong for a screen: a screen root should fill its window and let the shell
inside it do the spacing. Skip this and the app runs with a margin of dead
space around it, which is exactly what it will look like when you export.

## 3. The header

Drop a **Group** into the shell's `header` slot — aim at the header strip on the
canvas, or drag onto the `header:` row in the Outline, which is easier when the
target is thin.

A Group is a horizontal row with a themed gap. Into it, in order:

1. **Title** — set Content to `Roster`, Order to 4.
2. **Badge** — Label `12 people`, Color blue.
3. **Spacer** — flexible space that pushes its siblings apart.
4. **Button** — Label `Add person`.

The Spacer is the whole trick of a title bar: everything before it sits left,
everything after it sits right, and the gap between them is whatever is left
over. Set the Group's Align to **Center** so the row is vertically centred in
the 64px strip.

## 4. The navbar

Drop a **Stack** into `navbar` — a vertical stack with a themed gap — with
padding 12 and gap Sm. Then three **Nav link**s inside it: `People`, `Teams`,
`Settings`. Give each an icon (`users`, `layers`, `settings`) and mark the first
one **Active**.

Icons come from Lucide, and the icon picker searches by name. Nothing to install
— the font ships inside guise.

## 5. The filter row

Into the shell's `children` slot, a **Stack** with padding 24 and gap Lg. That
Stack is the body of the screen; everything else goes inside it.

First child: a **Group** with gap Sm and Align **End**, holding three inputs.
Align End matters here — the fields have labels above them and the switch does
not, so aligning on the bottom edge lines up the controls rather than the tops
of their boxes.

1. **Text input** — Label `Search`, Placeholder `Name or role`.
2. **Select** — Label `Role`, Data `Everyone`, `Engineering`, `Design`,
   `Support`.
3. **Switch** — Label `Active only`, Checked on.

Three of these own state, and the Outline marks them: a text input, a select and
a switch keep a value between frames. That has a consequence you will see in
section 9 — it decides the shape of the Rust that comes out.

## 6. A component of your own

A person row is the same shape repeated, so build it once.

**File → New Component** — or right-click a tab and choose *Add a component*.
Name it `PersonRow`.

A component document generates a `RenderOnce` builder rather than a screen. Set
its root Frame to width Full, gap 0, padding 0 — a row is spaced by whatever
places it, so it should carry no padding of its own.

Inside, a **Group** with Align Center and gap Md:

1. **Avatar** — Initials `AW`.
2. **Stack** with gap Xs, holding two **Text**s: `Ada Whitfield` at weight
   Medium, and `Engineering` with Dimmed on and size Sm.
3. **Spacer**.
4. **Badge** — Label `Active`, Color green.

That is the component. Here is the entire file it generates:

```rust
//! PersonRow — generated by Tailor from Roster. Edit the design and
//! regenerate, or take this file and own it; it has no dependency on Tailor.

use gpui::prelude::*;
use gpui::{App, FontWeight, Window, div};
use guise::prelude::*;

#[derive(IntoElement, Default)]
pub struct PersonRow;

impl PersonRow {
    pub fn new() -> Self {
        PersonRow
    }
}

impl RenderOnce for PersonRow {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w_full()
            .child(
                Group::new()
                    .align(Align::Center)
                    .child(Avatar::new("AW"))
                    .child(
                        Stack::new()
                            .gap(Size::Xs)
                            .child(
                                Text::new("Ada Whitfield")
                                    .weight(FontWeight::MEDIUM)
                            )
                            .child(
                                Text::new("Engineering")
                                    .size(Size::Sm)
                                    .dimmed()
                            )
                    )
                    .child(
                        div()
                            .flex_grow()
                            .flex()
                            .flex_col()
                    )
                    .child(
                        Badge::new("Active")
                            .color(ColorName::Green)
                    )
            )
    }
}
```

Read that against what you dragged. `Group` with `.align(Align::Center)`, the
avatar, the two texts, the spacer as a `flex_grow` div, the badge. Nothing is
abbreviated and nothing is generated that you did not place.

Note what is *missing*: the badge's variant. You left it on `light`, which is
the default, and Tailor does not restate defaults — the file is what you would
have written, not an exhaustive dump of every prop.

### Extract, when you did not plan ahead

Building the component first is the tidy way round. The usual way round is
realising halfway through that you have built the same row twice.

Select the nodes, right-click, **Extract to a component…**. Tailor lifts them
into a new component document and leaves a reference behind in their place. The
selection has to share one parent — that is where the reference goes back.

## 7. Placing it

Back on `People`. Under the filter Group, add a **Card** with padding Lg and
Border on, and a **Stack** with gap Sm inside it.

Now look at the Library's **This project** section: `PersonRow` is in it. Drag
it into the Stack three times.

A placed component is a reference, not a copy. Change `PersonRow` and all three
change. The Outline shows them as `PersonRow [@PersonRow]` — the `@` is how a
reference is written in the file, and how the generator knows to emit
`PersonRow::new()` rather than inline the row's contents.

Tailor will not let you place a component inside itself, or inside anything that
would make a cycle — `PersonRow` cannot contain a `PersonRow`, and if two
components each place the other, the second placement is refused rather than
accepted and left to hang the app that opens the file.

Your Outline should now read:

```
#1 Screen [frame]
  #2 App shell [appshell]
    #12 Body [stack]
      #13 Filters [group]
        #14 Search [textinput]
        #15 Select [select]
        #16 Switch [switch]
      #17 Card [card]
        #18 Stack [stack]
          #19 PersonRow [@PersonRow]
          #20 PersonRow [@PersonRow]
          #21 PersonRow [@PersonRow]
    header:
      #7 Title bar [group]
        #8 Title [title]
        #9 Badge [badge]
        #10 Spacer [spacer]
        #11 Button [button]
    navbar:
      #3 Stack [stack]
        #4 Nav link [navlink]
        #5 Nav link [navlink]
        #6 Nav link [navlink]
```

## 8. State

A design that only draws is a mockup. Two variables make this one an app.

Open the inspector's **Connections** tab with nothing selected — that is the
document's own inspector — and add two state variables:

| Name | Type | Initial |
| --- | --- | --- |
| `query` | text | *(empty)* |
| `only_active` | bool | `true` |

Each becomes a `Signal<T>` field on the generated type. A `Signal` is guise's
reactive cell: read it in `render` and the component redraws when it changes.

### Binding

Select the search field, and in **Connections** bind its **Value** prop to
`query`. Select the switch and bind **Checked** to `only_active`.

A binding is two-way, and guise has two shapes for it depending on what it is
binding to. You will see both in the generated file, and the difference is worth
understanding because it is the difference between the two kinds of component in
the whole library:

- A **text input owns its state** — it has a buffer, a caret, an IME. It is a
  gpui entity, and it binds with a call: `TextInput::bind(&search, &query, cx)`.
- A **switch owns nothing** — it is handed a value and an on-change. It binds in
  the builder chain: `.bind(self.only_active.binding())`.

Either way, typing writes to the signal and setting the signal updates the
control.

## 9. An action

In the same document inspector, add an action called `add_person`. Then select
the *Add person* button, and in **Connections** wire its **Click** event to it.

An action generates as a method with a `// TODO` body. Tailor never runs your
code — it places a method where the handler belongs so the file is a starting
point and not a stub you have to re-wire.

## 10. Reading what came out

Switch the canvas to **Split** (⌘3). The right half is the generated Rust,
regenerated on every edit. Drag something and watch the file change.

Here is what `People` generates, top to bottom:

```rust
pub struct People {
    pub search: Entity<TextInput>,
    pub select: Entity<Select>,
    pub query: Signal<String>,
    pub only_active: Signal<bool>,
}

impl People {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let query = Signal::new(cx, "".to_string());
        let only_active = Signal::new(cx, true);
        let search = cx.new(|cx| {
            TextInput::new(cx)
                .placeholder("Name or role")
                .label("Search")
        });
        let select = cx.new(|cx| {
            Select::new(cx)
                .data(["Everyone", "Engineering", "Design", "Support"])
                .label("Role")
        });
        TextInput::bind(&search, &query, cx);
        People {
            search,
            select,
            query,
            only_active,
        }
    }

    pub fn add_person(&mut self, cx: &mut Context<Self>) {
        // TODO
        let _ = cx;
    }
}
```

Four things to notice.

**The document decided the shape.** `People` holds state, so it generates a
`Render` entity with a constructor. `PersonRow` holds none, so it generated a
`RenderOnce` builder with `#[derive(IntoElement, Default)]`. You did not choose
between them; what you placed did. Put a text input in `PersonRow` and it will
be promoted to an entity, and the Problems panel will tell you why.

**Order is not arbitrary.** The signals are built first because the fields built
after them read them, and the binding is made last because it needs both sides
to exist. A field that captures another field is built after it, for the same
reason.

**The fields are public.** `search`, `query` — these handles are how a host
reads a value or drives the control later. `self.query.get(cx)` from anywhere in
your own code.

**The binding is not in the chain.** `TextInput::new(cx)` sets the placeholder
and the label but not the value, because `TextInput::bind` sets it — emitting
both would have them fighting over the same value.

And in `render`, the switch's binding, in the other shape:

```rust
Switch::new("node-16")
    .bind(self.only_active.binding())
    .label("Active only")
```

## 11. Problems

The **Problems** panel (⌥⌘4) runs a lint over the document on every edit.

It catches what will not generate — a binding to a variable you deleted, a
component reference that no longer resolves, a document whose name collides with
a guise component — and what probably was not meant: an event wired to no
action, a container with nothing in it, a node pushed outside its parent.

Click a row to select the node it is about. Right-click it for **Reveal**, or to
copy the message.

A clean Problems panel is not a promise that your app is right. It is a promise
that the file will generate and the generated file will compile.

## 12. Watching it run

⌘⇧L opens the **live window**: the document at its real device size, no canvas
chrome, every component interactive. Type in the search field. Toggle the
switch. It updates on the same edit that updates the canvas, so you can leave it
on a second display while you work.

When something looks wrong and you cannot see why, right-click the design and
choose **Inspect element**. That opens guise's own inspector docked under the
window and selects the deepest component under your pointer, scrolls its tree to
it, and boxes it in the window:

```
▾ Card
    Text        size: sm, dimmed
    Title       order: 2
    Sparkline
```

The Styles pane beside it shows the resolved declarations and the source line
each came from — `text.rs:76:14` — which is the fastest way to find out why a
thing is the colour it is.

## 13. Export

**File → Export Code** (⌘E). Pick a directory. Tailor writes:

```
roster/
├── Cargo.toml
└── src/
    ├── main.rs
    ├── theme.rs
    └── ui/
        ├── mod.rs
        ├── people.rs
        └── person_row.rs
```

The `mod.rs` ties them together:

```rust
//! The Roster interface, generated by Tailor.

mod people;
mod person_row;

pub use people::People;
pub use person_row::PersonRow;
```

`main.rs` is a real entry point — it installs the theme you chose in the
document, opens a window at the canvas size, and shows the screen:

```rust
fn main() {
    Application::new().run(|cx: &mut gpui::App| {
        theme::build().init(cx);

        let bounds = Bounds::centered(None, size(px(1280.0), px(800.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Roster".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(ui::People::new),
        )
        .unwrap();
        cx.activate(true);
    });
}
```

And the manifest depends on the library, not on the builder:

```toml
[dependencies]
gpui = "0.2.2"
guise-ui = "1"
```

## 14. Run it

```sh
cd roster
cargo run
```

The window opens: sidebar, header, filter row, three rows in a card. Type in the
search field — the `query` signal is following you. Click the switch — so is
`only_active`.

You will get one warning:

```
warning: field `query` is never read
```

That is true and it is the point: you declared state and have not used it yet.
Filtering the list on `query` is your first edit to the file — and it is *your*
file now.

## 15. Owning it, and going back

Two directions from here, and Tailor is fine with either.

**Take the file.** Delete the `.tailor`, keep the Rust. Nothing in the output
references Tailor; it is guise and gpui. This is the normal ending for a screen
that has stopped changing shape and started growing behaviour.

**Keep designing.** Re-export whenever the layout moves. Export overwrites the
generated files, so keep your own code out of them — put behaviour in the action
methods and the types they call, not in `render`. The generated file is a view;
the moment you want it to be more than a view, it wants to be yours.

Between the two there is a middle: export once, own the file, and keep the
`.tailor` around as the thing you show people when you are arguing about layout.

## 16. Doing all of this from an agent

Everything above has an MCP equivalent. `tailor-mcp` speaks to the same document
model over stdio, and the app watches the file it has open — so a screen built
over MCP appears on the canvas as it is built.

```jsonc
{"name": "create_project", "arguments": {"path": "roster.tailor", "name": "Roster"}}
{"name": "add_node", "arguments": {"kind": "appshell", "parent": 1,
                                   "props": {"navbar_width": 220, "header_height": 64}}}
{"name": "add_node", "arguments": {"kind": "button", "parent": 7,
                                   "props": {"label": "Add person"}}}
{"name": "add_state", "arguments": {"name": "query", "type": "text"}}
{"name": "bind_prop", "arguments": {"node": 14, "prop": "value", "variable": "query"}}
{"name": "generate_code", "arguments": {}}
```

Call `catalog` for the 101 kinds and `component` for one kind's props, slots and
events before setting props you have not set before. See
[the MCP server](tailormcp.md) for the full tool list.

This tutorial's project was built exactly this way, which is why its code blocks
are output rather than prose.

## Where to go next

- [The canvas](tailorcanvas.md) — selection, resizing, layout modes, snapping
- [Components and slots](tailorcomponents.md) — the catalog, the five drawn
  containers, adding your own
- [State, bindings and actions](tailorstate.md) — the wiring, in depth
- [What gets generated](tailorcodegen.md) — flavours, export, the file format
- [The library tutorial](tutorial.md) — the same ground, written by hand
