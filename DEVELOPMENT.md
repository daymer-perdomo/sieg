# development notes

Internal notes on what this project is, how it got here, and how to ship an
update. `README.md` is for end users; this file is for whoever (human or
agent) works on the repo next.

## what this is

`sieg` has two independent halves, both in the same binary:

1. **The visual TUI mockup** (`sieg`, no args) — a standalone Rust TUI built
   with `ratatui` + `crossterm` that renders a fake sidebar, tab bar, panes,
   status bar, and onboarding modal with **hardcoded sample data**. No PTYs,
   no real agent detection. Exists to iterate on layout and color.
2. **The pane manager** (`sieg spawn|list|send|read|kill`) — real, not
   mocked. A local Unix-socket server owns real PTY-backed child processes
   (via `portable-pty`); the CLI subcommands are thin clients that talk to it.
   This is the first piece of *real* functionality, built specifically so a
   Claude Code skill (`skills/sieg/SKILL.md`) has something genuine to drive.

**These two halves don't talk to each other yet.** The TUI still renders only
`src/data.rs`'s hardcoded fake panes; it does not show panes spawned via the
CLI. Wiring the TUI to visualize real pane manager state is a future step,
not done.

The TUI's layout and catppuccin color values were originally modeled after
the [herdr](https://herdr.dev) TUI (`herdr-v2` repo) as a visual reference,
then fully renamed and decoupled — `sieg` does not depend on or import from
herdr in any way. It's its own repo, its own binary. The pane-manager +
CLI-skill pattern is also modeled after herdr (which calls this
"agent-native": agents drive it through a CLI/socket API), but sieg's version
is a from-scratch, much smaller implementation — no agent state detection
(idle/working/blocked), no multi-workspace/tab model, just spawn/list/send/
read/kill against a flat name → pane registry.

## project layout

```
src/
  main.rs     — entry point; dispatches to the TUI or a CLI subcommand
  ui.rs       — TUI rendering: sidebar, tab bar, panes, status bar, onboarding modal
  data.rs     — hardcoded mock data for the TUI (workspaces, tabs, panes, agent states)
  palette.rs  — catppuccin color values used across the TUI
  protocol.rs — Request/Response types shared by the CLI client and server (serde/JSON)
  server.rs   — the pane manager: owns real PTYs (portable-pty), one thread per pane
                for reading output + waiting on exit; listens on a Unix socket
  client.rs   — connects to the server socket, auto-spawning it (detached, via
                setsid) if it isn't already running
  cli.rs      — argument parsing + printing for spawn/list/send/read/kill
skills/sieg/SKILL.md          — Claude Code skill teaching the pane manager CLI
.github/workflows/release.yml — builds + publishes macOS binaries on tag push
install.sh    — curl-installable script, fetches latest GitHub release binary
```

### pane manager design notes

- **Socket**: `~/.sieg/sieg.sock`, plain JSON-lines request/response, one
  request per connection (not a persistent stream).
- **Server lifecycle**: not started by anything explicit. The first CLI
  command that needs it (`ensure_server_running` in `client.rs`) tries to
  connect; on failure it spawns `sieg __serve` detached (`setsid`, stdio to
  `/dev/null`) and retries the connection for up to 3s. It keeps running
  after the CLI process exits — that's the point (panes survive).
- **Per-pane threads**: each spawned pane gets two threads — one blocking on
  `reader.read()` to append PTY output into a capped `Vec<u8>` (200KB,
  oldest bytes dropped), and one polling `child.try_wait()` every 200ms
  (also draining a kill-request channel) to record the exit code without
  holding a lock that would block concurrent `kill` calls.
- **No cleanup command**: `kill` stops the process but leaves it in the
  registry as `exited(<code>)`. There's no `sieg remove`; re-spawning a used
  name is refused. Documented as a known limitation in the skill file.
- **macOS/Unix only**: uses `std::os::unix::net::UnixListener` and
  `libc::setsid` directly (no cross-platform abstraction), consistent with
  the rest of the project being macOS-only for now.

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

- Wire the TUI to the real pane manager (show actual `sieg list` state
  instead of `src/data.rs`'s hardcoded panes).
- Real agent state detection (idle/working/blocked) — herdr's version of
  this is a whole manifest-based detection engine; sieg has nothing like it,
  `sieg list` only reports process running/exited.
- A `sieg remove` command to clear exited panes from the registry.
- Optional: an in-binary update check / `sieg update` command, since
  installs are currently fully manual (see "shipping an update" above).
