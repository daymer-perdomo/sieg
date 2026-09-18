mod data;
mod palette;
mod ui;

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, ExecutableCommand};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use ui::AppState;

fn main() -> io::Result<()> {
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
        terminal.draw(|frame| ui::render(frame, app))?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        if app.show_onboarding {
                            app.show_onboarding = false;
                        } else {
                            return Ok(());
                        }
                    }
                    KeyCode::Char('o') => app.show_onboarding = !app.show_onboarding,
                    KeyCode::Down | KeyCode::Char('j') if !app.show_onboarding => {
                        app.select_workspace(1)
                    }
                    KeyCode::Up | KeyCode::Char('k') if !app.show_onboarding => {
                        app.select_workspace(-1)
                    }
                    KeyCode::Right | KeyCode::Char('l') if !app.show_onboarding => {
                        app.select_tab(1)
                    }
                    KeyCode::Left | KeyCode::Char('h') if !app.show_onboarding => {
                        app.select_tab(-1)
                    }
                    _ => {}
                }
            }
        }
    }
}
