# Workspace Project System Design

## Overview

Workspace is an **optional enhancement** for organizing multiple projects. Projects remain first-class citizens that work independently — workspace is a lens, not a requirement.

## Implementation Status

- [x] Phase 1: Core Structure (Tasks 1-4) - WorkspaceConfig, ProjectEntry, persistence, CLI commands
- [x] Phase 2: Memory Integration (Task 3, 8) - MemoryScope::Workspace, workspace memory directory
- [x] Phase 3: CLI Integration (Tasks 4-6) - /workspace command with create/activate/add-project
- [x] Phase 4: Project Scoping (Task 7) - project-scoped session loading
- [ ] Phase 5: TUI Integration (future) - workspace view in TUI

---

## Core Principles

1. **Projects are independent** — always work standalone, no workspace required
2. **Workspace is optional** — activated when user wants grouped view + cross-project context
3. **Session per-project** — each project has its own sessions, memory is per-session
4. **Workspace adds** — grouped view, workspace-level memory, shared notes

---

## File Structure

### `.workspace` (JSON — Machine Readable)

Location: User-specified (e.g., `~/my-workspace/.workspace`)

```json
{
  "version": "1.0",
  "name": "My Workspace",
  "path": "/Users/me/my-workspace",
  "projects": [
    { "name": "backend-api", "path": "/Users/me/projects/backend-api" },
    { "name": "frontend-react", "path": "/Users/me/projects/frontend-react" }
  ],
  "created_at": "2026-05-14T00:00:00Z",
  "last_activated": "2026-05-14T12:00:00Z"
}
```

### `.workspace.md` (Markdown — Human Readable)

Location: Same directory as `.workspace`

```markdown
# Workspace: My Workspace

## Projects
- [[backend-api]] — Backend API service
- [[frontend-react]] — React frontend

## Auto-Generated Summary

### Recent Activity
- backend-api: 5 sessions, last active 2h ago
- frontend-react: 3 sessions, last active 1d ago

### Shared Context
<!-- Cross-project context from workspace-level memory -->

## Notes

<!-- User notes section -->
```

---

## Project/Session Independence

### Without Workspace (Standalone)

```
Project → Open directly → Works
    ├── Own session(s) in ~/.jcode/sessions/{project-id}/
    ├── Own memory context (per-session)
    └── No linking to workspace needed
```

### With Workspace Activated

```
Workspace Activated
    ├── Project 1 → sessions + memory
    ├── Project 2 → sessions + memory
    └── Workspace Memory (cross-project context)
         └── Available to all projects
```

**Key**: Projects still work standalone. Workspace just adds grouped view + workspace-level context.

---

## Workspace Activation Flow

```
1. User selects/opens workspace
         │
         ▼
2. Load .workspace (JSON)
   - Verify file exists
   - Parse config
   - Validate project paths
         │
         ▼
3. For each project in config:
   - Verify path exists on disk
   - Load/open sessions for that project
   - Load project-level memory
         │
         ▼
4. Load workspace-level memory (cross-project context)
         │
         ▼
5. Render workspace view
   - Grid of all projects
   - Each project shows its surfaces/sessions
         │
         ▼
6. Workspace fully active
   - All project paths and context available
   - User can navigate between projects
   - .workspace.md notes accessible
```

---

## Workspace Creation Flow

### Command: `/workspace create`

```
User types: /workspace create
         │
         ▼
┌─────────────────────────────────────────────┐
│ PROMPT: Workspace name?                      │
│ Input: "My Workspace"                        │
└─────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────┐
│ PROMPT: Workspace location?                  │
│ Input: ~/jcode/workspaces/my-workspace        │
│        (or /custom/path)                     │
└─────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────┐
│ PROMPT: Add projects now?                    │
│ Options: [y] Yes, [n] No (add later)         │
│                                             │
│ If yes:                                     │
│   PROMPT: Select projects to add             │
│   - Browse/project picker                   │
│   - Multi-select supported                  │
│   - Add existing project paths manually      │
└─────────────────────────────────────────────┘
         │
         ▼
5. Create directory structure:
   /path/to/my-workspace/
   ├── .workspace        (JSON config)
   └── .workspace.md     (notes template)
         │
         ▼
6. Write .workspace
   - version: "1.0"
   - name: "My Workspace"
   - path: "/full/path"
   - projects: [] (or selected)
   - created_at: timestamp
         │
         ▼
7. Write .workspace.md template
   - Title, projects section (empty initially)
   - Auto-generated summary section
   - Notes section (empty)
         │
         ▼
8. SUCCESS: Workspace created
   Message: "Workspace 'My Workspace' created at ~/jcode/workspaces/my-workspace"
   Options: [Activate Now] or [Later]
```

---

## Adding Projects to Workspace

### Command: `/workspace add-project`

```
User types: /workspace add-project
         │
         ▼
┌─────────────────────────────────────────────┐
│ PROMPT: Select project type                 │
│                                             │
│ [1] Existing project on disk                │
│     → Browse/search directories             │
│                                             │
│ [2] Enter path manually                     │
│     → Type /path/to/project                  │
│                                             │
│ [3] New project (create from template)      │
│     → Scaffold new project                  │
└─────────────────────────────────────────────┘
         │
         ▼ (if 1 or 2)
┌─────────────────────────────────────────────┐
│ Verify project path exists                  │
│ Get project name (from dir or input)        │
└─────────────────────────────────────────────┘
         │
         ▼
Update .workspace:
{
  "projects": [
    { "name": "backend-api", "path": "/Users/me/projects/backend-api" },
    { ...existing... },
    { "name": "new-project", "path": "/Users/me/projects/new-project" }
  ]
}
         │
         ▼
Update .workspace.md:
- Add project to [[wiki-links]] list
- Refresh auto-generated summary
         │
         ▼
SUCCESS: Project added
Message: "Project 'new-project' added to workspace"
```

---

## Opening Projects (Two Paths)

### Path A: Direct Open (No Workspace)

```
User opens project: /project open /path/to/project
         │
         ▼
┌─────────────────────────────────────────────┐
│ Check: Is project in any workspace?         │
│                                             │
│ If yes: Load workspace context too          │
│ If no: Load standalone (default)             │
└─────────────────────────────────────────────┘
         │
         ▼
Create/open session:
  ~/.jcode/sessions/{project-id}/*.json
         │
         ▼
Load session memory:
  jcode-memory/{project-id}/*.md
         │
         ▼
Show project view (single project)
```

### Path B: Workspace Activation

```
User opens workspace: /workspace activate ~/my-workspace
         │
         ▼
Load .workspace config
         │
         ▼
For each project in config:
  - Load sessions
  - Load project memory
         │
         ▼
Load workspace memory
         │
         ▼
Show workspace view (all projects grouped)
```

---

## Memory Hierarchy

```
Workspace Level (cross-project)
  └── Project A Level (shared across sessions)
        └── Session 1 (per-conversation)
        └── Session 2
  └── Project B Level
        └── Session 3

Inheritance: Each layer inherits parent context but can override
```

---

## Session Behavior

| Scenario | Current | Proposed |
|----------|---------|----------|
| Open project directly | Floating session | Standalone session (no change) |
| Add to workspace | Not linked | Links to workspace (config) |
| Session memory | Per-session only | Per-session + project + workspace |
| Project without workspace | Works | Works (no change) |

---

## Edge Cases

### 1. Project path no longer exists
- Show warning in workspace view
- Mark project as "unavailable"
- Allow user to remove or update path

### 2. .workspace file deleted
- Workspace deactivates
- Projects remain open (standalone)
- Warn user workspace config missing

### 3. Project opened both standalone and via workspace
- Merge session history
- Unified memory (single project context)
- No duplicate sessions

### 4. Creating session in workspace vs outside
- Same behavior — session belongs to project
- Workspace provides view/context, doesn't own sessions

---

## File Locations

| File | Location | Created By |
|------|----------|------------|
| `.workspace` | User-specified | `/workspace create` |
| `.workspace.md` | Same dir as `.workspace` | Auto + manual |
| Sessions | `~/.jcode/sessions/` | Per-session |
| Project Memory | `jcode-memory/{project-id}/` | Per-project |
| Workspace Memory | `{workspace-path}/.workspace-memory/` | Workspace-level |

---

## Implementation Priority

1. **Phase 1: Core Structure**
   - `.workspace` file format
   - Workspace creation command
   - Add/remove project commands
   - Workspace activation flow

2. **Phase 2: Memory Integration**
   - Project-level memory
   - Workspace-level memory
   - Memory hierarchy implementation

3. **Phase 3: UI/UX**
   - Workspace view in TUI
   - Project switching
   - `.workspace.md` generation

4. **Phase 4: Polish**
   - Auto-generated summaries
   - Session persistence across restarts
   - Edge case handling