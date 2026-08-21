# Tailor and Zed

What Interface Builder gives you inside Xcode is a loop: click a control, land
on its code; sit on a line of code, find the control. Tailor and Zed do that
across two apps.

- **Component → code.** Select something in Tailor, **View → Open in Zed**
  (⌥⌘O). Zed opens the generated file with the cursor on that component's line.
- **Code → component.** Put the cursor on a line of generated Rust in Zed and
  run **Reveal in Tailor**. Tailor comes forward with that component selected,
  on the canvas and in the outline.

Neither direction needs an extension, an agent, or a network. There is a
[Zed extension](#the-extension-optional) as well, but it is for a different
job — driving Tailor from Zed's agent — and it is not part of this loop.

## What is not possible

A Tailor canvas inside a Zed pane. Zed extensions are WebAssembly and the
capability list is closed: languages, debuggers, themes, icon themes, snippets
and MCP servers. **There is no UI API** — nothing in the extension surface can
draw, so this is absent rather than difficult. Two windows and a fast jump
between them is the shape that is actually available.

## Reveal in Tailor

The code → component direction. Tailor's binary resolves a file and a line back
to the node that made it:

```sh
tailordev --reveal src/ui/people.rs:106
# Roster · People · node 11 — selecting it in Tailor
```

A Zed **task** hands it the cursor. In `.zed/tasks.json`, either in the exported
project or in `~/.config/zed/tasks.json` for every project:

```jsonc
[
  {
    "label": "Reveal in Tailor",
    "command": "tailordev",
    "args": ["--reveal", "$ZED_FILE:$ZED_ROW"],
    "reveal": "on_error",
    "hide": "on_success",
    "shell": "system"
  }
]
```

And a key for it, in `keymap.json`:

```jsonc
{
  "context": "Editor",
  "bindings": {
    "alt-cmd-r": ["task::Spawn", { "task_name": "Reveal in Tailor" }]
  }
}
```

`command` has to be findable: use the app's own copy —
`/Applications/Tailor.app/Contents/MacOS/tailor` — if `tailordev` is not on your
`$PATH`.

### How it finds the project

Exporting records which project wrote which directory, in Tailor's own config
rather than in your source tree — generated code stays code, with no dotfiles or
absolute local paths committed beside it. `--reveal` looks up the innermost
export directory that contains the file, opens that project, matches the file
name to a document, and takes the node whose expression starts at or above the
cursor.

Then it leaves a request that an open Tailor window picks up on the poll it
already runs for the project file. A window with a different project open leaves
the request alone, so the right window answers it.

If nothing is open, `--reveal` still prints what it resolved and exits without
error — the task tells you what it found either way.

## The extension (optional)

This part is not the loop above. It is for building a design *from* Zed's agent
panel, and you can skip it entirely.

`extensions/zed/` registers `tailor-mcp` as a context server. The whole of it is
a manifest —

```toml
[context_servers.tailor]
```

— and one trait method that says where the binary is. Zed compiles it on
install; there is no build step for you.

### Installing it

Extensions → **Install Dev Extension** → pick `extensions/zed`. Zed compiles it
and it appears under **MCP Servers**, with *Rebuild*, *Uninstall* and
*Configure*.

### Pointing it at the server

The extension looks in two places, in order:

1. `/Applications/Tailor.app/Contents/MacOS/tailor-mcp` — where the DMG puts it,
   so an installed Tailor needs no configuration at all.
2. `tailor-mcp` on `$PATH`, which Zed resolves when it spawns.

If it is somewhere else — a `cargo build` in a checkout, most likely — say so in
Zed's settings. `context_server_command` is handed a project and not a worktree,
so there is no `which` for the extension to ask on your behalf; settings are the
only thing it can read:

```jsonc
"context_servers": {
  "tailor": {
    "source": "extension",
    "command": { "path": "/path/to/guise/target/debug/tailor-mcp" }
  }
}
```

Zed starts context servers lazily, when a thread needs them — not when the
extension loads. If nothing seems to happen, that is why.

### What it gets you

Everything in [the MCP server](tailormcp.md), from the agent panel: place
components, wire state, generate and export. The server saves after every
change and Tailor watches the file it has open, so a screen built from Zed
appears on the canvas a moment later. Nothing is wired between the two
processes — the file is the integration.

## Open in Zed

The component → code direction, and it needs no extension either.

Select a component, then **View → Open in Zed** (⌥⌘O), or *Open in Zed* on the
right-click menu. Zed opens the generated file with the cursor on that
component's line:

```
people.rs › impl Render for People › fn render

105                     .child(
106                         Button::new("node-11", "Add person")
107                             .on_click({
```

It needs two things, and says so if either is missing: the project has to have
been **exported** at least once, because that is what creates the file, and the
node has to appear in the output — a hidden node does not.

### How the line is found

The generator tags every node's expression while it writes the file, then takes
the tags back out and records the line each one was removed from. That map ships
on `Generated::lines`, keyed by node id.

Doing it with tags rather than by searching the finished text matters for
coverage. A `Button` carries its node id into the output — `Button::new("node-11", …)`
— so you could find it by searching. A `Text` carries nothing:
`Text::new("Ada Whitfield")` is indistinguishable from any other. The tag pass
covers every node, including the ones with nothing to search for.

When a call collapses onto one line the tag sits in front of it, and when it
expands the tag goes *inside* — so the line you land on is the constructor and
not the `.child(` that wraps it. Landing on the wrapper is landing next to the
component instead of on it.

### Where the file is

`export_dir` on the document, written whenever you export — from the app or over
MCP. That is why *Open in Zed* does not ask you every time, and why exporting
somewhere new silently starts sending you there instead.

### If you use a different editor

`open_in_editor` shells the `zed` CLI, which takes `path:line:column`. Most
editors have the same thing (`code -g`, `subl`, `idea --line`), so this is a
small change rather than a design problem — but Tailor only ships the Zed one
today.

The CLI is not always on a GUI app's `$PATH` — a bundle launched from Finder
inherits a minimal one — so Tailor falls back to
`/Applications/Zed.app/Contents/MacOS/cli`, which is where every macOS install
puts it.
