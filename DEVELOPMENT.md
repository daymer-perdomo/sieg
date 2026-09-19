# development notes

Internal notes on what this project is, how it got here, and how to ship an
update. `README.md` is for end users; this file is for whoever (human or
agent) works on the repo next.

## what this is

`sieg` is a real (if small) terminal multiplexer, in one binary:

1. **The pane manager** (`sieg spawn|list|send|read|kill`) — a local
   Unix-socket server that owns real PTY-backed child processes (via
   `portable-pty`); the CLI subcommands are thin clients that talk to it.
   Built first, specifically so a Claude Code skill (`skills/sieg/SKILL.md`)
   had something genuine to drive.
2. **The TUI** (`sieg`, no args) — a `ratatui` + `crossterm` client of the
   *same* server. No more mock data: it lists real panes, and focusing one
   forwards real keystrokes to its real stdin. Output rendering is a
   lightweight scrollback interpreter (`src/pty_text.rs`), not a real
   terminal emulator — see "rendering limits" below.

Originally the TUI was a pure visual mockup with hardcoded sample data and
the pane manager didn't exist yet; wiring them together was a deliberate
second step (see history below), not the initial design.

The TUI's layout and catppuccin color values were originally modeled after
the [herdr](https://herdr.dev) TUI (`herdr-v2` repo) as a visual reference,
then fully renamed and decoupled — `sieg` does not depend on or import from
herdr in any way. It's its own repo, its own binary. The pane-manager +
CLI-skill pattern is also modeled after herdr (which calls this
"agent-native": agents drive it through a CLI/socket API), but sieg's version
is a from-scratch, much smaller implementation — no agent state detection
(idle/working/blocked), no multi-workspace/tab model, just spawn/list/send/
read/kill against a flat name → pane registry.

### rendering limits

`src/pty_text.rs` turns raw PTY bytes into scrollback lines: CSI/OSC escape
sequences are stripped, bare `\r` clears the current line (so redrawn
prompts/progress bars don't spam duplicate lines), and backspace bytes erase
the last character. That's it — no cursor positioning, no 2D grid, no color.
It makes ordinary shell usage (bash/zsh prompts, line editing, simple
command output) read correctly. Full-screen programs that repaint a grid
(`vim`, `htop`, `less` without `-F`) will render as garbage. A real terminal
emulator (what herdr vendors `libghostty-vt` for) is a much bigger project
and explicitly out of scope here.

## project layout

```
src/
  main.rs     — entry point; dispatches to the TUI or a CLI subcommand
  ui.rs       — TUI: AppState (polls the server), rendering (pane list, detail,
                live pane view, spawn-pane modal, onboarding, status bar)
  pty_text.rs — turns raw PTY bytes into scrollback lines (see "rendering limits"),
                plus keycode → raw-byte mapping for forwarding keystrokes
  palette.rs  — catppuccin color values used across the TUI
  protocol.rs — Request/Response types shared by the CLI client, TUI, and server
                (serde/JSON)
  server.rs   — the pane manager: owns real PTYs (portable-pty), one thread per pane
                for reading output + waiting on exit; listens on a Unix socket
  client.rs   — connects to the server socket, auto-spawning it (detached, via
                setsid) if it isn't already running; used by both the CLI and the TUI
  cli.rs      — argument parsing + printing for spawn/list/send/read/kill
skills/sieg/SKILL.md          — Claude Code skill teaching the pane manager CLI
.github/workflows/release.yml — builds + publishes macOS binaries on tag push
install.sh    — curl-installable script, fetches latest GitHub release binary
```

### TUI ↔ server wiring

- `ui::AppState::refresh()` runs every draw tick (~150ms, or immediately
  after a keypress) — one `Request::List` (sorted by name for stable
  ordering; the server's registry is a `HashMap`, whose iteration order is
  not guaranteed) and, if a pane is selected, one `Request::Read` with
  `lines: None` (the full ~200KB buffer, re-parsed client-side every tick —
  cheap for a local socket, avoids the server's naive newline-count `lines`
  filter cutting mid-redraw).
- Selection is tracked by **pane name**, not index — the registry never
  shrinks (see "no cleanup command" below), but relying on index stability
  into a freshly-fetched, freshly-sorted `Vec` on every tick is fragile;
  name-based lookup isn't.
- Two input modes: **Nav** (arrow keys move selection, letters are
  commands) and **Pane** (every keystroke is mapped to raw bytes via
  `pty_text::key_to_bytes` and sent as-is — no line buffering, no
  client-side echo; the pane's own PTY echoes typed input back into its
  output, which the next `Read` picks up, exactly like a real terminal).
  `ctrl+b` in Pane mode detaches back to Nav — chosen to match the
  onboarding copy's existing `ctrl+b` prefix-key mention, and to leave
  plain `Esc` forwardable to real programs (vim, etc.) instead of stealing
  it as a detach key.

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
6. Wired the TUI to the real pane manager (`v0.3.0`): `src/data.rs` deleted,
   `AppState` now polls the real server, added `src/pty_text.rs` for
   scrollback rendering + keystroke forwarding, added Nav/Pane focus modes
   and a spawn-pane form. Prompted by trying to type into a mock pane and
   having nothing happen — the mockup looked like a real terminal but
   wasn't one.

## next steps (not done yet)

- A real terminal emulator (cursor grid, color) instead of the linear
  scrollback interpreter in `pty_text.rs` — needed for full-screen programs
  to render correctly. Large scope; see "rendering limits" above.
- Real agent state detection (idle/working/blocked) — herdr's version of
  this is a whole manifest-based detection engine; sieg has nothing like it,
  `sieg list` only reports process running/exited.
- A `sieg remove` command to clear exited panes from the registry.
- Multiple panes per screen (split view) — the TUI currently shows one
  focused pane at a time, no tab/workspace grouping.
- Optional: an in-binary update check / `sieg update` command, since
  installs are currently fully manual (see "shipping an update" above).
