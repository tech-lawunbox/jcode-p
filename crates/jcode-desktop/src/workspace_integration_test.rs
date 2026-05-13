//! Integration tests for full workspace lifecycle

#[cfg(test)]
mod tests {
    use crate::workspace_cli::{
        activate_workspace, add_project_to_workspace, create_workspace,
    };
    use crate::workspace_project::load_workspace_config;
    use crate::session_data::load_session_cards_for_project;
    use std::fs;

    #[test]
    fn test_full_workspace_lifecycle() {
        let temp_dir = std::env::temp_dir().join("workspace_lifecycle_test");
        let _ = fs::remove_dir_all(&temp_dir);

        // 1. Create workspace with projects
        fs::create_dir_all(temp_dir.join("project1")).unwrap();
        fs::create_dir_all(temp_dir.join("project2")).unwrap();

        let config = create_workspace(
            "LifecycleTest".to_string(),
            temp_dir.clone(),
            vec![temp_dir.join("project1"), temp_dir.join("project2")],
        )
        .unwrap();

        assert_eq!(config.name, "LifecycleTest");
        assert_eq!(config.projects.len(), 2);

        // 2. Activate workspace
        let activated = activate_workspace(&temp_dir).unwrap();
        assert!(activated.last_activated.is_some());

        // 3. Add another project
        fs::create_dir_all(temp_dir.join("project3")).unwrap();
        let updated = add_project_to_workspace(
            &temp_dir,
            temp_dir.join("project3"),
            Some("Project Three".to_string()),
        )
        .unwrap();

        assert_eq!(updated.projects.len(), 3);

        // 4. Load workspace status
        let status_config = load_workspace_config(&temp_dir.join(".workspace"))
            .unwrap()
            .unwrap();
        assert_eq!(status_config.available_projects_count(), 3);

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_project_independence() {
        // Verify project works WITHOUT workspace
        let temp_dir = std::env::temp_dir().join("standalone_project_test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Can load sessions without workspace
        let cards = load_session_cards_for_project(&temp_dir).unwrap();
        assert!(cards.is_empty()); // No sessions

        let _ = fs::remove_dir_all(temp_dir);
    }
}