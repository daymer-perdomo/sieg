use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::client::send_request;
use crate::palette::Palette;
use crate::protocol::{PaneInfo, PaneStatus, Request};
use crate::pty_text::{render_pty_lines, AnsiColor, Span as PtySpan};

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Focus {
    /// Arrow keys move the sidebar selection; letters are commands.
    Nav,
    /// Keystrokes are forwarded to the selected pane's stdin.
    Pane,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SpawnField {
    Name,
    Command,
}

pub struct SpawnForm {
    pub field: SpawnField,
    pub name: String,
    pub command: String,
}

/// The shell a real terminal would launch for the user — used to prefill
/// the spawn form's command field so a fresh pane behaves like Terminal.app
/// (cwd-aware prompt, no "default shell is now zsh" nag) instead of the
/// bare, unconfigured `bash` a blank field would otherwise invite.
pub fn default_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string())
}

/// Shortens a pane's cwd the way a shell prompt would (`/Users/x/Documents`
/// -> `~/Documents`), so the highlighted path in the title bar stays short.
fn abbreviate_home(path: &str) -> String {
    match std::env::var("HOME") {
        Ok(home) if !home.is_empty() && path == home => "~".to_string(),
        Ok(home) if !home.is_empty() => match path.strip_prefix(&home) {
            Some(rest) if rest.starts_with('/') => format!("~{rest}"),
            _ => path.to_string(),
        },
        _ => path.to_string(),
    }
}

pub struct AppState {
    pub palette: Palette,
    pub show_onboarding: bool,
    pub focus: Focus,
    pub panes: Vec<PaneInfo>,
    pub selected_name: Option<String>,
    pub pane_lines: Vec<Vec<PtySpan>>,
    pub pane_cwd: Option<String>,
    pub spawn_form: Option<SpawnForm>,
    pub status_message: Option<String>,
}

impl AppState {
    pub fn new() -> Self {
        let mut app = Self {
            palette: Palette::catppuccin(),
            show_onboarding: true,
            focus: Focus::Nav,
            panes: Vec::new(),
            selected_name: None,
            pane_lines: Vec::new(),
            pane_cwd: None,
            spawn_form: None,
            status_message: None,
        };
        app.refresh();
        app
    }

    /// Re-fetches the pane list and the selected pane's output from the
    /// real server. Called every draw tick so the view stays live.
    pub fn refresh(&mut self) {
        match send_request(&Request::List) {
            Ok(response) if response.ok => {
                let mut panes = response.panes;
                panes.sort_by(|a, b| a.name.cmp(&b.name));
                self.panes = panes;
                if self.selected_name.is_none() {
                    self.selected_name = self.panes.first().map(|p| p.name.clone());
                }
            }
            Ok(response) => {
                self.status_message = response.error.or(Some("failed to list panes".into()));
            }
            Err(err) => {
                self.status_message = Some(format!("server unreachable: {err}"));
            }
        }

        match &self.selected_name {
            Some(name) => match send_request(&Request::Read {
                name: name.clone(),
                lines: None,
            }) {
                Ok(response) if response.ok => {
                    self.pane_lines = render_pty_lines(response.output.unwrap_or_default().as_bytes());
                    self.pane_cwd = response.cwd.map(|cwd| abbreviate_home(&cwd));
                }
                _ => {}
            },
            None => {
                self.pane_lines = Vec::new();
                self.pane_cwd = None;
            }
        };
    }

    fn selected_index(&self) -> Option<usize> {
        let name = self.selected_name.as_ref()?;
        self.panes.iter().position(|p| &p.name == name)
    }

    pub fn move_selection(&mut self, delta: i32) {
        if self.panes.is_empty() {
            return;
        }
        let len = self.panes.len() as i32;
        let current = self.selected_index().unwrap_or(0) as i32;
        let next = (current + delta).rem_euclid(len) as usize;
        self.selected_name = Some(self.panes[next].name.clone());
    }

    pub fn kill_selected(&mut self) {
        let Some(name) = self.selected_name.clone() else {
            return;
        };
        match send_request(&Request::Kill { name: name.clone() }) {
            Ok(response) if response.ok => {
                self.status_message = Some(format!("killed {name:?}"));
            }
            Ok(response) => {
                self.status_message = response.error;
            }
            Err(err) => {
                self.status_message = Some(format!("kill failed: {err}"));
            }
        }
    }

    pub fn submit_spawn(&mut self) {
        let Some(form) = &self.spawn_form else { return };
        let name = form.name.trim().to_string();
        let command: Vec<String> = form.command.split_whitespace().map(String::from).collect();
        if name.is_empty() || command.is_empty() {
            self.status_message = Some("name and command are both required".into());
            return;
        }

        match send_request(&Request::Spawn {
            name: name.clone(),
            command,
        }) {
            Ok(response) if response.ok => {
                self.status_message = Some(format!("spawned {name:?}"));
                self.selected_name = Some(name);
                self.spawn_form = None;
                self.focus = Focus::Pane;
            }
            Ok(response) => {
                self.status_message = response.error;
            }
            Err(err) => {
                self.status_message = Some(format!("spawn failed: {err}"));
            }
        }
    }

    pub fn send_bytes(&mut self, bytes: Vec<u8>) {
        let Some(name) = self.selected_name.clone() else {
            return;
        };
        let text = String::from_utf8_lossy(&bytes).into_owned();
        if let Err(err) = send_request(&Request::Send { name, text }) {
            self.status_message = Some(format!("send failed: {err}"));
        }
    }
}

pub fn render(frame: &mut Frame, app: &AppState) {
    let area = frame.area();
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(32), Constraint::Min(1)])
        .split(root[0]);

    render_sidebar(frame, body[0], app);
    render_main(frame, body[1], app);
    render_status_bar(frame, root[1], app);

    if let Some(form) = &app.spawn_form {
        render_spawn_form(frame, area, app, form);
    } else if app.show_onboarding {
        render_onboarding(frame, area, app);
    }
}

fn render_sidebar(frame: &mut Frame, area: Rect, app: &AppState) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);
    render_pane_list(frame, sections[0], app);
    render_pane_detail(frame, sections[1], app);
}

fn status_glyph(status: &PaneStatus, p: &Palette) -> (&'static str, ratatui::style::Color) {
    match status {
        PaneStatus::Running => ("◐", p.yellow),
        PaneStatus::Exited { code: 0 } => ("✓", p.green),
        PaneStatus::Exited { .. } => ("⚠", p.red),
    }
}

fn render_pane_list(frame: &mut Frame, area: Rect, app: &AppState) {
    let p = &app.palette;
    let block = Block::default()
        .borders(Borders::RIGHT | Borders::BOTTOM)
        .border_style(Style::default().fg(p.surface_dim))
        .title(Span::styled(" panes ", Style::default().fg(p.overlay1)));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.panes.is_empty() {
        let hint = Paragraph::new(vec![
            Line::from(Span::styled(" no panes yet", Style::default().fg(p.overlay0))),
            Line::from(Span::styled(" press n to spawn one", Style::default().fg(p.overlay0))),
        ]);
        frame.render_widget(hint, inner);
        return;
    }

    let selected = app.selected_name.as_deref();
    let lines: Vec<Line> = app
        .panes
        .iter()
        .map(|pane| {
            let active = selected == Some(pane.name.as_str());
            let bg = if active { p.active_row_bg } else { p.panel_bg };
            let (icon, icon_color) = status_glyph(&pane.status, p);
            Line::from(vec![
                Span::styled(" ", Style::default().bg(bg)),
                Span::styled(icon, Style::default().bg(bg).fg(icon_color)),
                Span::styled(" ", Style::default().bg(bg)),
                Span::styled(
                    pane.name.clone(),
                    Style::default()
                        .bg(bg)
                        .fg(if active { p.text } else { p.subtext0 })
                        .add_modifier(if active { Modifier::BOLD } else { Modifier::empty() }),
                ),
            ])
            .style(Style::default().bg(bg))
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_pane_detail(frame: &mut Frame, area: Rect, app: &AppState) {
    let p = &app.palette;
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(p.surface_dim))
        .title(Span::styled(" detail ", Style::default().fg(p.overlay1)));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(pane) = app
        .selected_name
        .as_deref()
        .and_then(|name| app.panes.iter().find(|p| p.name == name))
    else {
        frame.render_widget(
            Paragraph::new(Span::styled(" nothing selected", Style::default().fg(p.overlay0))),
            inner,
        );
        return;
    };

    let (icon, icon_color) = status_glyph(&pane.status, p);
    let status_text = match pane.status {
        PaneStatus::Running => "running".to_string(),
        PaneStatus::Exited { code } => format!("exited({code})"),
    };

    let lines = vec![
        Line::from(vec![
            Span::raw(" "),
            Span::styled(icon, Style::default().fg(icon_color)),
            Span::raw(" "),
            Span::styled(status_text, Style::default().fg(icon_color)),
        ]),
        Line::from(""),
        Line::from(Span::styled(" command", Style::default().fg(p.overlay0))),
        Line::from(Span::styled(
            format!(" {}", pane.command.join(" ")),
            Style::default().fg(p.text),
        )),
    ];
    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_main(frame: &mut Frame, area: Rect, app: &AppState) {
    let p = &app.palette;
    let border_color = if app.focus == Focus::Pane { p.accent } else { p.surface1 };

    let title: Line = match app.selected_name.as_deref() {
        Some(name) => {
            let mode = if app.focus == Focus::Pane { "typing — ctrl+b to detach" } else { "enter to focus" };
            let mut spans = vec![Span::styled(format!(" {name} "), Style::default().fg(p.text))];
            // Highlight the pane's current directory in the title bar — the
            // one visible, always-on answer to "which folder am I in".
            if let Some(cwd) = &app.pane_cwd {
                spans.push(Span::styled("· ", Style::default().fg(p.overlay0)));
                spans.push(Span::styled(
                    cwd.clone(),
                    Style::default().fg(p.teal).add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::raw(" "));
            }
            spans.push(Span::styled(format!("· {mode} "), Style::default().fg(p.text)));
            Line::from(spans)
        }
        None => Line::from(Span::styled(" no pane selected ", Style::default().fg(p.text))),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.selected_name.is_none() {
        let hint = Paragraph::new(Span::styled(
            " press n in the sidebar to spawn a real background pane",
            Style::default().fg(p.overlay0),
        ));
        frame.render_widget(hint, inner);
        return;
    }

    let height = inner.height as usize;
    let start = app.pane_lines.len().saturating_sub(height);
    let visible = &app.pane_lines[start..];
    let lines: Vec<Line> = visible
        .iter()
        .map(|line| {
            let spans: Vec<Span> = line
                .iter()
                .map(|span| {
                    let mut style = Style::default().fg(ansi_color(span.style.color, p));
                    if span.style.bold {
                        style = style.add_modifier(Modifier::BOLD);
                    }
                    Span::styled(span.text.clone(), style)
                })
                .collect();
            Line::from(spans)
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);
}

/// Maps a pane's SGR-parsed color onto the TUI's own catppuccin palette,
/// so a shell prompt's `\e[36m` (say, highlighting the cwd) reads as an
/// on-theme accent instead of a raw ANSI color clashing with the rest of
/// the UI.
fn ansi_color(color: Option<AnsiColor>, p: &Palette) -> Color {
    match color {
        None => p.text,
        Some(AnsiColor::Black) => p.surface1,
        Some(AnsiColor::Red) => p.red,
        Some(AnsiColor::Green) => p.green,
        Some(AnsiColor::Yellow) => p.yellow,
        Some(AnsiColor::Blue) => p.blue,
        Some(AnsiColor::Magenta) => p.mauve,
        Some(AnsiColor::Cyan) => p.teal,
        Some(AnsiColor::White) => p.text,
    }
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &AppState) {
    let p = &app.palette;
    let hint = match (app.focus, app.spawn_form.is_some()) {
        (_, true) => "tab switch field · enter spawn · esc cancel".to_string(),
        (Focus::Nav, false) => "↑/↓ select · enter focus · n spawn · x kill · o onboarding · q quit".to_string(),
        (Focus::Pane, false) => "typing → pane · ctrl+b detach to nav".to_string(),
    };
    let mut spans = vec![
        Span::styled(" sieg ", Style::default().fg(p.accent)),
        Span::styled(format!("· {hint}"), Style::default().fg(p.overlay0)),
    ];
    if let Some(message) = &app.status_message {
        spans.push(Span::styled("  —  ", Style::default().fg(p.overlay0)));
        spans.push(Span::styled(message.clone(), Style::default().fg(p.peach)));
    }
    let line = Line::from(spans).style(Style::default().bg(p.panel_bg));
    frame.render_widget(Paragraph::new(line), area);
}

fn render_spawn_form(frame: &mut Frame, area: Rect, app: &AppState, form: &SpawnForm) {
    let p = &app.palette;
    let width = 56.min(area.width.saturating_sub(4));
    let height = 9.min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let modal_area = Rect::new(x, y, width, height);

    frame.render_widget(Clear, modal_area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.accent))
        .style(Style::default().bg(p.panel_bg))
        .title(Span::styled(" spawn pane ", Style::default().fg(p.text)));
    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let field_style = |active: bool| {
        if active {
            Style::default().fg(p.text).bg(p.selection_bg)
        } else {
            Style::default().fg(p.text).bg(p.surface0)
        }
    };

    let lines = vec![
        Line::from(Span::styled("name", Style::default().fg(p.overlay0))),
        Line::from(Span::styled(
            format!(" {} ", form.name),
            field_style(form.field == SpawnField::Name),
        )),
        Line::from(""),
        Line::from(Span::styled("command", Style::default().fg(p.overlay0))),
        Line::from(Span::styled(
            format!(" {} ", form.command),
            field_style(form.field == SpawnField::Command),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "no quoting yet — split on whitespace",
            Style::default().fg(p.overlay0),
        )),
    ];
    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_onboarding(frame: &mut Frame, area: Rect, app: &AppState) {
    let p = &app.palette;
    let width = 56.min(area.width.saturating_sub(4));
    let height = 11.min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let modal_area = Rect::new(x, y, width, height);

    frame.render_widget(Clear, modal_area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.accent))
        .style(Style::default().bg(p.panel_bg));
    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let mut lines = vec![
        Line::from(Span::styled(
            "  Sieg",
            Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  real background panes, no PTY simulation",
            Style::default().fg(p.subtext0),
        )),
        Line::from(""),
    ];
    for line in [
        "  n spawns a real background process.",
        "  enter focuses it — you're typing into the",
        "  real thing. ctrl+b detaches back to nav.",
    ] {
        lines.push(Line::from(Span::styled(line, Style::default().fg(p.text))));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  press o to close",
        Style::default().fg(p.overlay1),
    )));

    frame.render_widget(Paragraph::new(lines), inner);
}
