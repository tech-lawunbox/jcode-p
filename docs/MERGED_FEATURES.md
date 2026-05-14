# Merged Features: Embedded Skills Harness + Tmux Pane Spawning

This document describes the merged features from:
- `chapzin/feature/embedded-skills-harness`
- `watzon/feature/tmux-pane-spawning`

## Merged Branch
- **Branch name**: `merge/feature/embedded-skills-harness`
- **Location**: `/Users/lawunbox/Desktop/jcode-master`

## Feature Summary

### 1. Embedded Skills Harness (chapzin)
Skills are now embedded in the binary using `include_str!`, enabling:
- Offline skill loading without internet access
- Built-in skills: `karpathy-guidelines`, `optimization`, `clean-code-guardian`, `llmwiki-memory`
- Deterministic skill router with priority: built-in → project → global → project-local
- CLI commands: `jcode skills list`, `jcode skills show`, `jcode skills sync`, `jcode skills doctor`

### 2. Tmux Pane Spawning (watzon)
Terminal multiplexing support for spawn operations:
- Native tmux integration for multi-pane workflows
- Enhanced session management in tmux environments
- Better terminal state preservation across spawns

## Usage

### Skills CLI
```bash
# List available skills
jcode skills list

# Show a specific skill
jcode skills show karpathy-guidelines

# Sync skills to ~/.jcode/skills
jcode skills sync

# Run diagnostics
jcode skills doctor
```

### Harness
```bash
jcode-harness run "fix this bug" --skill karpathy-guidelines
jcode-harness skills match "optimize memory" --json
```

### Tmux Integration
```bash
# Spawn sessions in tmux panes
jcode spawn --tmux

# List tmux sessions
jcode sessions list
```

## Installation

### Option 1: Build from source
```bash
cd /Users/lawunbox/Desktop/jcode-master
git checkout merge/feature/embedded-skills-harness
cargo build --release
cp target/release/jcode ~/.local/bin/jcode
```

### Option 2: Copy binary
```bash
cp /Users/lawunbox/Desktop/jcode-master/target/release/jcode ~/.local/bin/jcode
```

## Verification
```bash
jcode --version
jcode skills list
jcode skills doctor
```

## Notes
- Conflict resolution favored the current branch (embedded-skills-harness) for overlapping tmux files
- The `aws-sdk-bedrockruntime` duplicate was resolved to use the enhanced version with tokio runtime
- All conflicts were resolved deterministically; manual review recommended for production use
