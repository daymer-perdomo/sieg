# sieg

**Sieg** has two parts today:

- A **visual-only TUI mockup** (`sieg`, no args) — layout and color
  exploration with hardcoded sample data. Built with `ratatui` + `crossterm`,
  using catppuccin palette values (originally copied from
  [herdr](https://herdr.dev)'s `Palette::catppuccin()`, the project this
  mockup's layout was inspired by).
- A **real background pane manager** (`sieg spawn`/`list`/`send`/`read`/
  `kill`) — a small local server that owns real PTY-backed processes, plus a
  CLI to control them. This part is real, not mocked.

These two don't talk to each other yet — the TUI still only shows fake data.

## install

```bash
curl -fsSL https://raw.githubusercontent.com/daymer-perdomo/sieg/main/install.sh | sh
```

macOS only (arm64 and x86_64).

## run from source

```bash
cargo run              # visual TUI mockup
cargo run -- list      # pane manager CLI
```

## TUI controls

- `↑`/`↓` or `j`/`k`: switch workspace
- `←`/`→` or `h`/`l`: switch tab
- `o`: toggle the onboarding screen
- `q` / `Esc`: close onboarding, or quit

## pane manager CLI

```bash
sieg spawn <name> -- <command...>   # start a real background pane
sieg list                           # list panes and their status
sieg send <name> <text...>          # send input to a pane
sieg read <name> [lines]            # read a pane's buffered output
sieg kill <name>                    # terminate a pane
```

The first `sieg spawn` auto-starts a detached local server
(`~/.sieg/sieg.sock`) that owns the panes; they keep running independently of
whatever shell created them.

## Claude Code skill

This repo ships a [Claude Code](https://claude.com/claude-code) skill that
teaches Claude to use the pane manager CLI above. Install it globally with:

```bash
npx skills add daymer-perdomo/sieg --skill sieg -g
```

See [`skills/sieg/SKILL.md`](skills/sieg/SKILL.md) for what it covers.

See [`DEVELOPMENT.md`](DEVELOPMENT.md) for project history and the release
process.
