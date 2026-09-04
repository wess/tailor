# Running your design

Press **Run** (⌘R). Tailor writes the project out as a crate, cargo builds it,
and the app opens in a window of its own. Nothing needs configuring first.

That loop is the difference between a mockup tool and a builder. A design that
has never been compiled is a drawing; one that has is an app.

| | |
| --- | --- |
| **Run** | ⌘R — build, then launch. The button becomes **Stop** while it runs. |
| **Build** | ⌘B — compile and stop there. What you press to check a change without a window appearing. |
| **Stop** | ⌘. — kill whatever is running. |
| **Clean Build Folder** | ⌘⇧K — throw away `target/`. The escape hatch for the one failure that is not in your design. |
| **Reveal Build Folder** | Opens it in Finder. |

All of them are on the **Product** menu, where Xcode puts them and for the same
reason: building is not editing.

## Where it builds

Two possibilities, and which one you get depends on whether you have said where
the code goes.

**You have** — the project has an export directory (Generator → the Export
command sets it). Run builds that same tree, because a build directory that
shadowed your export would let the two drift and you would spend an afternoon
debugging the wrong file.

**You have not** — Tailor picks one, under its own config directory:

```
~/Library/Application Support/tailor/builds/<project>-<hash>/
```

The name is for you; the hash is of the project's path, so two projects called
`Demo` in different folders do not fight over one `target/`. It is stable across
edits, which is what lets a rebuild reuse what the last one compiled — the first
build of a gpui app is about a minute, and the second is about a second.

Keeping it out of your source tree is deliberate, and the same call the
[editor bridge](zed.md) made: generated code stays code, with no build
artifacts and no absolute local paths appearing beside it.

## The console

The bottom pane has two tabs. **Problems** is what will not generate and what
probably was not meant; **Console** is what the build and the app printed.

Lines are tagged. `build` is cargo — its progress, and rustc's diagnostics with
their caret diagrams. `app` is your program's own stdout and stderr. They are
one list because a build and the run that follows it are one sequence you read
down, and telling them apart afterwards is what the tag is for.

Under the hood they really are two phases: `cargo build --message-format=json`,
then the binary it linked, run directly. One `cargo run` would have merged
cargo's JSON with your program's output on the same stream, and a program that
printed a line of JSON would have become a compiler error.

## When it does not compile

Every diagnostic lands in three places.

1. **The console**, as rustc rendered it — the message, the source line, the
   carets, the suggested fix.
2. **The editor's gutter**, underlining the columns rustc named. Switch to
   Split (⌘3) to see them.
3. **Problems**, as a row. Clicking it opens the file, scrolls to the line, and
   **selects the component that generated it**.

That third one is the point. Codegen tags every node's expression with the line
it lands on, so a type error in generated Rust resolves back to the button that
produced it. Interface Builder never closed that loop; a compiler error here is
a thing on the canvas.

## From the command line

```sh
tailor --build project.tailor
```

Compiles the design and prints what the compiler said, with no window. Exit code
0 built, 1 did not, 2 could not try — which is what a CI job asking "does this
design still compile" wants.

## If cargo is not found

Tailor looks for it in `$CARGO`, `$CARGO_HOME/bin`, `~/.cargo/bin`, Homebrew's
prefix, `/usr/local/bin`, and `PATH`. If none of those has one, install Rust
from [rustup.rs](https://rustup.rs) and reopen Tailor.

The reason it looks rather than assuming: an app launched from Finder does not
inherit your shell's environment. Its `PATH` is
`/usr/bin:/bin:/usr/sbin:/sbin`, and rustup installs to `~/.cargo/bin`, which is
in none of them. Without the search, Run would work from a terminal and fail
from the Dock with "No such file or directory".
