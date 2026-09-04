# Changelog

Notable changes to Tailor. Versions follow [semver](https://semver.org).

Tailor shipped inside the [guise](https://github.com/wess/guise) repository
through 1.6.0, on that project's version line and in
[its changelog](https://github.com/wess/guise/blob/main/CHANGELOG.md). Entries
from 1.1.0 (Tailor's first release) to 1.6.0 are there. This file starts where
Tailor became its own project.

## 0.2.0 — 2026-09-04

Tailor could draw an interface and print the Rust for it. It could not tell you
whether that Rust compiled, and there was nowhere to write what a button
*does* — so what came out was a very good mockup.

This release closes both. **⌘R** builds the design and runs it, and what the
compiler says comes back pointing at the component that caused it. **An action
has a body**, written in the app and stored in the `.tailor` file, so
regenerating writes around your code rather than over it. Between them, a
design becomes an app.

### It runs

Tailor could generate Rust. It could not tell you whether the Rust compiled,
which meant the loop ended at a folder of files and an Export button. **⌘R**
now writes the project out as a crate, builds it, and opens your app in a
window of its own — with nothing to configure first: a project you have never
exported gets a managed build directory, keyed by a stable hash of its path so
a rebuild reuses what the last one compiled.

What comes back is the interesting half. `cargo build --message-format=json`
reports every diagnostic with a file and a line, and codegen has always tagged
each node's expression with the line it landed on — so a type error in
generated Rust resolves to the component that generated it. A row in Problems
opens the file, scrolls to the line, and selects that component on the canvas.
Interface Builder never closed that loop.

Build and run are two phases on purpose. One `cargo run --message-format=json`
would merge cargo's JSON with your program's stdout on one stream, and a
program that printed a line of JSON would become a compiler error.

`tailordev --build project.tailor` does the same headlessly and exits non-zero,
which is what a CI job asking "does this design still compile" wants.

### Actions have bodies

The other half of Interface Builder's trick: a control is wired to a method,
and *you write the method*. Tailor had the wiring and nowhere to write, so every
project came out a mockup with `// TODO` where the app should be.

Clicking an action now opens a Rust buffer over its body — line numbers,
highlighting, the signature it generates above it, and what the handler can
reach listed below: the document's state signals and the entity fields codegen
will give it. ⌃Space completes, ranked so the document's own names beat Rust's
keywords; nothing is offered after a `.`, because this completer does not know
types and guessing a method would be worse than staying quiet.

The body lives in the `.tailor` file. That is what makes it survive: it is
design data, so regenerating writes the file *around* your code rather than over
it, and no export is ever the thing being preserved.

### The code pane is an editor

It showed one file, read-only, highlighted by a regex. It now shows every file a
build would compile — a strip across the top with the screen you have open, its
components, `mod.rs`, `main.rs`, `theme.rs`, `Cargo.toml` — parsed by
tree-sitter, the same parser Zed reads with. Unpinned it follows the canvas;
click a file to pin it.

Diagnostics underline the columns rustc named. ⌘F finds in the file with smart
case, every hit highlighted and the active one brighter, ⏎ and ⌘G to walk them.

It stays read-only, on purpose: the design is the source and the code is the
output, and an editable buffer over a file rewritten on the next keystroke would
be a promise Tailor cannot keep. Where you *do* write code is an action body,
which is design data and survives.

### Also

- **Open Quickly** (⌘⇧O) searches documents, generated files, actions and the
  nodes in the open document, all at once, and picking one navigates. The node
  half is the reason it earns its keystroke: in a screen of two hundred nodes
  the outline is a tree you scroll, and "the submit button" is a name you
  already know.
- A **Product** menu: Run, Build, Stop, Debug/Release, Clean Build Folder,
  Reveal Build Folder. Release is what you check before shipping — the manifest
  Tailor generates already asks for `lto = "fat"` and `strip`, which is a 4 MB
  binary where debug is 25 MB.
- Run restarts while something is running, and Stop is its own button beside
  it. The app runs with `RUST_BACKTRACE=1`: a panic that says "run with
  RUST_BACKTRACE=1" is a dead end when the console is the only place to see it.
- The toolbar has room — 52px with a 16px inset, and three groups of which only
  the middle one shrinks, so a narrow window truncates the device presets rather
  than pushing the panel toggles off the edge. Run and Export were a filled pill
  and a bordered pill in a bar made of flat chips; both are chips now, in one
  shape with Live, and Run is the only coloured thing in the bar.
- The device presets are gone — desktop, laptop, tablet, phone, panel, square,
  and the rotate button. gpui targets the desktop, so five of the six were a
  size with a lie on it. The artboard is one size, set as **Width** and
  **Height** in the Document inspector, which is the window the generated app
  opens at. `Canvas::preset` left the file format with them; a project that
  still carries one loads and ignores it.
- Fixed: `ComponentSpec::required` was added to the provider seam in 0.1.0-beta
  and the linter rewired to read it, but guise's catalog was never filled in —
  so "Button has no label" and its seven siblings had silently stopped firing.

## 0.1.0-beta — 2026-09-04

The first release off Tailor's own version line. It rode guise's to 1.6.0
inside that repository; out on its own it starts where the work is, and the
number says what a beta is for — the app is complete and the seam underneath it
is new.

### guise is a plug-in, not a built-in

Tailor is written in guise and it *targets* guise. Those were the same sentence
and now they are not. What it targets is a provider: two crates behind three
traits, with guise as the first implementation rather than a special case.

| Trait | Crate it plugs into | Needs gpui | What it answers |
| --- | --- | --- | --- |
| `Library` | `tailor-model` | no | the components, their props, the theme presets, what a token's type is called |
| `Generator` | `tailor-codegen` | no | the Rust for the shapes a chained constructor cannot express |
| `Renderer` | `tailor-render` | **yes** | the live component, and the entity behind a stateful one |

Three rather than one because of that gpui column. `tailor-mcp` lists
components, sets props and exports Rust with no window anywhere near it, so
describing a library must not require being able to draw one.

Two new crates carry what used to be scattered through five:

- **`tailor-guise`** — the ten catalog tables, the theme presets, the `Library`
  impl and the `Generator` impl for the thirty-odd components whose Rust is not
  one call. It depends on neither gpui nor `guise-ui`: a catalog *describes* a
  library, and a description compiles without it.
- **`tailor-guiserender`** — one arm per catalog kind, and the per-kind builder
  behind the canvas's entity cache.

Registration is explicit, from `main`. The app calls
`tailor_guiserender::register()`; the MCP server calls `tailor_guise::register()`
and links no gpui at all. A registry read with nothing registered panics with
the call to make, because that is a wiring mistake rather than a runtime
condition.

`docs/libraries.md` is the guide to writing one.

### What moved, and what it cost

`tailor-model` no longer knows a single guise fact. The catalog tables, the
theme presets, the `EXCLUDED` list and the surface ratchets all moved to
`tailor-guise`; what stayed is the vocabulary they are written in — the
`comp!` macro, `ComponentSpec`, `Ctor`, `PropSpec`.

Four facts that used to be hardcoded in library-agnostic code are now questions
the provider answers:

- **`Library::token_paths`** — a document stores `md`, and a token now knows
  only its variant name. `Size::Md` is assembled by the generator from the
  library's own type name.
- **`Library::boxes`** — the kinds whose expression *is* a styled box, so a
  node's layout goes onto it rather than into a wrapper around it.
- **`Library::drawn`** — the containers whose regions are `'static` closures.
  Both the canvas (which draws them from the theme to make their slots real
  drop targets) and the linter (which warns before an export fails to compile)
  read this instead of matching on kind names.
- **`ComponentSpec::required`** — the props a component is pointless without.
  The linter's "Button has no label" used to be a match on eight guise kinds;
  it is a table entry now.

`Project` gained a `library` field. It is skipped when empty and empty resolves
to whatever registered first, so every `.tailor` file written before this opens
unchanged, on guise, with no migration. An id this build does not ship falls
back rather than refusing to open the file — a project you cannot look at is
worse than one drawn with the wrong catalog.

The animation vocabulary — `Motion::enter_from`, `Easing::Out(Curve::Back)`,
`TransitionKind::SlideUp` — and the whole of `theme.rs` generation went to the
provider with everything else. `EaseToken::path` and friends are gone from the
model.

### Also

- Three agent-facing fields on every catalog entry: `docs` (when to reach for
  this rather than something adjacent), `example`, and `aliases`. Search ranks
  an alias above a blurb match, so an agent asking for a "dropdown" lands on
  the component a person would have recognised from the palette. The fields are
  declared and empty for now; filling them is the next piece of work.
- `tailor-model`'s own tests run against a ten-component fixture library rather
  than guise's catalog. Partly because they are the model's rules and should be
  asserted as such, and partly for a mechanical reason worth writing down: a
  dev-dependency from `tailor-model` onto a provider compiles `tailor-model`
  twice, and a `&dyn Library` from one copy is a different type from the
  other's. The generator's guise-output tests moved to
  `crates/guise/tests/codegen.rs` for the same reason.
- The eight documentation pages lost the `tailor` prefix when they moved out of
  guise's `docs/`, and their cross-links did not. Fixed.
- 219 tests, up from 214.

### Still guise-shaped

Written down rather than glossed. `tailor-render`'s own chrome is drawn in guise
— that is Tailor's UI, not what Tailor draws *with*, but it does mean a
provider's components sit in boxes styled by guise's theme. And
`tailor-codegen`'s `expr.rs` still assumes guise's three-way colour vocabulary;
the token *names* it prints already come from `token_paths`, the shape does not.
Neither blocks a second library that spells colours the same way.

### Its own repository

Tailor moved out of the guise workspace. It was six `publish = false` crates
riding a library's version line, which meant a Tailor fix could not ship without
a library release and a library release always shipped a Tailor build. They are
different products with different release cadences, and the coupling was costing
both.

The crates moved with their history — `crates/tailor/model` is `crates/model`,
and `git log --follow` still reaches back through it. `docs/tailor*.md` lost the
prefix that only made sense next to a library's pages.

Nothing about the app changed. The workspace builds against **`guise-ui` from
crates.io**, pinned, with no path dependency and no patch section: Tailor is now
a consumer of the library like any other app, which is the only way the boundary
between "the builder" and "the library it targets" becomes real rather than
asserted.

### `libraries/guise.surface`

Two tests kept the catalog honest by reading guise's own source out of the next
crate over — the ratchets that fail when the library gains a component the
catalog neither offers nor excludes with a reason, and when the theme preset
table drifts. There is no next crate over any more.

They now read `libraries/guise.surface`: a checked-in record of what the pinned
library ships, generated from the *published* crate by `cargo run -p
tailor-surface`. It resolves the source through `cargo metadata`, so it
describes exactly the version that will be compiled against, and it carries that
version in its first line — checked against `Cargo.lock`, so a dependency bump
without a regeneration fails the tests instead of quietly loosening what they
assert. CI regenerates it and fails on any diff, which catches the other half:
a patch release that adds a component without changing the version line.
