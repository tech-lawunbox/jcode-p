# Plan: Subagent & Background Task Enhancements

## Overview

Two enhancements planned:
1. **Batch Max Parallel Config** - Make `MAX_PARALLEL=10` configurable via `config.toml`
2. **Subagent Config System** - Make all subagent parameters configurable with defaults

---

## Feature 1: Batch Max Parallel Config

### Problem
Hardcoded `const MAX_PARALLEL: usize = 10` in `tool/batch.rs:10` - no config option.

### Solution

#### Files to Modify

| File | Change |
|------|--------|
| `src/config/config_file.rs` | Add `BatchConfig` struct with `max_parallel` |
| `src/config/env_overrides.rs` | Add `JCODE_BATCH_MAX_PARALLEL` env override |
| `src/config/default_file.rs` | Add `[batch]` section template |
| `src/tool/batch.rs` | Replace hardcoded constant with config lookup |

#### Config Section

```toml
[batch]
# Maximum number of parallel tool calls in a single batch operation
# Setting this higher may increase API costs and memory usage
# Env: JCODE_BATCH_MAX_PARALLEL
max_parallel = 10
```

---

## Feature 2: Subagent Config System

### Problem
Subagent tool parameters are hardcoded defaults. No way to set project-wide defaults.

### Current SubagentInput Fields

| Field | Type | Hardcoded Default |
|-------|------|-------------------|
| `description` | String | (required) |
| `prompt` | String | (required) |
| `subagent_type` | String | (required) |
| `model` | Option<String> | None |
| `session_id` | Option<String> | None |
| `output_mode` | SubagentOutputMode | `Answer` |
| `run_in_background` | bool | `false` |

### Solution: Unified `[subagent]` Config Section

#### Config Structure

```toml
[subagent]
# Default behavior for subagent tool
# Env prefix: JCODE_SUBAGENT_

# --- Execution Mode ---
# Run subagents in background by default
# true = non-blocking, false = blocking (wait for completion)
# Env: JCODE_SUBAGENT_RUN_IN_BACKGROUND
run_in_background = false

# --- Output Control ---
# Output mode: "answer" | "compact" | "full_transcript"
# "answer" = final answer only (lowest tokens)
# "compact" = answer + human-readable transcript
# "full_transcript" = answer + raw JSON messages (debugging)
# Env: JCODE_SUBAGENT_OUTPUT_MODE
output_mode = "answer"

# --- Model Selection ---
# Default model for subagents (overridden by tool param)
# Set to "" or omit to use parent's model
# Env: JCODE_SUBAGENT_MODEL
model = ""

# --- Skill Selection ---
# How skills are selected for subagents
# "auto" = model sees all skills, decides via /skill (recommended)
# "explicit" = use skill_map below
# "none" = no skills by default
# Env: JCODE_SUBAGENT_SKILL_SELECTION
skill_selection = "auto"

# Explicit skill patterns per subagent_type (when skill_selection="explicit")
# Patterns: "skill-name" or "prefix-*" for wildcards
[subagent.skill_map]
general = ["*"]           # All skills (default)
coding = ["coding-*", "engineering", "refactor"]
research = ["research-*", "analysis"]
qa = ["qa-*", "testing-*"]
marketing = ["growth-*", "content-*"]

# --- Blocked Tools ---
# Tools blocked from subagent execution (always blocked: subagent, task, todo, todowrite, todoread)
# Add more tools to block here
blocked_tools = []

# --- Session Behavior ---
# Reuse existing session if available (vs creating new)
# true = use session_id if provided, false = always create new
reuse_session = false

# --- Notifications ---
# Send notification when background subagent completes
notify_on_completion = true

# Wake the agent immediately when background subagent completes
wake_on_completion = true

# --- Limits ---
# Maximum concurrent background subagents per session
# 0 = unlimited
max_concurrent = 0

# --- Swarm Integration ---
# Use swarm for multi-agent orchestration when spawning 2+ subagents
# true = auto-create swarm, false = sequential spawning
auto_swarm = false

# Swarm model override (uses agents.swarm_model if not set)
swarm_model = ""
```

### Additional SubagentInput Field for Skills

```rust
// New field in SubagentInput (tool/task.rs)
#[derive(Deserialize)]
struct SubagentInput {
    // ... existing fields ...

    /// Skills to load for this subagent (overrides config default)
    /// Can be: "auto", "none", or list of skill names/patterns
    /// Env: JCODE_SUBAGENT_SKILLS (default)
    #[serde(default)]
    skills: Option<SubagentSkills>,
}

enum SubagentSkills {
    Auto,                    // Model decides (respects config skill_selection)
    None,                    // No skills
    List(Vec<String>),       // Specific skill names
    Patterns(Vec<String>),   // Glob patterns
}
```

### Schema Update

```rust
// tool/task.rs - parameters_schema()
"skills": {
    "type": "string|array",
    "description": "Skills to load. 'auto' (model decides), 'none', or list of skill names."
}
```

---

## Files to Modify

### Config Layer

| File | Changes |
|------|---------|
| `src/config/config_file.rs` | Add `SubagentConfig`, `BatchConfig` structs |
| `src/config/env_overrides.rs` | Add all env var parsing |
| `src/config/default_file.rs` | Add `[subagent]` and `[batch]` sections |

### Tool Layer

| File | Changes |
|------|---------|
| `src/tool/task.rs` | Add `SubagentSkills` enum, config defaults in execute() |
| `src/tool/batch.rs` | Replace `MAX_PARALLEL` constant with config lookup |

### Skill Resolution (New File)

| File | Changes |
|------|---------|
| `src/tool/subagent_skills.rs` | NEW: Skill resolution based on subagent_type |

### System Prompt (Optional Enhancement)

| File | Changes |
|------|---------|
| `src/agent/prompting.rs` | Include skill manifest in subagent system prompt |

---

## Config Resolution Order

For each parameter, resolution order is:

```
Tool Input > Session Config > Global Config > Hardcoded Default
```

Example for `run_in_background`:
1. Check `params.run_in_background` (tool call value)
2. If None, check session's `subagent_model` (future: session-specific overrides)
3. If None, check global `[subagent]` config
4. If None, use hardcoded default (`false`)

---

## Environment Variables

All config options have env var overrides:

```bash
# Batch
JCODE_BATCH_MAX_PARALLEL=10

# Subagent
JCODE_SUBAGENT_RUN_IN_BACKGROUND=false
JCODE_SUBAGENT_OUTPUT_MODE=answer
JCODE_SUBAGENT_MODEL=
JCODE_SUBAGENT_SKILL_SELECTION=auto
JCODE_SUBAGENT_BLOCKED_TOOLS=
JCODE_SUBAGENT_REUSE_SESSION=false
JCODE_SUBAGENT_NOTIFY_ON_COMPLETION=true
JCODE_SUBAGENT_WAKE_ON_COMPLETION=true
JCODE_SUBAGENT_MAX_CONCURRENT=0
JCODE_SUBAGENT_AUTO_SWARM=false
JCODE_SUBAGENT_SWARM_MODEL=
```

---

## Implementation Order

### Phase 1: Batch Config (Quick)
1. Add `BatchConfig` to config_file.rs
2. Add env override in env_overrides.rs
3. Update tool/batch.rs
4. Add template to default_file.rs

### Phase 2: Subagent Basic Config
1. Add `SubagentConfig` to config_file.rs
2. Add env overrides for: `run_in_background`, `output_mode`, `model`
3. Update tool/task.rs execute() to read config
4. Update schema to reflect config-driven defaults

### Phase 3: Skill Selection
1. Add `SubagentSkills` enum to tool/task.rs
2. Create tool/subagent_skills.rs
3. Update Agent creation to use filtered skills
4. Update system prompt for skill manifest

### Phase 4: Advanced Config
1. Add `skill_map` support
2. Add `blocked_tools` config
3. Add `max_concurrent` enforcement
4. Add `auto_swarm` trigger logic

---

## Testing Plan

### Batch Config Tests
```rust
#[test]
fn batch_respects_config_max_parallel() {
    // Set JCODE_BATCH_MAX_PARALLEL=5
    // Call batch with 6 tools
    // Expect error "Maximum 5 parallel tool calls allowed"
}

#[test]
fn batch_respects_env_override() {
    // Test env var JCODE_BATCH_MAX_PARALLEL overrides config
}
```

### Subagent Config Tests
```rust
#[test]
fn subagent_uses_config_defaults() {
    // Set config: subagent.run_in_background = true
    // Call subagent without run_in_background param
    // Verify it runs in background
}

#[test]
fn subagent_tool_param_overrides_config() {
    // Set config: output_mode = "full_transcript"
    // Call subagent with output_mode = "answer"
    // Verify answer mode is used
}

#[test]
fn subagent_skills_auto_loads_all() {
    // Set skill_selection = "auto"
    // Spawn subagent
    // Verify all skills available
}

#[test]
fn subagent_skills_explicit_mapping() {
    // Set skill_map.coding = ["coding-*"]
    // Spawn subagent with subagent_type="coding"
    // Verify only coding-* skills available
}
```

---

## Backward Compatibility

- All config options have sensible defaults matching current behavior
- `run_in_background = false` preserves blocking default
- `output_mode = "answer"` preserves current token-saving default
- `skill_selection = "auto"` provides all skills (current behavior)
- Env vars override but don't break existing deployments

---

## Example Usage After Implementation

### User config (~/.jcode/config.toml)
```toml
[subagent]
run_in_background = true
output_mode = "compact"
skill_selection = "explicit"

[subagent.skill_map]
coding = ["coding-*", "engineering"]
review = ["review-*", "qa-*"]
research = ["research-*"]
```

### Tool call (minimal - uses config defaults)
```json
{
  "description": "Review PR",
  "prompt": "Review changes...",
  "subagent_type": "review"
}
```

### Tool call (override config)
```json
{
  "description": "Quick check",
  "prompt": "Check syntax...",
  "subagent_type": "coding",
  "run_in_background": false,
  "skills": ["coding-lint"]
}
```

---

## Approval Request

Please review and approve:

1. **Batch Config** - Make MAX_PARALLEL configurable? ✓/✗
2. **Subagent Config Section** - Add [subagent] with all these options? ✓/✗
3. **Skill Selection Modes** - Auto/Explicit/None? ✓/✗
4. **Env Var Overrides** - All options overridable via env? ✓/✗
5. **Phase Implementation Order** - 4 phases as outlined? ✓/✗
6. **Backward Compatibility** - Current defaults preserved? ✓/✗

Any modifications needed before I start implementation?