# sieg

**Sieg** is a small real terminal multiplexer:

- `sieg` (no args) opens a TUI that shows **real background panes** — spawn
  one, focus it, and you're typing directly into its actual stdin. There is
  no mock data left; the TUI is a client of the same server the CLI talks to.
- `sieg spawn`/`list`/`send`/`read`/`kill` — the CLI side of the same pane
  manager, for scripting or driving it from Claude Code.

Rendering is intentionally simple: no cursor grid, no color passthrough from
the child process — CSI/OSC escape sequences are stripped and `\r`/backspace
are interpreted so ordinary shell usage reads correctly in a linear
scrollback. Full-screen programs (`vim`, `htop`, ...) will look wrong; a real
terminal emulator is future work, not done.

## install

```bash
curl -fsSL https://raw.githubusercontent.com/daymer-perdomo/sieg/main/install.sh | sh
```

macOS only (arm64 and x86_64).

## run from source

```bash
cargo run              # the TUI
cargo run -- list      # the pane manager CLI
```

## TUI

- **Nav mode** (default): `↑`/`↓` or `j`/`k` select a pane · `Enter` focuses
  the selected pane · `n` spawns a new one · `x` kills the selected one ·
  `o` toggles onboarding · `q` quits.
- **Pane focus mode**: keystrokes go straight to the pane's stdin, like a
  real terminal. `ctrl+b` detaches back to nav mode.

## pane manager CLI

```bash
sieg spawn <name> -- <command...>   # start a real background pane
sieg list                           # list panes and their status
sieg send <name> <text...>          # send input to a pane
sieg read <name> [lines]            # read a pane's buffered output
sieg kill <name>                    # terminate a pane
```

The first `sieg spawn` (from either the CLI or the TUI) auto-starts a
detached local server (`~/.sieg/sieg.sock`) that owns the panes; they keep
running independently of whatever shell or TUI session created them, and the
TUI and CLI both see the exact same pane state.

## Claude Code skill

This repo ships a [Claude Code](https://claude.com/claude-code) skill that
teaches Claude to use the pane manager CLI above. Install it globally with:

```bash
npx skills add daymer-perdomo/sieg --skill sieg -g
```

See [`skills/sieg/SKILL.md`](skills/sieg/SKILL.md) for what it covers.

See [`DEVELOPMENT.md`](DEVELOPMENT.md) for project history and the release
process, or open [`docs/project-map.html`](docs/project-map.html) in a
browser for a visual walkthrough of the architecture, the CLI, and the
roadmap.
