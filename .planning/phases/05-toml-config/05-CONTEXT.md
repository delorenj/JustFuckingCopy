# Phase 5: TOML Config - Context

**Gathered:** 2026-03-23
**Status:** Ready for planning

<domain>
## Phase Boundary

Add a `config.rs` module that loads runtime configuration from `~/.config/justfuckingcopy/config.toml`. Provides typed access to watch directory, global hotkey, and Ollama endpoint. Writes sane defaults when config file is missing. Falls back to defaults on malformed config with a warning.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion

All implementation choices are at Claude's discretion -- pure infrastructure phase.

Key constraints from REQUIREMENTS.md and PROJECT.md:
- Config path: `~/.config/justfuckingcopy/config.toml` (platform-correct via `dirs` crate or equivalent)
- Default watch directory: `~/data/ssbnk/hosted`
- Default hotkey: `Ctrl+Shift+C`
- Default Ollama endpoint: `http://192.168.1.12:11434`
- Config struct managed as Tauri state (like SharedState)
- Use `toml` crate for parsing, `serde` for derive
- Write default config file when missing (create parent dirs)
- Malformed config: warn to stderr, fall back to defaults (no crash)

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `serde` already in Cargo.toml with derive feature
- Tauri `manage()` pattern established for SharedState and LifecycleState in lib.rs
- `ollama.rs` has hardcoded `OLLAMA_ENDPOINT` constant that will later read from config

### Established Patterns
- State managed via `.manage()` in `run()` builder
- Error handling uses `Result<T, String>` throughout
- Constants defined at module top (e.g., `MAIN_WINDOW_LABEL`, `TRAY_TOOLTIP`)

### Integration Points
- `lib.rs:run()` -- config loaded at startup, managed as Tauri state
- `ollama.rs` -- endpoint will eventually read from config (Phase 7 wiring, not this phase)
- Phase 6 (watcher) and Phase 7 (hotkey) will consume config values

</code_context>

<specifics>
## Specific Ideas

No specific requirements -- infrastructure phase.

</specifics>

<deferred>
## Deferred Ideas

None -- discussion stayed within phase scope.

</deferred>
