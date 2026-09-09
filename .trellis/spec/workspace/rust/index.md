# Rust 编码规范(工作区通用)

> 适用于本仓库**所有** Rust 代码:`crates/*` 的库与二进制 crate、`xtask/`。
> 各层专属约定见 `../../crates/library/index.md`、`../../crates/binary/index.md`、`../../xtask/automation/index.md`;它们只写「本层特有」的部分,通用规则以本目录为准。

---

## 规范来源与取舍

本目录的规则来自三处,冲突时按顺序取舍:

1. **本仓库已强制执行的约束**:根 `Cargo.toml` `[workspace.lints]`(`clippy::pedantic` 基线、`unsafe_code = "forbid"`、`missing_docs`、`unwrap_used` / `expect_used`、禁 print)、`rustfmt.toml`、`deny.toml`、`typos.toml`、`cargo xtask ci` / `.github/workflows/ci.yml`。`AGENTS.md` 只是入口路由,不含规则。
2. **`cargo xtask new` 生成的模板与 `xtask/src/*.rs` 的既有写法**——这是仓库里唯一的「活代码」,spec 中的范例都指向它们。
3. **业内共识**:Rust API Guidelines(`C-XXX` 条目号)、rust-analyzer 《Style》、tokio / tracing / bevy 的 lint 基线、BurntSushi 系 crate 与 `clap` 的错误 / 文档结构、axum 的 doctest 与 CHANGELOG 约定。只吸收与前两条不冲突且能落地到本仓库的部分;明确**不采用**的:rust-analyzer 不用 Conventional Commits(本仓库用)、英文注释(本仓库中文)、`FxHashMap` / `AbsPath` 等 rust-analyzer 内部类型。

---

## 硬约束速览(CI 会拦,或后果不可逆)

细则在下方各文件;这里只是让你一眼知道红线在哪。

1. **分层**:库 crate 纯逻辑,不做 IO、不读环境变量、不打印;二进制只做参数解析、日志装配、分派、打印,业务逻辑一律调用库。依赖方向只能「二进制 → 库 → 更底层库」,不允许环 → `../../crates/library/crate-anatomy.md` §2、`../../crates/binary/cli-structure.md` §3
2. **错误**:库用 `thiserror` 在 `error.rs` 扩展唯一的 `Error` 枚举,公开可失败函数返回 `Result<T, Error>`,外部输入在公开边界校验;二进制用 `anyhow` + `.with_context`,`main` 统一打印 `错误: {err:#}`。禁止 `unwrap` / `expect` / `panic!`(测试模块顶部放开)→ [error-handling.md](./error-handling.md)
3. **输出**:只用 `tracing` 宏;`println!` / `eprintln!` 只允许在二进制 `main.rs` 与 xtask;禁 `dbg!` / `todo!()` → [documentation.md](./documentation.md) §5、`../../crates/binary/cli-structure.md` §5
4. **文档**:所有 `pub` 项有中文 `///`;可失败函数写 `# Errors`;注释、文案中文,标识符、命令保持原文 → [documentation.md](./documentation.md)
5. **依赖**:先加根 `[workspace.dependencies]` + 一行中文用途注释,成员 `.workspace = true`;许可证在 `deny.toml` 白名单;`Cargo.lock` 与改动同一提交 → [dependencies-and-changelog.md](./dependencies-and-changelog.md) §1
6. **MSRV**:只用根 `Cargo.toml` `rust-version` 已稳定的语法与 API(let-chain 需要 1.88,声明 1.85 就不能用)→ [dependencies-and-changelog.md](./dependencies-and-changelog.md) §3
7. **版本**:只在根 `[workspace.package].version` 维护,全员 lockstep;不手改成员或 `Cargo.lock` 的版本;发版只用 `cargo xtask release`,crates.io 发布不可撤销 → `../process/commit-and-release.md` §2-3
8. **CHANGELOG**:使用者可感知的改动在同一提交里写进 `CHANGELOG.md` `[Unreleased]`,破坏性以 **BREAKING** 开头;为空时无法发版 → [dependencies-and-changelog.md](./dependencies-and-changelog.md) §4
9. **同步点**:改 `xtask/src/ci.rs` 步骤要同步 `.github/workflows/ci.yml`,反之亦然;改 `[[bin]] name` 要同步 `tests/cli.rs` 与 `release.yml` 的 `PKG_NAME` / `BIN_NAME` → `../../xtask/automation/conventions.md` §7、`../../crates/binary/cli-structure.md` §8
10. **子进程**:一律传数组参数,不拼 shell 字符串 → `../../xtask/automation/conventions.md` §4
11. **拼写**:过 `typos`;专有名词加 `typos.toml` 白名单,不整文件排除 → [dependencies-and-changelog.md](./dependencies-and-changelog.md) §5
12. **`#[allow]`**:必须带原因注释;整仓库噪音的 lint 改根 `[workspace.lints]`,不在代码里散放 → [module-layout.md](./module-layout.md) §5
13. **提交**:Conventional Commits + 中文描述,scope 为 crate 短名,破坏性带 `!`;提交前 `cargo xtask ci` 通过 → `../process/commit-and-release.md` §1
14. **新建 crate**:只用 `cargo xtask new <name> [--bin]`,不手写 `Cargo.toml`;生成后 `cargo build` 并提交 `Cargo.lock`;`【新项目必改】` 标记的占位在首次发布前替换 → `../../crates/library/crate-anatomy.md` §1、§3

---

## Guidelines Index

| 文件 | 回答什么问题 | 何时读 |
|---|---|---|
| [module-layout.md](./module-layout.md) | 文件内条目顺序、`use` 三组分隔、模块拆分、`pub(crate)`、`#[allow]` 规则 | 新建文件 / 新模块 / 调整可见性 |
| [naming-and-api-design.md](./naming-and-api-design.md) | 命名(C-CASE / C-CONV / C-GETTER)、参数与返回类型、`Config` 结构体、构造与 `Default`、必须 derive 的 trait、`non_exhaustive` / newtype、控制流写法 | 设计任何 `pub` 签名;写 `match` / `if let` 犹豫时 |
| [error-handling.md](./error-handling.md) | 库 `thiserror` 枚举 vs 二进制 `anyhow`;`InvalidInput` 边界校验;文案口径;`# Errors` | 新增可失败函数 / 新错误变体 |
| [documentation.md](./documentation.md) | `//!` / `///` 要求、doctest、intra-doc link、行内注释、clap 帮助文本 | 写任何 `pub` 项;写注释 |
| [testing.md](./testing.md) | 三层测试放置、命名、禁止项、库 / 二进制专项、跨平台 | 写测试;review 别人测试 |
| [dependencies-and-changelog.md](./dependencies-and-changelog.md) | 新增依赖流程、feature、lint 基线含义、MSRV、CHANGELOG 写法、版本判定 | 加依赖 / 改公开行为 / 发版前 |
| [references.md](./references.md) | 每条规则的来源、实证证据、明确不采纳的业内做法 | 质疑「为什么要这样」时;修改规则前 |

---

## Pre-Development Checklist

动手前过一遍:

- [ ] **改的是哪一层?** 库 crate(纯逻辑、无 IO、`Result<T, Error>`)/ 二进制(参数、日志、分派、打印)/ xtask。业务逻辑落在库里,`main.rs` 只做胶水。依赖方向只能 二进制 → 库 → 更底层库。
- [ ] **要新建 crate?** 用 `cargo xtask new <name> [--bin]`,不要手写 `Cargo.toml`;生成后 `cargo build` 并提交 `Cargo.lock`。
- [ ] **要加依赖?** 先 `cargo tree -d` 看有没有;加到根 `[workspace.dependencies]` + 中文注释 + 最小 feature;许可证在 `deny.toml` 白名单。
- [ ] **改公开 API / 行为 / MSRV?** 同一提交在 `CHANGELOG.md` `[Unreleased]` 记一条;破坏性以 **BREAKING** 开头。
- [ ] **新增可失败函数?** 库:`Result<T>` + `# Errors`;二进制:`anyhow::Result` + `.with_context`。外部输入在公开边界校验。
- [ ] **新公开类型?** `#[derive(Debug, Clone, ..)]` 按固定顺序;字段私有(库);枚举 `#[non_exhaustive]`;有 `///`。
- [ ] **用的语法 / API 在 Rust 1.85 已稳定?**(let-chains 需要 1.88,不能用)
- [ ] **改 `xtask/src/ci.rs` 的步骤?** 同步 `.github/workflows/ci.yml`,反之亦然。
- [ ] 需要跨层数据流或复用判断时,读 `../../guides/index.md`。

---

## Quality Check

提交前必须通过(等价于 CI 的 typos → fmt → clippy → test → doc):

```bash
cargo xtask ci
```

拆开跑:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items
typos                                   # 装了 typos-cli 时
cargo deny check                        # 装了 cargo-deny 时;CI 必跑
cargo +1.85 check --workspace --all-targets   # MSRV;CI 必跑
cargo publish --workspace --dry-run --locked  # 有库 crate 时;CI 必跑
```

**Review 时额外看**(工具查不出来的):

- [ ] 文件顺序:`//!` → `mod` → 三组 `use` → 常量 → 类型 → impl → 函数 → `mod tests`;公开在私有之前
- [ ] 二进制 / xtask 里没有多余的 `pub`(应为 `pub(crate)`);没有 `utils` / `helpers` 模块
- [ ] 参数是 `&str` / `&[T]` / `&Path`;没有全是字面量的 `bool` / `Option` 参数;≥ 5 个参数已收成结构体
- [ ] `if let ... else` → `match`;`Err(e)?` → `return Err(e)`;没有 `ref`
- [ ] 每个 `#[allow]` 有原因;测试模块用标准头部
- [ ] 错误文案中文、无句尾标点、每层只描述自己这层
- [ ] 测试名是行为陈述;成功 / 失败路径成对;没有 `#[should_panic]` / `#[ignore]`
- [ ] CHANGELOG 条目写的是「对使用者的影响」
- [ ] 同义反复测试:把功能删掉测试还能过 → 打回
