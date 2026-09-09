# 规范来源与调研记录

> 本目录的规则不是凭空定的。这里记录每条来源是什么、核实到什么程度、以及我们明确**没有**采纳的部分,方便日后有人质疑「为什么要这样」时能追溯。

---

## 1. 一手风格文档(已抓取原文核对)

| 来源 | 位置 | 采纳到 |
|---|---|---|
| rust-analyzer《Style》 | `rust-lang/rust-analyzer` → `docs/book/src/contributing/style.md` | `module-layout.md`(导入顺序、条目顺序、`#[allow]`)、`naming-and-api-design.md`(前置条件、`Config` 结构体、拆函数、`Default`、控制流表)、`testing.md`(最小夹具、禁 `should_panic` / `ignore`) |
| Rust API Guidelines | `rust-lang/api-guidelines` → `src/checklist.md` 及各章节 | `naming-and-api-design.md`(`C-CASE` / `C-CONV` / `C-GETTER` / `C-CTOR` / `C-COMMON-TRAITS` / `C-DEBUG` / `C-STRUCT-PRIVATE` / `C-NEWTYPE` / `C-SEALED` / `C-CUSTOM-TYPE`)、`error-handling.md`(`C-GOOD-ERR` / `C-VALIDATE` / `C-FAILURE`)、`documentation.md`(`C-CRATE-DOC` / `C-EXAMPLE` / `C-QUESTION-MARK` / `C-LINK`)、`dependencies-and-changelog.md`(`C-FEATURE` / `C-STABLE` / `C-RELNOTES`) |
| tokio `CONTRIBUTING.md` + `tokio/src/lib.rs` | `tokio-rs/tokio` | MSRV / 版本策略(`dependencies-and-changelog.md` §3);lint 基线对照(`missing_debug_implementations` / `unreachable_pub`) |
| tracing `tracing/src/lib.rs` | `tokio-rs/tracing` | 同上 lint 基线对照 |
| axum `CONTRIBUTING.md` | `tokio-rs/axum` | 每个 API 至少一个 doctest(`documentation.md` §3、`testing.md` §1) |
| bevy 根 `Cargo.toml` `[workspace.lints]` | `bevyengine/bevy` | `undocumented_unsafe_blocks` / `allow_attributes_without_reason` 的做法(`dependencies-and-changelog.md` §2「尚未开启」) |
| rustc-dev-guide《Coding conventions》 | `rust-lang/rustc-dev-guide` → `src/conventions.md` | 穷尽匹配、TODO 注释要带主体、跟随周围代码风格 |

---

## 2. 真实项目源码(本机 cargo registry 实读)

读的是已发布到 crates.io 的源码(`~/.cargo/registry/src/`),版本以调研时缓存为准:
ignore 0.4 / globset 0.4 / walkdir 2.5 / regex 1.13 / jiff 0.2(BurntSushi);serde 1.0 / anyhow 1.0 / thiserror 2.0 / semver 1.0(dtolnay);clap 4.6 / clap_builder / toml 1.1 / predicates 3.1 / assert_cmd 2.2 / typos-cli 1.50(epage);tracing 0.1 / tracing-subscriber 0.3 / tokio 1.53;indexmap 2 / camino 1 / cargo_metadata 0.19。

从中得到并写进规范的**实证结论**:

| 结论 | 证据 | 写在 |
|---|---|---|
| 公开类型 100% 实现 `Debug` 是事实标准 | 23 个采样类型中 18 个 derive,其余 5 个手写 `impl Debug`;regex / jiff / tracing / tokio 开 `warn(missing_debug_implementations)` | `naming-and-api-design.md` §4 |
| derive 顺序无业内共识,只有「语义配对相邻」 | BurntSushi 字母序 vs clap / toml / cargo_metadata `Debug` 领头 | `naming-and-api-design.md` §4(本仓库选 `Debug` 领头,与 xtask 一致) |
| `#[non_exhaustive]` 几乎只加在公开 enum(`Error` / `ErrorKind`)上,与 derive 分行 | regex `Error`、globset / clap `ErrorKind`、semver `Op` | `error-handling.md` §2、`naming-and-api-design.md` §5 |
| 错误类型在业内有多种形态(公开 enum / struct+公开 kind / struct+私有 inner / 不透明 Box / thiserror enum);本仓库只采用两种:thiserror enum,上下文多时演进为 `struct Error { ctx, kind: ErrorKind }` | globset / walkdir / clap_builder / jiff / cargo_metadata(唯一 thiserror 用户) | `error-handling.md` §2.1「形态升级路线」 |
| 新库不再实现 `Error::description()` | toml / jiff / camino 不实现;ignore / regex 保留并 `#[allow(deprecated)]` | `error-handling.md` §2.1 |
| `mod.rs` 仍是多数派,`foo.rs + foo/` 是少数派(indexmap / std) | 目录布局实查 | `module-layout.md` §3(本仓库选 `foo.rs`,注明是 house 选择) |
| 小 crate 全部平铺单文件 | globset / walkdir / semver / anyhow | `module-layout.md` §3 |
| import 三组空行分隔是跨生态一致的 | ignore `gitignore.rs`、globset `glob.rs`、walkdir `dent.rs`、jiff `date.rs` | `module-layout.md` §2 |
| 测试名 snake_case 短句无 `test_` 前缀(BurntSushi)vs `test_` 前缀(dtolnay) | globset `set_works`、jiff `date_invalid_day` vs semver `test_parse` | `testing.md` §2(本仓库与 xtask 一致:无前缀) |
| `#[track_caller]` 用在测试辅助函数与会 panic 的内部辅助上 | semver `tests/util/mod.rs`、clap `arg_matches.rs`、indexmap | `testing.md` §2 |
| clap 二进制标配 `Cli::command().debug_assert()` 单测 | clap `examples/tutorial_derive/05_01_assert.rs`、typos-cli `args.rs` | `../../crates/binary/cli-structure.md` §8 |
| lint 放 `Cargo.toml` `[lints]` 而非 `lib.rs` 属性块是新趋势 | clap_builder / predicates / assert_cmd 用 `[lints.clippy]` / `[lints.rust]`,typos-cli 用 `[lints] workspace = true` | `../../crates/library/crate-anatomy.md` §1.3(那里只点名 typos-cli) |
| doctest 收尾 `# Ok::<(), Box<dyn std::error::Error>>(())` 或隐藏 `# fn main()` | BurntSushi 系 vs tokio-rs 系 | `documentation.md` §3 |
| feature 用 `dep:` 语法、以依赖名 / 能力名命名;`unstable-*` 前缀标非稳定 API | semver / globset / clap / predicates | `dependencies-and-changelog.md` §1.1 |
| `#![cfg_attr(docsrs, feature(doc_cfg))]` + `rustdoc-args = ["--cfg", "docsrs"]` 是有 feature 的库的标配 | 15 个 crate 中 15 个 | `dependencies-and-changelog.md` §1.1 |

---

## 3. 明确不采纳的业内做法

| 做法 | 出处 | 不采纳的原因 |
|---|---|---|
| 提交信息不用 Conventional Commits,用用户视角祈使句 | rust-analyzer | 本仓库规定 Conventional Commits + 中文描述(`../process/commit-and-release.md` §1) |
| 英文注释与文档 | 所有开源项目 | 本仓库规定中文(`index.md`「硬约束速览」第 4 条);标识符、命令保持原文 |
| 手写 `impl Display` 而不用 thiserror | BurntSushi / dtolnay / clap 全部手写 | thiserror 已是本仓库既定约定(模板 `error.rs`、`error-handling.md`),对 crates.io 库同样合适;手写只在需要按 feature 收缩表示时才值得 |
| `FxHashMap` / `AbsPath` / `stdx::never!` | rust-analyzer 内部类型 | 项目特有;本仓库用 std 集合、`std::path`、返回 `Error` |
| `#![allow(clippy::missing_errors_doc)]` | serde / anyhow / semver | 本仓库 pedantic 基线要求写 `# Errors`,按 jiff 风格执行 |
| `quickcheck` 作为默认属性测试库 | BurntSushi 系 | 未预置;需要时在根 `[workspace.dependencies]` 选 `proptest` 或 `quickcheck` 之一 |
| `human_panic` + `proc_exit::sysexits` 退出码体系 | typos-cli | 本仓库 `panic = "abort"` 且默认只区分 0 / 1;需要更多退出码时用枚举 + `From<..> for ExitCode` |
| `# Example` 单数 | jiff | 与 clippy / rustdoc 惯例(`# Examples` / `# Errors` / `# Panics`)保持复数 |

---

## 4. 待验证 / 未来可能升级为编译期约束

- `missing_debug_implementations`、`unreachable_pub`、`clippy::allow_attributes_without_reason`:目前是自检清单项;评估后可加进根 `Cargo.toml` `[workspace.lints]`(改动属内部,不记 CHANGELOG,但要同步 `README.md` §4「工作区级 lint 基线」与 `dependencies-and-changelog.md` §2)。
- 第一个业务 crate 落地后,回头用真实代码替换本目录里指向 `xtask/src/new_crate.rs` 模板字符串的范例引用。
