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

    #[test]
    fn test_load_workspace_from_path() {
        use std::fs;

        let temp_dir = std::env::temp_dir().join("load_ws_test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Create workspace files
        let config = crate::workspace_cli::create_workspace(
            "LoadTest".to_string(),
            temp_dir.clone(),
            vec![],
        ).unwrap();

        // Load it back
        let loaded = load_workspace(&temp_dir).unwrap();
        assert_eq!(loaded.name(), "LoadTest");

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_load_workspace_nonexistent() {
        let result = load_workspace(&std::path::PathBuf::from("/nonexistent/path"));
        assert!(result.is_err());
    }
}