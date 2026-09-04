# Releasing

Tailor ships as a signed, notarized macOS app. One version, in the root
`[workspace.package]`; one tag; one DMG on the release.

Nothing here reaches crates.io — every crate is `publish = false`.

## Cutting one

Releasing is bumping the version and pushing. There is no tag to type.

1. Write the CHANGELOG section. `## <version> — <date>`, and make it the notes
   you would want to read: `release.yml` lifts that section verbatim, so a list
   of commit subjects is not an option.
2. Bump `version` in the root `Cargo.toml`, then `cargo build --workspace` to
   regenerate `Cargo.lock` — it is committed, and CI builds `--locked`, so a
   bump without the lockfile fails.
3. Commit and push to `main`.

```sh
git commit -am "Release 0.3.0: …"
git push origin main
```

The workflow watches `Cargo.toml` on `main`, releases when the version has no
tag yet, and makes the tag itself. That last part is the point: a tag typed
into a terminal is a name anyone can put anywhere, and this workflow turns one
into a signed, notarized download.

A semver pre-release version (`0.1.0-beta`) is marked as a pre-release on
GitHub, so it never becomes what `releases/latest` reports, and it does not
update the Homebrew cask. The `Info.plist` gets the release part of the version
— macOS compares `CFBundleVersion` numerically, and a suffix sorts in ways
nobody intends — while the app itself reports the full string through
`CARGO_PKG_VERSION`, which is where a beta should be visible.

If the release also moves to a new `guise-ui`, that is its own two-step: change
the version *and* `cargo run -p tailor-surface` in the same commit. CI
regenerates the surface file itself and fails on a diff.

## Why the workflow makes the tag

Tailor's history came out of the guise repository with `git filter-repo`, which
carried guise's twenty-eight tags along with the commits. They point at
guise-era work and share a version line with Tailor's own, so `git tag v0.2.0`
in a working copy that still had them silently did nothing — and the push that
followed put somebody else's five-year-old commit on the remote under Tailor's
next version number, seconds from being signed and shipped.

The tags are gone. The shape of the mistake is not, so the version in
`Cargo.toml` is the only thing anyone has to get right.

## What the workflow does

1. **`github-release`** — opens the release as a **draft**, with notes from the
   CHANGELOG. Draft, because a published release is what `releases/latest`
   reports: publishing first would advertise a version for the length of a
   notarization run with none of its assets attached.
2. **`macos`** — builds `dist/Tailor.app` (`scripts/bundle.sh`), signs it,
   notarizes and staples it, packages `Tailor.dmg` (`scripts/dmg.sh`), notarizes
   that too, and uploads it to the draft.
3. **`publish`** — flips the draft live, once the DMG is attached.
4. **`homebrew`** — takes the published DMG's SHA-256 and updates
   `Casks/tailor.rb` in [wess/homebrew-packages], so
   `brew install --cask wess/packages/tailor` gets the new version. Skipped
   for pre-releases: a beta should not become what `brew` installs.

[wess/homebrew-packages]: https://github.com/wess/homebrew-packages

`workflow_dispatch` runs the same thing without a push. It is idempotent: a
version that already has a tag is skipped, so re-running after fixing a signing
secret re-uploads with `--clobber` and never moves a tag.

## Secrets

Seven, and they gate two things separately because one is usually set up before
the other. `APPLE_SIGNING_IDENTITY`, `APPLE_CERT_P12`, `APPLE_CERT_PASSWORD`
and `KEYCHAIN_PASSWORD` sign; `APPLE_ID`, `APPLE_APP_PASSWORD` and
`APPLE_TEAM_ID` notarize on top. `HOMEBREW_TAP_TOKEN` is a PAT with write
access to the tap.

## Signing & notarization

Optional, and gated in two halves, because they need different credentials and
one is usually set up before the other:

| Secret | What it is | Gates |
|--------|------------|-------|
| `APPLE_SIGNING_IDENTITY` | e.g. `Developer ID Application: Your Name (TEAMID)` | signing |
| `APPLE_CERT_P12` | base64 of the exported Developer ID `.p12` | signing |
| `APPLE_CERT_PASSWORD` | password for that `.p12` | signing |
| `KEYCHAIN_PASSWORD` | any password, for the throwaway CI keychain | signing |
| `APPLE_ID` | Apple ID email for `notarytool` | notarization |
| `APPLE_TEAM_ID` | Apple Developer Team ID | notarization |
| `APPLE_APP_PASSWORD` | app-specific password for that Apple ID | notarization |

Three outcomes, and the `Verify bundle` step says which one you got:

- **Notarized** — all seven. Gatekeeper opens it without a word.
- **Signed, not notarized** — the first four. Gatekeeper still refuses a
  *downloaded* copy: notarization, not the signature, is what clears that.
- **Ad-hoc** — none. It runs on the machine that built it and nowhere else.

The app is signed with a hardened runtime and `assets/tailor.entitlements`;
gpui renders through Metal and JITs shaders, so it needs the JIT and
unsigned-executable-memory entitlements to run under notarization at all.

Setting a secret without putting it in a shell history or a scrollback:

```sh
gh secret set APPLE_APP_PASSWORD --repo wess/tailor   # reads from stdin
base64 < cert.p12 | gh secret set APPLE_CERT_P12 --repo wess/tailor
```

Exporting the certificate, if you need a fresh `.p12`:

```sh
security export -t identities -f pkcs12 -k login.keychain-db -P "$PW" -o cert.p12
```

That exports every identity in the keychain, which is fine — CI signs with the
name in `APPLE_SIGNING_IDENTITY` and ignores the rest. Delete the file
afterwards; it carries a private key.

## Building the app locally

```sh
scripts/icon.sh      # assets/icon.png + icon.icns, from scripts/icon.swift
scripts/bundle.sh    # dist/Tailor.app   (CODESIGN_IDENTITY to sign it)
scripts/dmg.sh       # dist/Tailor.dmg
```

`bundle.sh` reads the version from `Cargo.toml`, renames the `tailordev` binary
to `tailor`, and puts `tailor-mcp` beside it in the bundle. Regenerate the icon
only when the design changes — the `.icns` is committed.
