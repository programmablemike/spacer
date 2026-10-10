use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs},
    Frame,
};

use super::app::{App, Tab};

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // tab bar
            Constraint::Min(0),    // content
            Constraint::Length(1), // help line
        ])
        .split(area);

    // Tab bar
    let tab_titles: Vec<Line> = Tab::ALL
        .iter()
        .map(|t| Line::from(Span::raw(t.title())))
        .collect();
    let selected_tab = Tab::ALL
        .iter()
        .position(|t| *t == app.active_tab)
        .unwrap_or(0);
    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::ALL).title(" Spacer "))
        .select(selected_tab)
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs, chunks[0]);

    // Content
    match app.active_tab {
        Tab::Spaces => render_spaces(frame, app, chunks[1]),
        Tab::Projects => render_projects(frame, app, chunks[1]),
        Tab::Changes => render_changes(frame, app, chunks[1]),
    }

    // Help line
    let help = Paragraph::new(" Read-only snapshot | q: quit  Tab: switch  j/k: select")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[2]);
}

fn render_spaces(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app
        .snapshot
        .spaces
        .iter()
        .map(|s| ListItem::new(format!("{:20} {}", s.name, s.path.display())))
        .collect();

    if items.is_empty() {
        frame.render_widget(
            Paragraph::new("No entries. Use CLI commands to create or register projects.")
                .block(Block::default().borders(Borders::ALL)),
            area,
        );
        return;
    }
    let mut state = ListState::default();
    if !items.is_empty() {
        state.select(Some(app.selected));
    }

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Spaces "))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, area, &mut state);
}

fn render_projects(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app
        .snapshot
        .projects
        .iter()
        .map(|p| ListItem::new(format!("{:20} {:20} {}", p.space, p.name, p.path.display())))
        .collect();

    if items.is_empty() {
        frame.render_widget(
            Paragraph::new("No entries. Use CLI commands to create or register projects.")
                .block(Block::default().borders(Borders::ALL)),
            area,
        );
        return;
    }
    let mut state = ListState::default();
    if !items.is_empty() {
        state.select(Some(app.selected));
    }

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Projects "))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, area, &mut state);
}

fn render_changes(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let mut items: Vec<ListItem> = app
        .snapshot
        .changes
        .iter()
        .map(|c| {
            ListItem::new(format!(
                "{} / {} / {} (legacy tracking)",
                c.space, c.project, c.name
            ))
        })
        .collect();

    items.extend(app.worktrees.iter().map(|entry| {
        ListItem::new(format!(
            "{} / {} / {}  {}",
            entry.space,
            entry.project,
            entry.worktree.branch.as_deref().unwrap_or("(detached)"),
            entry.worktree.path.display()
        ))
    }));
    if items.is_empty() {
        frame.render_widget(
            Paragraph::new("No entries. Use CLI commands to create or register projects.")
                .block(Block::default().borders(Borders::ALL)),
            area,
        );
        return;
    }
    let mut state = ListState::default();
    if !items.is_empty() {
        state.select(Some(app.selected));
    }

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Changes "))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, area, &mut state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};
    use spacer_core::{ProjectWorktree, WorkspaceSnapshot, WorkspaceView, Worktree};
    #[test]
    fn renders_empty_and_detached_worktree_snapshots() {
        let mut terminal = Terminal::new(TestBackend::new(120, 12)).unwrap();
        let mut app = App::new(WorkspaceView {
            registry: WorkspaceSnapshot::default(),
            worktrees: vec![],
        });
        terminal.draw(|frame| draw(frame, &app)).unwrap();
        let screen = format!("{:?}", terminal.backend().buffer());
        assert!(screen.contains("No entries"));
        app.active_tab = Tab::Changes;
        app.worktrees.push(ProjectWorktree {
            space: "alpha".into(),
            project: "web".into(),
            worktree: Worktree {
                path: "/custom-checkout".into(),
                branch: None,
                is_primary: false,
            },
        });
        terminal.draw(|frame| draw(frame, &app)).unwrap();
        let screen = format!("{:?}", terminal.backend().buffer());
        assert!(screen.contains("detached"));
        assert!(screen.contains("custom-checkout"));
    }
}
