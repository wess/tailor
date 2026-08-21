# Tailor: state, bindings and actions

A screen you can only look at is a mockup. Three things in the inspector's
**Connections** tab turn a document into a component you can wire up: state
variables, bindings, and actions.

## State variables

With nothing selected, the inspector shows the *document's* own panel. Its state
table takes a name, a type, and a starting value.

| Type | Becomes | Initial written as |
| --- | --- | --- |
| text | `Signal<String>` | anything |
| bool | `Signal<bool>` | `true` / `false` |
| int | `Signal<i64>` | a whole number |
| float | `Signal<f64>` | a number |
| items | `Signal<Vec<String>>` | one per line |

Each becomes a public `Signal<T>` field on the generated type. A
[`Signal`](reactive.md) is guise's reactive cell: read it during `render` and
the component redraws when it changes.

The name becomes a Rust identifier, so `Email Address` becomes `email_address`
and `type` becomes `type_`. Whatever it turns into, no state variable and no
generated field will ever collide — the generator seeds its name table with your
variable names first, so a text input called *Email* next to a variable called
*email* gives you `email_field` and `email`, not two `email`s.

## Bindings

Select a component, and in Connections bind a prop to a variable instead of
giving it a literal. The canvas shows the variable's starting value, so you can
see what the first frame will look like.

**A binding is two-way.** guise has two shapes for that, and which one you get
depends on the kind of component — the same split described in
[components](tailorcomponents.md#two-kinds-of-component):

A **stateful entity** — a text input, a select, a slider — binds with a call
after both sides exist:

```rust
let query = Signal::new(cx, "".to_string());
let search = cx.new(|cx| {
    TextInput::new(cx)
        .placeholder("Name or role")
        .label("Search")
});
TextInput::bind(&search, &query, cx);
```

A **controlled builder** — a checkbox, a switch, a chip, a rating — binds in the
builder chain, because it has no constructor to bind after:

```rust
Switch::new("node-16")
    .bind(self.only_active.binding())
    .label("Active only")
```

Either way, typing writes to the signal and setting the signal updates the
control.

Two details follow from that, and both are visible in the output:

- **The setter the binding drives is not also emitted.** A bound text input's
  chain has no `.value(..)` in it, because `TextInput::bind` sets the value.
  Emitting both would leave them fighting over it.
- **The signals are built first.** In the generated constructor, every
  `Signal::new` comes before the fields, because a field that reads a signal
  reads it while it is being built — and because a binding needs both sides to
  exist before it can be made.

Binding a prop that is *not* the one guise binds two-way — a placeholder, a
label — is a one-shot read of the signal at construction. That is still useful
and it still compiles; it just does not write back.

## Actions

The same document panel takes actions: a name, and an optional body.

Each becomes a method on the generated type:

```rust
pub fn add_person(&mut self, cx: &mut Context<Self>) {
    // TODO
    let _ = cx;
}
```

If you typed a body, that is what is inside instead. Tailor never runs your
code — it places a method where the handler belongs, so the file is a starting
point rather than a stub you have to re-wire.

## Events

Select a component and connect one of its events to an action. Which events a
component has comes from the catalog: a button has `click`, a select has
`change`, a nav link has `click`, a modal has `close`.

How the connection is generated depends, again, on the kind of component:

- A **builder** takes a handler:
  `.on_click(cx.listener(|this, _event, _window, cx| this.add_person(cx)))`
- An **entity** emits, so the wiring goes in the constructor:
  `cx.subscribe(&select, |this, _entity, _event, cx| this.pick(cx)).detach();`
- A builder **inside one of the five drawn containers** goes through a weak
  handle, because those regions are `'static` closures and a borrowed context
  cannot outlive the method that made it:

```rust
.header(64., {
    let view = cx.entity().downgrade();
    move |_window, _cx| {
        // …
        Button::new("node-11", "Add person")
            .on_click({
                let view = view.clone();
                move |_event, _window, cx| {
                    view.update(cx, |this, cx| this.add_person(cx)).ok();
                }
            })
    }
})
```

The handle is weak because a live component tree must not own the view that
renders it, and it is cloned again per handler because the region closure is
`Fn` — a `move` handler inside it would move the shared handle out of the
closure that owns it.

You do not have to think about any of that. It is here because it is what you
will read in the file, and a generated file you cannot read is a generated file
you cannot own.

## Problems

The lint pass runs on every edit — off the main thread, debounced — and the
Problems panel (⌥⌘4) shows what it found.

Errors, which mean the document will not generate or will not compile:

- a prop bound to a variable you renamed or deleted
- an event pointing at an action that is gone
- a component reference to a document that no longer exists
- two documents that generate the same Rust type name
- a document whose name collides with a guise component
- a component that would contain itself

Warnings, which mean it probably was not meant:

- a stateful component inside `Tabs`, `Accordion` or `SplitPanel` — their
  regions are `'static` closures, so extract that part into its own component
- an event wired to no action
- a button with no label, an image with no source, an empty container
- a node pushed outside its parent

Clicking a row opens the document and selects the node. Right-clicking offers
**Reveal** and *copy the message*.

A clean Problems panel is not a promise that your app is right. It is a promise
that the file will generate and the generated file will compile.
