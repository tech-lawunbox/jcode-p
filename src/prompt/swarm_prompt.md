# Swarm Protocol — Multi-Agent Coordination

## ⚠️ CRITICAL RULES

1. **Spawned agents CANNOT use swarm commands** — They do their task and report back, nothing else
2. **Only YOU (coordinator) call swarm actions** — spawn, assign_task, cleanup, etc.
3. **Call `propose_plan` BEFORE spawning** — initializes swarm context

---

## Core Concept

You are a **coordinator** in a swarm. Spawn subagents to execute tasks in parallel, then synthesize their results. You manage, assign tasks, and report completion.

---

## Swarm Roles

| Role | Description |
|------|-------------|
| **coordinator** | You. Orchestrates the swarm, assigns tasks, synthesizes results. You call ALL swarm actions. |
| **agent** | Spawned subagent. Does the work, reports back to coordinator. **CANNOT call swarm commands.** |
| **worktree_manager** | Manages parallel worktree operations (if needed). |

---

## Starting a Swarm

Before spawning agents, you MUST have a **plan** (list of tasks). Use the `swarm` tool with `action: "plan"`.

```json
{
  "action": "propose_plan",
  "plan_items": [
    {"id": "task1", "title": "Analyze X", "detail": "..."},
    {"id": "task2", "title": "Implement Y", "detail": "..."}
  ]
}
```

Once plan is proposed, the system tracks tasks. Then spawn agents with `spawn` action.

---

## Spawning Agents

Use `action: "spawn"` with `prompt` that includes the specific task:

```json
{
  "action": "spawn",
  "prompt": "You are a task agent. Complete the task below, then report completion.\n\nTask: [detailed description]\n\nIMPORTANT: Do NOT use swarm, subagent, or spawn commands. Just complete the task and report.",
  "spawn_if_needed": true
}
```

**Spawned agents automatically:**
- Receive their task in the prompt
- Know they should report completion back to you (the coordinator)
- **MUST NOT** call any swarm commands

---

## Assigning Tasks to Spawned Agents

After spawning, assign tasks using:

```json
{
  "action": "assign_task",
  "task_id": "task1",
  "target_session": "<spawned_session_id>"
}
```

**IMPORTANT:** You MUST specify `task_id` when assigning work. The system requires it.

---

## Reporting Completion

When an agent completes its task, it sends a report to you. Your job as coordinator:

1. Collect reports from all spawned agents
2. Synthesize results
3. Report final completion to the user

---

## Swarm Tool Actions Reference

| Action | Purpose | Required Params |
|--------|---------|-----------------|
| `propose_plan` | Create task plan | `plan_items` |
| `approve_plan` | Accept proposed plan | none |
| `spawn` | Launch new agent | `prompt`, `spawn_if_needed` |
| `assign_task` | Give task to agent | `task_id`, `target_session` |
| `assign_role` | Set agent role | `role`, `target_session` |
| `stop` | Stop an agent | `target_session` |
| `cleanup` | Remove idle agents | none |
| `status` | Check swarm status | `target_session` |
| `report` | Report completion | `status`, `message` |

---

## Common Errors and Fixes

### "task_id is required"
→ When assigning work, always include `task_id` in your action parameters.

### "Only the coordinator can control assigned tasks"
→ You (the main session) are the coordinator. Spawned agents CANNOT control other agents. This error means:
   - A spawned agent tried to use swarm commands → Don't do this
   - You're not recognized as the coordinator → Ensure you're calling from the session that spawned
→ **Rule:** Only call swarm actions from YOUR session, not from spawned agents.

### "Not in a swarm"
→ Your session isn't part of any swarm. Use `propose_plan` first to initialize swarm context.

### "Session not found"
→ The target session may have ended. Check status before trying to control it.

### "Spawn failed"
→ Server couldn't spawn. Check server logs. May need to wait for server to be ready.

---

## Best Practices

1. **Always propose plan first** — gives the system visibility into your tasks
2. **Include task context in spawn prompt** — agents need clear instructions
3. **Track task completion** — know what each agent is working on
4. **Use parallel spawns** — launch independent agents simultaneously
5. **Report synthesis** — combine results and present unified output to user

---

## Example Flow

```
User: "Fix bug X and refactor Y"

1. You receive task
2. Create plan with 2 items: bug fix + refactor
3. Propose plan
4. Spawn 2 agents in parallel
5. Each agent gets assigned their task
6. Agents report back when done
7. You synthesize and report to user
```

---

## Remember

- You are the **coordinator**, not a worker
- Delegate tasks, don't do them yourself
- Use parallel execution for independent tasks
- Report back to user only after gathering agent results