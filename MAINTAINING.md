# Maintaining Meety

How the maintainer's day-to-day workflow runs. Strangers shipping a
one-off fix should start with [`CONTRIBUTING.md`](./CONTRIBUTING.md)
instead — this doc describes maintainer conventions and is not required
for outside contributions.

## How the work is organized

Design docs drive the architecture, this repo drives the code, and an
issue tracker drives the work. Every shipped PR closes one tracked
issue and, where relevant, cites the design rationale behind it.

## Daily flow

1. **Pick an issue.** Move it to `In Progress`.
2. **Branch.** `git checkout -b <type>/<slug>` per
   `CONTRIBUTING.md` branching conventions.
3. **Write the change.** Follow `docs/CODE_STYLE.md`.
4. **Test locally.** `cargo fmt && cargo clippy --workspace
   --all-targets -- -D warnings && cargo test --workspace
   --all-targets && bun run typecheck && bun run lint && bun run test`.
5. **Commit.** Conventional commit (`feat: <subject>`).
6. **Push and open a PR.** `gh pr create --title "<type>: <title>"`
7. **Merge.** `gh pr merge --merge --delete-branch`. Branch protection
   on `main` requires CI green; use `--admin` for emergency fixes only.
8. **`git pull --ff-only`** back on main.

## Release cadence

See `docs/guidelines/release-engineering.md`. The short version:

- Tag a release with `git tag 2026-MM-DD.R<N> && git push --tags`.
- The release workflow builds signed binaries for macOS, Windows, and Linux.
- See `docs/guidelines/release-engineering.md` for the full procedure.

## Where to go when stuck

For unanswered questions, file a GitHub issue and let the maintainer
answer publicly so the answer is searchable.