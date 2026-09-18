use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::data::MockWorkspace;
use crate::palette::Palette;

pub struct AppState {
    pub workspaces: Vec<MockWorkspace>,
    pub selected_workspace: usize,
    pub selected_tab: usize,
    pub show_onboarding: bool,
    pub palette: Palette,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            workspaces: crate::data::mock_workspaces(),
            selected_workspace: 0,
            selected_tab: 0,
            show_onboarding: true,
            palette: Palette::catppuccin(),
        }
    }

    pub fn select_workspace(&mut self, delta: i32) {
        let len = self.workspaces.len() as i32;
        if len == 0 {
            return;
        }
        let next = (self.selected_workspace as i32 + delta).rem_euclid(len);
        self.selected_workspace = next as usize;
        self.selected_tab = 0;
    }

    pub fn select_tab(&mut self, delta: i32) {
        let tabs = self.workspaces[self.selected_workspace].tabs.len() as i32;
        if tabs == 0 {
            return;
        }
        let next = (self.selected_tab as i32 + delta).rem_euclid(tabs);
        self.selected_tab = next as usize;
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
        .constraints([Constraint::Length(30), Constraint::Min(1)])
        .split(root[0]);

    render_sidebar(frame, body[0], app);
    render_main(frame, body[1], app);
    render_status_bar(frame, root[1], app);

    if app.show_onboarding {
        render_onboarding(frame, area, app);
    }
}

fn render_sidebar(frame: &mut Frame, area: Rect, app: &AppState) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    render_workspace_list(frame, sections[0], app);
    render_agent_panel(frame, sections[1], app);
}

fn render_workspace_list(frame: &mut Frame, area: Rect, app: &AppState) {
    let p = &app.palette;
    let block = Block::default()
        .borders(Borders::RIGHT | Borders::BOTTOM)
        .border_style(Style::default().fg(p.surface_dim))
        .title(Span::styled(
            " workspaces ",
            Style::default().fg(p.overlay1),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines: Vec<Line> = app
        .workspaces
        .iter()
        .enumerate()
        .map(|(idx, ws)| {
            let active = idx == app.selected_workspace;
            let bg = if active { p.active_row_bg } else { p.panel_bg };
            let name_style = Style::default().bg(bg).fg(if active { p.text } else { p.subtext0 });
            let mut spans = vec![
                Span::styled(format!(" {} ", idx + 1), Style::default().bg(bg).fg(p.overlay0)),
                Span::styled(ws.name, name_style.add_modifier(if active {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                })),
                Span::styled("  ", Style::default().bg(bg)),
                Span::styled(ws.branch, Style::default().bg(bg).fg(p.mauve)),
            ];
            if ws.ahead > 0 {
                spans.push(Span::styled(
                    format!(" ↑{}", ws.ahead),
                    Style::default().bg(bg).fg(p.green),
                ));
            }
            if ws.behind > 0 {
                spans.push(Span::styled(
                    format!(" ↓{}", ws.behind),
                    Style::default().bg(bg).fg(p.red),
                ));
            }
            Line::from(spans).style(Style::default().bg(bg))
        })
        .collect();

    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_agent_panel(frame: &mut Frame, area: Rect, app: &AppState) {
    let p = &app.palette;
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(p.surface_dim))
        .title(Span::styled(" agents ", Style::default().fg(p.overlay1)));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let ws = &app.workspaces[app.selected_workspace];
    let mut lines = Vec::new();
    for tab in &ws.tabs {
        lines.push(Line::from(Span::styled(
            format!(" {}", tab.name),
            Style::default().fg(p.overlay0),
        )));
        for pane in &tab.panes {
            let state_style = Style::default().fg(match pane.state {
                crate::data::AgentState::Idle => p.overlay1,
                crate::data::AgentState::Working => p.yellow,
                crate::data::AgentState::Blocked => p.red,
                crate::data::AgentState::Done => p.green,
            });
            lines.push(Line::from(vec![
                Span::raw("   "),
                Span::styled(pane.state.icon(), state_style),
                Span::raw(" "),
                Span::styled(pane.agent_name, Style::default().fg(p.text)),
                Span::styled(format!("  {}", pane.state.label()), state_style),
            ]));
        }
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_main(frame: &mut Frame, area: Rect, app: &AppState) {
    let p = &app.palette;
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(area);

    render_tab_bar(frame, layout[0], app);

    let ws = &app.workspaces[app.selected_workspace];
    let tab = &ws.tabs[app.selected_tab];
    let panes = &tab.panes;
    let constraints: Vec<Constraint> = panes
        .iter()
        .map(|_| Constraint::Ratio(1, panes.len().max(1) as u32))
        .collect();
    let pane_areas = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(layout[1]);

    for (pane, pane_area) in panes.iter().zip(pane_areas.iter()) {
        let state_color = match pane.state {
            crate::data::AgentState::Idle => p.overlay1,
            crate::data::AgentState::Working => p.yellow,
            crate::data::AgentState::Blocked => p.red,
            crate::data::AgentState::Done => p.green,
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(p.surface1))
            .title(Line::from(vec![
                Span::styled(format!(" {} ", pane.state.icon()), Style::default().fg(state_color)),
                Span::styled(pane.agent_name, Style::default().fg(p.text)),
                Span::raw(" "),
            ]));
        let inner = block.inner(*pane_area);
        frame.render_widget(block, *pane_area);

        let content_lines: Vec<Line> = pane
            .content
            .iter()
            .map(|line| Line::from(Span::styled(*line, Style::default().fg(p.text))))
            .collect();
        frame.render_widget(Paragraph::new(content_lines), inner);
    }
}

fn render_tab_bar(frame: &mut Frame, area: Rect, app: &AppState) {
    let p = &app.palette;
    let ws = &app.workspaces[app.selected_workspace];
    let mut spans = Vec::new();
    for (idx, tab) in ws.tabs.iter().enumerate() {
        let active = idx == app.selected_tab;
        let style = if active {
            Style::default().fg(p.text).bg(p.selection_bg).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.subtext0).bg(p.panel_bg)
        };
        spans.push(Span::styled(format!(" {} ", tab.name), style));
    }
    let line = Line::from(spans).style(Style::default().bg(p.panel_bg));
    frame.render_widget(Paragraph::new(line), area);
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &AppState) {
    let p = &app.palette;
    let line = Line::from(vec![
        Span::styled(" sieg-ui-mockup ", Style::default().fg(p.accent)),
        Span::styled(
            "· ↑/↓ workspace · ←/→ tab · o onboarding · q quit",
            Style::default().fg(p.overlay0),
        ),
    ])
    .style(Style::default().bg(p.panel_bg));
    frame.render_widget(Paragraph::new(line), area);
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
            "  terminal workspace manager for coding agents",
            Style::default().fg(p.subtext0),
        )),
        Line::from(""),
    ];
    for line in [
        "  this is a mouse-first terminal.",
        "  click the sidebar to switch workspaces, drag pane",
        "  borders to resize, right-click for context menus.",
    ] {
        lines.push(Line::from(Span::styled(line, Style::default().fg(p.text))));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  ctrl+b", Style::default().fg(p.mauve)),
        Span::styled(" enters prefix mode · ", Style::default().fg(p.overlay0)),
        Span::styled("?", Style::default().fg(p.mauve)),
        Span::styled(" shows keybinds and settings", Style::default().fg(p.overlay0)),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  press o to close",
        Style::default().fg(p.overlay1),
    )));

    frame.render_widget(Paragraph::new(lines), inner);
    let _ = app;
}
