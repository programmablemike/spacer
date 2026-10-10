mod app;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use app::App;

pub fn run(snapshot: spacer_core::WorkspaceView) -> Result<()> {
    let previous = std::sync::Arc::new(std::panic::take_hook());
    let hook = previous.clone();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        hook(info);
    }));
    let result = (|| {
        let _guard = TerminalGuard;
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;
        run_app(&mut terminal, snapshot)
    })();
    std::panic::set_hook(Box::new(move |info| previous(info)));
    result
}

struct TerminalGuard;
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore_terminal();
    }
}
fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(
        io::stdout(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        crossterm::cursor::Show
    );
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    snapshot: spacer_core::WorkspaceView,
) -> Result<()> {
    let mut app = App::new(snapshot);

    loop {
        terminal.draw(|frame| ui::draw(frame, &app))?;

        if let Event::Key(key) = event::read()? {
            match (key.modifiers, key.code) {
                (KeyModifiers::CONTROL, KeyCode::Char('c'))
                | (KeyModifiers::NONE, KeyCode::Char('q')) => return Ok(()),
                (_, KeyCode::Down) | (_, KeyCode::Char('j')) => app.next(),
                (_, KeyCode::Up) | (_, KeyCode::Char('k')) => app.prev(),
                (_, KeyCode::Tab) => app.next_tab(),
                _ => {}
            }
        }
    }
}
