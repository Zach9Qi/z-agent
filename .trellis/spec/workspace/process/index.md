# 协作流程规范(提交、版本、发版)

> 与语言无关、但每个提交都会碰到的流程约定。Rust 编码规范在 `../rust/index.md`。

---

## 适用范围

- 任何要 `git commit` 的改动:提交信息格式、CHANGELOG / `Cargo.lock` 与代码同提交
- 版本号维护与 `cargo xtask release` 发版全流程,以及推 tag 之后 `release.yml` 的行为与失败处理
- 分支与 PR 约定

---

## Pre-Development Checklist

- [ ] 这次改动使用者能感知吗?能 → 想好 `CHANGELOG.md` `[Unreleased]` 放哪个小节、是否 **BREAKING**
- [ ] 会碰 `Cargo.toml` 依赖或新增 crate 吗?会 → 记得 `cargo build` 后把 `Cargo.lock` 一起提交
- [ ] 提交 type / scope 想好了(`feat(core)` / `fix(cli)` / `chore(xtask)` / `docs` / `ci`)
- [ ] 要发版?先读 `commit-and-release.md` §3.1 清单,先 `--dry-run`

---

## Guidelines Index

| 文档 | 内容 | 何时读 |
|---|---|---|
| [commit-and-release.md](./commit-and-release.md) | Conventional Commits 中文规则与 scope 取法、版本单一来源、发版 SOP(前 / 中 / 后)、`release.yml` 阶段与失败处理、分支 / PR | 提交前;发版前;发版失败时 |
| [../rust/dependencies-and-changelog.md](../rust/dependencies-and-changelog.md) | CHANGELOG 条目写法、版本级别判定、MSRV | 写 CHANGELOG 条目时 |
| [../../xtask/automation/conventions.md](../../xtask/automation/conventions.md) | `cargo xtask release` / `version check` / `changelog notes` 的实现约定 | 要改发版脚本本身时 |

---

## Quality Check

```bash
cargo xtask ci                                   # 提交前门禁
git status --short                               # Cargo.lock / CHANGELOG.md 是否和代码一起在暂存区
git log --oneline -5                             # 提交信息符合 <type>(<scope>): 中文描述
cargo xtask version check                        # 版本号各处一致
cargo xtask release <级别> --dry-run              # 发版前看计划
```
