# Thinking Guides

> **Purpose**: Expand your thinking to catch things you might not have considered.

---

## Why Thinking Guides?

**Most bugs and tech debt come from "didn't think of that"**, not from lack of skill:

- Didn't think about what happens at layer boundaries → cross-layer bugs
- Didn't think about code patterns repeating → duplicated code everywhere
- Didn't think about edge cases → runtime errors
- Didn't think about future maintainers → unreadable code

These guides help you **ask the right questions before coding**.

---

## Available Guides

| Guide | Purpose | When to Use |
|-------|---------|-------------|
| [Code Reuse Thinking Guide](./code-reuse-thinking-guide.md) | Identify patterns and reduce duplication | When you notice repeated patterns |
| [Cross-Layer Thinking Guide](./cross-layer-thinking-guide.md) | Think through data flow across layers | Features spanning multiple layers |

**Rust coding specs (this repo)** live outside `guides/` — start from the layer you are touching:

| Spec | Scope |
|------|-------|
| [`../workspace/rust/index.md`](../workspace/rust/index.md) | Workspace-wide Rust rules: file layout, naming/API design, error handling, docs, tests, dependencies & CHANGELOG. Applies to every `.rs` file |
| [`../crates/library/index.md`](../crates/library/index.md) | Library crates under `crates/` (published to crates.io) |
| [`../crates/binary/index.md`](../crates/binary/index.md) | Binary (clap CLI) crates under `crates/` |
| [`../xtask/automation/index.md`](../xtask/automation/index.md) | `xtask/` repo automation |

---

## Quick Reference: Thinking Triggers

### When to Think About Cross-Layer Issues

- [ ] Feature touches 3+ layers (API, Service, Component, Database)
- [ ] Data format changes between layers
- [ ] Multiple consumers need the same data
- [ ] You're not sure where to put some logic
- [ ] You are adding an event kind, JSONL record, RPC payload, or config field
- [ ] UI / command code starts casting raw payload fields directly

→ Read [Cross-Layer Thinking Guide](./cross-layer-thinking-guide.md)

### When to Think About Code Reuse

- [ ] You're writing similar code to something that exists
- [ ] You see the same pattern repeated 3+ times
- [ ] You're adding a new field to multiple places
- [ ] **You're modifying any constant or config**
- [ ] **You're creating a new utility/helper function** ← Search first!
- [ ] Two files read the same untyped payload field with local casts
- [ ] Multiple branches update the same derived state from `kind` / `action`

→ Read [Code Reuse Thinking Guide](./code-reuse-thinking-guide.md)

### Rust-Specific Triggers (this repo)

- [ ] Adding a fallible `pub fn` → `Result<T>` + `# Errors`; validate external input at the boundary → [`error-handling.md`](../workspace/rust/error-handling.md)
- [ ] Writing `main.rs` logic that is not parsing / logging / dispatch / printing → it belongs in a library crate → [`crates/binary`](../crates/binary/index.md)
- [ ] Reaching for `unwrap` / `expect` / `dbg!` anywhere outside tests, or `println!` / `eprintln!` outside a binary's `main.rs` / xtask → clippy will fail CI; use `?` / `tracing`
- [ ] A `bool` or `Option` parameter that is always a literal at call sites → split the function or use an enum → [`naming-and-api-design.md`](../workspace/rust/naming-and-api-design.md) §2.3
- [ ] Adding a dependency → root `[workspace.dependencies]` + Chinese purpose comment + license in `deny.toml` → [`dependencies-and-changelog.md`](../workspace/rust/dependencies-and-changelog.md)
- [ ] Changing anything a user can observe → `CHANGELOG.md` `[Unreleased]` entry in the same commit
- [ ] Editing `xtask/src/ci.rs` steps → mirror in `.github/workflows/ci.yml` (and vice versa)

### When Verifying AI Cross-Review Results

- [ ] Reviewer claims "user input can be malicious" → Check the actual data source (internal manifest? user config? external API?)
- [ ] Reviewer flags "missing validation" → Is the data from a trusted internal source?
- [ ] Reviewer says "behavior change" → Read the code comments — is it intentional design?
- [ ] Reviewer identifies a "bug" in test → Mentally delete the feature being tested — does the test still pass? If yes → tautological test

**Common AI reviewer false-positive patterns**:
1. **Trust boundary confusion**: Treating internal data (bundled JSON manifests) as untrusted external input
2. **Ignoring design comments**: Flagging intentional behavior documented in code comments as bugs
3. **Variable misreading**: Not tracing a variable to its actual definition (e.g., Map keyed by path vs name)

**Verification rule**: Every CRITICAL/WARNING finding must be verified against the actual code before prioritizing. Budget ~35% false-positive rate for AI reviews.

---

## Pre-Modification Rule (CRITICAL)

> **Before changing ANY value, ALWAYS search first!**

```bash
# Search for the value you're about to change
grep -r "value_to_change" .
```

This single habit prevents most "forgot to update X" bugs.

---

## How to Use This Directory

1. **Before coding**: Skim the relevant thinking guide
2. **During coding**: If something feels repetitive or complex, check the guides
3. **After bugs**: Add new insights to the relevant guide (learn from mistakes)

---

## Contributing

Found a new "didn't think of that" moment? Add it to the relevant guide.

---

**Core Principle**: 30 minutes of thinking saves 3 hours of debugging.
