# 更新日志

本文件记录项目所有值得注意的变更。格式遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/),版本号遵循[语义化版本](https://semver.org/lang/zh-CN/)。

约定:

- 日常开发把变更写在 `[Unreleased]` 下的 `Added` / `Changed` / `Deprecated` / `Removed` / `Fixed` / `Security` 小节,面向使用者描述「对我有什么影响」,而不是复述提交信息;
- `cargo xtask release` 发版时会把 `[Unreleased]` 下的条目切到 `## [x.y.z] - 日期` 段落并维护底部链接;`[Unreleased]` 为空时拒绝发版;
- 该段落同时作为 GitHub Release 的说明(`cargo xtask changelog notes vX.Y.Z`);
- MSRV(`rust-version`)提升记入 `Changed`,破坏性变更条目以 **BREAKING** 开头。

## [Unreleased]

### Added

- 基于 rust-monorepo-starter 模板初始化项目

[Unreleased]: https://github.com/Zach9Qi/rust-monorepo-starter/commits/main
