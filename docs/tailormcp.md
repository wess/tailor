# Tailor: the MCP server

`tailor-mcp` is an MCP server over the same document model the app edits, so an
agent can build and generate interfaces without a window open.

```sh
cargo build -p tailor-mcp
claude mcp add tailor -- /path/to/tailor-mcp
```

The binary also ships inside `Tailor.app`, beside the executable:
`/Applications/Tailor.app/Contents/MacOS/tailor-mcp`.

## Pairing with the app

The server works on `.tailor` files and saves after every change. Tailor watches
the file it has open, so **a screen built over MCP appears on the canvas a
moment later**, with nothing wired between the two processes — the file is the
integration.

Open the project in the app, point the agent at the same path, and watch it
build. It is the fastest way to see what an agent is actually doing to a layout.

Unsaved work always wins: if the file changes on disk while there are edits in
the window that are not in it, the reload is refused and said out loud rather
than one of the two being quietly picked.

## The tools

| Tool | What it does |
| --- | --- |
| `create_project` | Make a new `.tailor` file. Refuses to write over one that exists. |
| `open_project` | Open one. Every other tool works on what this opened. |
| `overview` | Documents, state, actions, theme, generator settings, problem counts |
| `outline` | One document's node tree, as indented text with node ids |
| `catalog` | Every kind that can be placed, with category, slots and whether it owns state |
| `component` | One kind in full: props with types and defaults, slots, events, constructor shape |
| `add_document` | Add a screen or a component |
| `add_node` | Place a component. Returns the new node's id. |
| `set_node` | Change props, style, motion, name, hidden, locked. All three objects are merged, not replaced. |
| `move_node` | Reparent or reorder |
| `remove_node` | Delete a subtree |
| `add_state` | A state variable: name, type, initial |
| `add_action` | A named action |
| `bind_prop` | Bind a prop to a state variable |
| `connect_event` | Wire an event to an action |
| `set_theme` | Scheme, primary colour, radius, font |
| `problems` | The lint pass |
| `generate_code` | The guise Rust for one document, without writing anything |
| `export_code` | Write the whole crate |

## Getting the names right

Two tools exist so an agent does not have to guess. `catalog` lists the 101
kinds; `component` gives one kind's exact props, slots and events.

```jsonc
{"name": "component", "arguments": {"kind": "button"}}
```

```jsonc
{
  "kind": "button", "rust": "Button", "category": "Controls",
  "constructor": "new(id, value)",
  "props": [
    {"key": "label", "type": "text", "default": ""},
    {"key": "variant", "type": "variant", "default": "filled"},
    {"key": "color", "type": "color", "default": "blue"}
  ],
  "events": ["click"]
}
```

Call it once rather than guessing four times. A wrong key answers with what the
component actually takes, so a mistake costs one round trip and not a broken
document.

## Props

Plain JSON, resolved through the catalog:

```jsonc
{"variant": "outline", "size": "lg", "color": "grape", "full_width": true}
```

`{"bind": "query"}` in place of a value binds the prop to a state variable
instead — the same thing the Connections inspector does.

Style is its own object, and takes numbers or the words a dimension can be:

```jsonc
{"width": "full", "height": 240, "gap": 12,
 "padding": {"top": 0, "right": 16, "bottom": 0, "left": 16}}
```

A size wants a number, or `"auto"`, `"full"` or `"grow"`.

`motion` is a third object, for the node's entrance:

```jsonc
{"enter": "slideup", "ease": "out-back", "duration": 320, "delay": 60,
 "distance": 16, "stagger": 0, "repeat": "once", "alternate": false}
```

`enter` is `fade`, `slideup`, `slidedown`, `slideleft` or `slideright` — or
`null`, which is how an entrance is taken away again. `ease` is one of
`linear`, `out-quad`, `out-cubic`, `out-quint`, `out-expo`, `out-circ`,
`out-back`, `out-elastic`, `out-bounce`, `in-quad`, `in-cubic`, `in-expo`,
`in-out-quad`, `in-out-cubic`, `in-out-sine`, `spring`. A word it does not know
is an error rather than a silent default, so a guessed `"easeOut"` comes back
saying so.

A non-zero `stagger` animates the node's *children*, one delay per index,
instead of the node itself. See [the canvas page](tailorcanvas.md#motion).

## Building a screen

The whole of [the tutorial](tailortutorial.md) in the shape an agent would write
it. Node ids come back from `add_node`; the document's root is id 1.

```jsonc
{"name": "create_project", "arguments": {"path": "roster.tailor", "name": "Roster"}}

// The shell, and its regions by name.
{"name": "add_node", "arguments": {
  "kind": "appshell", "parent": 1, "name": "App shell",
  "props": {"navbar_width": 220, "header_height": 64}}}

{"name": "add_node", "arguments": {
  "kind": "group", "parent": 2, "slot": "header", "name": "Title bar",
  "props": {"gap": "sm", "align": "center"}}}

{"name": "add_node", "arguments": {
  "kind": "title", "parent": 7, "props": {"content": "Roster", "order": 4}}}
{"name": "add_node", "arguments": {
  "kind": "button", "parent": 7, "props": {"label": "Add person"}}}

// A component of its own, then three placements of it.
{"name": "add_document", "arguments": {"name": "PersonRow", "kind": "component"}}
{"name": "add_node", "arguments": {"document": "person_row", "kind": "group", "parent": 1}}
{"name": "add_node", "arguments": {"document": "people", "kind": "@PersonRow", "parent": 18}}

// State, bound to a control, and an action behind a click.
{"name": "add_state", "arguments": {"name": "query", "type": "text"}}
{"name": "bind_prop", "arguments": {"node": 14, "prop": "value", "variable": "query"}}
{"name": "add_action", "arguments": {"name": "add_person"}}
{"name": "connect_event", "arguments": {"node": 11, "event": "click", "action": "add_person"}}

{"name": "generate_code", "arguments": {}}
{"name": "export_code", "arguments": {"directory": "./roster"}}
```

`@PersonRow` is how a placement of your own component is written — the same
`@Name` reference the file format uses.

Documents are addressed by id, not by display name: a document called
`PersonRow` is `person_row`. `overview` lists both.

## What it will and will not do

The server reads and writes wherever it is told, the same as the app it pairs
with — it is a document tool, not a sandbox. Two limits are enforced anyway,
because they are the ones that lose work rather than the ones that look risky:

- `create_project` will not write over an existing file.
- `export_code` only ever writes below the directory you name.

Everything the app refuses, the server refuses too, because both go through the
same model: a component cannot contain itself, a node cannot be dropped into its
own child, a document cannot take a name that collides with a guise component,
and a project holding a NaN will not save.

## Scaffolding from the shell

For the cases that do not need a conversation:

```sh
tailordev --template dashboard out.tailor   # empty | sign in | dashboard | settings
```

Writes a project and exits, so a script can scaffold one without opening a
window — a starting point an agent can then edit over MCP, or you can open.
