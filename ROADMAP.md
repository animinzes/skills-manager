# Skill Control development plan

## Phase 1: governance metadata

- add schema version 8 and governance tables
- add validated Rust domain types
- add atomic read and replace operations in `SkillStore`
- expose Tauri commands and TypeScript bindings
- add a governance editor to the skill detail view
- add migration, validation, CRUD, and cascade-delete tests

Deliverable: every skill has an editable governance profile.

## Phase 2: suites

- add suite, suite member, and suite dependency tables
- distinguish required and optional members
- record version constraints, installer ownership, and upgrade policy
- add suite completeness and compatibility validation
- add suite list, detail, import, and export views

Deliverable: Claude Scholar, OMX, OpenCLI, and AMiner families can be modeled as
versioned suites.

## Phase 3: health checks

- validate skill entry files and governance completeness
- implement dependency probes for CLI, MCP, runtime, service, and config
- compare central content, deployed targets, and recorded revisions
- aggregate member results into suite health
- store evidence, timestamps, and repair suggestions

Deliverable: the dashboard identifies missing dependencies, drift, and broken
suites with evidence.

## Phase 4: change plans and recovery

- model planned actions and preconditions
- preview batch deployment, upgrade, repair, and removal
- add explicit confirmation boundaries
- journal action-level results
- attach recovery data and reuse existing Git backup safeguards

Deliverable: batch changes can be reviewed before execution and audited after
execution.

## Phase 5: local API and MCP adapter

- extract shared domain services from Tauri command wrappers
- expose read-only inventory and health endpoints first
- add authenticated local mutation endpoints
- implement an MCP adapter over the same services

Deliverable: Codex and Claude Code can query Skill Control without parsing its
database or filesystem directly.

## Phase 6: management interface

- add table and graph inventory views
- add multi-field filtering and bulk selection
- add suite and dependency graph views
- add change-plan review and execution history views
- add import and export for governance and suite manifests

Deliverable: the complete system can be operated from the desktop application.

## Development rules

- `main` mirrors upstream.
- `skill-control` is the personal working branch.
- each phase uses a short-lived feature branch and a reversible commit series.
- completed work is fast-forwarded into `skill-control` after local checks.
- personal development does not create pull requests or GitHub issues.
- schema changes include migration and cascade behavior tests.
- dependency updates use separate commits from product features.
