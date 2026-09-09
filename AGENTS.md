# AI 助手工作指南

本仓库是一个 Rust Cargo 工作区(monorepo)模板:业务 crate 放 `crates/<name>/`(用 `cargo xtask new <name> [--bin]` 生成),`xtask/` 是仓库自动化工具,根 `Cargo.toml` 统一管理版本、依赖版本表与 lint。

**所有编码规范、硬约束、流程约定都在 `.trellis/spec/`,本文件不复述。** 写代码前读对应层的 `index.md`:

| 你要改的 | 先读 |
|---|---|
| 任何 `.rs`(通用 Rust 规范、硬约束速览) | `.trellis/spec/workspace/rust/index.md` |
| `crates/<name>/` 库 crate | `.trellis/spec/crates/library/index.md` |
| `crates/<name>/` 二进制 crate | `.trellis/spec/crates/binary/index.md` |
| `xtask/` | `.trellis/spec/xtask/automation/index.md` |
| 提交信息、版本、发版 | `.trellis/spec/workspace/process/index.md` |

提交前必须通过 `cargo xtask ci`。规范与 spec 冲突时以 spec 为准;发现 spec 有误,先修 spec。

<!-- TRELLIS:START -->
# Trellis Instructions

These instructions are for AI assistants working in this project.

This project is managed by Trellis. The working knowledge you need lives under `.trellis/`:

- `.trellis/workflow.md` — development phases, when to create tasks, skill routing
- `.trellis/spec/` — package- and layer-scoped coding guidelines (read before writing code in a given layer)
- `.trellis/workspace/` — per-developer journals and session traces
- `.trellis/tasks/` — active and archived tasks (PRDs, research, jsonl context)

If a Trellis command is available on your platform (e.g. `/trellis:finish-work`, `/trellis:continue`), prefer it over manual steps. Not every platform exposes every command.

If you're using Codex or another agent-capable tool, additional project-scoped helpers may live in:
- `.agents/skills/` — reusable Trellis skills
- `.codex/agents/` — optional custom subagents

Managed by Trellis. Edits outside this block are preserved; edits inside may be overwritten by a future `trellis update`.

<!-- TRELLIS:END -->
