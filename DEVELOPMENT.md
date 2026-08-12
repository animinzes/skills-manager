# Skill Control development guide

This fork extends Skills Manager with skill governance, suite management,
health checks, and reviewable change plans while preserving compatibility with
the upstream storage and deployment model.

## Baseline

- Upstream: `xingkongliang/skills-manager`
- Starting release: `v1.33.1`
- Starting commit: `f4488b7f3ddfb3b76afc521a2fa6634031c784d1`
- Primary branch: `main`
- Integration branch: `feature/skill-control-foundation`

Keep `upstream` read-only. Use `origin` for the fork. Before starting a feature:

```powershell
git fetch upstream --prune --tags
git switch main
git merge --ff-only upstream/main
git push origin main
git switch -c feature/<scope>
```

## Local prerequisites

- Node.js 22 or later
- npm 11 or later
- Rust 1.77.2 or later through rustup
- Windows: MSVC C++ Build Tools and Microsoft Edge WebView2 Runtime

Install JavaScript dependencies with `npm ci`. Do not run an automatic
dependency fix without reviewing the resulting manifest and lockfile changes.

## Required checks

Run the checks relevant to the changed paths before opening a pull request:

```powershell
npm ci
npm run lint
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

The GitHub workflows provide the Rust matrix and frontend checks when a local
platform cannot run every target.

## Work packages

Use separate pull requests for these packages:

1. `feature/governance-metadata`: schema, ownership, provenance, permissions,
   supported agents, and migration rules.
2. `feature/suites`: suite manifest, required and optional members, versions,
   dependencies, and compatibility validation.
3. `feature/health-checks`: static validation, dependency probes, deployment
   drift detection, and suite-level status aggregation.
4. `feature/change-plans`: preview, approval boundary, execution journal, and
   recovery metadata for batch operations.
5. `feature/local-api`: a local API and MCP-facing adapter over the same domain
   services used by the GUI and CLI.

Every package must include migration behavior, failure behavior, and tests. A
pull request must avoid mixing dependency upgrades with feature implementation.

## Pull request policy

- Target the fork's `main` branch unless the change is intended for upstream.
- Keep each pull request independently buildable and reversible.
- Record schema and storage changes explicitly.
- Include Windows path, symlink, and junction cases when filesystem behavior
  changes.
- Use Conventional Commit prefixes such as `feat:`, `fix:`, `test:`, `docs:`,
  and `chore:`.

