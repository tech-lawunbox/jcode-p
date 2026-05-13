# Workspace TUI Integration Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Integrate workspace activation into desktop TUI — display active workspace in status bar, show project surfaces in grid, allow project switching.

**Architecture:** Extend existing `DesktopApp` and `Workspace` types to track active workspace config. Use existing `Workspace` rendering infrastructure for multi-project view. Add workspace status to status bar.

**Tech Stack:** Rust, ratatui, existing jcode-desktop workspace types.

---

## File Structure

```
crates/jcode-desktop/src/
├── main.rs                    (MODIFY: add workspace state, status bar integration)
├── workspace.rs                (MODIFY: add workspace config loading, project surfaces)
├── workspace_project.rs        (USE: load workspace config, get projects)
├── session_data.rs            (USE: load_session_cards_for_project)
└── workspace_tui.rs            (NEW: TUI-specific workspace rendering helpers)

src/tui/
├── ui_status.rs               (MODIFY: add workspace info to status bar)
└── ui.rs                      (MODIFY: workspace view integration)
```

---

## Task 1: Add Workspace Activation State to DesktopApp

**Files:**
- Modify: `crates/jcode-desktop/src/main.rs` (add workspace_state module, ActiveWorkspace struct)
- Use: `crates/jcode-desktop/src/workspace_project.rs`

- [ ] **Step 1: Write the failing test**

Add to `crates/jcode-desktop/src/main_tests.rs`:

```rust
#[test]
fn test_workspace_activation_state() {
    let temp_dir = std::env::temp_dir().join("tui_workspace_test");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    // Create a workspace
    let config = workspace_cli::create_workspace(
        "TestWS".to_string(),
        temp_dir.clone(),
        vec![],
    ).unwrap();

    // Create workspace state
    let state = ActiveWorkspace::new(config);
    assert_eq!(state.name(), "TestWS");
    assert_eq!(state.projects().len(), 0);

    let _ = std::fs::remove_dir_all(temp_dir);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --package jcode-desktop test_workspace_activation_state -- --nocapture 2>&1`
Expected: FAIL — ActiveWorkspace doesn't exist

- [ ] **Step 3: Implement ActiveWorkspace struct**

Create `crates/jcode-desktop/src/workspace_state.rs`:

```rust
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
        Self {
            focused_project: 0,
            project_sessions: vec![vec![]; config.projects.len()],
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
```

Add module to `crates/jcode-desktop/src/main.rs`:

```rust
mod workspace_state;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --package jcode-desktop test_workspace_activation_state -- --nocapture 2>&1`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/jcode-desktop/src/workspace_state.rs crates/jcode-desktop/src/main.rs
git commit -m "feat: add ActiveWorkspace state for TUI"
```

---

## Task 2: Add Workspace Loading to DesktopApp

**Files:**
- Modify: `crates/jcode-desktop/src/main.rs`
- Use: `workspace_state.rs`, `workspace_project.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn test_load_workspace_into_desktop_app() {
    use crate::workspace_state::ActiveWorkspace;

    let temp_dir = std::env::temp_dir().join("desktop_app_workspace_test");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    // Create workspace
    let config = workspace_cli::create_workspace(
        "DesktopTest".to_string(),
        temp_dir.clone(),
        vec![],
    ).unwrap();

    // Create active workspace
    let ws = ActiveWorkspace::new(config);

    // Verify structure
    assert_eq!(ws.name(), "DesktopTest");

    let _ = std::fs::remove_dir_all(temp_dir);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --package jcode-desktop test_load_workspace_into_desktop_app -- --nocapture 2>&1`
Expected: PASS (functionality already implemented in task 1)

- [ ] **Step 3: Add helper to load workspace config from path**

Add to `workspace_state.rs`:

```rust
use anyhow::Result;

/// Load and activate workspace from path
pub fn load_workspace(path: &PathBuf) -> Result<ActiveWorkspace> {
    let config_path = path.join(".workspace");
    let config = crate::workspace_project::load_workspace_config(&config_path)
        .context("Failed to load workspace config")?
        .context("No workspace at specified path")?;

    Ok(ActiveWorkspace::new(config))
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --package jcode-desktop test_load_workspace -- --nocapture 2>&1`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/jcode-desktop/src/workspace_state.rs
git commit -m "feat: add load_workspace helper"
```

---

## Task 3: Add Workspace Status to Status Bar

**Files:**
- Modify: `src/tui/ui_status.rs`
- Use: `main.rs` DesktopApp

- [ ] **Step 1: Write the failing test**

In `src/tui/ui_status.rs`, find existing status rendering and add:

```rust
#[test]
fn test_workspace_status_display() {
    // Test that workspace name appears in status
    // This would require mocking DesktopApp with ActiveWorkspace
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --package jcode-ui test_workspace_status_display -- --nocapture 2>&1`
Expected: FAIL (or skip if complex to test)

- [ ] **Step 3: Add workspace info to status bar rendering**

Find `ui_status.rs` and look at how status is rendered. Add workspace badge:

```rust
/// Render workspace indicator in status bar
pub fn render_workspace_badge(workspace: Option<&ActiveWorkspace>, area: Rect, buf: &mut Buffer) {
    let workspace_text = match workspace {
        Some(ws) => {
            if ws.is_empty() {
                format!("WS: {} (no projects)", ws.name())
            } else {
                format!("WS: {} [{}/{}]",
                    ws.name(),
                    ws.focused_project_index() + 1,
                    ws.project_count()
                )
            }
        }
        None => return, // No workspace active, render nothing
    };

    let style = Style::new().fg(Color::Cyan).bg(Color::DarkGray);
    let text = Text::from(workspace_text);
    Paragraph::new(text)
        .style(style)
        .render(area, buf);
}
```

- [ ] **Step 4: Verify compilation**

Run: `cargo build --package jcode-ui 2>&1 | tail -20`

- [ ] **Step 5: Commit**

```bash
git add src/tui/ui_status.rs
git commit -m "feat: add workspace status to status bar"
```

---

## Task 4: Add Project Switching Commands

**Files:**
- Modify: `crates/jcode-desktop/src/workspace.rs` (add KeyOutcome variants)
- Modify: `crates/jcode-desktop/src/main.rs` (handle workspace keys)

- [ ] **Step 1: Add KeyInput variants for project switching**

In `workspace.rs`, find `KeyInput` enum and add:

```rust
pub enum KeyInput {
    // ... existing variants ...

    // Project switching (in workspace context)
    WorkspaceNext,      // Move to next project
    WorkspacePrev,      // Move to previous project
    WorkspaceActivate,   // Activate workspace picker
}
```

And in `KeyOutcome`:

```rust
pub enum KeyOutcome {
    // ... existing variants ...

    SwitchToProject(usize),  // Switch to project by index
    ShowWorkspacePicker,      // Show workspace/project picker
    NoWorkspace,             // No workspace active
}
```

- [ ] **Step 2: Add key handling for workspace switching**

In `workspace.rs` `handle_navigation_key`:

```rust
fn handle_navigation_key(&mut self, key: KeyInput) -> KeyOutcome {
    match key {
        // ... existing cases ...

        KeyInput::WorkspaceNext => {
            if let Some(ref mut ws) = self.active_workspace {
                if ws.focus_next_project() {
                    return KeyOutcome::SwitchToProject(ws.focused_project_index());
                }
            }
            KeyOutcome::None
        }

        KeyInput::WorkspacePrev => {
            if let Some(ref mut ws) = self.active_workspace {
                if ws.focus_previous_project() {
                    return KeyOutcome::SwitchToProject(ws.focused_project_index());
                }
            }
            KeyOutcome::None
        }

        _ => KeyOutcome::None,
    }
}
```

- [ ] **Step 3: Add active_workspace field to Workspace struct**

In `workspace.rs`:

```rust
pub struct Workspace {
    // ... existing fields ...
    pub active_workspace: Option<crate::workspace_state::ActiveWorkspace>,
}
```

- [ ] **Step 4: Commit**

```bash
git add crates/jcode-desktop/src/workspace.rs
git commit -m "feat: add project switching key handling"
```

---

## Task 5: Render Project Surfaces in Workspace Grid

**Files:**
- Modify: `crates/jcode-desktop/src/render_helpers.rs`
- Use: `workspace.rs`, `session_data.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn test_render_project_surfaces() {
    // Test that project sessions render in grid
}
```

- [ ] **Step 2: Run test to verify it fails/skips**

Run: `cargo test --package jcode-desktop test_render_project_surfaces -- --nocapture 2>&1`

- [ ] **Step 3: Extend surface rendering to show project context**

In `render_helpers.rs`, find existing surface rendering and extend to show project name when in workspace mode:

```rust
/// Get surface title including project context
pub fn surface_title_with_project(surface: &Surface, workspace: Option<&ActiveWorkspace>) -> String {
    match (surface, workspace) {
        (Surface::ProjectSession { project_name, .. }, Some(ws)) => {
            format!("[{}] {}", ws.name(), project_name)
        }
        _ => surface.title().unwrap_or_default(),
    }
}
```

- [ ] **Step 4: Commit**

```bash
git add crates/jcode-desktop/src/render_helpers.rs
git commit -m "feat: add project context to surface rendering"
```

---

## Task 6: Load Workspace on Startup

**Files:**
- Modify: `crates/jcode-desktop/src/main.rs`
- Use: `workspace_state.rs`, `workspace_project.rs`

- [ ] **Step 1: Add workspace auto-detection on startup**

In `main.rs` `run()` function, after desktop app initialization:

```rust
// Check for workspace in current directory or home
fn detect_workspace() -> Option<PathBuf> {
    let current = std::env::current_dir().ok()?;
    workspace_project::find_workspace_config(&current)
        .ok()
        .flatten()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
}

// In run() async fn, after app initialization:
if let Some(ws_path) = detect_workspace() {
    if let Ok(ws) = workspace_state::load_workspace(&ws_path) {
        app.set_active_workspace(ws);
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo build --bin jcode 2>&1 | tail -20`

- [ ] **Step 3: Commit**

```bash
git add crates/jcode-desktop/src/main.rs
git commit -m "feat: auto-detect and load workspace on startup"
```

---

## Task 7: Integration Test - Full TUI Workspace Flow

**Files:**
- Create: `crates/jcode-desktop/src/workspace_tui_integration_test.rs`

- [ ] **Step 1: Write integration test**

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_workspace_tui_full_flow() {
        use std::fs;

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

        // 4. Status text
        let status = format!("WS: {} [{}/{}]",
            ws.name(),
            ws.focused_project_index() + 1,
            ws.project_count()
        );
        assert_eq!(status, "WS: TUITest [1/2]");

        let _ = fs::remove_dir_all(temp_dir);
    }
}
```

- [ ] **Step 2: Run test**

Run: `cargo test --package jcode-desktop workspace_tui_integration_test -- --nocapture 2>&1`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add crates/jcode-desktop/src/workspace_tui_integration_test.rs
git commit -m "test: add workspace TUI integration test"
```

---

## Task 8: Documentation Update

**Files:**
- Modify: `docs/superpowers/specs/2026-05-14-workspace-project-design.md`

- [ ] **Step 1: Update with TUI integration status**

Add to Implementation Status:

```markdown
- [x] Phase 5: TUI Integration (Tasks 1-7)
  - [x] Task 1: ActiveWorkspace state
  - [x] Task 2: Workspace loading
  - [x] Task 3: Status bar workspace indicator
  - [x] Task 4: Project switching keys
  - [x] Task 5: Project surface rendering
  - [x] Task 6: Startup workspace detection
  - [x] Task 7: Integration tests
```

- [ ] **Step 2: Commit**

```bash
git add docs/superpowers/specs/2026-05-14-workspace-project-design.md
git commit -m "docs: update workspace design with TUI integration status"
```

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | ActiveWorkspace state struct | `workspace_state.rs` (new) |
| 2 | Workspace loading helper | `workspace_state.rs` |
| 3 | Status bar workspace indicator | `ui_status.rs` |
| 4 | Project switching key handling | `workspace.rs`, `main.rs` |
| 5 | Project surface rendering | `render_helpers.rs` |
| 6 | Startup workspace detection | `main.rs` |
| 7 | Integration tests | `workspace_tui_integration_test.rs` (new) |
| 8 | Documentation | design doc |

---

## Keyboard Shortcuts (Proposed)

| Key | Action |
|-----|--------|
| `gw` | Switch to next project in workspace |
| `gW` | Switch to previous project |
| `g=` | Show workspace picker |
| `Esc` | Exit workspace view (back to single session) |