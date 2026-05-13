# Status Bar Enhancement — Minimal Pulse Design

**Date:** 2026-05-14
**Status:** Approved
**Approach:** A — Minimal pulse

---

## Overview

Add a small animated indicator on the left side of the existing status bar that pulses during streaming. Memory state shown as a quiet icon badge. No layout changes to existing elements.

---

## Visual Elements

### 1. Streaming Indicator

- **Position:** Inside left padding of status bar, before existing content
- **Shape:** Small dot (8px circle) or wave pattern
- **Animation:** Subtle pulse — opacity cycles 0.5 → 1.0 → 0.5 at 1Hz
- **Colors:**
  - Streaming: accent green (`[0.030, 0.125, 0.080, 1.0]`)
  - Idle: dim gray (`[0.5, 0.5, 0.5, 0.3]`)

### 2. Memory State Badge

- **Position:** Next to model name display
- **Icon:** Small indicator (idle/active/privacy)
- **States:**
  - `Idle` — dim, no animation
  - `Active` — accent color, subtle glow
  - `Privacy` — lock icon when privacy mode engaged

---

## Implementation Scope

### Files to Modify

1. **`crates/jcode-desktop/src/main.rs`**
   - Add streaming pulse indicator rendering in `render_status_bar()`
   - Integrate with existing `ConnectionPhase` state for streaming detection
   - Pulse animation driven by `focus_pulse` timing or separate animation clock

2. **`crates/jcode-memory-types/src/lib.rs`**
   - Expose `MemoryActivity` state for UI polling
   - Ensure `MemoryState` enum has necessary variants for badge display

3. **`crates/jcode-tui-style/src/theme.rs`**
   - Add pulse colors if not already present in theme
   - Ensure streaming/active colors are defined

### No Changes To

- Status bar height (`STATUS_BAR_HEIGHT: 30.0`)
- Existing status text rendering
- Panel content rendering
- Layout calculations (`workspace_render_layout`, `visible_column_layout`)

---

## Animation Details

### Streaming Pulse

```
Cycle: 1000ms
Keyframes:
  - 0ms:   opacity 0.5
  - 500ms: opacity 1.0
  - 1000ms: opacity 0.5
```

Implementation via:
- Use `focus_pulse` (0→1→0 at ~2Hz) already used for surface focus
- Or add `streaming_pulse` f32 that increments each frame
- Color interpolation from idle dim to streaming accent

### Memory Badge

- No animation when idle
- Subtle glow pulse when `MemoryActivity::is_processing()` returns true

---

## State Sources

### Streaming State

From `ConnectionPhase` in `single_session.rs`:
- `Streaming` — stream active, show pulse
- `WaitingForResponse` — show pulse (anticipating stream)
- All other states — idle dim

### Memory State

From `MemoryActivity` struct:
- `state: MemoryState::Idle` → badge idle
- `state: MemoryState::Embedding` | `Extracting` | `Maintaining` | `ToolAction` → badge active
- Privacy mode → lock icon

---

## Success Criteria

1. Status bar renders without height/layout changes
2. Streaming indicator visible and animating during active stream
3. Memory badge shows correct state (idle/active/privacy)
4. No visual overlap with existing status text
5. Performance: animation runs at 60fps, no jank