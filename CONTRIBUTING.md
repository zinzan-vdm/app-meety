# Contributing to Meety

This document covers setup, conventions, and the review process.

## Setup

### Prerequisites

- macOS 13.3 or later (Apple Silicon recommended), or Windows 10+, or Linux with PipeWire or PulseAudio.
- Rust 1.88+ via `rustup` (pinned in `rust-toolchain.toml`).
- [Bun](https://bun.sh) 1.3+ (the only JS package manager and runtime).
- Xcode command-line tools: `xcode-select --install` (macOS only).
- Linux: development packages for PipeWire or PulseAudio.

### First-time setup

```sh
git clone git@github.com:zinzan-vdm/app-meety.git
cd app-meety
bun install
pre-commit install
pre-commit install --hook-type commit-msg
pre-commit install --hook-type pre-push
```

If `pre-commit` is not installed:

```sh
brew install pre-commit         # or: pip install pre-commit
```

### Editor

The repo ships with `.editorconfig`, `.prettierrc.json`, `rustfmt.toml`, `eslint.config.js`, and `clippy.toml`.
Recommended VS Code or Cursor extensions: `rust-analyzer`, `tauri-vscode`, `tailwindcss-intellisense`, `prettier-vscode`, `dbaeumer.vscode-eslint`.

## Conventions

The authoritative style contract is [`docs/CODE_STYLE.md`](docs/CODE_STYLE.md). Read it first. It covers naming, comments, errors, logging, tests, concurrency, performance, security, and git hygiene. The summary below highlights the rules contributors hit most often.

### Rust

- Typed errors via `thiserror`. New variants go in `crates/meety-core/src/error.rs` (`MeetyError`).
- No `unwrap` outside `#[cfg(test)]`. `expect("reason")` is acceptable for invariants.
- All `unsafe` blocks need a `// SAFETY:` comment.
- Logging via `tracing`, never `println!`.
- 4-space indent, 100-char width (`rustfmt.toml` enforces).
- No allocations on audio hot paths. Pre-allocate buffers.
- **No inline `//` body comments.** Doc-comments above declarations only. See [`docs/CODE_STYLE.md` §1](docs/CODE_STYLE.md#1-comments).

### TypeScript or React

- Strict mode is on. No `any`. Use `import type` for type-only imports.
- Function components only. Effects must have a cleanup if they subscribe.
- Tauri command names live in `src/shared/lib/ipc.ts`. Types are generated from Rust via `ts-rs` into `src/shared/types/` — never edit those by hand.
- **No inline `//` or `/* */` body comments.** JSDoc above the export only.
- Effects MUST clean up subscriptions. Use a local `cancelled` flag for race-condition guards.

### Git

Conventional-commit style with a scope, lowercase subject, imperative mood:

```
<type>(<scope>): <subject>

<optional body explaining WHY>
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`. Enforced by the `commit-msg` hook.

- Commits are **GPG-signed**. Pre-commit hooks enforce this.
- Commits are **never co-authored by an AI agent.** No `Co-Authored-By:` trailers.
- See [`docs/CODE_STYLE.md` §10](docs/CODE_STYLE.md#10-git-hygiene) for the full rules.

## Workflow

### Branching

```sh
git checkout -b feat/<short-name>     # new feature
git checkout -b fix/<short-name>      # bug fix
git checkout -b refactor/<scope>      # internal restructuring
git checkout -b docs/<topic>          # documentation only
git checkout -b chore/<scope>         # tooling, build, deps
```

### Local commands

```sh
# Frontend
bun run dev           # vite dev server
bun run typecheck     # tsc --noEmit
bun run lint          # eslint
bun run format        # prettier --write

# Backend
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo fmt --all

# Full app
bun tauri dev         # development build
bun tauri build       # release build
```

`pre-commit` runs the relevant checks on every commit.

### Pull requests

- Open a PR against `main`.
- CI must be green: `fmt`, `clippy`, `cargo test`, `bun run typecheck`, `bun run lint`, `cargo-deny`.
- Describe the _why_. The diff shows the _what_.
- Link to the issue if one exists.
- Keep PRs scoped to one concern. Refactors and feature work go in separate PRs.

### Reporting bugs and proposing features

Use the templates under `.github/ISSUE_TEMPLATE/`. For non-trivial features, open a discussion or issue first.

## Maintainer workflow

This section is for the project maintainer. Outside contributors can skip it.

1. **Pick an issue.** Move it to `In Progress`.
2. **Branch.** `git checkout -b <type>/<slug>`.
3. **Write the change.** Follow `docs/CODE_STYLE.md`.
4. **Test locally.** `cargo fmt && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace --all-targets && bun run typecheck && bun run lint && bun run test`.
5. **Commit.** Conventional commit (`feat: <subject>`).
6. **Push and open a PR.** `gh pr create --title "<type>: <title>"`.
7. **Merge.** `gh pr merge --merge --delete-branch`. CI must be green.
8. **Pull.** `git pull --ff-only` back on main.

### Release

See `docs/guidelines/release-engineering.md`. The short version:

- Update the CHANGELOG.
- Tag: `git tag 2026-MM-DD.R<N> && git push --tags`.
- The release workflow builds signed binaries for macOS, Windows, and Linux.

## License

By contributing, you agree that your contributions are licensed under MIT (see `LICENSE`).