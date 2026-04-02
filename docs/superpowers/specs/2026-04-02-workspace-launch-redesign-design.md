# Workspace Launch Redesign

## Context

The launch screen currently renders a generic "file opener" — a hero search input, a two-column layout with a folder button and recent sessions list, and a review queue at the bottom. This redesign transforms it into a state-aware command center that prioritizes active unfinished work over passive history, separating GitHub PRs from local workspace sessions with explicit "heat" ordering (Local > Inbox > Archive).

---

## Layout

```
┌─ brand bar (top-left anchor) ─────────────── ⚙ ─┐

  [ Paste GitHub URL or type to search local sessions... ]

LOCAL · N                              + Open Folder
  [local workspace cards — dirty/active only]

INBOX · N                         [sort pills]
  [github pr cards]

──────────────────────────────────────────────────
  Open Folder (large CTA)  │  Recent Sessions
   only when LOCAL empty   │  (max 5, filtered)
```

- **LOCAL** appears above INBOX ("hotter" — work physically on the machine)
- **LOCAL** is hidden entirely when no workspace-worthy sessions exist
- **+ Open Folder** ghost button lives in the LOCAL section header; falls back to the large CTA at the bottom when LOCAL is empty
- **Recent Sessions** capped at 5, excludes any session already shown in LOCAL or INBOX
- **Search bar** is slim (40px, no hero padding), utility-first

---

## Agent Status Contract

### Communication flow

The agent announces its state to the existing local HTTP server (port 8789) rather than the app hunting for files. Event-driven, zero polling overhead.

**Endpoint:** `POST http://localhost:8789/agent/status`

**Payload:**
```json
{
  "session_id": "uuid-123",
  "status": "busy",
  "task": "Running clippy",
  "pid": 12345
}
```

**Fields:**
- `status`: `"busy"` | `"idle"` | `"error"`
- `task`: human-readable current operation (e.g., `"Running clippy"`, `"Applying patches"`, `"Analyzing auth.rs"`)
- `pid`: agent process ID — used for crash detection on app restart
- `session_id`: identifies which local session to update; the agent must know its session ID (provided by the app when the session is created)

**Server behavior on receiving POST:**
1. Write `agent_status`, `agent_task`, `agent_pid`, `last_heartbeat` to the sessions SQLite table for the given `session_id`
2. Emit `"agent-status-changed"` Tauri event to the frontend immediately

### Crash detection — two layers

**Layer 1 — Heartbeat timeout (frontend):** When a LOCAL card shows `status = busy`, the frontend checks `last_heartbeat`. If `now - last_heartbeat > 60s` → amber dot transitions to red dot + `"Agent: Stalled"`. 60s accounts for heavy synchronous work (Rust compilation, large tree-sitter sweeps) where the agent may be CPU-pegged and unable to send interim heartbeats. Agents should emit a heartbeat immediately before starting any long-running synchronous task.

**Layer 2 — PID check on app launch (Rust):** On startup, before the frontend loads, the Rust backend queries all sessions with `agent_status = busy`, checks each stored PID via `kill(pid, 0)`. Dead PID → write `agent_status = interrupted` to the DB. Frontend sees the correct state from first render.

### UI behavior

- `status = busy`, PID alive, heartbeat fresh → pulsing amber dot + `"Agent: {task}"`
- `status = busy`, heartbeat stale (>30s) → red dot + `"Agent: Stalled"`
- `status = interrupted` (set at launch) → red dot + `"Agent: Interrupted"` + × button to clear
- `status = idle` or no agent columns set → no dot shown
- Clicking × on interrupted/stalled → clears agent columns in DB, emits `"agent-status-changed"`, triggers git status refresh so the app re-evaluates `is_workspace_local` (the agent may have left the repo in a dirty/non-compiling state — the session should stay in LOCAL for manual repair if so)

### Database schema additions (sessions table)

```sql
agent_status    TEXT     -- 'idle' | 'busy' | 'error' | 'interrupted'
agent_task      TEXT
agent_pid       INTEGER
last_heartbeat  TEXT     -- RFC3339 timestamp
```

### Log streaming (contract only — UI deferred)

**Endpoint:** `POST http://localhost:8789/agent/log`

**Payload:**
```json
{ "session_id": "uuid-123", "line": "warning: unused variable `x`", "stream": "stderr" }
```

Agents can start sending logs now. Rendering them (expanded card / activity panel) is a separate spec — the launch screen card is too small for a log feed.

---

## Rust: WorkspaceLocal Classification

### New fields on `ReviewHistoryItem`

```rust
pub branch_name: Option<String>,   // HEAD branch shortname; None if detached
pub is_workspace_local: bool,       // true = show in LOCAL section
pub agent_status: Option<AgentStatus>, // from agent.status file, if present
```

```rust
pub struct AgentStatus {
    pub status: String,   // "busy" | "idle" | "error"
    pub task: String,
    pub is_pid_alive: bool,
}
```

### is_workspace_local logic

A local session is `is_workspace_local = true` if **either**:
1. `repo.statuses(include_untracked: false)` is non-empty (tracked dirty files in index or workdir), **or**
2. `agent_status.status == "busy"` (agent is actively running)

A clean repo with an idle/absent agent is **not** workspace-local — it migrates immediately to Recent Sessions the moment the last commit lands and the agent stops. This gives the user a clear "sense of completion."

Branch name from `repo.head()?.shorthand()`. Detached HEAD → `None`.

### recent_files vs workspace_local routing

- `is_workspace_local = true` → included in a new `workspace_local: Vec<ReviewHistoryItem>` field on `ReviewHistory`
- `is_workspace_local = false` → stays in `recent_files` (the archive)
- Recent Sessions on the frontend excludes IDs present in `workspace_local` or GitHub inbox queue

---

## UI: LOCAL Section Card

```
● Agent: Running clippy    feature/auth-refactor    3m ago
                                                   [Resume]
```

- **Amber pulsing dot** when agent is busy and PID alive
- **Red dot** when file exists but PID is dead ("Agent: Interrupted")
- **Branch name** — primary text, high contrast
- **Relative time** — secondary, muted
- Button label: **"Resume"** (not "Open") — reinforces ongoing context
- Clicking opens the session via `resumeReviewSession()`

---

## UI: Transitions

- LOCAL section appearance/disappearance: **200ms ease-in-out** CSS height/opacity transition — no abrupt layout shift when first folder is selected
- INBOX already has sort pills, no changes to card rendering

---

## UI: Search Bar

- Slim — 40px height, reduced padding (`10px 16px 10px 40px`)
- Placeholder: `Paste GitHub URL or type to search local sessions...`
- Remove "Start a review" label above input (redundant with placeholder)
- Remove "or drag a folder anywhere" hint line (already deleted in prior session)

---

## Recent Sessions

- Capped at 5 (down from 6)
- Filtered: excludes session IDs present in `workspace_local` or `githubReview.queue`
- No scroll — if all 5 slots are filled, oldest drops off
- Rendered identically to today

---

## Files to Modify

**Rust:**
- `src-tauri/src/commands/review_history.rs` — add `WorkspaceLocal` classification, branch name fetch, read agent columns from DB for `ReviewHistoryItem`
- `src-tauri/src/storage.rs` — add `agent_status`, `agent_task`, `agent_pid`, `last_heartbeat` columns to sessions table; add `update_agent_status()` and `clear_agent_status()` methods
- `redpen-server` (HTTP server crate) — add `POST /agent/status` and `POST /agent/log` (store only) routes; emit `"agent-status-changed"` Tauri event on status POST
- `src-tauri/src/lib.rs` — on startup, PID-check all `busy` sessions before frontend loads; write `interrupted` for dead PIDs

**TypeScript bindings:**
- `src/lib/bindings/ReviewHistory.ts` — add `workspaceLocal` field
- `src/lib/bindings/ReviewHistoryItem.ts` — add `branchName`, `isWorkspaceLocal`, `agentStatus` fields

**Frontend:**
- `src/components/GitHubInbox.svelte` — restructure layout, add LOCAL section, agent status rendering, slim search, transition CSS, Recent Sessions dedup filter
- `src/lib/tauri.ts` — add Tauri event listener for agent status poll events

---

## Verification

1. Dirty repo → local session appears in LOCAL with correct branch name
2. Clean repo, active session → still appears in LOCAL
3. Clean repo, completed session → does not appear in LOCAL, appears in Recent Sessions
4. Agent POSTs `status=busy` → amber dot + task text appears on correct LOCAL card immediately
5. Agent POSTs `status=idle` → dot disappears
6. No agent POST / columns null → no dot
7. App restarts with `agent_status=busy` in DB, PID dead → red dot + "Agent: Interrupted" from first render
8. Heartbeat goes stale (>30s since last POST) → frontend transitions amber to red + "Agent: Stalled"
9. User clicks × on interrupted/stalled card → agent columns cleared, dot disappears
10. LOCAL empty → section hidden, large "Open Folder" CTA visible at bottom
11. LOCAL has items → "Open Folder" in LOCAL header, large CTA hidden
12. Selecting folder → LOCAL slides in with 200ms transition, INBOX doesn't jump
13. Recent Sessions excludes items shown in LOCAL/INBOX, capped at 5
