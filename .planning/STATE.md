---
gsd_state_version: 1.0
milestone: v2.0
milestone_name: Ambient Tray
status: unknown
stopped_at: Completed 08-01-PLAN.md
last_updated: "2026-03-23T07:42:01.144Z"
progress:
  total_phases: 5
  completed_phases: 5
  total_plans: 5
  completed_plans: 5
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-21)

**Core value:** Capture visible text from any screen region and get clean, deduplicated clipboard content with zero workflow interruption.
**Current focus:** Phase 08 — status-panel-ui

## Current Position

Phase: 08
Plan: Not started

## Performance Metrics

**Velocity:**

- Total plans completed: 4
- Average duration: -
- Total execution time: -

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 1 | 1 | - | - |
| 2 | 1 | - | - |
| 3 | 1 | - | - |
| 4 | 1 | - | - |

**Recent Trend:**

- Last 5 plans: 01-01, 02-01, 03-01, 04-01
- Trend: v1.0 shipped; v2.0 execution has started cleanly

*Updated after each plan completion*
| Phase 05 P01 | 3m 1s | 2 tasks | 3 files |
| Phase 06 P01 | 2m 28s | 2 tasks | 3 files |
| Phase 07 P01 | 92s | 2 tasks | 2 files |
| Phase 08 P01 | 109s | 2 tasks | 5 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- [v1.0] Ollama over local OCR: consistent quality, single code path across platforms
- [v1.0] Hardcoded endpoint (192.168.1.12:11434): simplicity, one known instance
- [v1.0] Hard fail on unreachable: no degraded experience; clear error over silent failure
- [v1.0] Delete old OCR code: clean codebase, no dead code or feature flags
- [v1.0] reqwest with rustls-tls (not native-tls): avoids OpenSSL link complexity on Linux
- [v1.0] tokio as dev-dep only: Tauri owns runtime; dev-dep provides `#[tokio::test]` without conflicts
- [v2.0] Ambient tray over modal window: eliminates friction of window blocking screenshots
- [v2.0] Archive + clear batch lifecycle: preserves history, auto-resets for next batch
- [v2.0] Config.toml over settings GUI: tight scope for v2.0, GUI deferred to v2.1+
- [v2.0] Full screenshots over marquee crop: dedup handles overlap, no precise selection needed
- [v2.0] Linux tray click events are unavailable in Tauri 2.10.3: provide tray-menu fallback instead of relying on click-only behavior
- [v2.0] Reuse the existing v1 panel during Phase 4: UI redesign waits until the dedicated status-panel phase
- [Phase 05]: AtomicU64 counter for unique test temp paths prevents parallel test race conditions
- [Phase 05]: AppConfig wired as Tauri managed state; downstream phases 06/07 can access via State<'_, AppConfig>
- [Phase 06]: Use notify::recommended_watcher directly (no extra dep) with EventKind::Create + RenameMode::To filtering
- [Phase 06]: Non-existent watch_dir logs warning and returns Ok (no app crash on startup)
- [Phase 07]: Global shortcut registered after app.build() using GlobalShortcutExt::on_shortcut (not in .setup()) to satisfy Tauri 2 plugin init ordering
- [Phase 07]: Only successfully OCR'd files are archived; failed files remain in watch_dir for retry
- [Phase 08]: Reuse existing process_batch async fn by wrapping it as process_batch_now Tauri command — zero duplication
- [Phase 08]: Auto-refresh batch state on window focus so panel reflects screenshots dropped while panel was hidden

### Pending Todos

None yet.

### Blockers/Concerns

- Config bootstrap for `watch_dir`, `hotkey`, and `ollama_endpoint` is the next dependency for watcher/hotkey phases
- Linux tray visibility still depends on system AppIndicator support at runtime
- Future phases must replace the marquee-focused UI without regressing the working OCR backend

## Session Continuity

Last session: 2026-03-23T07:41:30.562Z
Stopped at: Completed 08-01-PLAN.md
Resume file: None
