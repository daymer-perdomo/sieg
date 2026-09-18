---
name: sieg
description: Manage real background terminal panes with the sieg CLI — spawn a process, send it input, read its output, check whether it's still running, or kill it. Use this whenever the user asks to run something in the background, check on a long-running command, or coordinate multiple concurrent processes without blocking the current shell.
---

# sieg

`sieg` is a small background pane manager. `sieg spawn` starts a real
PTY-backed process owned by a local `sieg` server (not a child of your
current shell), so it keeps running independently — you can check on it,
feed it input, or kill it later, from any shell on this machine.

## Commands

- `sieg spawn <name> -- <command...>` — start a background pane running
  `<command...>`, tracked under `<name>`. Fails if `<name>` is already
  registered (running or exited).
- `sieg list` — list all panes with their status (`running` or
  `exited(<code>)`) and the command they were started with.
- `sieg send <name> <text...>` — write `<text...>` plus a trailing newline
  to the pane's stdin, as if typed into its terminal.
- `sieg read <name> [lines]` — print the pane's buffered output (stdout +
  stderr, PTY-merged, capped at ~200KB). Pass `lines` to only print the
  last N lines.
- `sieg kill <name>` — terminate the pane's process. This does not remove
  it from the list; it just stops the process (status becomes `exited`).

## When to use this

- The user asks you to run something long-running (a dev server, a build
  watch, a test suite) without blocking the conversation: `sieg spawn` it,
  then `sieg read` back later to check progress instead of running it
  inline and waiting.
- You need to run several things concurrently and check on each
  independently: give each a distinct `<name>`.
- A background pane appears stuck, or you started it by mistake:
  `sieg kill` it.
- You need to answer an interactive prompt in a running background process:
  `sieg send <name> <answer>`.

## Constraints

- There is currently no `sieg remove` — an exited pane's name stays taken
  until the server restarts. If you need to re-run the same logical task,
  pick a new name (e.g. append a counter) rather than trying to reuse one
  that already exists.
- `sieg` only tracks pane lifecycle and raw output. It does not detect
  "idle / working / blocked" agent state — that's not built yet. Judge
  whether a pane needs attention from its actual output via `sieg read`.
- This only works on the machine where the `sieg` server is running; there
  is no remote/SSH support.
