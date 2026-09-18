# sieg-ui-mockup

Visual-only mockup of **Sieg** — layout and color exploration, no real
agent/PTY functionality. Built with `ratatui` + `crossterm`, using catppuccin
palette values (originally copied from [herdr](https://herdr.dev)'s
`Palette::catppuccin()`, the project this mockup's layout was inspired by).
All data (workspaces, tabs, agents) is hardcoded for visual iteration.

## install

```bash
curl -fsSL https://raw.githubusercontent.com/daymer003/sieg-ui-mockup/main/install.sh | sh
```

macOS only (arm64 and x86_64).

## run from source

```bash
cargo run
```

## controls

- `↑`/`↓` or `j`/`k`: switch workspace
- `←`/`→` or `h`/`l`: switch tab
- `o`: toggle the onboarding screen
- `q` / `Esc`: close onboarding, or quit
