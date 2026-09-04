<div align="center">

# Tailor

**A visual interface builder for [gpui](https://github.com/zed-industries/zed).**

</div>

Tailor is a drag-and-drop interface builder shaped like Interface Builder and
Android Studio's layout editor. Lay out a screen from real components, wire the
state and the actions, and export idiomatic Rust that has no dependency on
Tailor left in it.

```sh
cargo run -p tailor-app          # from a checkout (binary: tailordev)
```

Or take the app: every [release](https://github.com/wess/tailor/releases)
attaches **`Tailor.dmg`**, signed and notarized, with the MCP server beside the
executable in the bundle.

The canvas is not a drawing of your interface — it *is* your interface. A
`Button` on it is a real `guise::Button`, reading the same theme, laid out by
the same flexbox. There is no second rendering path to keep in step, so a
component cannot look right in the builder and wrong in the app.

| Part | What it does |
| --- | --- |
| **The workbench** | A searchable library of every placeable component, the node outline, the artboard, a five-tab inspector (Attributes, Size, Style, Connections, Identity) and a Problems panel. Every panel resizes, folds away, and remembers where you left it. |
| **Direct manipulation** | Eight resize knobs around the selection, drag to move, snapping to the grid and to siblings' edges with guides drawn where it caught, a live size readout, and arrow-key nudging. |
| **What comes out** | A `Render` entity when the document holds state, a `RenderOnce` builder when it does not. State variables become `Signal<T>` fields, events become `cx.listener` / `cx.subscribe`, and every resolved colour is hoisted into a `let` at the top of `render` the way guise's conventions require. |
| **A live window** | A second window rendering the document for real, following every edit — with the guise DevTools inspector in it, and right-click-to-inspect the way a browser does it. |
| **An MCP server** | `tailor-mcp` drives the same document model, so an agent can place components, wire state and generate Rust with no window open. It saves after every change and the app watches the file, so a screen built by an agent appears on the canvas as it is built. |
| **An editor jump** | Both directions. **Open in Editor** (⌥⌘O) puts your cursor on the line a component generated; `tailordev --reveal <file>:<line>` goes the other way, and a Zed task binds it to a key. Zed, VS Code, Sublime, IntelliJ, Emacs and Neovim. |

## Documentation

Full docs live in [`docs/`](docs/readme.md).

[Overview](docs/readme.md) · [Tutorial](docs/tutorial.md) ·
[The canvas](docs/canvas.md) · [Components & slots](docs/components.md) ·
[State & actions](docs/state.md) · [Generated code](docs/codegen.md) ·
[MCP server](docs/mcp.md) · [Zed & other editors](docs/zed.md) ·
[Changelog](CHANGELOG.md)

The [tutorial](docs/tutorial.md) builds a complete app end to end — every code
block in it is output Tailor actually produced.

## Workspace

```
crates/
├── model/     # the document: catalog, node tree, tokens, state, file format
├── codegen/   # document -> idiomatic Rust
├── store/     # project files, recents, editor settings, export, the editor bridge
├── render/    # document -> live components (the canvas)
├── app/       # the gpui workbench
├── mcp/       # an MCP server over the same document model
└── surface/   # regenerates libraries/*.surface (a dev tool, not shipped)

libraries/     # what each target library ships, as a checked-in file
extensions/    # a Zed context server for tailor-mcp — its own cargo
               # workspace, because it targets wasm32-wasip2
```

`model`, `codegen`, and `store` are free of gpui and carry most of the tests:
the reparent rules, the cycle checks, the undo stack, the generated output and
the file round-trip are all plain-data logic, and that is where a builder
actually goes wrong.

## The component library

Tailor targets [guise](https://github.com/wess/guise), and depends on it
through crates.io like any other consumer — there is no path dependency and no
patch section. That boundary is what makes a second target library possible.

`libraries/guise.surface` records what the pinned version ships. It is
generated, not written:

```sh
cargo run -p tailor-surface      # reads the version Cargo.lock resolves
```

Two tests read it. One fails when guise gains a component the catalog neither
offers nor excludes with a reason; the other fails when the theme preset table
drifts. Both used to read guise's source out of the same workspace; the surface
file is what replaced that when Tailor moved out. Bumping guise is therefore two
steps in one commit — change the version, regenerate the file — and CI fails the
build if you do only the first.

## Building

```sh
cargo run -p tailor-app                                   # the app
cargo test --workspace                                    # everything
cargo test -p tailor-model -p tailor-codegen -p tailor-store   # the pure half
scripts/bundle.sh                                         # dist/Tailor.app
scripts/dmg.sh                                            # dist/Tailor.dmg
```

`Cargo.lock` is committed and CI builds `--locked`, so a version bump must
include the regenerated lockfile. Releasing is pushing a `v*` tag: the workflow
opens a draft release with notes from the matching `## <version>` section of the
CHANGELOG, builds and notarizes the DMG, attaches it, and only then publishes.

## License

MIT — see [LICENSE](LICENSE).
