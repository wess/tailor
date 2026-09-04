# Changelog

Notable changes to Tailor. Versions follow [semver](https://semver.org).

Tailor shipped inside the [guise](https://github.com/wess/guise) repository
through 1.6.0, on that project's version line and in
[its changelog](https://github.com/wess/guise/blob/main/CHANGELOG.md). Entries
from 1.1.0 (Tailor's first release) to 1.6.0 are there. This file starts where
Tailor became its own project.

## Unreleased

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
