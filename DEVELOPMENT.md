# development notes

Internal notes on what this project is, how it got here, and how to ship an
update. `README.md` is for end users; this file is for whoever (human or
agent) works on the repo next.

## what this is

`sieg` is a **visual-only mockup**, not the real app. It's a standalone Rust
TUI built with `ratatui` + `crossterm` that renders a fake sidebar, tab bar,
panes, status bar, and onboarding modal with **hardcoded sample data** — no
PTYs, no real agent detection, no server/client architecture. It exists to
iterate on layout and color before any real functionality gets built.

The layout and catppuccin color values were originally modeled after the
[herdr](https://herdr.dev) TUI (`herdr-v2` repo) as a visual reference, then
fully renamed and decoupled — `sieg` does not depend on or import from herdr
in any way. It's its own repo, its own binary.

## project layout

```
src/
  main.rs     — crossterm/ratatui event loop (keyboard input, draw loop)
  ui.rs       — all rendering: sidebar, tab bar, panes, status bar, onboarding modal
  data.rs     — hardcoded mock data (workspaces, tabs, panes, agent states)
  palette.rs  — catppuccin color values used across the UI
.github/workflows/release.yml — builds + publishes macOS binaries on tag push
install.sh    — curl-installable script, fetches latest GitHub release binary
```

## accounts / hosting

- GitHub repo: **`daymer-perdomo/sieg`** (public) — https://github.com/daymer-perdomo/sieg
  - Not `daymer003` — an earlier version of this repo was accidentally created
    under the `daymer003` account and was abandoned in place (never deleted,
    just unused). If you see `daymer003/sieg` referenced anywhere, it's stale.
  - The `daymer-perdomo` account is authenticated locally via `gh auth login`
    (multiple accounts are stored; `gh auth switch` if it's not active).
- Local checkout: `/Users/daymer/Documents/Developer/sieg`
- No CI/tests configured — it's a mockup, `cargo build` is the only check.

## how releases work

The GitHub Actions workflow (`.github/workflows/release.yml`) triggers on any
pushed tag matching `v*`. It builds **both** macOS targets
(`aarch64-apple-darwin` and `x86_64-apple-darwin`) on a single `macos-14`
(Apple Silicon) runner — Xcode's toolchain cross-compiles x86_64 without
needing a separate Intel runner. This was a deliberate fix: the original
workflow used a `macos-13` runner for the x86_64 build, and GitHub's
`macos-13` hosted runner pool was badly queue-congested (10+ minute waits).
Don't reintroduce a `macos-13` runner for this.

Once both builds finish, a second job downloads both artifacts and publishes
them as a GitHub Release via `softprops/action-gh-release`, with
`generate_release_notes: true` (auto-generated from commits since the last
tag).

`install.sh` always downloads from `releases/latest/download/<asset>`, so it
always gets whatever tag was published most recently — no version pinning
logic needed there.

## shipping an update

```bash
cd /Users/daymer/Documents/Developer/sieg
# 1. make your changes, commit them to main
git add -A
git commit -m "describe the change"
git push origin main

# 2. tag the release
git tag -a v0.2.0 -m "v0.2.0"
git push origin v0.2.0
```

Pushing the tag triggers the workflow automatically. Watch it with:

```bash
gh run watch --repo daymer-perdomo/sieg
```

Once it's green, the existing install command re-installs the new version —
users just need to re-run it:

```bash
curl -fsSL https://raw.githubusercontent.com/daymer-perdomo/sieg/main/install.sh | sh
```

**There is currently no in-app update check.** `sieg` does not know its own
version against the latest release and has no `sieg update` command — every
update requires manually re-running the curl install. Adding that is real
functionality (not visual), tracked as a "next step" below, not done yet.

### if you need to re-publish the same tag

Delete and recreate it, then force-push:

```bash
git tag -d v0.2.0
git tag -a v0.2.0 -m "v0.2.0"
git push origin v0.2.0 --force
```

(This happened a few times while setting up the repo — every rename required
re-tagging `v0.1.0` because the release assets baked the old name into their
filenames.)

## history (why things look the way they do)

1. Built as a visual-only mockup in a separate folder from `herdr-v2`, by
   hand-copying herdr's real catppuccin palette values and roughly
   replicating its sidebar/tabs/panes/onboarding layout with fake data.
2. Renamed everything from `herdr`-branded to `Sieg` (package, binary, UI
   text, repo) — first to `sieg-ui-mockup`, then dropped the `-ui-mockup`
   suffix down to just `sieg`, per request.
3. Set up real distribution: public GitHub repo, a release workflow that
   builds macOS binaries on tag push, and a `curl | sh` install script — same
   pattern as herdr's own install flow, scoped to macOS only (arm64 +
   x86_64), since that covers this machine.
4. Repo was first created under the wrong GitHub account (`daymer003`);
   re-created under the correct one (`daymer-perdomo`) once that account was
   authenticated locally, and all references (`install.sh`, `README.md`)
   were repointed. The old `daymer003/sieg` repo was left in place, unused.
5. Fixed the release workflow to stop using a `macos-13` runner for the
   x86_64 build (chronic queue delays) in favor of cross-compiling both
   targets on `macos-14`.

## next steps (not done yet)

- Real functionality beyond the visual mockup — PTYs, real agent state, etc.
  Nothing here is real; every "agent", "workspace", and "tab" is a hardcoded
  string in `src/data.rs`.
- Optional: an in-binary update check / `sieg update` command, since
  installs are currently fully manual (see "shipping an update" above).
