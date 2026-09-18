# herdr-ui-mockup

Visual-only mockup of the [herdr](https://herdr.dev) TUI — layout and color
exploration, no real agent/PTY functionality. Built with `ratatui` +
`crossterm`, using herdr's real catppuccin palette values. All data
(workspaces, tabs, agents) is hardcoded for visual iteration.

## install

```bash
curl -fsSL https://raw.githubusercontent.com/daymer003/herdr-ui-mockup/main/install.sh | sh
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
