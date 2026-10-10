use crate::{Error, Result, WorkspaceSnapshot};
use std::collections::HashSet;

// Portable directory names, independent of the host platform.
pub(super) fn entity_name(name: &str) -> Result<()> {
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    let device = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ["COM", "LPT"].iter().any(|prefix| {
            stem.strip_prefix(prefix).is_some_and(|suffix| {
                matches!(
                    suffix,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
        });
    if name.trim().is_empty()
        || name.starts_with('.')
        || name.ends_with(['.', ' '])
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
        || device
    {
        return Err(Error::InvalidOperation(format!("invalid name '{name}': use a non-hidden portable directory name without separators or reserved characters")));
    }
    Ok(())
}

// Branch-like tracking names also become relative checkout paths. Git still
// validates real branch operations in the adapter.
pub(super) fn change_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name == "HEAD"
        || name.starts_with('-')
        || name.ends_with('.')
        || name.contains("..")
        || name.contains("@{")
        || name
            .chars()
            .any(|c| c.is_control() || " ~^:?*[\\".contains(c))
        || name
            .split('/')
            .any(|part| part.is_empty() || part.starts_with('.') || part.ends_with(".lock"))
    {
        return Err(Error::InvalidOperation(format!(
            "invalid change name '{name}': use a relative Git branch name such as 'feat/login'"
        )));
    }
    // Check path portability as well as Git syntax.
    for part in name.split('/') {
        entity_name(part)?;
    }
    Ok(())
}

pub(super) fn validate_state(state: &WorkspaceSnapshot) -> Result<()> {
    let invalid = |detail: String| {
        Error::InvalidOperation(format!("invalid workspace registry: {detail}; repair the configuration before continuing (no data was changed)"))
    };
    let mut spaces = HashSet::new();
    for s in &state.spaces {
        entity_name(&s.name).map_err(|e| invalid(e.to_string()))?;
        if !spaces.insert(s.name.as_str()) {
            return Err(invalid(format!("duplicate space '{}'", s.name)));
        }
    }
    let mut projects = HashSet::new();
    for p in &state.projects {
        entity_name(&p.name).map_err(|e| invalid(e.to_string()))?;
        if !spaces.contains(p.space.as_str()) {
            return Err(invalid(format!(
                "project '{}' refers to missing space '{}'",
                p.name, p.space
            )));
        }
        if !projects.insert((p.space.as_str(), p.name.as_str())) {
            return Err(invalid(format!(
                "duplicate project '{}/{}'",
                p.space, p.name
            )));
        }
    }
    let mut changes = HashSet::new();
    for c in &state.changes {
        change_name(&c.name).map_err(|e| invalid(e.to_string()))?;
        if !projects.contains(&(c.space.as_str(), c.project.as_str())) {
            return Err(invalid(format!(
                "change '{}' refers to missing project '{}/{}'",
                c.name, c.space, c.project
            )));
        }
        if !changes.insert((&c.space, &c.project, &c.name)) {
            return Err(invalid(format!(
                "duplicate change '{}/{}/{}'",
                c.space, c.project, c.name
            )));
        }
    }
    Ok(())
}

pub(super) fn clear_stale_context(state: &mut WorkspaceSnapshot) {
    if !state
        .spaces
        .iter()
        .any(|s| Some(&s.name) == state.active_space.as_ref())
    {
        state.active_space = None;
        state.active_project = None;
    } else if !state.projects.iter().any(|p| {
        Some(&p.space) == state.active_space.as_ref()
            && Some(&p.name) == state.active_project.as_ref()
    }) {
        state.active_project = None;
    }
}
