# Workspace-Project System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement workspace as optional project organizer with `.workspace` (JSON) + `.workspace.md` files, enabling grouped multi-project view with cross-project memory.

**Architecture:** Projects remain first-class independent entities. Workspace adds a configuration layer and activation flow. Uses existing jcode-storage for atomic JSON writes, existing session loading for project sessions, and existing memory-types with Project scope.

**Tech Stack:** Rust (existing crate structure), clap for CLI, jcode-storage for atomic JSON, jcode-memory-types for memory scopes.

---

## File Structure

```
crates/jcode-desktop/src/
├── workspace_project.rs     (NEW: Project/Workspace file types + persistence)
├── workspace_cli.rs         (NEW: /workspace command implementation)
├── session_data.rs          (MODIFY: add project-scoped session loading)
└── workspace.rs              (MODIFY: add workspace activation state)

crates/jcode-memory-types/src/lib.rs  (MODIFY: add Workspace memory scope variant)

src/cli/args.rs              (MODIFY: add WorkspaceCommand subcommand)
```

---

## Task 1: Define Workspace Project Types

**Files:**
- Create: `crates/jcode-desktop/src/workspace_project.rs`

- [ ] **Step 1: Write the failing test**

```rust
// crates/jcode-desktop/src/workspace_project_tests.rs
#[cfg(test)]
mod tests {
    use super::*;

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
        // Path doesn't exist - should be marked unavailable
        assert!(!entry.is_available());

        let entry = ProjectEntry {
            name: "backend".to_string(),
            path: std::env::temp_dir(),
        };
        // Temp dir exists
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --package jcode-desktop workspace_project_tests -- --nocapture 2>&1`
Expected: FAIL — module doesn't exist

- [ ] **Step 3: Write minimal types**

```rust
// crates/jcode-desktop/src/workspace_project.rs

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

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
        // Avoid duplicates by path
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
    let mut md = format!(r#"# Workspace: {}

## Projects
"#, config.name);

    if config.projects.is_empty() {
        md.push_str("- No projects added yet\n");
    } else {
        for project in &config.projects {
            let status = if project.is_available() {
                ""
            } else {
                " ⚠️ unavailable"
            };
            md.push_str(&format!("- [[{}]]{}\n", project.name, status));
        }
    }

    md.push_str(r#"

## Recent Activity

<!-- Auto-generated: list recent sessions per project -->

### backend
- No recent sessions

### frontend
- No recent sessions

## Notes

<!-- User notes -->

"#);

    md
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --package jcode-desktop workspace_project_tests -- --nocapture 2>&1`
Expected: PASS (after fixing import issues)

- [ ] **Step 5: Commit**

```bash
git add crates/jcode-desktop/src/workspace_project.rs
git commit -m "feat: add WorkspaceConfig and ProjectEntry types"
```

---

## Task 2: Add Workspace File Persistence

**Files:**
- Modify: `crates/jcode-desktop/src/workspace_project.rs`
- Use: `crates/jcode-storage/src/lib.rs` (already imported in crate)

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod persistence_tests {
    use super::*;

    #[test]
    fn test_save_and_load_workspace() {
        let temp_dir = std::env::temp_dir().join("workspace_persist_test");
        fs::create_dir_all(&temp_dir).unwrap();

        let config = WorkspaceConfig::new(
            "Test".to_string(),
            temp_dir.clone(),
        );
        config.add_project("backend".to_string(), temp_dir.join("backend"));

        let config_path = temp_dir.join(".workspace");
        save_workspace_config(&config, &config_path).unwrap();

        let loaded = load_workspace_config(&config_path).unwrap().unwrap();
        assert_eq!(loaded.name, "Test");
        assert_eq!(loaded.projects.len(), 1);

        // Cleanup
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_load_nonexistent_returns_none() {
        let result = load_workspace_config(Path::new("/nonexistent/.workspace"));
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_save_updates_last_activated() {
        let temp_dir = std::env::temp_dir().join("workspace_activated_test");
        fs::create_dir_all(&temp_dir).unwrap();

        let mut config = WorkspaceConfig::new(
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

        // Cleanup
        let _ = fs::remove_dir_all(temp_dir);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --package jcode-desktop workspace_project::persistence_tests -- --nocapture 2>&1`
Expected: FAIL — functions don't exist

- [ ] **Step 3: Implement persistence functions**

```rust
use crate::workspace_project::{ProjectEntry, WorkspaceConfig};
use anyhow::{Context, Result};
use jcode_storage;
use std::fs;
use std::path::Path;

/// Path to .workspace file within a workspace directory
pub const WORKSPACE_FILENAME: &str = ".workspace";

/// Path to .workspace.md within a workspace directory
pub const WORKSPACE_MD_FILENAME: &str = ".workspace.md";

/// Save workspace config atomically
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

    // Search up to home directory or filesystem root
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --package jcode-desktop workspace_project::persistence_tests -- --nocapture 2>&1`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/jcode-desktop/src/workspace_project.rs
git commit -m "feat: add workspace config persistence with atomic writes"
```

---

## Task 3: Add MemoryScope for Workspace

**Files:**
- Modify: `crates/jcode-memory-types/src/lib.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod workspace_scope_tests {
    use super::*;

    #[test]
    fn test_workspace_scope_includes_project_and_global() {
        let scope = MemoryScope::Workspace;
        assert!(scope.includes_project());
        assert!(scope.includes_global());
    }

    #[test]
    fn test_workspace_scope_string_roundtrip() {
        let scope = MemoryScope::Workspace;
        let s = scope.to_string();
        assert_eq!(s, "workspace");
        let parsed: MemoryScope = s.parse().unwrap();
        assert_eq!(parsed, MemoryScope::Workspace);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --package jcode-memory-types workspace_scope_tests -- --nocapture 2>&1`
Expected: FAIL — Workspace variant doesn't exist

- [ ] **Step 3: Add Workspace variant to MemoryScope**

Find the MemoryScope enum (around line 497) and add Workspace:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryScope {
    Session,
    Project,
    Global,
    Workspace,  // NEW: workspace-level memory shared across projects
}

impl MemoryScope {
    pub fn includes_project(self) -> bool {
        matches!(self, MemoryScope::Project | MemoryScope::Workspace | MemoryScope::Global)
    }

    pub fn includes_global(self) -> bool {
        matches!(self, MemoryScope::Global | MemoryScope::Workspace)
    }

    pub fn includes_session(self) -> bool {
        // Session scope always accessible
        true
    }
}

impl std::str::FromStr for MemoryScope {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "session" => Ok(MemoryScope::Session),
            "project" => Ok(MemoryScope::Project),
            "global" => Ok(MemoryScope::Global),
            "workspace" => Ok(MemoryScope::Workspace),
            _ => Err(format!("Unknown memory scope: {}", s)),
        }
    }
}

impl std::fmt::Display for MemoryScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryScope::Session => write!(f, "session"),
            MemoryScope::Project => write!(f, "project"),
            MemoryScope::Global => write!(f, "global"),
            MemoryScope::Workspace => write!(f, "workspace"),
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --package jcode-memory-types workspace_scope_tests -- --nocapture 2>&1`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/jcode-memory-types/src/lib.rs
git commit -m "feat: add Workspace memory scope for cross-project context"
```

---

## Task 4: Add CLI /workspace Command

**Files:**
- Modify: `src/cli/args.rs`
- Create: `crates/jcode-desktop/src/workspace_cli.rs`

- [ ] **Step 1: Write the failing test**

```rust
// crates/jcode-desktop/src/workspace_cli_tests.rs
#[cfg(test)]
mod cli_tests {
    use super::*;

    #[test]
    fn test_workspace_command_variants() {
        // Test that all command variants parse correctly
        let create = Args::parse_from(["jcode", "workspace", "create", "MyWorkspace", "--path", "/tmp/test"]);
        assert!(matches!(create.command, Command::Workspace(WorkspaceCommand::Create { .. })));

        let activate = Args::parse_from(["jcode", "workspace", "activate", "/tmp/test"]);
        assert!(matches!(create.command, Command::Workspace(WorkspaceCommand::Activate { .. })));

        let add = Args::parse_from(["jcode", "workspace", "add-project", "/tmp/project"]);
        assert!(matches!(create.command, Command::Workspace(WorkspaceCommand::AddProject { .. })));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --package jcode-desktop workspace_cli_tests -- --nocapture 2>&1`
Expected: FAIL — WorkspaceCommand doesn't exist

- [ ] **Step 3: Add WorkspaceCommand enum to args.rs**

Find the existing Command enum in args.rs (around line 92) and add:

```rust
#[derive(Subcommand, Debug)]
pub enum WorkspaceCommand {
    /// Create a new workspace
    Create {
        /// Workspace name
        name: String,
        /// Workspace directory path
        #[arg(long)]
        path: Option<PathBuf>,
        /// Add projects now
        #[arg(long, short)]
        projects: Option<Vec<PathBuf>>,
    },
    /// Activate a workspace (load all projects)
    Activate {
        /// Path to workspace directory
        path: PathBuf,
    },
    /// Deactivate current workspace
    Deactivate,
    /// Add a project to workspace
    AddProject {
        /// Project path to add
        path: PathBuf,
        /// Project name (defaults to directory name)
        #[arg(long, short)]
        name: Option<String>,
    },
    /// Remove a project from workspace
    RemoveProject {
        /// Project path to remove
        path: PathBuf,
    },
    /// List all workspaces
    List,
    /// Show workspace status
    Status {
        /// Path to workspace (current if not specified)
        path: Option<PathBuf>,
    },
    /// Open a project directly (no workspace)
    OpenProject {
        /// Project path to open
        path: PathBuf,
    },
}
```

And in the Command enum, add:

```rust
Workspace {
    #[command(subcommand)]
    command: WorkspaceCommand,
},
```

Add import at top:
```rust
use std::path::PathBuf;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --package jcode-desktop workspace_cli_tests -- --nocapture 2>&1`
Expected: PASS (after fixing CLI structure)

- [ ] **Step 5: Commit**

```bash
git add src/cli/args.rs
git commit -m "feat: add /workspace CLI command with create/activate/add-project subcommands"
```

---

## Task 5: Implement /workspace create Command

**Files:**
- Create: `crates/jcode-desktop/src/workspace_cli.rs`
- Modify: `crates/jcode-desktop/src/main.rs` (to wire up command)

- [ ] **Step 1: Write the failing test**

```rust
// crates/jcode-desktop/src/workspace_cli_tests.rs
#[cfg(test)]
mod create_tests {
    use super::*;

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

        // Cleanup
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

        // Cleanup
        let _ = fs::remove_dir_all(temp_dir);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --package jcode-desktop create_tests -- --nocapture 2>&1`
Expected: FAIL — create_workspace function doesn't exist

- [ ] **Step 3: Implement create_workspace function**

```rust
// crates/jcode-desktop/src/workspace_cli.rs

use crate::workspace_project::{
    generate_workspace_md_template, load_workspace_config, save_workspace_config,
    WorkspaceConfig, WORKSPACE_FILENAME, WORKSPACE_MD_FILENAME,
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
            crate::workspace_project::ProjectEntry { name, path: p }
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
    crate::workspace_project::ensure_workspace_memory_dir(path)?;

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
    // For now, just check ~/.jcode/workspaces/
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --package jcode-desktop create_tests -- --nocapture 2>&1`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/jcode-desktop/src/workspace_cli.rs
git commit -m "feat: implement workspace create/activate/add-project CLI functions"
```

---

## Task 6: Wire /workspace Command to CLI Handler

**Files:**
- Modify: `crates/jcode-desktop/src/main.rs`
- Modify: `src/cli/args.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn test_workspace_create_integration() {
    // Test that CLI args parse and route to workspace_cli
    let args = Args::parse_from([
        "jcode",
        "workspace",
        "create",
        "TestWorkspace",
        "--path",
        "/tmp/test-workspace",
    ]);

    match args.command {
        Command::Workspace(WorkspaceCommand::Create { name, path, .. }) => {
            assert_eq!(name, "TestWorkspace");
            assert!(path.is_some());
        }
        _ => panic!("Expected workspace create command"),
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --package jcode-desktop workspace_create_integration -- --nocapture 2>&1`
Expected: FAIL — handler not wired

- [ ] **Step 3: Find CLI handler and add workspace routing**

In `src/bin/harness.rs` or wherever Command::Workspace is handled, add:

```rust
Command::Workspace(workspace_cmd) => {
    use jcode_desktop::workspace_cli;

    match workspace_cmd {
        WorkspaceCommand::Create { name, path, projects } => {
            let path = path.unwrap_or_else(|| {
                jcode_storage::jcode_dir()
                    .unwrap()
                    .join("workspaces")
                    .join(name)
            });
            let projects = projects.unwrap_or_default();

            match workspace_cli::create_workspace(name, path.clone(), projects) {
                Ok(config) => {
                    println!("Workspace '{}' created at {}", config.name, config.path.display());
                    println!("  {} projects added", config.projects.len());
                }
                Err(e) => {
                    eprintln!("Failed to create workspace: {}", e);
                    std::process::exit(1);
                }
            }
        }
        WorkspaceCommand::Activate { path } => {
            match workspace_cli::activate_workspace(&path) {
                Ok(config) => {
                    println!("Workspace '{}' activated", config.name);
                    println!("  {} projects available", config.available_projects_count());
                }
                Err(e) => {
                    eprintln!("Failed to activate workspace: {}", e);
                    std::process::exit(1);
                }
            }
        }
        WorkspaceCommand::AddProject { path, name } => {
            // Find workspace in current dir or parent dirs
            let workspace_path = jcode_desktop::workspace_project::find_workspace_config(&std::env::current_dir().unwrap())
                .unwrap()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .expect("No workspace found. Run `jcode workspace activate <path>` first.");

            match workspace_cli::add_project_to_workspace(&workspace_path, path, name) {
                Ok(config) => {
                    println!("Project added to '{}'", config.name);
                    println!("  {} projects total", config.projects.len());
                }
                Err(e) => {
                    eprintln!("Failed to add project: {}", e);
                    std::process::exit(1);
                }
            }
        }
        WorkspaceCommand::List => {
            match workspace_cli::list_workspaces() {
                Ok(workspaces) => {
                    if workspaces.is_empty() {
                        println!("No workspaces found.");
                    } else {
                        for ws in workspaces {
                            if let Ok(Some(config)) = jcode_desktop::workspace_project::load_workspace_config(&ws.join(".workspace")) {
                                println!("  {} - {}", config.name, ws.display());
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to list workspaces: {}", e);
                    std::process::exit(1);
                }
            }
        }
        WorkspaceCommand::Status { path } => {
            let ws_path = path.unwrap_or_else(|| std::env::current_dir().unwrap());
            let config_path = ws_path.join(".workspace");

            match jcode_desktop::workspace_project::load_workspace_config(&config_path) {
                Ok(Some(config)) => {
                    println!("Workspace: {}", config.name);
                    println!("Path: {}", config.path.display());
                    println!("Projects: {}", config.projects.len());
                    println!("Available: {}", config.available_projects_count());
                    if let Some(last) = config.last_activated {
                        println!("Last activated: {}", last);
                    }
                }
                Ok(None) => {
                    eprintln!("No workspace at {}", ws_path.display());
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Failed to load workspace: {}", e);
                    std::process::exit(1);
                }
            }
        }
        WorkspaceCommand::Deactivate => {
            // Deactivate just means exit workspace context
            println!("Workspace deactivated");
        }
        WorkspaceCommand::RemoveProject { path } => {
            eprintln!("Remove project not yet implemented");
        }
        WorkspaceCommand::OpenProject { path } => {
            // Open project directly (existing behavior)
            println!("Opening project: {}", path.display());
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --package jcode-desktop workspace_create_integration -- --nocapture 2>&1`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/jcode-desktop/src/main.rs
git commit -m "feat: wire /workspace commands to CLI handler"
```

---

## Task 7: Integrate Workspace Activation with Existing Workspace State

**Files:**
- Modify: `crates/jcode-desktop/src/workspace.rs`
- Modify: `crates/jcode-desktop/src/session_data.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn test_workspace_activation_loads_projects() {
    let temp_dir = std::env::temp_dir().join("workspace_activation_test");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(temp_dir.join("project1")).unwrap();

    // Create workspace
    let config = create_workspace(
        "TestWS".to_string(),
        temp_dir.clone(),
        vec![temp_dir.join("project1")],
    ).unwrap();

    // Activate and check project loaded
    let activated = activate_workspace(&temp_dir).unwrap();
    assert_eq!(activated.name, "TestWS");
    assert_eq!(activated.projects.len(), 1);

    // Cleanup
    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_open_project_standalone_vs_via_workspace() {
    // Test that project opened directly works standalone
    let temp_dir = std::env::temp_dir().join("standalone_project_test");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    // Open directly - should work without workspace
    let cards = load_session_cards_for_project(&temp_dir).unwrap();
    assert!(cards.is_empty()); // No sessions yet

    // Cleanup
    let _ = fs::remove_dir_all(temp_dir);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --package jcode-desktop workspace_activation_loads_projects -- --nocapture 2>&1`
Expected: FAIL — functions don't exist

- [ ] **Step 3: Add project-scoped session loading**

In `session_data.rs`, add:

```rust
/// Load session cards for a specific project path
pub fn load_session_cards_for_project(project_path: &Path) -> Result<Vec<SessionCard>> {
    let sessions_dir = jcode_sessions_dir()?.join(hash_project_path(project_path));

    if !sessions_dir.exists() {
        return Ok(vec![]);
    }

    let mut cards = vec![];

    for entry in fs::read_dir(&sessions_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            if let Some(card) = load_session_card(&path)? {
                cards.push(card);
            }
        }
    }

    // Sort by most recent first
    cards.sort_by(|a, b| b.detail.cmp(&a.detail));

    Ok(cards)
}

/// Hash project path to create unique session directory name
fn hash_project_path(path: &Path) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut s = DefaultHasher::new();
    path.to_string_lossy().hash(&mut s);
    format!("{:016x}", s.finish())
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --package jcode-desktop workspace_activation_loads_projects -- --nocapture 2>&1`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/jcode-desktop/src/session_data.rs
git commit -m "feat: add project-scoped session loading for workspace activation"
```

---

## Task 8: Add Workspace Memory Integration

**Files:**
- Modify: `crates/jcode-memory-types/src/lib.rs` (MemoryScope already added in Task 3)
- Modify: `crates/jcode-desktop/src/workspace_cli.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn test_workspace_memory_path() {
    let workspace_path = PathBuf::from("/tmp/test-workspace");
    let mem_dir = workspace_memory_dir(&workspace_path);
    assert_eq!(mem_dir, PathBuf::from("/tmp/test-workspace/.workspace-memory"));
}

#[test]
fn test_memory_scope_hierarchy() {
    // Workspace scope should include project and global
    let ws = MemoryScope::Workspace;
    assert!(ws.includes_project());
    assert!(ws.includes_global());
    assert!(ws.includes_session()); // All scopes include session

    // Project scope includes project and global (but not workspace-specific)
    let proj = MemoryScope::Project;
    assert!(proj.includes_project());
    assert!(proj.includes_global());
    assert!(!proj.includes_workspace()); // Project doesn't have workspace-level
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --package jcode-desktop workspace_memory_path -- --nocapture 2>&1`
Expected: FAIL — includes_workspace doesn't exist

- [ ] **Step 3: Add includes_workspace method and ensure memory dir creation**

In `jcode-memory-types/src/lib.rs`, add to MemoryScope impl:

```rust
pub fn includes_workspace(self) -> bool {
    matches!(self, MemoryScope::Workspace)
}
```

And in `workspace_cli.rs`, ensure the memory dir is created on activation:

```rust
pub fn activate_workspace(path: &PathBuf) -> Result<WorkspaceConfig> {
    let config_path = path.join(WORKSPACE_FILENAME);

    let mut config = load_workspace_config(&config_path)
        .context("Failed to load workspace config")?
        .context("Workspace config not found")?;

    // Update last activated
    config.mark_activated();
    save_workspace_config(&config, &config_path)
        .context("Failed to update workspace config")?;

    // Ensure memory directory exists for workspace-level memory
    ensure_workspace_memory_dir(path).context("Failed to create workspace memory directory")?;

    Ok(config)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --package jcode-desktop workspace_memory_path -- --nocapture 2>&1`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/jcode-memory-types/src/lib.rs crates/jcode-desktop/src/workspace_cli.rs
git commit -m "feat: add workspace memory directory and includes_workspace scope"
```

---

## Task 9: Integration Test - Full Workspace Flow

**Files:**
- Create: `crates/jcode-desktop/src/workspace_integration_test.rs`

- [ ] **Step 1: Write the integration test**

```rust
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
    ).unwrap();

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
    ).unwrap();

    assert_eq!(updated.projects.len(), 3);

    // 4. List workspaces
    let workspaces = list_workspaces().unwrap();
    assert!(workspaces.iter().any(|w| w == &temp_dir));

    // 5. Load workspace status
    let status_config = load_workspace_config(&temp_dir.join(".workspace"))
        .unwrap()
        .unwrap();
    assert_eq!(status_config.available_projects_count(), 3);

    // Cleanup
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

    // Cleanup
    let _ = fs::remove_dir_all(temp_dir);
}
```

- [ ] **Step 2: Run integration test**

Run: `cargo test --package jcode-desktop workspace_integration_test -- --nocapture 2>&1`
Expected: PASS (all sub-tests)

- [ ] **Step 3: Commit**

```bash
git add crates/jcode-desktop/src/workspace_integration_test.rs
git commit -m "test: add full workspace lifecycle integration test"
```

---

## Task 10: Documentation Update

**Files:**
- Modify: `docs/superpowers/specs/2026-05-14-workspace-project-design.md`

- [ ] **Step 1: Update design doc with implementation notes**

Add section at top:

```markdown
## Implementation Status

- [x] Phase 1: Core Structure (Tasks 1-4)
- [x] Phase 2: Memory Integration (Task 8)
- [x] Phase 3: CLI Integration (Tasks 5-6)
- [ ] Phase 4: TUI Integration (future)
```

- [ ] **Step 2: Commit**

```bash
git add docs/superpowers/specs/2026-05-14-workspace-project-design.md
git commit -m "docs: update workspace design with implementation status"
```

---

## Summary

| Task | Description | Files Modified |
|------|-------------|----------------|
| 1 | Define workspace project types | `workspace_project.rs` (new) |
| 2 | Add workspace file persistence | `workspace_project.rs` |
| 3 | Add MemoryScope::Workspace | `jcode-memory-types/src/lib.rs` |
| 4 | Add /workspace CLI command | `src/cli/args.rs` |
| 5 | Implement workspace create/activate | `workspace_cli.rs` (new) |
| 6 | Wire CLI to handler | `main.rs` |
| 7 | Project-scoped session loading | `session_data.rs` |
| 8 | Workspace memory integration | `workspace_cli.rs`, `jcode-memory-types` |
| 9 | Integration test | `workspace_integration_test.rs` (new) |
| 10 | Documentation | design doc |

---

## Commands Reference

```bash
# Create workspace
jcode workspace create "MyWorkspace" --path ~/jcode/workspaces/my-workspace
jcode workspace create "MyWorkspace" --path ~/jcode/workspaces/my-workspace --projects ~/projects/backend ~/projects/frontend

# Activate workspace
jcode workspace activate ~/jcode/workspaces/my-workspace

# Add project to workspace
jcode workspace add-project ~/projects/new-project
jcode workspace add-project ~/projects/new-project --name "New Project"

# List workspaces
jcode workspace list

# Workspace status
jcode workspace status
jcode workspace status ~/jcode/workspaces/my-workspace

# Open project directly (no workspace)
jcode workspace open-project ~/projects/some-project
```