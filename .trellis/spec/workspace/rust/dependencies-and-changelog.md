# 依赖、feature、MSRV 与更新日志

> 每一条新依赖都是编译时间、供应链风险与未来破坏性变更的来源;每一个使用者可感知的改动都要在 CHANGELOG 留痕。
> 来源:根 `Cargo.toml` `[workspace.dependencies]` / `[workspace.lints]`、`deny.toml`、`typos.toml`、`CHANGELOG.md` 顶部约定、tokio CONTRIBUTING(MSRV / 版本策略)、rust-analyzer 《Style · Crates.io Dependencies》、Rust API Guidelines C-FEATURE / C-METADATA / C-RELNOTES / C-STABLE。

---

## 1. 新增依赖的流程

1. **先问要不要**:标准库能做的不引依赖;几十行能写出来的「小工具 crate」不引(rust-analyzer 的原则:只允许 `itertools` / `either` 这种级别的例外)。`cargo tree -d` 看看是否已经在依赖树里。
2. **版本只写在根 `Cargo.toml` `[workspace.dependencies]`**,上方一行中文注释说明用途(现有条目全部如此),成员 crate 只写 `<name>.workspace = true`。
3. **只开需要的 feature**:`default-features = false` + 显式 `features = [...]`,尤其是 `tokio` / `reqwest` 这类重量级依赖;工作区表里 `serde = { features = ["derive"] }`、`tracing-subscriber = { features = ["env-filter"] }` 就是显式声明的例子。
4. **许可证在 `deny.toml` `[licenses].allow` 白名单内**(MIT / Apache-2.0 / BSD / ISC / Unicode-3.0 / Zlib);新许可证评估后显式加入,不要为了省事把 `allow` 列表放宽到未评估过的许可证。
5. **来源只允许 crates.io**(`deny.toml` `[sources]`);git 依赖需登记 `allow-git` 并写明为什么不能用发布版。
6. `cargo build` 一次让 `Cargo.lock` 更新,**锁文件与代码同一提交**(CI / 发版都 `--locked`)。
7. **CHANGELOG**:库 crate 新增公开依赖(类型出现在公开 API 里)记 `Changed`;纯内部依赖不记。

### 1.1 库 crate 的额外约束

- **公开 API 里出现的第三方类型(`serde::Serialize`、`chrono::DateTime`)让该依赖成为「公开依赖」**:它的破坏性升级就是你的破坏性升级(C-STABLE)。能用 newtype 包一层就包(`naming-and-api-design.md` §5);不能包的至少放到可选 feature 后面。
- **可选依赖 = feature**:`serde = { workspace = true, optional = true }` 然后 `[features] serde = ["dep:serde"]`;feature 名就叫依赖名或能力名,不加 `use-` / `with-` / `enable-` 前缀(C-FEATURE)。
- **默认 feature 尽量为空或最小**;`std` feature 只在真的支持 `no_std` 时提供,不要装样子。不稳定 / 实验性 API 放 `unstable-<主题>` feature 后(clap `unstable-doc` / `unstable-v5`,predicates `unstable` 的做法),并在文档声明不受 semver 约束。不要用 `serde1` 这种带版本号后缀的老式命名。
- **feature 门控的公开项在 docs.rs 上要显示「需要哪个 feature」**:有了可选 feature 后,在 `lib.rs` 加 `#![cfg_attr(docsrs, feature(doc_cfg))]`,`[package.metadata.docs.rs]` 加 `rustdoc-args = ["--cfg", "docsrs"]`(tracing / tokio / indexmap 的做法);没有可选 feature 时不要加。
- `[package.metadata.docs.rs] all-features = true`(模板已带)让每个可选 API 都出现在 docs.rs。
- 内部 crate 依赖必须 `{ path, version }` 都有(`cargo xtask new` 自动登记),否则发布后下游解析不到;`publish = false` 的二进制可以只写 path(`deny.toml` `allow-wildcard-paths = true`)。

---

## 2. 工作区 lint 基线(现状与含义)

根 `Cargo.toml` `[workspace.lints]`,所有成员 `[lints] workspace = true`;CI `-D warnings`,所以 warn 等同 deny。

| lint | 级别 | 对写代码的含义 |
|---|---|---|
| `rust::unsafe_code` | **forbid** | 全仓库不能写 `unsafe`,`#![allow]` 也无法覆盖 forbid。真需要 FFI / 性能 unsafe 时:单独一个 crate,在其 `Cargo.toml` `[lints.rust] unsafe_code = "deny"` **不继承工作区 lint**(cargo 不支持成员局部覆盖 `workspace = true`),每个 `unsafe` 块上方写 `// SAFETY:` 注释,并开 `clippy::undocumented_unsafe_blocks`——这是 bevy / tokio 的做法 |
| `rust::missing_docs` | warn | 所有 `pub` 项要 `///` |
| `rust::unused_qualifications` | warn | 已 `use` 的东西不要再写全路径 |
| `clippy::pedantic` | warn(priority -1) | 基线;单条噪音大的在下方显式 allow 并注释,不在代码里散放 `#[allow]` |
| `module_name_repetitions` | allow | `error::Error` 这种允许 |
| `missing_panics_doc` | allow | 库不应 panic,靠 `unwrap_used` 约束 |
| `unwrap_used` / `expect_used` | warn | 库与二进制都禁;测试模块放开 |
| `print_stdout` / `print_stderr` / `dbg_macro` / `todo` | warn | 只有 `main.rs` / xtask 顶部放行 print;`dbg!` / `todo!()` 不能进仓库 |

**业内常见但本仓库尚未开启、值得在新公开 crate 出现时评估的 lint**(tokio / tracing / bevy 都开):`missing_debug_implementations`(所有公开类型要 `Debug`)、`unreachable_pub`(二进制里多余的 `pub`)、`clippy::allow_attributes_without_reason`(强制 `#[allow(..., reason = "...")]`)。在 spec 里它们目前是自检清单项;要变成编译期约束就改根 `Cargo.toml` 并同步 `README.md` §4 的 lint 表与本文件 §2(属内部改动,不记 CHANGELOG)。

**rustfmt**(`rustfmt.toml`):`max_width = 100`、4 空格、LF、`reorder_imports` / `reorder_modules`。只用 stable 选项;`group_imports` / `imports_granularity` 仍是 nightly,所以 import 分组靠 `module-layout.md` §2 的人工约定。

---

## 3. MSRV

- 声明在根 `Cargo.toml` `rust-version = "1.85"`(edition 2024 的最低要求),成员继承。
- **只用该版本已稳定的语法与 API**:例如 let-chains(`if let Some(x) = a && cond`)需要 1.88,当前不能用;`Option::is_none_or` 1.82 可用。不确定就查 `std` 文档里的 "since" 标注。
- CI `msrv` job 用声明的版本实际 `cargo check --workspace --all-targets --locked`,本地可 `rustup toolchain install 1.85 && cargo +1.85 check --workspace --all-targets` 复现。
- **提升 MSRV = 次版本变更**,CHANGELOG `Changed` 记「MSRV 提升到 1.xx(原因)」;跟随依赖被迫提升也要记。参考 tokio 策略:MSRV 至少落后 stable 6 个月,只在 minor 版本提升。
- 依赖升级导致 MSRV 被动提升时优先锁旧版本,除非新版本有安全修复。

---

## 4. 更新日志(`CHANGELOG.md`)

格式:Keep a Changelog 1.1.0;`cargo xtask release` 负责把 `[Unreleased]` 切成版本段落,`[Unreleased]` 为空时**拒绝发版**(`xtask/src/changelog.rs` `assert_unreleased_has_entries`)。

### 4.1 要记什么

| 变化 | 小节 | 写法 |
|---|---|---|
| 新公开 API / 新子命令 / 新 feature | `Added` | 「新增 `xxx`,用于…」 |
| 行为变化、默认值变化、MSRV 提升、公开依赖大版本升级 | `Changed` | 破坏性以 **BREAKING** 开头,说明迁移方法 |
| 标记 `#[deprecated]` | `Deprecated` | 给出替代 API |
| 删除公开项 | `Removed` | **BREAKING** |
| 修 bug | `Fixed` | 描述用户可见的症状,不写内部原因 |
| 安全修复 / 依赖漏洞 | `Security` | 引用 RUSTSEC 编号 |

**不记**:内部重构、CI、测试、纯 xtask 改动、注释修正。

### 4.2 怎么写

- **面向使用者说影响**,不复述 diff:
  - GOOD:`- 解析 Cargo.toml 时对缺少 version 字段给出明确错误,而不是 panic`
  - BAD:`- 修改 parse_version 函数增加 None 检查`
- 一条一行,以 `- ` 开头;涉及的标识符用反引号;有 issue / PR 就在末尾 `(#123)`。
- 同一次提交里写,不要攒到发版前补(那时已经忘了影响是什么)。
- 破坏性变更:`- **BREAKING** \`Config::new\` 改为返回 \`Result\`;调用方需处理 \`Error::InvalidInput\``。

### 4.3 版本与发版

- 版本号只在根 `[workspace.package].version`;全部成员 lockstep 同版本一起发。不手改成员 `Cargo.toml` 或 `Cargo.lock` 里的版本(`cargo xtask version check` 会报不一致)。
- 语义化版本判定(0.x 阶段 minor 即破坏性):
  - `patch`:只有 `Fixed` / 文档 / 内部
  - `minor`:有 `Added` / `Deprecated` / MSRV 提升 / 非破坏 `Changed`
  - `major`(1.0 后):任何 **BREAKING**
- 发到 crates.io 不可撤销:`cargo publish --workspace --dry-run --locked` 是 CI 常规步骤,发版前本地也跑一遍。

---

## 5. 拼写(`typos`)

- 注释、文档、标识符都过 `typos`;误报确认是专有名词后加 `typos.toml` `[default.extend-words]`(如已有 `ser` / `de`)或 `[default.extend-identifiers]`,**不要**整文件 `extend-exclude`。
- 中文不会被检查;英文注释与标识符用美式拼写。

---

## 6. 自检清单

- [ ] 新依赖:根 `[workspace.dependencies]` + 中文用途注释 + 成员 `.workspace = true` + 最小 feature + 许可证在白名单
- [ ] `Cargo.lock` 与代码同一提交;`cargo deny check` 本地能过(装了 cargo-deny 时)
- [ ] 没用到 > 1.85 才稳定的语法 / API
- [ ] 使用者可感知的改动在 `CHANGELOG.md` `[Unreleased]` 对应小节有一条,破坏性以 **BREAKING** 开头
- [ ] 库 crate:公开 API 没有暴露新的第三方类型,或已放在 feature 后面
