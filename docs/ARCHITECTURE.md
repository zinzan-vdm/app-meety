# Architecture

This document describes the on-disk structure of the repository: the
module map, the IPC contract between the Rust core and the React
frontend, and the key design decisions.

## Top-level layout

The workspace root is at `Cargo.toml` (virtual manifest). It has four
crate members and a React frontend:

- `crates/meety-core/` — the framework-agnostic library. Audio capture,
  transcription, diarization, storage, and LLM plumbing.
- `crates/meety-cli/` — CLI test harness (package name: `folio-cli`).
- `crates/meety-mcp/` — local MCP stdio server (package name: `folio-mcp`).
- `src-tauri/` — Tauri 2 desktop binary (package name: `meety-app`).
- `src/` — React + TypeScript + Tailwind frontend.
- `docs/` — architecture and engineering guidelines.
- `.github/workflows/` — CI and release pipelines.

Design docs drive the architecture. Coding standards are in
`docs/CODE_STYLE.md`. The `VERSION` file at the repo root holds the
display version string (calendar-based: `YYYY-MM-DD.R<N>`).

## meety-core (`crates/meety-core/`)

The framework-agnostic library. It talks to the OS for audio capture,
produces WAV files on disk, runs local Whisper for transcription, and
diarizes the system-audio track on-device (pyannote-segmentation-3.0
plus a WeSpeaker embedding model, both run through sherpa-onnx, with
the microphone always labelled **You**). It also owns the agent, memory,
and task stores, and talks to OpenAI for cloud transcription, chat
completions, and embeddings. It is designed to be embedded by the Tauri
desktop app, the CLI test harness, or a future Swift app via UniFFI.

The diarization pipeline lives in `diarization/`: `models.rs` resolves
and downloads the ONNX weights (gated by `cloud_guard`), `runtime.rs`
runs segmentation, embedding, and clustering, `identify.rs` matches
voice clusters against the speaker registry, and `session_speakers.rs`
and `label.rs` turn clusters into the speaker labels in the note.

### Rules

- `MeetyError` is the single public error type. New error categories
  are added there, not invented per module.
- Logging uses `tracing`, never `println!`. Audio callbacks are
  alloc-free hot paths; do not log from inside cpal or
  ScreenCaptureKit callback bodies.
- macOS-specific code is gated by `#[cfg(target_os = "macos")]` and
  has a stub for non-macOS targets so the workspace still builds.
- Types that cross the Tauri IPC boundary derive `ts_rs::TS` with
  `#[ts(export, export_to = "../../../src/shared/types/")]`. `cargo
  test` regenerates the bindings; CI catches drift.
- Two-phase write for any file-backed store: write the canonical
  on-disk file first (`.md` for memory, `.json` for tasks), update
  the derived index second. The index is rebuildable from files.
- New deps land in `[workspace.dependencies]` first (`Cargo.toml`),
  then crates reference them with `{ workspace = true }`.

## src-tauri (`src-tauri/`)

The Tauri 2 desktop binary. It is a thin wrapper: it imports
`meety-core`, exposes commands to the React frontend, and owns
macOS-specific window glue (Dock icon).

The `commands/` directory has one module per domain: health, devices,
settings, recording, transcription, library, llm, agents, tasks, memory,
maintenance, captions, webhooks, permissions, preferences, tray, and
windows.

### Capabilities

`src-tauri/capabilities/` holds one capability file per window class:

- `default.json` — main window only. Narrowed: no recursive `$HOME` fs
  grants; opener allowlist is per-host, not blanket `https://*`.
- `captions.json` — captions window. Renderer only; no fs or opener
  grants.
- `preferences.json` — Cmd-, NSWindow. No fs (everything funnels through
  canonicalised Tauri commands); opener limited to docs and Apple system
  settings.
- `secondary.json` — record, library, and editor secondary windows.
  Same surface as main minus the home-recursive fs grants.

Add new URL schemes through the relevant capability file's
`opener:allow-open-url` allowlist — never in JS.

### IPC contract

Every `#[tauri::command]` is the contract with the frontend. Command
names and argument shapes are stable; renaming one is a breaking
change. Argument and return types are defined in `meety-core` and
generated as TypeScript by `ts-rs`. Browse `src/shared/lib/ipc.ts`
for the authoritative list; `cargo test` regenerates the bindings.

Errors flow back as JSON strings on the `Err` side of the Result.
The frontend wraps them in `IpcError` for transport failures; domain
errors come through as strings.

## src/ (React frontend)

Feature-based layout. The key files and directories:

- `App.tsx` — router and providers (ErrorBoundary, Toaster, modals).
- `main.tsx` — React mount, `applyInitialTheme` before paint.
- `error-boundary.tsx` — root render-error fallback.

`shared/` holds cross-feature code:

- `shared/ui/` — shadcn primitives (button, dialog, switch, etc.).
- `shared/lib/` — typed IPC wrappers (`ipc.ts`), utilities (`utils.ts`),
  cost estimator, feedback sounds, power helpers.
- `shared/stores/` — Zustand stores for recording, settings, tasks,
  memories, jobs, and cloud cost confirmation.
- `shared/hooks/` — `use-theme`, `use-window-drag`.
- `shared/types/` — GENERATED by `ts-rs`. Do not hand-edit.

Each feature lives in `features/<name>/`: recording, library, editor,
inbox, tasks, memory, captions, onboarding, preferences-window, and
settings.

`chrome/` holds window chrome: sidebar, drag strip, job strip,
cheatsheet overlay, command palette, deep-link handler, and
global-shortcuts.

`styles/globals.css` has Tailwind layers, CSS-variable theme tokens,
and `prefers-reduced-motion` overrides.

### Rules

- `@/shared/types/*` is the single source of truth for IPC types.
  Never define a Tauri-side type by hand in TS; add it to `meety-core`
  with a `TS` derive and re-run `cargo test`.
- Cross-route state lives in Zustand stores under `shared/stores/`.
  Page-local state stays in `useState` inside the feature.
- Tauri calls go through `shared/lib/ipc.ts`. Components never call
  `invoke` directly.
- For Zustand selectors, subscribe to raw fields and `useMemo` derived
  values inside the consumer. Returning new object references from the
  selector triggers React's `Maximum update depth exceeded` guard.
- Sounds, motion, and confirm-dialog patterns route through shared
  `lib/` helpers and Zustand stores so they can be triggered from any
  layer without prop drilling.

## Data flow

The React frontend sends commands to the Tauri shell over JSON IPC.
The shell delegates to `meety-core` through direct function calls.
`meety-core` talks to OS APIs, OpenAI, whisper.cpp, and SQLite. All
audio capture, transcription, and storage happens in the Rust layer;
the frontend is a view layer.

## CI

`.github/workflows/ci.yml` runs on every push to `main` and every PR
against `main`. Jobs (all required):

- `rust-fmt` — `cargo fmt --all -- --check`
- `rust-clippy` — `cargo clippy --workspace --all-targets -- -D warnings`
- `rust-test` — `cargo build --workspace --all-targets`, then
  `cargo test --workspace --lib --bins`
- `rust-deny` — `cargo deny check`
- `typos` — `crate-ci/typos`
- `no-telemetry` — `scripts/check-no-telemetry.sh`
- `frontend` — `bun run lint`, `bun run typecheck`,
  `bun run format:check`, `bun run test`

`.pre-commit-config.yaml` mirrors most of these locally.

## Code styling — quick reference

For the full source-cited guidance see `docs/CODE_STYLE.md` and
`docs/guidelines/`. This is the condensed cheat sheet.

### Rust

- **Format**: `cargo fmt` (rustfmt config in `rustfmt.toml`). Run
  before every commit; pre-commit hook enforces.
- **Lint**: `cargo clippy --workspace --all-targets -- -D warnings`.
  Treat warnings as errors. Use `#[allow(clippy::name)]` with a
  one-line comment explaining why.
- **Errors**: return `Result<T, MeetyError>`. New error categories
  go on the public enum in `meety-core/src/error.rs`.
- **Async**: prefer `tauri::async_runtime::spawn_blocking` for any
  IPC command that touches the filesystem, SQLite, or whisper.cpp.
  See `docs/guidelines/rust-async.md`.
- **Locks**: `parking_lot::Mutex` for sync code; `tokio::sync::Mutex`
  only when held across `.await`.
- **Naming**: `snake_case` for modules, functions, variables;
  `PascalCase` for types, enums, trails; `SCREAMING_SNAKE` for consts.
- **Visibility**: prefer `pub(crate)` over `pub` unless the symbol
  is part of the external API.
- **Tests**: every public function gets at least a round-trip or
  happy-path test under `#[cfg(test)]`. Integration tests in `tests/`.
  CI runs `cargo test --workspace --lib --bins`.
- **Doc comments**: `///` on every `pub` item. First line is a
  noun phrase. Body explains WHY.

### TypeScript / React

- **Format**: `bun x prettier --write`.
- **Lint**: `bun run lint` (`eslint src --max-warnings 0`).
- **Files**: `kebab-case.tsx` or `kebab-case.ts`. Components export
  one default per file; named exports for utilities.
- **Imports**: order — React or 3rd party, lucide-react, shadcn UI,
  `@/shared/...`, `@/features/...`, relative.
- **State**: Zustand for cross-route state, `useState` for local.
- **IPC**: only `src/shared/lib/ipc.ts` imports `@tauri-apps/api/core`.
- **Types**: never hand-edit `@/ shared/types/*` — generated by `ts-rs`.
- **Styling**: Tailwind utilities + CSS variables in `globals.css`.
  No hard-coded hex oclours in components.
- **Motion**: honor `prefers-reduced-motion`.

## Conventions

See `docs/guidelines/` for the deep-dives, `AGENTS.md` for Rust-
specific rules, `CONTRIBUTING.md` for the human-facing setup and PR
flow, and `SECURITY.md` for vulnerability reporting.