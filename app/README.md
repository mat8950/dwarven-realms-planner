# Dwarven Realms Planner: desktop app

Tauri 2 + Svelte 5 UI of the planner. See the [main README](../README.md) for installing, running and building it.

- `src/`: the UI (Svelte 5, TypeScript). `src/lib/locales/` holds the planner's translations.
- `src-tauri/`: the desktop shell. It runs the engine (`crates/dr-core`) on two worker threads.
- `tests/i18n-check.mjs`: translation check, run by CI.
- `tests/shots*.mjs`: headless screenshots of the main views, against the HTTP bridge (`cargo run --release --bin serve`).
