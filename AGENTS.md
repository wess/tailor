# Tailor

A visual interface builder for [gpui](https://github.com/zed-industries/zed)
(Zed's GPU-accelerated Rust UI framework). Eight crates plus a dev tool; full
human docs live in [`docs/`](docs/readme.md), starting at
[`docs/readme.md`](docs/readme.md).

Tailor is *written in* [guise](https://github.com/wess/guise) and it *targets*
guise. Those are two different facts and the second one is a plug-in: a catalog,
a generator and a renderer behind three traits, with guise as the first
implementation rather than a special case. `docs/libraries.md` is the guide.

It depends on `guise-ui` from **crates.io**, pinned — no path dependency, no
`[patch.crates-io]`. That boundary is load-bearing: it is what makes the plug-in
real rather than asserted.

## Commands

```sh
cargo run -p tailor-app                     # launch Tailor (binary: tailordev)
cargo check --workspace                     # fast type-check
cargo test --workspace                      # 219 tests
cargo test -p tailor-model -p tailor-codegen -p tailor-guise   # the gpui-free half
cargo run -p tailor-surface                 # regenerate libraries/guise.surface
cargo build --workspace --locked            # what CI builds (on macOS)
cargo fmt --all                             # CI checks formatting, advisory
cargo clippy --workspace --all-targets      # CI runs clippy, advisory
scripts/bundle.sh                           # dist/Tailor.app
scripts/dmg.sh                              # dist/Tailor.dmg
```

## Build constraints

- Everything builds against **plain crates.io `gpui = "0.2.2"`** and
  **`guise-ui = "1.6.0"`** — no git pins, no `[patch.crates-io]`. Don't use
  gpui APIs newer than that snapshot.
- **`Cargo.lock` is committed and CI builds `--locked`**, so a version bump must
  include the regenerated lockfile. Releasing = push a `v*` tag. `release.yml`
  opens a **draft** release with notes lifted from the matching `## <version>`
  section of the CHANGELOG (so that section is the release notes), builds and
  notarizes `dist/Tailor.app` into `Tailor.dmg` via `scripts/bundle.sh` +
  `scripts/dmg.sh`, attaches it, and only then publishes — a published release
  is what `releases/latest` reports, and advertising one with no assets attached
  is what the draft avoids.
- Every crate is `publish = false`. Nothing here goes to crates.io.

## `libraries/` — what a target library ships

`libraries/guise.surface` is a checked-in record of every component guise
defines and every theme preset it offers. It is **generated, never
hand-edited**: `cargo run -p tailor-surface` resolves the pinned crate's
unpacked source through `cargo metadata` and rewrites it.

Two ratchets read it, and they are the reason the catalog does not silently
drift behind the library:

- `coverage` (in `guise/src/coverage.rs`) fails unless every component in the
  surface is either catalogued or in `EXCLUDED` with a reason.
- `presets_match_guise` (in `guise/src/surface.rs`) fails when `theme::PRESETS`
  drifts from what guise defines.

The file's first line carries the version it was generated from, checked against
`Cargo.lock`. So bumping guise is **two steps in one commit** — change the
version, regenerate the surface — and CI regenerates it independently and fails
on any diff, which catches a patch release that added a component without
changing the version line.

## Architecture

- **`model/`** — the document: the node arena, tokens, state variables, actions,
  undo, and the `.tailor` file format — plus `library::Library`, which is what a
  component library *is*. It knows none: guise's catalog lives in `tailor-guise`
  and registers itself at start-up. No gpui, and it carries most of the tests:
  reparent rules, cycle checks, the lint pass, and the file round-trip are where
  a builder actually goes wrong. Its own tests run against
  `library::fixture` — a ten-component stand-in — rather than guise's catalog,
  because a dev-dependency back onto a provider would compile `tailor-model`
  twice and a `&dyn Library` from one copy is a different type from the other's.
- **`codegen/`** — document → Rust, plus `Generator`, the library's half of it.
  Driven by the same catalog the canvas reads, so a component cannot render one
  way and generate another.
- **`store/`** — project files, recents, editor settings, export.
- **`render/`** — the canvas, plus `Renderer`. The chrome around a node — its
  style box, its selection outline, its drop strips, its entrance replay — is
  this crate's; the component inside is the provider's. Interaction never
  reaches back into the app directly: it goes through `Hooks`, built from a
  *weak* handle, because a live component tree must not own the view that
  renders it.
- **`guise/`** — guise as a target library: the catalog tables, the theme
  presets, the `Library` impl, and the `Generator` impl for the ~30 shapes a
  chained constructor cannot express. No gpui and no `guise-ui` — a catalog
  *describes* a library and compiles without it. `tailor-mcp` takes this and
  nothing more.
- **`guiserender/`** — guise on the canvas: one arm per catalog kind, plus the
  entity cache's per-kind builder. The gpui half, split out so the MCP server
  can list and generate components without a window.
- **`app/`** — one `Workbench` entity owns the project and every panel; the
  panels are render methods in sibling files, not views of their own.
- **`mcp/`** — an MCP server over the same model (`tailor-mcp`). Hand-rolled
  JSON-RPC over stdio; it saves after every change, and the app polls the file
  it has open, which is the whole integration between them. It registers
  `tailor_guise` only — no renderer, so no gpui.
- **`surface/`** — the generator for `libraries/*.surface`. A dev tool; it is
  not in the app bundle.
- **The editor bridge** (`store/src/bridge.rs` + `--reveal`) is the pair of jumps
  between a component and its code. Out: *Open in Editor* reads
  `Generated::lines`, the map codegen builds by tagging each node's expression
  while it writes the file and recording the line the tag came off — tagging
  rather than searching, because a `Text` carries nothing to search for. In:
  `tailordev --reveal <file>:<line>` resolves the file through an export index
  in Tailor's config dir (never in the user's source tree), and leaves a focus
  request the open window picks up on the poll it already runs. `extensions/zed/`
  is a separate thing — an MCP context server for Zed's agent panel — and is its
  own cargo workspace because it targets `wasm32-wasip2`.
- **`MotionProps`** on a node is the designer's slice of `guise::anim`: an
  entrance, an easing, timings, and a stagger. `Document::motion_of` resolves
  it — a node's own motion, or its parent's with the index folded into the
  delay when that parent staggers. Codegen and the canvas both go through that
  one function, which is what keeps the preview and the export the same
  animation. A staggering container animates its children and **not itself**.

## Providers

Three traits, split by whether they need a window:

| Trait | Crate | gpui | Answers |
| --- | --- | --- | --- |
| `Library` | `tailor-model` | no | components, props, presets, token type names |
| `Generator` | `tailor-codegen` | no | the Rust for shapes a chained constructor can't express |
| `Renderer` | `tailor-render` | yes | the live component, and the entity behind a stateful one |

Registration is explicit, from `main`: `tailor_guiserender::register()` in the
app (it calls `tailor_guise::register()` for you), `tailor_guise::register()` in
the MCP server. Both idempotent — tests call them too. **A registry read with
nothing registered panics with the call to make**, deliberately: it is a wiring
mistake, not a runtime condition.

A `Project` carries `library: String`. Empty resolves to whatever registered
first, so every `.tailor` file written before this opens unchanged; an unknown
id falls back rather than refusing to open, because a project you cannot look at
is worse than one drawn with the wrong catalog.

Two things are still guise-shaped inside library-agnostic crates, and are
written down rather than glossed: `tailor-render`'s own chrome is drawn in guise
(that is Tailor's UI, not what it draws with), and `codegen/src/expr.rs`'s
colour expressions assume guise's three-way colour vocabulary. Token *names*
already come from `Library::token_paths`.

## Two things that are load-bearing and easy to break

- **The catalog is the single source of truth**, and two tests enforce it.
  Adding a component is one `comp!` entry in `guise/src/catalog/` plus one arm in
  `guiserender/src/nodes.rs`. `PropSpec::emit` decides what the generator prints.
  `coverage` reads `libraries/guise.surface` and fails unless every component is
  catalogued or in `EXCLUDED` with a reason — so a guise bump that adds a
  component breaks `cargo test` rather than going unnoticed.
  `every_catalog_kind_generates_something` (in `guise/tests/codegen.rs`) catches
  an entry the generator has no support for.
- **Seven containers are drawn, not instantiated** (`Tabs`, `Accordion`,
  `SplitPanel`, `AppShell`, `Carousel`, `SettingsView`, `VirtualList`). Their
  regions take `'static` closures, which a designer cannot drop into; drawing
  them from the theme is what makes their slots real drop targets. Generated
  code uses the real component — `SettingsView` is the one whose generated
  shape differs most, declaring `.page(id, title)` and filling them from one
  `.content(|page, ..| match page { .. })` closure.
- **Tailor's theme goes through `guise::ThemeManager`** (`app/src/theme.rs`),
  which is the only thing that writes the `Theme` global. Its registry holds
  Tailor's two chrome themes under the ids `light`/`dark` — the pair
  `ThemeChoice::System` resolves through, which is what buys the start screen's
  "follow the OS" setting — plus a `project` entry re-registered whenever the
  open project's theme changes. A project's theme resolves inline JSON, then a
  preset, then plain light/dark (`ThemeSpec::effective_scheme`); the JSON lives
  *in* the `.tailor` file rather than beside it, and exports as `theme.json`.

Tailor wears the *project's* theme app-wide, because guise resolves colours when
a component paints rather than when you build it — there is no way to scope a
second theme to the canvas subtree.

The project is held as `Arc<Project>` and edited through `Arc::make_mut`: the
history and the canvas share the same allocation, so a commit and a frame are
free and an edit pays for exactly one copy. Codegen, lint, autosave, export and
the file watcher run on the background executor, debounced, guarded by a
revision counter and cancelled by dropping the held `Task`. Anything touching
gpui entities — the preview store — stays on the main thread. Tests zero the
debounce and call `cx.run_until_parked()` before asserting on generated code or
problems.

## File/naming conventions

- **One component per file**, lowercase, no `-`/`_`/spaces. Group with
  directories (`editor/inspector.rs`), never concatenated names.
- Every file opens with a `//!` module doc that says *why* the thing exists.
- Formatting is `max_width = 100`, `tab_spaces = 2` (see `rustfmt.toml`).

## Adding a component to the catalog

1. One `comp!` entry in the right file under `guise/src/catalog/`.
2. One arm in `guiserender/src/nodes.rs` that builds the real component.
3. If its constructor is not one call, an arm in `GuiseGenerator::special` in
   `guise/src/codegen.rs` too.
4. Document it in `docs/components.md`.
5. `cargo test` — the coverage ratchet and
   `every_catalog_kind_generates_something` both have to pass.
