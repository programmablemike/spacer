use spacer_core::Config;

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
    pub config: Config,
    pub active_tab: Tab,
    pub selected: usize,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self { config, active_tab: Tab::Spaces, selected: 0 }
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
            Tab::Spaces => self.config.spaces.len(),
            Tab::Projects => self.config.projects.len(),
            Tab::Changes => self.config.changes.len(),
        }
    }
}
