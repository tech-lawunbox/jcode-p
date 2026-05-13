use crate::workspace_project::{ProjectEntry, WorkspaceConfig};
use std::path::PathBuf;

/// Active workspace state in the TUI
#[derive(Debug, Clone)]
pub struct ActiveWorkspace {
    config: WorkspaceConfig,
    /// Currently focused project index
    focused_project: usize,
    /// Project session cards loaded from disk
    project_sessions: Vec<Vec<crate::workspace::SessionCard>>,
}

impl ActiveWorkspace {
    /// Create new active workspace from config
    pub fn new(config: WorkspaceConfig) -> Self {
        let project_count = config.projects.len();
        Self {
            focused_project: 0,
            project_sessions: vec![vec![]; project_count],
            config,
        }
    }

    /// Workspace name
    pub fn name(&self) -> &str {
        &self.config.name
    }

    /// All projects in workspace
    pub fn projects(&self) -> &[ProjectEntry] {
        &self.config.projects
    }

    /// Currently focused project
    pub fn focused_project(&self) -> Option<&ProjectEntry> {
        self.config.projects.get(self.focused_project)
    }

    /// Focused project index
    pub fn focused_project_index(&self) -> usize {
        self.focused_project
    }

    /// Move focus to next project
    pub fn focus_next_project(&mut self) -> bool {
        if self.focused_project < self.config.projects.len().saturating_sub(1) {
            self.focused_project += 1;
            true
        } else {
            false
        }
    }

    /// Move focus to previous project
    pub fn focus_previous_project(&mut self) -> bool {
        if self.focused_project > 0 {
            self.focused_project -= 1;
            true
        } else {
            false
        }
    }

    /// Set focus by index
    pub fn set_focused_project(&mut self, index: usize) {
        if index < self.config.projects.len() {
            self.focused_project = index;
        }
    }

    /// Project count
    pub fn project_count(&self) -> usize {
        self.config.projects.len()
    }

    /// Check if workspace has any projects
    pub fn is_empty(&self) -> bool {
        self.config.projects.is_empty()
    }

    /// Get workspace config (immutable reference)
    pub fn config(&self) -> &WorkspaceConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_workspace_activation_state() {
        let temp_dir = std::env::temp_dir().join("tui_workspace_test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Create a workspace config directly
        let mut config = WorkspaceConfig::new(
            "TestWS".to_string(),
            temp_dir.clone(),
        );
        // Add a project
        config.add_project("project1".to_string(), temp_dir.join("project1"));
        fs::create_dir_all(temp_dir.join("project1")).unwrap();

        // Create workspace state
        let state = ActiveWorkspace::new(config);
        assert_eq!(state.name(), "TestWS");
        assert_eq!(state.projects().len(), 1);
        assert_eq!(state.focused_project_index(), 0);

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_project_focus_navigation() {
        let temp_dir = std::env::temp_dir().join("focus_nav_test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let mut config = WorkspaceConfig::new("TestWS".to_string(), temp_dir.clone());
        config.add_project("p1".to_string(), temp_dir.join("p1"));
        config.add_project("p2".to_string(), temp_dir.join("p2"));
        config.add_project("p3".to_string(), temp_dir.join("p3"));
        fs::create_dir_all(temp_dir.join("p1")).unwrap();
        fs::create_dir_all(temp_dir.join("p2")).unwrap();
        fs::create_dir_all(temp_dir.join("p3")).unwrap();

        let mut state = ActiveWorkspace::new(config);

        // Start at 0
        assert_eq!(state.focused_project_index(), 0);

        // Next -> 1
        assert!(state.focus_next_project());
        assert_eq!(state.focused_project_index(), 1);

        // Next -> 2
        assert!(state.focus_next_project());
        assert_eq!(state.focused_project_index(), 2);

        // Next -> stays at 2 (already at end)
        assert!(!state.focus_next_project());
        assert_eq!(state.focused_project_index(), 2);

        // Previous -> 1
        assert!(state.focus_previous_project());
        assert_eq!(state.focused_project_index(), 1);

        // Previous -> 0
        assert!(state.focus_previous_project());
        assert_eq!(state.focused_project_index(), 0);

        // Previous -> stays at 0
        assert!(!state.focus_previous_project());

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_set_focused_project() {
        let temp_dir = std::env::temp_dir().join("set_focus_test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let mut config = WorkspaceConfig::new("TestWS".to_string(), temp_dir.clone());
        config.add_project("p1".to_string(), temp_dir.join("p1"));
        config.add_project("p2".to_string(), temp_dir.join("p2"));
        fs::create_dir_all(temp_dir.join("p1")).unwrap();
        fs::create_dir_all(temp_dir.join("p2")).unwrap();

        let mut state = ActiveWorkspace::new(config);

        state.set_focused_project(1);
        assert_eq!(state.focused_project_index(), 1);

        // Out of bounds should be ignored
        state.set_focused_project(99);
        assert_eq!(state.focused_project_index(), 1);

        let _ = fs::remove_dir_all(temp_dir);
    }
}