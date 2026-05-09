# Fix: Pass Coordinator's Swarm ID Explicitly to Spawned Agents

**Date:** 2026-05-09
**Status:** ✅ IMPLEMENTED
**Root Cause:** When spawning agents via `swarm spawn`, if `working_dir` is not explicitly passed, the spawned agent defaults to root swarm `/Users/lawunbox/.git` instead of coordinator's swarm.

---

## FOUND: Request::CommSpawn Definition

**File:** `crates/jcode-protocol/src/lib.rs` (line 436)

```rust
/// Spawn a new agent session (coordinator only)
#[serde(rename = "comm_spawn")]
CommSpawn {
    id: u64,
    session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    working_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    initial_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    request_nonce: Option<String>,
    /// Optional run/generation id used to group workers spawned by one orchestration run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    run_id: Option<String>,
    // MISSING: swarm_id field!
},
```

---

## Key Structures Found

### 1. Request::CommSpawn (Protocol)
**File:** `crates/jcode-protocol/src/lib.rs` (line 436)

```rust
#[serde(rename = "comm_spawn")]
CommSpawn {
    id: u64,
    session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    working_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    initial_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    request_nonce: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    run_id: Option<String>,
    // MISSING: swarm_id field!
},
```

### 2. ToolContext (Tool Core)
**File:** `crates/jcode-tool-core/src/lib.rs` (line 30)

```rust
#[derive(Clone)]
pub struct ToolContext {
    pub session_id: String,
    pub message_id: String,
    pub tool_call_id: String,
    pub working_dir: Option<PathBuf>,
    pub stdin_request_tx: Option<tokio::sync::mpsc::UnboundedSender<StdinInputRequest>>,
    pub graceful_shutdown_signal: Option<InterruptSignal>,
    pub execution_mode: ToolExecutionMode,
    // MISSING: swarm_id field!
}
```

### 3. spawn_swarm_agent (Server)
**File:** `src/server/comm_session.rs` (line 264)

Function signature needs `explicit_swarm_id: Option<String>` parameter

---

## Problem Summary

### Current Flow (Buggy)
```
Coordinator (in swarm X: .../mobile/lawunbox_mobile/.git)
  → swarm spawn prompt="..."
    → Request::CommSpawn { working_dir: None, ... }
      → spawn_swarm_agent(working_dir=None)
        → resolve_spawn_working_dir() → falls back to coordinator's working_dir
        → BUT create_headless_session("create_session") ← NO PATH!
          → Agent has working_dir=None
          → swarm_id_for_dir(None) → None
        → Agent joins wrong swarm (root) or no swarm
```

### Why Reports Don't Reach Coordinator
1. Spawned agents join root swarm (`/Users/lawunbox/.git`) instead of coordinator's swarm
2. `swarm report` goes to root swarm, not to coordinator's session
3. Coordinator never receives the completion reports

---

## Solution: Add Explicit `swarm_id` Field to Spawn Request

### Files to Modify

| File | Change Type | Description |
|------|-------------|-------------|
| `src/tool/communicate.rs` | Modify | Add `swarm_id` to spawn request |
| `src/server/comm_session.rs` | Modify | Pass `swarm_id` to spawn handler |
| `src/server/headless.rs` | Modify | Use explicit `swarm_id` in session creation |
| `src/server/comm_control.rs` | Modify | Pass coordinator's swarm_id to spawn |
| `src/protocol_tests/comm_requests.rs` | Modify | Update test with new field |
| `src/server/comm_session_tests.rs` | Modify | Add test for swarm_id propagation |

---

## Implementation Details

### 1. Protocol Definition (src/protocol_tests/comm_requests.rs)

```rust
// BEFORE (line ~439)
let req = Request::CommSpawn {
    id: 59,
    session_id: "sess_coord".to_string(),
    working_dir: Some("/tmp/project".to_string()),
    initial_message: Some("Start here".to_string()),
    request_nonce: Some("planner-fresh-123".to_string()),
    run_id: Some("run-spawn".to_string()),
    // NO swarm_id field!
};

// AFTER
let req = Request::CommSpawn {
    id: 59,
    session_id: "sess_coord".to_string(),
    working_dir: Some("/tmp/project".to_string()),
    initial_message: Some("Start here".to_string()),
    request_nonce: Some("planner-fresh-123".to_string()),
    run_id: Some("run-spawn".to_string()),
    swarm_id: Some("/path/to/project/.git".to_string()),  // NEW
};
```

**Need to find the actual Request::CommSpawn struct definition.** Search for:
- `struct CommSpawn`
- `enum Request` with CommSpawn variant
- Proto file if exists

### 2. Tool Communication (src/tool/communicate.rs)

**Change 1: Line ~1729-1740 (spawn action)**
```rust
// BEFORE
"spawn" => {
    let request = Request::CommSpawn {
        id: REQUEST_ID,
        session_id: ctx.session_id.clone(),
        working_dir: params.working_dir.clone(),
        initial_message: params.spawn_initial_message(),
        request_nonce: Some(spawn_request_nonce(&ctx, params.operation_id.as_deref())),
        run_id: params.run_id.clone().or_else(|| Some(fresh_swarm_run_id(&ctx))),
        // NO swarm_id!
    };
```

```rust
// AFTER
"spawn" => {
    // Get coordinator's current swarm_id
    let coordinator_swarm_id = ctx.swarm_id.clone();  // NEW - need to add this field to ToolContext
    
    let request = Request::CommSpawn {
        id: REQUEST_ID,
        session_id: ctx.session_id.clone(),
        working_dir: params.working_dir.clone(),
        initial_message: params.spawn_initial_message(),
        request_nonce: Some(spawn_request_nonce(&ctx, params.operation_id.as_deref())),
        run_id: params.run_id.clone().or_else(|| Some(fresh_swarm_run_id(&ctx))),
        swarm_id: coordinator_swarm_id,  // NEW
    };
```

**Change 2: spawn_assignment_session (line ~499-506)**
```rust
// BEFORE
let spawn_request = Request::CommSpawn {
    id: REQUEST_ID,
    session_id: ctx.session_id.clone(),
    working_dir: params.working_dir.clone(),
    initial_message: None,
    request_nonce: Some(spawn_request_nonce(ctx, params.operation_id.as_deref())),
    run_id,
    // NO swarm_id!
};

// AFTER
let spawn_request = Request::CommSpawn {
    id: REQUEST_ID,
    session_id: ctx.session_id.clone(),
    working_dir: params.working_dir.clone(),
    initial_message: None,
    request_nonce: Some(spawn_request_nonce(ctx, params.operation_id.as_deref())),
    run_id,
    swarm_id: ctx.swarm_id.clone(),  // NEW
};
```

**Note:** May need to add `swarm_id` field to `ToolContext` struct. Search for `struct ToolContext`.

### 3. Server Comm Control (src/server/comm_control.rs)

**Change: Around line ~1207 (spawn_if_needed path)**

```rust
// BEFORE
match super::comm_session::spawn_swarm_agent(
    &req_session_id,
    &swarm_id,
    working_dir.clone(),
    None,  // initial_message
    run_id.clone(),
    sessions,
    // ... other params
)
.await

// AFTER
match super::comm_session::spawn_swarm_agent(
    &req_session_id,
    &swarm_id,
    working_dir.clone(),
    None,  // initial_message
    run_id.clone(),
    Some(swarm_id.clone()),  // NEW: explicit_swarm_id to ensure same swarm
    sessions,
    // ... other params
)
.await
```

### 4. Server Comm Session (src/server/comm_session.rs)

**Change 1: spawn_swarm_agent function signature (line ~264)**

```rust
// BEFORE
pub(super) async fn spawn_swarm_agent(
    req_session_id: &str,
    swarm_id: &str,
    working_dir: Option<String>,
    initial_message: Option<String>,
    run_id: Option<String>,
    // ... other params
) -> anyhow::Result<String>

// AFTER
pub(super) async fn spawn_swarm_agent(
    req_session_id: &str,
    swarm_id: &str,
    working_dir: Option<String>,
    initial_message: Option<String>,
    run_id: Option<String>,
    explicit_swarm_id: Option<String>,  // NEW: override for targeted swarm
    // ... other params
) -> anyhow::Result<String>
```

**Change 2: Inside spawn_swarm_agent, use explicit_swarm_id**

```rust
// Find where swarm_id is derived and add fallback
let resolved_working_dir = /* existing logic */;

let effective_swarm_id = explicit_swarm_id
    .or_else(|| swarm_id_for_dir(resolved_working_dir.as_ref().map(|s| PathBuf::from(s.as_str()))));
```

**Change 3: When calling create_headless_session**

```rust
// Before: create_headless_session(...)
// After: pass effective_swarm_id or ensure command includes path

let cmd = if let Some(ref dir) = resolved_working_dir {
    format!("create_session:{dir}")  // This already includes path
} else if let Some(ref explicit_id) = explicit_swarm_id {
    // If we have explicit swarm_id but no working_dir,
    // we need to either:
    // 1. Add swarm_id to headless session creation
    // 2. Derive working_dir from swarm_id
    format!("create_session:{explicit_id}")  // Use swarm_id as path fallback
} else {
    "create_session".to_string()
};
```

### 5. Headless Session (src/server/headless.rs)

**Change: Around line ~119**

```rust
// BEFORE
let swarm_id = if swarm_enabled {
    swarm_id_for_dir(working_dir.clone())
} else {
    None
};

// AFTER
let swarm_id = if swarm_enabled {
    // First check if we have explicit swarm_id passed via command
    // Command format: "create_session:/path/to/dir" or "create_session:/path/.git"
    // If path ends with .git, use it directly as swarm_id
    if let Some(ref dir) = working_dir {
        if dir.to_string_lossy().ends_with(".git") {
            Some(dir.to_string_lossy().to_string())
        } else {
            swarm_id_for_dir(working_dir.clone())
        }
    } else {
        swarm_id_for_dir(None)  // Will return None
    }
} else {
    None
};
```

**Better approach:** Pass swarm_id separately from working_dir in command:

```rust
// NEW command format: "create_session:/path/to/dir|SWARM_ID"
let (working_dir, explicit_swarm_id) = if let Some(path_str) = command.strip_prefix("create_session:") {
    let parts: Vec<&str> = path_str.split('|').collect();
    let dir = parts.first().map(|s| std::path::PathBuf::from(s.trim()));
    let swarm_id = parts.get(1).map(|s| s.trim().to_string());
    (dir, swarm_id)
} else {
    (None, None)
};

let final_swarm_id = explicit_swarm_id
    .or_else(|| swarm_id_for_dir(working_dir.clone()));
```

### 6. Test Updates (src/protocol_tests/comm_requests.rs)

```rust
#[test]
fn test_comm_spawn_roundtrip_with_swarm_id() -> Result<()> {
    let req = Request::CommSpawn {
        id: 59,
        session_id: "sess_coord".to_string(),
        working_dir: Some("/tmp/project".to_string()),
        initial_message: Some("Start here".to_string()),
        request_nonce: Some("planner-fresh-123".to_string()),
        run_id: Some("run-spawn".to_string()),
        swarm_id: Some("/tmp/project/.git".to_string()),  // NEW
    };
    // ... rest of test
}
```

---

## New Test Cases to Add

### Test 1: Spawn inherits coordinator's swarm_id
```rust
#[tokio::test]
async fn test_spawn_uses_coordinator_swarm_id() {
    // Setup: Create session in swarm X
    // Action: spawn with no working_dir
    // Assert: spawned agent is in swarm X
}
```

### Test 2: Explicit working_dir overrides swarm_id derivation
```rust
#[tokio::test]
async fn test_explicit_working_dir_takes_precedence() {
    // Setup: Coordinator in swarm X
    // Action: spawn with working_dir pointing to different git repo
    // Assert: spawned agent is in working_dir's swarm, not coordinator's
}
```

### Test 3: Report back to coordinator
```rust
#[tokio::test]
async fn test_spawned_agent_reports_to_coordinator() {
    // Setup: Create coordinator in swarm X
    // Action: spawn agent, agent calls swarm report
    // Assert: coordinator receives the report
}
```

---

## Files Summary

| # | File | Lines to Change | Description |
|---|------|-----------------|-------------|
| 1 | `tool/communicate.rs` | ~1733, ~502 | Add swarm_id to spawn requests |
| 2 | `server/comm_session.rs` | ~264, ~283 | Add explicit_swarm_id param, use in spawn |
| 3 | `server/comm_control.rs` | ~1207 | Pass swarm_id to spawn handler |
| 4 | `server/headless.rs` | ~40, ~119 | Parse swarm_id from command |
| 5 | `protocol_tests/comm_requests.rs` | ~439 | Update test with new field |
| 6 | `server/comm_session_tests.rs` | new | Add propagation test |

---

## Pre-Implementation Checklist

- [ ] Find actual struct definition for `Request::CommSpawn`
- [ ] Check if `ToolContext` has `swarm_id` field
- [ ] Verify `spawn_swarm_agent` signature in `comm_session.rs`
- [ ] Check for any other places that construct `Request::CommSpawn`

---

## Implementation Order

1. **First:** Find struct definitions (blocker - can't implement without knowing exact types)
2. **Second:** Add `swarm_id` field to Request enum/variant
3. **Third:** Update tool/communicate.rs to pass swarm_id
4. **Fourth:** Update server/comm_control.rs to pass coordinator's swarm_id
5. **Fifth:** Update server/comm_session.rs to accept and use explicit_swarm_id
6. **Sixth:** Update server/headless.rs to use explicit swarm_id
7. **Seventh:** Update tests
8. **Eighth:** Build and test

---

## Implementation Order (Updated with Verified Paths)

1. **`crates/jcode-protocol/src/lib.rs`** (~line 436) - Add `swarm_id` field to CommSpawn
2. **`crates/jcode-tool-core/src/lib.rs`** (~line 30) - Add `swarm_id` field to ToolContext
3. **`src/bin/harness.rs`** (~line 3815) - Populate swarm_id in ToolContext creation
4. **`src/tool/communicate.rs`** (~line 1730, ~499) - Pass swarm_id in spawn requests
5. **`src/server/comm_control.rs`** (~line 1207) - Pass coordinator's swarm_id to spawn
6. **`src/server/comm_session.rs`** (~line 264) - Accept and use explicit_swarm_id
7. **`src/server/headless.rs`** (~line 40, ~119) - Use explicit swarm_id in session creation
8. **`crates/jcode-protocol/src/protocol_tests/comm_requests.rs`** (~line 439) - Update tests

---

## Verification Checklist

Before implementing, verify these locations:
- [ ] Confirm line numbers with `grep -n "CommSpawn {" crates/jcode-protocol/src/lib.rs`
- [ ] Check `spawn_swarm_agent` full signature in `src/server/comm_session.rs`
- [ ] Find all places that construct `Request::CommSpawn` (8 found: grep output)
- [ ] Check `agent.swarm_id()` method exists

---

## PROBLEM VALIDATION

### Bug #1: Missing Path in Headless Session Command

**Location:** `src/server/comm_session.rs` (line 327-330)

```rust
let cmd = if let Some(ref dir) = resolved_working_dir {
    format!("create_session:{dir}")  // ✓ Has path
} else {
    "create_session".to_string()  // ✗ NO PATH - causes working_dir=None
};
```

**Impact:** When `resolved_working_dir` is None:
- Command becomes `"create_session"` (no path)
- Headless session receives None for working_dir
- `swarm_id_for_dir(None)` returns None (or `JCODE_SWARM_ID` env var if set)

---

### Bug #2: JCODE_SWARM_ID Environment Variable Override

**Location:** `src/server/util.rs` (line 94-100)

```rust
pub(crate) fn swarm_id_for_dir(dir: Option<PathBuf>) -> Option<String> {
    // BUG: This overrides any working_dir-based derivation!
    if let Ok(sw_id) = std::env::var("JCODE_SWARM_ID") {
        let trimmed = sw_id.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());  // Returns this regardless of working_dir
        }
    }
    let dir = dir?;
    // ...
}
```

**Impact:** If `JCODE_SWARM_ID` is set to root swarm path, all spawns go to root swarm regardless of coordinator's context.

---

### Bug #3: append_swarm_completion_report_instructions Adds Instructions But Agent is in Wrong Swarm

**Location:** `crates/jcode-swarm-core/src/lib.rs` (line 277-297)

```rust
pub fn append_swarm_completion_report_instructions(message: &str) -> String {
    // Adds "<system-reminder>SWARM COMPLETION REPORT REQUIRED..."
    // But agent is in wrong swarm, so report goes to wrong place
}
```

**Impact:** Even when instructions are added (line 314 in comm_session.rs), the agent joins the wrong swarm, so the report never reaches the coordinator.

---

## SOLUTION VALIDATION

### Proposed Fix (Solution 2: Pass Coordinator's Swarm ID Explicitly)

**Flow after fix:**

```
Coordinator (in swarm X)
  → swarm spawn prompt="..."
    → Request::CommSpawn { 
        working_dir: None,  // Could be None
        swarm_id: Some(".../mobile/lawunbox_mobile/.git")  // NEW: Explicit!
      }
    → spawn_swarm_agent(explicit_swarm_id=Some(".../mobile/lawunbox_mobile/.git"))
      → resolved_working_dir = resolve_spawn_working_dir(...)
      → target_swarm_id = explicit_swarm_id  // Used first!
      → create_headless_session("create_session:path|SWARM_ID")  // NEW: Includes swarm_id
        → Headless parses SWARM_ID from command
        → Agent created with correct swarm_id
        → Agent joins coordinator's swarm
        → swarm report reaches coordinator ✓
```

**Why it works:**
1. `swarm_id` is passed explicitly in the protocol
2. Server uses it even when `working_dir` is None
3. Headless session can use it directly or derive from working_dir

---

## Additional Fix: Update Swarm Instructions

**The `append_swarm_completion_report_instructions` function** (in `crates/jcode-swarm-core/src/lib.rs`) adds the report instructions, but the agent is in the wrong swarm.

**This function is NOT buggy** - it's working correctly. The fix to ensure agents join the correct swarm will make it work as intended.

---

## Testing Validation

### Test Case 1: Spawn Without Working Dir
```rust
#[test]
fn test_spawn_without_working_dir_joins_coordinator_swarm() {
    // Setup: Coordinator in swarm X
    // Action: spawn with working_dir=None, swarm_id=Some("swarm-X")
    // Assert: spawned agent has swarm_id = "swarm-X"
}
```

### Test Case 2: Report Delivery
```rust
#[test]
fn test_spawned_agent_reports_to_coordinator() {
    // Setup: Coordinator in swarm X
    // Action: spawn, agent calls swarm report
    // Assert: coordinator receives the report
}
```

### Test Case 3: Idempotency (Existing)
Already exists in `swarm_report_delivery_test.rs`:
```rust
fn append_swarm_completion_report_instructions_is_idempotent()
```

---

*Plan generated: 2026-05-09T17:34:00+00:00*
*Updated with verified file paths: 2026-05-09T17:39:00+00:00*
*Validation added: 2026-05-09T17:48:00+00:00*