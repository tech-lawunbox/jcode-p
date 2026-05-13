use crate::cli::workspace_project::{
    generate_workspace_md_template, load_workspace_config, save_workspace_config,
    WorkspaceConfig, WORKSPACE_FILENAME, WORKSPACE_MD_FILENAME,
    ensure_workspace_memory_dir,
};
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// Create a new workspace at the specified path
pub fn create_workspace(
    name: String,
    path: PathBuf,
    project_paths: Vec<PathBuf>,
) -> Result<WorkspaceConfig> {
    // Create directory if it doesn't exist
    fs::create_dir_all(&path).context("Failed to create workspace directory")?;

    // Build config
    let mut config = WorkspaceConfig::new(name, path.clone());
    config.projects = project_paths
        .into_iter()
        .filter(|p| p.exists() && p.is_dir())
        .map(|p| {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            crate::cli::workspace_project::ProjectEntry { name, path: p }
        })
        .collect();

    // Save .workspace
    let config_path = path.join(WORKSPACE_FILENAME);
    save_workspace_config(&config, &config_path)
        .context("Failed to save workspace config")?;

    // Generate and save .workspace.md
    let md = generate_workspace_md_template(&config);
    let md_path = path.join(WORKSPACE_MD_FILENAME);
    fs::write(&md_path, md).context("Failed to create workspace notes file")?;

    Ok(config)
}

/// Activate a workspace (load config, verify projects)
pub fn activate_workspace(path: &PathBuf) -> Result<WorkspaceConfig> {
    let config_path = path.join(WORKSPACE_FILENAME);

    let mut config = load_workspace_config(&config_path)
        .context("Failed to load workspace config")?
        .context("Workspace config not found")?;

    // Update last activated
    config.mark_activated();
    save_workspace_config(&config, &config_path)
        .context("Failed to update workspace config")?;

    // Ensure memory directory exists
    ensure_workspace_memory_dir(path).context("Failed to create workspace memory directory")?;

    Ok(config)
}

/// Add a project to an existing workspace
pub fn add_project_to_workspace(
    workspace_path: &PathBuf,
    project_path: PathBuf,
    name: Option<String>,
) -> Result<WorkspaceConfig> {
    let config_path = workspace_path.join(WORKSPACE_FILENAME);

    let mut config = load_workspace_config(&config_path)
        .context("Failed to load workspace config")?
        .context("Workspace config not found")?;

    let project_name = name.unwrap_or_else(|| {
        project_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    });

    config.add_project(project_name, project_path);

    save_workspace_config(&config, &config_path)
        .context("Failed to save workspace config")?;

    // Update .workspace.md
    let md = generate_workspace_md_template(&config);
    let md_path = workspace_path.join(WORKSPACE_MD_FILENAME);
    fs::write(&md_path, md).context("Failed to update workspace notes")?;

    Ok(config)
}

/// List all workspaces (searches standard locations)
pub fn list_workspaces() -> Result<Vec<PathBuf>> {
    let workspaces_dir = jcode_storage::jcode_dir()?.join("workspaces");

    if !workspaces_dir.exists() {
        return Ok(vec![]);
    }

    let mut workspaces = vec![];
    for entry in fs::read_dir(&workspaces_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && path.join(WORKSPACE_FILENAME).exists() {
            workspaces.push(path);
        }
    }

    Ok(workspaces)
}