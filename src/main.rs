mod cli;
mod client;
mod palette;
mod protocol;
mod pty_text;
mod server;
mod ui;

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, ExecutableCommand};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use ui::{default_shell, AppState, Focus, SpawnField, SpawnForm};

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None => run_tui(),
        Some("__serve") => server::run(),
        Some("spawn") => cli::spawn(&args[1..]),
        Some("list") => cli::list(),
        Some("send") => cli::send(&args[1..]),
        Some("read") => cli::read(&args[1..]),
        Some("kill") => cli::kill(&args[1..]),
        Some("help" | "-h" | "--help") => {
            cli::print_help();
            Ok(())
        }
        Some(other) => {
            eprintln!("unknown command: {other}\n");
            cli::print_help();
            std::process::exit(2);
        }
    }
}

fn run_tui() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = AppState::new();
    let result = run(&mut terminal, &mut app);

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut AppState) -> io::Result<()> {
    loop {
        app.refresh();
        terminal.draw(|frame| ui::render(frame, app))?;

        if event::poll(Duration::from_millis(150))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if handle_key(app, key) {
                    return Ok(());
                }
            }
        }
    }
}

/// Returns `true` if the app should quit.
fn handle_key(app: &mut AppState, key: KeyEvent) -> bool {
    if let Some(form) = &mut app.spawn_form {
        match key.code {
            KeyCode::Esc => app.spawn_form = None,
            KeyCode::Tab => {
                form.field = match form.field {
                    SpawnField::Name => SpawnField::Command,
                    SpawnField::Command => SpawnField::Name,
                };
            }
            KeyCode::Enter => app.submit_spawn(),
            KeyCode::Backspace => match form.field {
                SpawnField::Name => {
                    form.name.pop();
                }
                SpawnField::Command => {
                    form.command.pop();
                }
            },
            KeyCode::Char(c) => match form.field {
                SpawnField::Name => form.name.push(c),
                SpawnField::Command => form.command.push(c),
            },
            _ => {}
        }
        return false;
    }

    if app.show_onboarding {
        match key.code {
            KeyCode::Char('o') | KeyCode::Esc => app.show_onboarding = false,
            KeyCode::Char('q') => return true,
            _ => {}
        }
        return false;
    }

    match app.focus {
        Focus::Nav => match key.code {
            KeyCode::Char('q') => return true,
            KeyCode::Char('o') => app.show_onboarding = true,
            KeyCode::Up | KeyCode::Char('k') => app.move_selection(-1),
            KeyCode::Down | KeyCode::Char('j') => app.move_selection(1),
            KeyCode::Enter if app.selected_name.is_some() => app.focus = Focus::Pane,
            KeyCode::Char('n') => {
                app.spawn_form = Some(SpawnForm {
                    field: SpawnField::Name,
                    name: String::new(),
                    command: default_shell(),
                });
            }
            KeyCode::Char('x') => app.kill_selected(),
            _ => {}
        },
        Focus::Pane => {
            let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
            if ctrl && key.code == KeyCode::Char('b') {
                app.focus = Focus::Nav;
                return false;
            }
            if let Some(bytes) = pty_text::key_to_bytes(key.code, ctrl) {
                app.send_bytes(bytes);
            }
        }
    }

    false
}
