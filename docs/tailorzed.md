# Tailor and Zed

Two halves, in opposite directions. Zed's agent can build a design; Tailor can
put your cursor on the line a component generated.

## What is not possible, first

Zed extensions are WebAssembly and the capability list is closed: languages,
debuggers, themes, icon themes, snippets, and MCP servers. **There is no UI
API.** A Tailor canvas inside a Zed pane is not hard, it is absent — nothing in
the extension surface can draw.

So this is not a Tailor panel in Zed. It is the two seams that do exist, and
between them they cover the thing you actually want, which is not having to
find the generated file by hand.

## The extension

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

The other direction, and it needs no extension at all.

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
