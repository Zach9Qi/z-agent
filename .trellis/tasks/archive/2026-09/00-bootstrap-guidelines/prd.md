# Bootstrap Task: Fill Project Development Guidelines

**You (the AI) are running this task. The developer does not read this file.**

The developer just ran `trellis init` on this project for the first time.
`.trellis/` now exists with empty spec scaffolding, and this bootstrap task
exists under `.trellis/tasks/`. When they want to work on it, they should start
this task from a session that provides Trellis session identity.

**Your job**: help them populate `.trellis/spec/` with the team's real
coding conventions. Every future AI session — this project's
`trellis-implement` and `trellis-check` sub-agents — auto-loads spec files
listed in per-task jsonl manifests. Empty spec = sub-agents write generic
code. Real spec = sub-agents match the team's actual patterns.

Don't dump instructions. Open with a short greeting, figure out if the repo
has any existing convention docs (CLAUDE.md, .cursorrules, etc.), and drive
the rest conversationally.

---

## Status (update the checkboxes as you complete each item)

- [x] Import conventions from `AGENTS.md` / `README.md` / root `Cargo.toml` / `rustfmt.toml` / `deny.toml` / `typos.toml` / `.editorconfig`
- [x] Research industry Rust style (rust-analyzer Style, Rust API Guidelines, tokio / tracing / axum / bevy docs; BurntSushi / dtolnay / epage crates read from local cargo registry) — sources and verdicts recorded in `.trellis/spec/workspace/rust/references.md`
- [x] Fill guidelines for workspace-wide Rust code (`spec/workspace/rust/`, 8 files)
- [x] Fill guidelines for process: commits / versioning / release (`spec/workspace/process/`)
- [x] Fill guidelines for `crates/` library and binary layers (`spec/crates/library/`, `spec/crates/binary/`)
- [x] Fill guidelines for `xtask/` (`spec/xtask/automation/`; template `spec/xtask/backend/` removed — database / logging templates did not fit a Rust automation crate)
- [x] Add code examples — every rule cites `xtask/src/*.rs` or the `cargo xtask new` templates in `xtask/src/new_crate.rs` with `path:line`
- [x] Two-round fact-check of all file:line references and Rust/tooling claims (reviewer sub-agent)
- [x] Reduce `AGENTS.md` to a pure entry point (repo one-liner + routing table to spec `index.md` files + Trellis managed block); all rules — including the hard-constraint list — live only in `.trellis/spec/workspace/rust/index.md`「硬约束速览」
- [x] `config.yaml` packages: `workspace(.)` / `crates` / `xtask`, default `crates`; `get_context.py --mode packages` lists all layers

---

## Spec files populated

```
.trellis/spec/
├── workspace/rust/        index, module-layout, naming-and-api-design, error-handling,
│                          documentation, testing, dependencies-and-changelog, references
├── workspace/process/     index, commit-and-release
├── crates/library/        index, crate-anatomy
├── crates/binary/         index, cli-structure
├── xtask/automation/      index, conventions
└── guides/                (pre-filled thinking guides; index.md gained Rust-specific triggers + routing)
```

Follow-ups noted for the developer (not part of bootstrap):
- `.cargo/config.toml:2` comment claims xtask is excluded via `default-members` — root `Cargo.toml` sets none.
- `README.md` §4 says `unsafe_code` forbid can be relaxed per crate — `forbid` cannot be overridden by `allow`.
- Candidate lints to add to `[workspace.lints]`: `missing_debug_implementations`, `unreachable_pub`, `clippy::allow_attributes_without_reason`.
- Once the first real crate exists, replace template-string references (`new_crate.rs`) in spec with real code paths.

---

## How to fill the spec

### Step 1: Import from existing convention files first (preferred)

Search the repo for existing convention docs. If any exist, read them and
extract the relevant rules into the matching `.trellis/spec/` files —
usually much faster than documenting from scratch.

| File / Directory | Tool |
|------|------|
| `CLAUDE.md` / `CLAUDE.local.md` | Claude Code |
| `AGENTS.md` | Codex / Claude Code / agent-compatible tools |
| `.cursorrules` | Cursor |
| `.cursor/rules/*.mdc` | Cursor (rules directory) |
| `.windsurfrules` | Windsurf |
| `.clinerules` | Cline |
| `.roomodes` | Roo Code |
| `.github/copilot-instructions.md` | GitHub Copilot |
| `.vscode/settings.json` → `github.copilot.chat.codeGeneration.instructions` | VS Code Copilot |
| `CONVENTIONS.md` / `.aider.conf.yml` | aider |
| `CONTRIBUTING.md` | General project conventions |
| `.editorconfig` | Editor formatting rules |

### Step 2: Analyze the codebase for anything not covered by existing docs

Scan real code to discover patterns. Before writing each spec file:
- Find 2-3 real examples of each pattern in the codebase.
- Reference real file paths (not hypothetical ones).
- Document anti-patterns the team clearly avoids.

### Step 3: Document reality, not ideals

**Critical**: write what the code *actually does*, not what it should do.
Sub-agents match the spec, so aspirational patterns that don't exist in the
codebase will cause sub-agents to write code that looks out of place.

If the team has known tech debt, document the current state — improvement
is a separate conversation, not a bootstrap concern.

---

## Quick explainer of the runtime (share when they ask "why do we need spec at all")

- Every AI coding task spawns two sub-agents: `trellis-implement` (writes
  code) and `trellis-check` (verifies quality).
- Each task has `implement.jsonl` / `check.jsonl` manifests listing which
  spec files to load.
- The platform hook auto-injects those spec files + the task's `prd.md`
  into every sub-agent prompt, so the sub-agent codes/reviews per team
  conventions without anyone pasting them manually.
- Source of truth: `.trellis/spec/`. That's why filling it well now pays
  off forever.

---

## Completion

When the developer confirms the checklist items above are done with real
examples (not placeholders), guide them to run:

```bash
python ./.trellis/scripts/task.py finish
python ./.trellis/scripts/task.py archive 00-bootstrap-guidelines
```

After archive, every new developer who joins this project will get a
`00-join-<slug>` onboarding task instead of this bootstrap task.

---

## Suggested opening line

"Welcome to Trellis! Your init just set me up to help you fill the project
spec — a one-time setup so every future AI session follows the team's
conventions instead of writing generic code. Before we start, do you have
any existing convention docs (CLAUDE.md, .cursorrules, CONTRIBUTING.md,
etc.) I can pull from, or should I scan the codebase from scratch?"
