use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const WORKSPACE_FILENAME: &str = ".workspace";
pub const WORKSPACE_MD_FILENAME: &str = ".workspace.md";

/// Workspace configuration stored in .workspace JSON file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub version: String,
    pub name: String,
    pub path: PathBuf,
    pub projects: Vec<ProjectEntry>,
    pub created_at: String,
    pub last_activated: Option<String>,
}

/// A project entry within a workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectEntry {
    pub name: String,
    pub path: PathBuf,
}

impl ProjectEntry {
    /// Check if the project path exists on disk
    pub fn is_available(&self) -> bool {
        self.path.exists() && self.path.is_dir()
    }
}

impl WorkspaceConfig {
    /// Create a new workspace config with empty projects
    pub fn new(name: String, path: PathBuf) -> Self {
        Self {
            version: "1.0".to_string(),
            name,
            path,
            projects: vec![],
            created_at: chrono::Utc::now().to_rfc3339(),
            last_activated: None,
        }
    }

    /// Add a project to the workspace
    pub fn add_project(&mut self, name: String, path: PathBuf) {
        if !self.projects.iter().any(|p| p.path == path) {
            self.projects.push(ProjectEntry { name, path });
        }
    }

    /// Remove a project by path
    pub fn remove_project(&mut self, path: &Path) {
        self.projects.retain(|p| &p.path != path);
    }

    /// Update last_activated timestamp
    pub fn mark_activated(&mut self) {
        self.last_activated = Some(chrono::Utc::now().to_rfc3339());
    }

    /// Get count of available projects (paths exist)
    pub fn available_projects_count(&self) -> usize {
        self.projects.iter().filter(|p| p.is_available()).count()
    }
}

/// Generate markdown template for .workspace.md
pub fn generate_workspace_md_template(config: &WorkspaceConfig) -> String {
    let mut md = format!("# Workspace: {}\n\n## Projects\n", config.name);

    if config.projects.is_empty() {
        md.push_str("- No projects added yet\n");
    } else {
        for project in &config.projects {
            let status = if project.is_available() {
                String::new()
            } else {
                " ⚠️ unavailable".to_string()
            };
            md.push_str(&format!("- [[{}]]{}\n", project.name, status));
        }
    }

    md.push_str(r#"

## Recent Activity

<!-- Auto-generated: list recent sessions per project -->

## Notes

<!-- User notes -->

"#);

    md
}

/// Save workspace config atomically using jcode_storage
pub fn save_workspace_config(config: &WorkspaceConfig, path: &Path) -> Result<()> {
    jcode_storage::write_json(path, config)
}

/// Load workspace config
pub fn load_workspace_config(path: &Path) -> Result<Option<WorkspaceConfig>> {
    if !path.exists() {
        return Ok(None);
    }
    let config = jcode_storage::read_json(path).with_context(|| {
        format!("Failed to parse workspace config at {}", path.display())
    })?;
    Ok(Some(config))
}

/// Find workspace config by searching up from a path
pub fn find_workspace_config(start_path: &Path) -> Result<Option<PathBuf>> {
    let mut current = start_path.to_path_buf();

    loop {
        let config_path = current.join(WORKSPACE_FILENAME);
        if config_path.exists() {
            return Ok(Some(config_path));
        }

        // Stop if we've reached home or root
        if current == current.parent().unwrap_or(&current) {
            break;
        }
        current = current.parent().unwrap().to_path_buf();
    }

    Ok(None)
}

/// Get workspace memory directory path (for workspace-level memory)
pub fn workspace_memory_dir(workspace_path: &Path) -> PathBuf {
    workspace_path.join(".workspace-memory")
}

/// Ensure workspace memory directory exists
pub fn ensure_workspace_memory_dir(workspace_path: &Path) -> Result<()> {
    let mem_dir = workspace_memory_dir(workspace_path);
    jcode_storage::ensure_dir(&mem_dir)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_workspace_config_json_roundtrip() {
        let config = WorkspaceConfig {
            version: "1.0".to_string(),
            name: "Test Workspace".to_string(),
            path: PathBuf::from("/tmp/test-workspace"),
            projects: vec![
                ProjectEntry {
                    name: "backend".to_string(),
                    path: PathBuf::from("/tmp/backend"),
                },
            ],
            created_at: "2026-05-14T00:00:00Z".to_string(),
            last_activated: None,
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        let parsed: WorkspaceConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "Test Workspace");
        assert_eq!(parsed.projects.len(), 1);
    }

    #[test]
    fn test_workspace_md_template_generation() {
        let config = WorkspaceConfig {
            version: "1.0".to_string(),
            name: "Test Workspace".to_string(),
            path: PathBuf::from("/tmp/test-workspace"),
            projects: vec![
                ProjectEntry {
                    name: "backend".to_string(),
                    path: PathBuf::from("/tmp/backend"),
                },
                ProjectEntry {
                    name: "frontend".to_string(),
                    path: PathBuf::from("/tmp/frontend"),
                },
            ],
            created_at: "2026-05-14T00:00:00Z".to_string(),
            last_activated: Some("2026-05-14T12:00:00Z".to_string()),
        };

        let md = generate_workspace_md_template(&config);
        assert!(md.contains("# Workspace: Test Workspace"));
        assert!(md.contains("[[backend]]"));
        assert!(md.contains("[[frontend]]"));
        assert!(md.contains("## Recent Activity"));
        assert!(md.contains("## Notes"));
    }

    #[test]
    fn test_project_entry_validation() {
        let entry = ProjectEntry {
            name: "backend".to_string(),
            path: PathBuf::from("/nonexistent/path"),
        };
        assert!(!entry.is_available());

        let entry = ProjectEntry {
            name: "backend".to_string(),
            path: std::env::temp_dir(),
        };
        assert!(entry.is_available());
    }

    #[test]
    fn test_workspace_config_empty_projects() {
        let config = WorkspaceConfig {
            version: "1.0".to_string(),
            name: "Empty Workspace".to_string(),
            path: PathBuf::from("/tmp/empty-workspace"),
            projects: vec![],
            created_at: "2026-05-14T00:00:00Z".to_string(),
            last_activated: None,
        };

        assert!(config.projects.is_empty());
        let md = generate_workspace_md_template(&config);
        assert!(md.contains("No projects added yet"));
    }
}

#[cfg(test)]
mod persistence_tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_save_and_load_workspace() {
        let temp_dir = std::env::temp_dir().join("workspace_persist_test");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mut config = WorkspaceConfig::new(
            "Test".to_string(),
            temp_dir.clone(),
        );
        config.add_project("backend".to_string(), temp_dir.join("backend"));

        let config_path = temp_dir.join(".workspace");
        save_workspace_config(&config, &config_path).unwrap();

        let loaded = load_workspace_config(&config_path).unwrap().unwrap();
        assert_eq!(loaded.name, "Test");
        assert_eq!(loaded.projects.len(), 1);

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_load_nonexistent_returns_none() {
        let result = load_workspace_config(Path::new("/nonexistent/.workspace"));
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_save_updates_last_activated() {
        let temp_dir = std::env::temp_dir().join("workspace_activated_test");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let config = WorkspaceConfig::new(
            "Test".to_string(),
            temp_dir.clone(),
        );
        let config_path = temp_dir.join(".workspace");

        save_workspace_config(&config, &config_path).unwrap();
        let loaded1 = load_workspace_config(&config_path).unwrap().unwrap();
        assert!(loaded1.last_activated.is_none());

        // Simulate activation
        let mut loaded2 = load_workspace_config(&config_path).unwrap().unwrap();
        loaded2.mark_activated();
        save_workspace_config(&loaded2, &config_path).unwrap();

        let loaded3 = load_workspace_config(&config_path).unwrap().unwrap();
        assert!(loaded3.last_activated.is_some());

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}