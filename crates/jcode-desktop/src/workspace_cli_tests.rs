#[cfg(test)]
mod tests {
    use crate::workspace_cli::activate_workspace;
    use crate::workspace_cli::create_workspace;
    use crate::workspace_project::{load_workspace_config, workspace_memory_dir};
    use std::fs;

    #[test]
    fn test_create_workspace_flow() {
        let temp_dir = std::env::temp_dir().join("workspace_create_test");
        let _ = fs::remove_dir_all(&temp_dir);

        let result = create_workspace(
            "TestWorkspace".to_string(),
            temp_dir.clone(),
            vec![],
        );

        assert!(result.is_ok());

        // Check .workspace file exists
        let config_path = temp_dir.join(".workspace");
        assert!(config_path.exists());

        // Check .workspace.md exists
        let md_path = temp_dir.join(".workspace.md");
        assert!(md_path.exists());

        // Check content
        let config = load_workspace_config(&config_path).unwrap().unwrap();
        assert_eq!(config.name, "TestWorkspace");
        assert!(config.projects.is_empty());

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_create_with_initial_projects() {
        let temp_dir = std::env::temp_dir().join("workspace_create_projects_test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(temp_dir.join("project1")).unwrap();

        let result = create_workspace(
            "TestWorkspace".to_string(),
            temp_dir.clone(),
            vec![temp_dir.join("project1")],
        );

        assert!(result.is_ok());
        let config = load_workspace_config(&temp_dir.join(".workspace")).unwrap().unwrap();
        assert_eq!(config.projects.len(), 1);

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_workspace_memory_dir_created_on_activation() {
        use std::fs;

        let temp_dir = std::env::temp_dir().join("workspace_memory_test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Create workspace
        let config = create_workspace(
            "MemoryTest".to_string(),
            temp_dir.clone(),
            vec![],
        ).unwrap();

        // Memory dir should not exist yet
        let mem_dir = workspace_memory_dir(&temp_dir);
        assert!(!mem_dir.exists());

        // Activate workspace
        let activated = activate_workspace(&temp_dir).unwrap();
        assert!(activated.last_activated.is_some());

        // Memory dir should now exist
        assert!(mem_dir.exists());

        let _ = fs::remove_dir_all(temp_dir);
    }
}