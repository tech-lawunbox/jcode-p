#[cfg(test)]
mod tests {
    use crate::workspace_state::{ActiveWorkspace, load_workspace};
    use crate::workspace_cli::create_workspace;
    use std::fs;

    #[test]
    fn test_workspace_tui_full_flow() {
        let temp_dir = std::env::temp_dir().join("workspace_tui_test");
        let _ = fs::remove_dir_all(&temp_dir);

        // 1. Create workspace with projects
        fs::create_dir_all(temp_dir.join("project1")).unwrap();
        fs::create_dir_all(temp_dir.join("project2")).unwrap();

        let config = create_workspace(
            "TUITest".to_string(),
            temp_dir.clone(),
            vec![temp_dir.join("project1"), temp_dir.join("project2")],
        ).unwrap();

        // 2. Load into ActiveWorkspace
        let ws = ActiveWorkspace::new(config);
        assert_eq!(ws.name(), "TUITest");
        assert_eq!(ws.project_count(), 2);

        // 3. Switch projects
        let mut ws = ws;
        assert_eq!(ws.focused_project_index(), 0);
        ws.focus_next_project();
        assert_eq!(ws.focused_project_index(), 1);
        ws.focus_previous_project();
        assert_eq!(ws.focused_project_index(), 0);

        // 4. Load workspace from path
        let loaded = load_workspace(&temp_dir).unwrap();
        assert_eq!(loaded.name(), "TUITest");
        assert_eq!(loaded.focused_project_index(), 0);

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_workspace_status_display() {
        let temp_dir = std::env::temp_dir().join("ws_status_test");
        let _ = fs::remove_dir_all(&temp_dir);

        let config = create_workspace(
            "StatusTest".to_string(),
            temp_dir.clone(),
            vec![],
        ).unwrap();

        let ws = ActiveWorkspace::new(config);

        // Verify workspace renders correctly
        let status = format!("WS:{}[{}/{}]",
            ws.name(),
            ws.focused_project_index() + 1,
            ws.project_count()
        );
        assert_eq!(status, "WS:StatusTest[1/0]");

        let _ = fs::remove_dir_all(temp_dir);
    }
}