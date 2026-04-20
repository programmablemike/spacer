use spacer_core::{Change, Project, Space};
use tabled::Tabled;

// ---------------------------------------------------------------------------
// Presentation row types
// ---------------------------------------------------------------------------

#[derive(Tabled)]
pub struct SpaceRow {
    #[tabled(rename = " ")]
    pub active: char,
    #[tabled(rename = "NAME")]
    pub name: String,
    #[tabled(rename = "PATH")]
    pub path: String,
}

#[derive(Tabled)]
pub struct ProjectRow {
    #[tabled(rename = " ")]
    pub active: char,
    #[tabled(rename = "SPACE")]
    pub space: String,
    #[tabled(rename = "NAME")]
    pub name: String,
    #[tabled(rename = "PATH")]
    pub path: String,
}

#[derive(Tabled)]
pub struct ChangeRow {
    #[tabled(rename = "SPACE")]
    pub space: String,
    #[tabled(rename = "PROJECT")]
    pub project: String,
    #[tabled(rename = "NAME")]
    pub name: String,
}

// ---------------------------------------------------------------------------
// Mapping functions
// ---------------------------------------------------------------------------

pub fn space_rows(spaces: &[Space], active: Option<&str>) -> Vec<SpaceRow> {
    spaces
        .iter()
        .map(|s| SpaceRow {
            active: if active == Some(s.name.as_str()) { '*' } else { ' ' },
            name: s.name.clone(),
            path: s.path.display().to_string(),
        })
        .collect()
}

pub fn project_rows(projects: &[&Project], active: Option<&str>) -> Vec<ProjectRow> {
    projects
        .iter()
        .map(|p| ProjectRow {
            active: if active == Some(p.name.as_str()) { '*' } else { ' ' },
            space: p.space.clone(),
            name: p.name.clone(),
            path: p.path.display().to_string(),
        })
        .collect()
}

pub fn change_rows(changes: &[&Change]) -> Vec<ChangeRow> {
    changes
        .iter()
        .map(|c| ChangeRow {
            space: c.space.clone(),
            project: c.project.clone(),
            name: c.name.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_space(name: &str) -> Space {
        Space { name: name.to_string(), path: PathBuf::from(name) }
    }

    fn make_project(name: &str, space: &str) -> Project {
        Project {
            name: name.to_string(),
            space: space.to_string(),
            path: PathBuf::from(space).join(name),
        }
    }

    fn make_change(name: &str, project: &str, space: &str) -> Change {
        Change {
            name: name.to_string(),
            project: project.to_string(),
            space: space.to_string(),
        }
    }

    #[test]
    fn space_rows_marks_active() {
        let spaces = vec![make_space("alpha"), make_space("beta")];
        let rows = space_rows(&spaces, Some("alpha"));
        assert_eq!(rows[0].active, '*');
        assert_eq!(rows[1].active, ' ');
        assert_eq!(rows[0].name, "alpha");
        assert_eq!(rows[1].name, "beta");
    }

    #[test]
    fn space_rows_no_active() {
        let spaces = vec![make_space("alpha"), make_space("beta")];
        let rows = space_rows(&spaces, None);
        assert!(rows.iter().all(|r| r.active == ' '));
    }

    #[test]
    fn project_rows_marks_active() {
        let projects = vec![make_project("web", "alpha"), make_project("api", "alpha")];
        let refs: Vec<&Project> = projects.iter().collect();
        let rows = project_rows(&refs, Some("web"));
        assert_eq!(rows[0].active, '*');
        assert_eq!(rows[1].active, ' ');
    }

    #[test]
    fn project_rows_path_matches() {
        let projects = vec![make_project("web", "alpha")];
        let refs: Vec<&Project> = projects.iter().collect();
        let rows = project_rows(&refs, None);
        assert_eq!(rows[0].path, "alpha/web");
    }

    #[test]
    fn change_rows_fields() {
        let changes = vec![make_change("feat/login", "web", "alpha")];
        let refs: Vec<&Change> = changes.iter().collect();
        let rows = change_rows(&refs);
        assert_eq!(rows[0].name, "feat/login");
        assert_eq!(rows[0].project, "web");
        assert_eq!(rows[0].space, "alpha");
    }
}
