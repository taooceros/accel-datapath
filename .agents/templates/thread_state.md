# Canonical Thread State Template

Use this template for every live thread file under `.agents/state/threads/`.

## Canonical location and authority

- Canonical mutable thread state lives only under `.agents/state/threads/`.
- `current.md` is a dashboard/index only. It is not the source of truth for per-thread details.
- Thread IDs must use the format `thr-YYYYMMDD-<slug>`.
- `source_of_truth_scope` should state that the thread file is the canonical mutable record for this thread.

## Ownership and lease rules

- A live thread is owned by one agent/session at a time.
- The default lease expires 4 hours after the most recent owner update.
- Treat `last_updated` as the most recent owner update time.
- Set `lease_expires_at` to 4 hours after `last_updated`.
- Takeover is allowed only after lease expiry or explicit handoff.
- Explicit handoff means `handoff_to` and `handoff_reason` are populated for transfer.

## Update ordering

1. Update the canonical thread file first.
2. Refresh the `current.md` dashboard entry second.

## Auto-resume rule

- On startup, match the current request against `index_label`, `summary`, `match_hints`, and `related_artifacts`.
- If exactly one live thread matches, resume it by writing a new `owner_session_id` and moving the old value into `previous_owner_session_id`.
- Matching `owner_agent` alone is insufficient for auto-resume.
- If no live thread matches, create a new thread.
- If multiple live threads match, ask one disambiguation question before claiming a thread.

## Template

```yaml
thread_id: thr-YYYYMMDD-slug
title: <human readable thread title>
status: active
owner_agent: <agent name>
owner_session_id: <current owning session id>
previous_owner_session_id: <prior owning session id or null>
lease_acquired_at: <ISO 8601 timestamp>
lease_expires_at: <ISO 8601 timestamp, default 4 hours after last_updated>
last_updated: <ISO 8601 timestamp for most recent owner update>
handoff_to: <target agent or session, or null>
handoff_reason: <reason for explicit handoff, or null>
resume_allowed: true
match_hints:
  - <request keyword or phrase>
superseded_by: <new thread_id or null>
source_of_truth_scope: .agents/state/threads/ canonical mutable thread state for this thread
index_label: <short dashboard label>
summary: <short thread summary>
next_actions:
  - <next action>
blocked_by:
  - <blocker or empty list>
related_artifacts:
  - <path to plan, report, code, or result>
```
