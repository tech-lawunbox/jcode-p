# Plan: Subagent & Batch Config System

## Overview

**Phase 1 only** - Config system for subagent and batch tools (no skill selection yet).

Skill selection will be planned separately.

---

## Feature 1: Batch Max Parallel Config

### Problem
Hardcoded `const MAX_PARALLEL: usize = 10` in `tool/batch.rs:10`

### Solution

**Files to modify:**
| File | Change |
|------|--------|
| `src/config/config_file.rs` | Add `BatchConfig { max_parallel: Option<usize> }` |
| `src/config/env_overrides.rs` | Add `JCODE_BATCH_MAX_PARALLEL` parsing |
| `src/config/default_file.rs` | Add `[batch]` section |
| `src/tool/batch.rs` | Replace constant with config lookup |

**Config:**
```toml
[batch]
max_parallel = 10  # Env: JCODE_BATCH_MAX_PARALLEL
```

---

## Feature 2: Subagent Config System

### Problem
Subagent tool params hardcoded, no project-wide defaults.

### Current SubagentInput Fields

| Field | Type | Current Default |
|-------|------|-----------------|
| `description` | String | (required) |
| `prompt` | String | (required) |
| `subagent_type` | String | (required) |
| `model` | Option<String> | None |
| `session_id` | Option<String> | None |
| `output_mode` | SubagentOutputMode | `Answer` |
| `run_in_background` | bool | `false` |

### Solution

**Config section:**
```toml
[subagent]
# Execution mode (Env: JCODE_SUBAGENT_RUN_IN_BACKGROUND)
run_in_background = false

# Output mode (Env: JCODE_SUBAGENT_OUTPUT_MODE)
# Options: "answer" | "compact" | "full_transcript"
output_mode = "answer"

# Default model for subagents (Env: JCODE_SUBAGENT_MODEL)
# Empty = use parent's model
model = ""

# Tools blocked from subagents (Env: JCODE_SUBAGENT_BLOCKED_TOOLS)
# Always blocked: subagent, task, todo, todowrite, todoread
blocked_tools = []

# Notification on completion (Env: JCODE_SUBAGENT_NOTIFY)
notify = true

# Wake agent on completion (Env: JCODE_SUBAGENT_WAKE)
wake = true
```

### Files to modify

| File | Changes |
|------|---------|
| `src/config/config_file.rs` | Add `SubagentConfig` struct |
| `src/config/env_overrides.rs` | Add env var parsing |
| `src/config/default_file.rs` | Add `[subagent]` section |
| `src/tool/task.rs` | Read config defaults in execute() |

---

## Config Resolution Order

For each param:
```
Tool Input > Config > Hardcoded Default
```

Example `run_in_background`:
1. Tool input `run_in_background` value
2. Config `subagent.run_in_background`
3. Hardcoded default (`false`)

---

## Environment Variables

```bash
# Batch
JCODE_BATCH_MAX_PARALLEL=10

# Subagent
JCODE_SUBAGENT_RUN_IN_BACKGROUND=false
JCODE_SUBAGENT_OUTPUT_MODE=answer
JCODE_SUBAGENT_MODEL=
JCODE_SUBAGENT_BLOCKED_TOOLS=
JCODE_SUBAGENT_NOTIFY=true
JCODE_SUBAGENT_WAKE=true
```

---

## Implementation Steps

### Step 1: Add Config Structs

**File: `src/config/config_file.rs`**

```rust
#[derive(Debug, Deserialize)]
pub struct BatchConfig {
    pub max_parallel: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct SubagentConfig {
    pub run_in_background: Option<bool>,
    pub output_mode: Option<String>,
    pub model: Option<String>,
    pub blocked_tools: Option<Vec<String>>,
    pub notify: Option<bool>,
    pub wake: Option<bool>,
}

// Add to Config struct:
pub batch: BatchConfig,
pub subagent: SubagentConfig,
```

### Step 2: Add Env Overrides

**File: `src/config/env_overrides.rs`**

```rust
// Batch
batch.max_parallel = env::var("JCODE_BATCH_MAX_PARALLEL")
    .ok()
    .and_then(|v| v.parse().ok());

// Subagent
subagent.run_in_background = env::var("JCODE_SUBAGENT_RUN_IN_BACKGROUND")
    .ok()
    .map(|v| v != "false" && v != "0");
subagent.output_mode = env::var("JCODE_SUBAGENT_OUTPUT_MODE").ok();
subagent.model = env::var("JCODE_SUBAGENT_MODEL").ok().filter(|v| !v.is_empty());
subagent.blocked_tools = env::var("JCODE_SUBAGENT_BLOCKED_TOOLS")
    .ok()
    .map(|v| v.split(',').map(|s| s.trim().to_string()).collect());
subagent.notify = env::var("JCODE_SUBAGENT_NOTIFY")
    .ok()
    .map(|v| v != "false" && v != "0");
subagent.wake = env::var("JCODE_SUBAGENT_WAKE")
    .ok()
    .map(|v| v != "false" && v != "0");
```

### Step 3: Update Batch Tool

**File: `src/tool/batch.rs`**

```rust
const MAX_PARALLEL_DEFAULT: usize = 10;

fn max_parallel() -> usize {
    crate::config::config()
        .batch
        .max_parallel
        .unwrap_or(MAX_PARALLEL_DEFAULT)
}

// Update usages at line 171, 174
if params.tool_calls.len() > max_parallel() {
    return Err(anyhow::anyhow!(
        "Maximum {} parallel tool calls allowed",
        max_parallel()
    ));
}
```

### Step 4: Update Subagent Tool

**File: `src/tool/task.rs`**

```rust
// In execute(), resolve config defaults before creating session

let run_in_background = params.run_in_background
    .or_else(|| crate::config::config().subagent.run_in_background);

let output_mode = params.output_mode;
    // .or_else(|| config default for output_mode)

let subagent_model = params.model
    .or_else(|| crate::config::config().subagent.model.clone());

// For blocked tools, merge config + always-blocked list
let mut blocked = vec![
    "subagent".to_string(),
    "task".to_string(),
    "todo".to_string(),
    "todowrite".to_string(),
    "todoread".to_string(),
];
if let Some(config_blocked) = crate::config::config().subagent.blocked_tools {
    blocked.extend(config_blocked);
}
```

### Step 5: Update Config Template

**File: `src/config/default_file.rs`**

Add section:
```toml
[batch]
# Maximum parallel tool calls in batch operation
# Env: JCODE_BATCH_MAX_PARALLEL
max_parallel = 10

[subagent]
# Default: run subagents in background (false = blocking)
# Env: JCODE_SUBAGENT_RUN_IN_BACKGROUND
run_in_background = false

# Output mode: "answer" | "compact" | "full_transcript"
# Env: JCODE_SUBAGENT_OUTPUT_MODE
output_mode = "answer"

# Default model for subagents (empty = use parent's model)
# Env: JCODE_SUBAGENT_MODEL
model = ""

# Additional tools to block from subagents (comma-separated)
# Env: JCODE_SUBAGENT_BLOCKED_TOOLS
blocked_tools = []

# Notification settings
# Env: JCODE_SUBAGENT_NOTIFY
notify = true

# Env: JCODE_SUBAGENT_WAKE
wake = true
```

---

## Testing

```rust
#[test]
fn batch_max_parallel_from_config() {
    // Set config: batch.max_parallel = 5
    // Call batch with 6 tools
    // Expect error
}

#[test]
fn subagent_run_in_background_from_config() {
    // Set config: subagent.run_in_background = true
    // Call subagent without param
    // Verify background execution
}

#[test]
fn tool_param_overrides_config() {
    // Set config: output_mode = "full_transcript"
    // Call with output_mode = "answer"
    // Verify answer used
}
```

---

## Future: Skill Selection (Separate Plan)

Auto skill mapping will be planned separately after this config phase.

---

## User Config Example

**~/.jcode/config.toml**
```toml
[batch]
max_parallel = 15

[subagent]
run_in_background = true
output_mode = "compact"
model = "claude-sonnet"
notify = true
wake = true
```

---

## Approval

Please confirm:

| Item | ✓/✗ |
|------|-----|
| Batch config (`max_parallel`) | ? |
| Subagent config (run_in_background, output_mode, model, blocked_tools, notify, wake) | ? |
| Env var overrides for all | ? |
| Tool params override config | ? |
| 5-step implementation | ? |

**Ready to start coding?**