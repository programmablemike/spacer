use spacer_core::{ProjectWorktree, WorkspaceSnapshot, WorkspaceView};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Spaces,
    Projects,
    Changes,
}

impl Tab {
    pub const ALL: &'static [Tab] = &[Tab::Spaces, Tab::Projects, Tab::Changes];

    pub fn title(self) -> &'static str {
        match self {
            Tab::Spaces => "Spaces",
            Tab::Projects => "Projects",
            Tab::Changes => "Changes",
        }
    }

    pub fn next(self) -> Tab {
        match self {
            Tab::Spaces => Tab::Projects,
            Tab::Projects => Tab::Changes,
            Tab::Changes => Tab::Spaces,
        }
    }
}

pub struct App {
    pub snapshot: WorkspaceSnapshot,
    pub worktrees: Vec<ProjectWorktree>,
    pub active_tab: Tab,
    pub selected: usize,
}

impl App {
    pub fn new(view: WorkspaceView) -> Self {
        Self {
            snapshot: view.registry,
            worktrees: view.worktrees,
            active_tab: Tab::Spaces,
            selected: 0,
        }
    }

    pub fn next_tab(&mut self) {
        self.active_tab = self.active_tab.next();
        self.selected = 0;
    }

    pub fn next(&mut self) {
        let len = self.list_len();
        if len > 0 {
            self.selected = (self.selected + 1) % len;
        }
    }

    pub fn prev(&mut self) {
        let len = self.list_len();
        if len > 0 {
            self.selected = self.selected.saturating_sub(1);
        }
    }

    fn list_len(&self) -> usize {
        match self.active_tab {
            Tab::Spaces => self.snapshot.spaces.len(),
            Tab::Projects => self.snapshot.projects.len(),
            Tab::Changes => self.snapshot.changes.len() + self.worktrees.len(),
        }
    }
}
