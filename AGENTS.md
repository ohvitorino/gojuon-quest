# AGENTS.md

Guidance for AI coding agents working in this repository.

## Project Overview

- Language: Rust (edition 2024)
- App type: terminal UI quiz game (hiragana practice)
- Entry point: `src/main.rs`
- Core modules:
  - `src/app.rs` - state machine and game logic
  - `src/input.rs` - key handling and state transitions
  - `src/ui.rs` - rendering with `ratatui`
  - `src/kana.rs` - kana datasets and column definitions

## Local Workflow

- Run app:
  - `cargo run`
- Run tests:
  - `cargo test`
- Recommended checks before finishing work:
  - `cargo fmt`
  - `cargo clippy --all-targets --all-features`
  - `cargo test`

## Testing Conventions

- Unit tests are colocated in module files with `#[cfg(test)] mod tests`.
- Prefer focused tests for:
  - state transitions in `AppState`
  - scoring, streaks, and mode limits
  - input handling for each key path
- Avoid brittle tests tied to UI formatting details unless behavior depends on them.

## Code Style Expectations

- Keep changes small and scoped.
- Preserve existing naming and module boundaries.
- Prefer clear state transitions over clever abstractions.
- Add comments only where intent is not obvious from code.

## Git and Remote Policy

- Canonical remote is GitHub:
  - `origin = git@github.com:ohvitorino/gojuon-quest.git`
- Do not re-add GitLab remote unless explicitly requested.
- Do not use force-push on `main` unless explicitly requested.

## Safety Notes

- Do not commit secrets or credentials.
- Avoid destructive git commands unless explicitly requested.
- If unexpected unrelated file changes appear, stop and ask before proceeding.
