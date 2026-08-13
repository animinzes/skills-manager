# Skill Control requirements

## Product goal

Skill Control provides one local desktop application for inventorying,
classifying, deploying, checking, and maintaining skills used by multiple
coding agents. Local files remain the source material. The application records
the additional relationships that a filesystem cannot express reliably.

## Managed objects

### Skill

A skill is a directory with `SKILL.md` or the compatible `skill.md` entry file.
Each skill has one primary kind:

- `capability`: an independent reusable capability
- `tool_guide`: instructions for a CLI, application, or fixed external tool
- `integration`: instructions and adapters for an API, MCP server, or service
- `workflow`: a repeatable multi-step working procedure
- `governance`: a skill that manages agents, skills, memory, or configuration

Suite membership is stored separately because one skill can belong to multiple
suites without changing its primary kind.

### Agent

Each agent has an independent adapter. The adapter owns its discovery paths,
deployment paths, link or copy behavior, and project-level conventions. Claude
Code and Codex state must remain distinguishable in every inventory and change
plan.

### Preset

A preset is a user-selected group used for batch enablement and deployment. It
does not define versions, dependencies, compatibility, or completeness.

### Suite

A suite is a versioned and validated skill package. It defines:

- required and optional members
- member version or revision constraints
- CLI, MCP, runtime, service, and configuration dependencies
- supported agents and compatibility rules
- installer ownership and upgrade policy
- suite-level health and recovery instructions

Examples include Claude Scholar, oh-my-opencode, OMX, OpenCLI support skills,
and an AMiner integration family.

## Governance metadata

Each skill can record:

- primary kind
- provenance: official, suite, third-party, local, or unknown
- owner and maintainer
- lifecycle: experimental, active, deprecated, or archived
- review risk: unreviewed, low, medium, or high
- supported agents and support level
- CLI, MCP, runtime, service, skill, and configuration dependencies
- filesystem, network, command, account, and secret access requirements
- maintenance notes and review time

Governance metadata is stored in SQLite and references the existing stable
`skill_id`. Skill content and upstream-compatible skill records remain intact.

## Health model

Health checks combine four sources:

1. static skill validation
2. declared dependency probes
3. deployed target existence and content drift
4. suite membership, version, and compatibility checks

Each result records status, evidence, check time, and a repair suggestion.

## Change model

Batch operations create a reviewable plan before writing files. A plan records:

- requested action
- affected skills, suites, agents, and paths
- expected creates, updates, links, copies, and removals
- permission and account impact
- preconditions
- recovery information
- execution result for each action

Account writes, destructive removals, and broad deployment changes require an
explicit confirmation step.

## Interfaces

The GUI, CLI, local API, and future MCP adapter call the same Rust domain
services. Filesystem and database mutations stay in the Rust backend.

## Non-functional requirements

- Windows paths, directory junctions, and symlinks receive automated coverage.
- Database migrations are transactional and idempotent.
- Deleting a skill removes its related governance records through foreign keys.
- Upstream Skills Manager storage and deployment behavior remains compatible.
- Personal builds use the `Skill Control` product identity and D-drive caches.
- Every batch mutation is auditable and recoverable when the operation permits.

## First milestone acceptance

The governance foundation is complete when:

1. an existing database upgrades automatically to schema version 8;
2. every skill can return a default governance profile;
3. a profile can save kind, provenance, ownership, lifecycle, risk, agent
   support, dependencies, permissions, and notes;
4. invalid enum values and empty dependency names are rejected;
5. replacing a profile is atomic;
6. deleting a skill deletes its governance, support, and dependency records;
7. the frontend can read and save the profile from a skill detail view;
8. Rust tests, frontend lint, and frontend build pass.
