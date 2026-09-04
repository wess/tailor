<div align="center">

# Tailor

**A visual interface builder for [gpui](https://github.com/zed-industries/zed).**

</div>

Tailor is a drag-and-drop interface builder shaped like Interface Builder and
Android Studio's layout editor. Lay out a screen from real components, wire the
state and the actions, and export idiomatic Rust that has no dependency on
Tailor left in it.

```sh
cargo run -p tailor-app                    # from a checkout (binary: tailordev)
tailordev --build project.tailor           # compile a design, no window, CI-friendly
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
| **Run** | ⌘R writes the project out as a crate, compiles it, and opens your app in its own window. No setup: a project you have never exported gets a managed build directory the way Xcode's DerivedData works. A compiler error comes back as a row you can click — it opens the file, scrolls to the line, and **selects the component that generated it**. |
| **Actions with bodies** | Wire a button's click to an action, then click the action and write the method. A Rust buffer with completion over what is actually in scope; the body lives in the `.tailor` file, so regenerating writes around your code rather than over it. |
| **A live window** | A second window rendering the document for real, following every edit — with the guise DevTools inspector in it, and right-click-to-inspect the way a browser does it. |
| **An MCP server** | `tailor-mcp` drives the same document model, so an agent can place components, wire state and generate Rust with no window open. It saves after every change and the app watches the file, so a screen built by an agent appears on the canvas as it is built. |
| **An editor jump** | Both directions. **Open in Editor** (⌥⌘O) puts your cursor on the line a component generated; `tailordev --reveal <file>:<line>` goes the other way, and a Zed task binds it to a key. Zed, VS Code, Sublime, IntelliJ, Emacs and Neovim. |

## Documentation

Full docs live in [`docs/`](docs/readme.md).

[Overview](docs/readme.md) · [Tutorial](docs/tutorial.md) ·
[The canvas](docs/canvas.md) · [Components & slots](docs/components.md) ·
[State & actions](docs/state.md) · [Generated code](docs/codegen.md) ·
[Running your design](docs/running.md) · [MCP server](docs/mcp.md) ·
[Zed & other editors](docs/zed.md) · [Component libraries](docs/libraries.md) ·
[Changelog](CHANGELOG.md)

The [tutorial](docs/tutorial.md) builds a complete app end to end — every code
block in it is output Tailor actually produced.

## Workspace

```
crates/
├── model/        # the document: node tree, tokens, state, file format,
│                 #   and what a component library *is* (the Library trait)
├── codegen/      # document -> idiomatic Rust (and the Generator trait)
├── store/        # project files, recents, editor settings, export, the editor bridge
├── build/        # cargo: where a design compiles, and what the compiler said
├── render/       # the canvas: chrome, drop targets, the entity cache
│                 #   (and the Renderer trait)
├── guise/        # guise as a target library: its catalog, presets, generator
├── guiserender/  # guise on the canvas — the half that needs a window
├── app/          # the gpui workbench
├── mcp/          # an MCP server over the same document model
└── surface/      # regenerates libraries/*.surface (a dev tool, not shipped)

libraries/     # what each target library ships, as a checked-in file
extensions/    # a Zed context server for tailor-mcp — its own cargo
               # workspace, because it targets wasm32-wasip2
```

The split down the middle is the gpui dependency. `model`, `codegen`, `store`
and `guise` are free of it and carry most of the tests: the reparent rules, the
cycle checks, the undo stack, the generated output and the file round-trip are
all plain-data logic, and that is where a builder actually goes wrong.

## The component library

Tailor draws with [guise](https://github.com/wess/guise), and it *targets*
guise. Those are two different facts, and only the second one is interesting.

What it targets is a plug-in. `tailor-guise` is a catalog, a set of theme
presets and a generator; `tailor-guiserender` is one `match` that builds the
live component. Neither is privileged — they use the same public traits a
second library would, and adding one is two crates plus a line in `main`. See
[component libraries](docs/libraries.md).

The dependency goes through crates.io like any other consumer's — no path
dependency, no patch section. That boundary is what makes the plug-in real
rather than asserted.

`libraries/guise.surface` records what the pinned version ships. It is
generated, not written:

```sh
cargo run -p tailor-surface      # reads the version Cargo.lock resolves
```

Two tests read it. One fails when guise gains a component the catalog neither
offers nor excludes with a reason; the other fails when the theme preset table
drifts. Bumping guise is therefore two steps in one commit — change the version,
regenerate the file — and CI fails the build if you do only the first.

## Building

```sh
cargo run -p tailor-app                                   # the app
cargo test --workspace                                    # everything
cargo test -p tailor-model -p tailor-codegen -p tailor-guise   # the pure half
scripts/bundle.sh                                         # dist/Tailor.app
scripts/dmg.sh                                            # dist/Tailor.dmg
```

`Cargo.lock` is committed and CI builds `--locked`, so a version bump must
include the regenerated lockfile. Releasing is pushing a `v*` tag: the workflow
opens a draft release with notes from the matching `## <version>` section of the
CHANGELOG, builds and notarizes the DMG, attaches it, and only then publishes.

## License

MIT — see [LICENSE](LICENSE).
