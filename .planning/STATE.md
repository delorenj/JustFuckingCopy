---
gsd_state_version: 1.0
milestone: v2.0
milestone_name: Ambient Tray
status: defining_requirements
stopped_at: Milestone v2.0 started
last_updated: "2026-03-21T12:00:00.000Z"
progress:
  total_phases: 0
  completed_phases: 0
  total_plans: 0
  completed_plans: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-21)

**Core value:** Capture visible text from any screen region and get clean, deduplicated clipboard content with zero workflow interruption.
**Current focus:** Defining requirements for v2.0 Ambient Tray

## Current Position

Phase: Not started (defining requirements)
Plan: —
Status: Defining requirements
Last activity: 2026-03-21 — Milestone v2.0 started

## Performance Metrics

**Velocity:**

- Total plans completed: 0
- Average duration: -
- Total execution time: -

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| - | - | - | - |

**Recent Trend:**

- Last 5 plans: -
- Trend: -

*Updated after each plan completion*

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- [v1.0] Ollama over local OCR: consistent quality, single code path across platforms
- [v1.0] Hardcoded endpoint (192.168.1.12:11434): simplicity, one known instance
- [v1.0] Hard fail on unreachable: no degraded experience; clear error over silent failure
- [v1.0] Delete old OCR code: clean codebase, no dead code or feature flags
- [v1.0] reqwest with rustls-tls (not native-tls): avoids OpenSSL link complexity on Linux
- [v1.0] tokio as dev-dep only: Tauri owns runtime; dev-dep provides #[tokio::test] without conflicts
- [v2.0] Ambient tray over modal window: eliminates friction of window blocking screenshots
- [v2.0] Archive + clear batch lifecycle: preserves history, auto-resets for next batch
- [v2.0] Config.toml over settings GUI: tight scope for v2.0, GUI deferred to v2.1+
- [v2.0] Full screenshots over marquee crop: dedup handles overlap, no precise selection needed

### Pending Todos

None yet.

### Blockers/Concerns

- Tauri 2 system tray plugin compatibility and API surface needs research
- Global hotkey registration approach (Tauri plugin vs native) needs research
- File watcher library selection (notify crate vs alternatives) needs research
- Watch directory must handle rapid successive writes from screenshot tools

## Session Continuity

Last session: 2026-03-21
Stopped at: Milestone v2.0 started
Resume file: None
