# 库 crate 解剖:骨架、边界、API 演进与发布

> 库 crate 是这个仓库唯一发布到 crates.io 的产物。它的每个公开项都是承诺,每次发布都不可撤销。
> 规则来源:`cargo xtask new <名字>` 的生成模板(`xtask/src/new_crate.rs:157-267`)、`../../workspace/rust/index.md`「硬约束速览」第 1 条、`README.md`「面向 crates.io 的约定」、根 `Cargo.toml`、`.github/workflows/ci.yml` 的 `package` job。

---

## 1. 生成方式与目录骨架

只用 `cargo xtask new <名字>` 生成,不手写 `Cargo.toml`。名字必须是 kebab-case(`new_crate.rs:46-57` `validate_name`)。生成器一次写出四个文件并登记依赖(`new_crate.rs:122-129`):

```text
crates/<名字>/
├── Cargo.toml     # lib_manifest():crates.io 元数据 + 继承工作区 + [lints] workspace = true
├── README.md      # lib_readme():crates.io 页面,首段 + 四个二级标题
└── src/
    ├── lib.rs     # lib_rs()://! 设计约束 + mod error + pub use
    └── error.rs   # ERROR_RS:thiserror Error + Result 别名
```

### 1.1 `Cargo.toml`(`new_crate.rs:157-189`)

| 字段 | 模板值 | 规则 |
|---|---|---|
| `description` | `"TODO:一句话说明这个 crate 负责什么"`(`:162`) | crates.io **必填**,发布前必须替换;`cargo publish --dry-run` 不会拦 TODO 文案,靠自检 |
| `keywords` | `[]`(`:164`) | ≤ 5 个,小写,搜索用词;留空可发布 |
| `categories` | `[]`(`:165`) | 只能用 <https://crates.io/category_slugs> 上的官方 slug;写错不会被拒收,crates.io 只打一行 warning 然后**静默丢弃**,crate 不会出现在任何分类页——发布前对照官方列表自检 |
| `readme` | `"README.md"`(`:166`) | 相对 crate 目录;模板已把它列入 `include`(即便漏掉,cargo 也会自动把 `readme` 指向的文件打进包) |
| `documentation` | `"https://docs.rs/<名字>"`(`:167`) | 不要改成别的站点,docs.rs 自动构建 |
| `version` / `authors` / `repository` / `license` / `edition` / `rust-version` | 全部 `.workspace = true`(`:168-173`) | 版本单一来源,禁止在成员里写具体值 |
| `include` | `["src/**/*", "Cargo.toml", "README.md"]`(`:175`) | 白名单模式;新增随包分发的文件(`LICENSE-*`、被 `include_str!` 引用的文件)必须加进来,否则打包后编不过 |
| `[dependencies]` | `thiserror` / `tracing` `.workspace = true`(`:177-179`) | 新依赖先进根 `[workspace.dependencies]`,见 `../../workspace/rust/dependencies-and-changelog.md` §1 |
| `[lints] workspace = true` | (`:181-182`) | 不得改成局部 lint 表 |
| `[package.metadata.docs.rs] all-features = true` | (`:185-186`) | 让可选 feature 的 API 也出现在 docs.rs |

### 1.2 `README.md`(`new_crate.rs:191-218`)

首段一句话说明,之后四个二级标题: `## 安装`(`cargo add <名字>`)→ `## 用法`(最小可运行示例)→ `## 最低支持的 Rust 版本 (MSRV)` → `## 许可证`。
用法示例与 `lib.rs` crate 文档的 doctest 保持同一份代码(`../../workspace/rust/documentation.md` §6)。

### 1.3 `src/lib.rs`(`new_crate.rs:220-236`)

模板原文(`new_crate.rs:222-233`)的形状,摘要如下:

```rust
//! `<名字>`:TODO 一句话说明这个 crate 负责什么。
//!
//! 设计约束:
//! - 不做 IO、不读环境变量、不打印;输入输出全部通过函数参数与返回值
//! - 可失败的公开函数一律返回 [`Result<T>`],错误类型是 [`Error`]
//! - 日志只用 `tracing` 宏,由调用方(二进制 crate)决定订阅器与输出级别
//!
//! 新增领域模块时按「一个领域一个文件」组织,在此处 `pub mod` 并 re-export 常用类型。

mod error;

pub use error::{Error, Result};
```

**`lib.rs` 顶部不写 lint 属性块**:tokio / tracing 那种十几行 `#![warn(...)]` 在本仓库由根 `Cargo.toml` `[workspace.lints]` 统一承担(typos-cli 同此做法),重复声明只会漂移。允许出现的 crate 级属性只有两种:有可选 feature 时的 `#![cfg_attr(docsrs, feature(doc_cfg))]`(配合 `rustdoc-args = ["--cfg", "docsrs"]`);或把 README 直接当 crate 文档的 `#![doc = include_str!("../README.md")]`(clap_builder 的做法,采用后 README 里的代码块就是 doctest,必须能编译)。

`lib.rs` 只做三件事:`//!` 文档、`mod` 声明、`pub use` re-export。新增领域「一个领域一个文件」:`src/<领域>.rs` + `pub mod <领域>;`,常用类型再 `pub use`(`../../workspace/rust/module-layout.md` §3)。不要在 `lib.rs` 里写业务函数。

### 1.4 `src/error.rs`(`new_crate.rs:238-267`)

`#[non_exhaustive] pub enum Error` + `pub type Result<T, E = Error>`,初始变体 `InvalidInput(String)` 与 `Io(#[from] std::io::Error)`。扩展规则全部在 `../../workspace/rust/error-handling.md` §2,此处不重复。

---

## 2. 分层约束:库里能做什么、不能做什么

| 禁止 | 原因 | 替代 |
|---|---|---|
| 文件 / 网络 / 进程 IO | 单测需要 mock、多前端无法复用(`README.md`「分层与依赖方向」) | 收 `&str` / `&[u8]` / 结构体,返回值;IO 由二进制 crate 做 |
| `std::env::var` | 行为随环境漂移,测试不可重现 | 作为参数传入 |
| `println!` / `eprintln!` / `dbg!` | `print_stdout` / `print_stderr` / `dbg_macro` lint(根 `Cargo.toml:59-62`) | `tracing::debug!` / `info!` / `warn!`(`README.md`「错误与日志」);订阅器与级别由调用方装配;不要 `error!` 之后再把同一个错误返回出去(重复上报) |
| `tracing_subscriber` 装配 | 订阅器属于二进制层 | 库只用 `tracing` 宏 |
| `unwrap` / `expect` / `panic!` | `unwrap_used` / `expect_used`(根 `Cargo.toml:56-57`) | `?` + `Error` 变体;测试模块顶部 allow |
| 依赖 `anyhow` | 下游无法 `match` | `thiserror` + 自己的 `Error` |
| 依赖二进制 crate 或同层 crate 互相引用 | 依赖方向只能「库 → 更底层的库」 | 抽公共部分到更底层的库 |

所有可失败公开函数返回 `Result<T>`;外部输入在公开函数第一行附近校验,不合法返回 `Error::InvalidInput`(文案含参数名与期望)。

---

## 3. 「新项目必改 / 发布前必改」占位清单

模板刻意留了 TODO 让 `cargo xtask new` 结束时提示(`new_crate.rs:138-142`)。**首次发布前全部替换**:

| 位置 | 占位 |
|---|---|
| `Cargo.toml:description` | `TODO:一句话说明这个 crate 负责什么` |
| `Cargo.toml:keywords` / `categories` | `[]`(注释标 `【发布前必改】`,`new_crate.rs:163`) |
| `README.md` 首段 | `TODO:一句话说明…` |
| `README.md` 用法 | `// TODO:一个最小可运行示例` |
| `src/lib.rs` 首行 | `//! \`<名字>\`:TODO 一句话说明…` |

检查命令(期望零输出):

```bash
grep -rn "TODO" crates/<名字>
```

---

## 4. 公开 API 演进与 semver

版本号全工作区 lockstep(根 `Cargo.toml:13`),任何一个库有破坏性变更就整体 bump。当前 `0.x`:**minor 即破坏性,patch 只能加东西或修 bug**。

### 4.1 什么算破坏性(必须 **BREAKING** + bump minor/major)

- 删除 / 重命名任何 `pub` 项(函数、类型、模块、re-export、feature)
- 给 `pub` 字段结构体加字段(下游用结构体字面量 / 穷尽解构会编不过);私有字段结构体加字段不算
- 给**没有** `#[non_exhaustive]` 的枚举加变体;模板 `Error` 已有该属性(`new_crate.rs:245`),新公开枚举也要加
- 改函数签名(参数类型、返回类型、泛型约束、`&self` ↔ `&mut self`)
- 公开类型失去某个 trait 实现(`Send` / `Sync` / `Clone` / `Debug`…)
- 提升 `rust-version`(记 `Changed`,0.x 走 minor)
- 公开 API 里暴露的第三方类型做了大版本升级

### 4.2 不算破坏性(patch 可发)

新增 `pub` 项、`#[non_exhaustive]` 枚举加变体、私有字段变更、错误文案修正、性能优化、文档。

### 4.3 弃用流程

1. 加 `#[deprecated(since = "<下一个版本>", note = "改用 `新名字`")]`,保留旧实现转调新实现
2. `CHANGELOG.md` `[Unreleased]` → `Deprecated` 写替代 API
3. 至少间隔一个 minor 后再 `Removed`(**BREAKING**)

### 4.4 CHANGELOG

任何公开 API 变化同一提交里在 `CHANGELOG.md` `[Unreleased]` 写一条(`Added` / `Changed` / `Deprecated` / `Removed` / `Fixed`);`[Unreleased]` 为空时 `cargo xtask release` 拒绝发版(`xtask/src/changelog.rs:88-100`)。写法见 `../../workspace/rust/dependencies-and-changelog.md` §4。

---

## 5. 内部依赖登记与依赖方向

`cargo xtask new` 生成库时调用 `register_workspace_dependency`(`new_crate.rs:61-93`),在根 `Cargo.toml` `[workspace.dependencies]` 段头及其紧随注释之后插入:

```toml
<名字> = { path = "crates/<名字>", version = "0.1.0" }
```

- **`version` 不能省**:发到 crates.io 后 `path` 失效,下游只能靠 `version` 解析(`README.md`「内部依赖必须带版本」)。`cargo xtask release` 同步该值(`xtask/src/version.rs:304-323` `write_workspace_version`),`cargo xtask version check` 校验一致(`version.rs:436-477`)。不要手改。
- 已存在同名条目时跳过登记(幂等,`new_crate.rs:68-75`)。
- 消费方(二进制或更上层的库)写 `<名字>.workspace = true`,不重复 `path`。
- 依赖方向:二进制 → 库 → 更底层的库;库之间不允许环,也不允许库依赖二进制 crate。`cargo tree -p <名字> -e normal` 确认没有意外的反向边。

---

## 6. 发布相关

| 事项 | 依据 |
|---|---|
| CI 每个 PR 跑 `cargo publish --workspace --dry-run --locked`(`.github/workflows/ci.yml:144`),在隔离目录用打包后源码重编译 | 漏 `include`、缺 `description`、内部依赖没 `version` 都在这一步暴露 |
| 发版 `cargo publish --workspace --locked`(`.github/workflows/release.yml:209-210`),`publish = false` 成员自动跳过 | 首个版本需本机 `cargo publish -p <名字>` 手动发一次再配置 Trusted Publishing(`release.yml:179-183`) |
| `include` 白名单只带 `src/**/*`、`Cargo.toml`、`README.md` | 加了 `LICENSE-*`、`benches/`、`include_str!` 资源就要同步 `include` |
| lockstep 版本:全部成员同版本一起发 | 单个库没改动也会跟着 bump,这是有意为之 |
| 发到 crates.io 不可撤销 | 发错只能 `cargo yank` 再 bump;发前本地 `cargo publish -p <名字> --dry-run` |

---

## 7. 自检清单

- [ ] crate 由 `cargo xtask new <名字>` 生成;`Cargo.toml` 所有元数据字段 `.workspace = true`,`[lints] workspace = true`
- [ ] `grep -rn "TODO" crates/<名字>` 为空;`description` 已填,`keywords` ≤ 5,`categories` 是官方 slug
- [ ] `lib.rs` 只有 `//!` / `mod` / `pub use`;新领域是独立文件
- [ ] 没有 IO / `std::env` / `println!` / `unwrap` / `anyhow`;可失败公开函数返回 `Result<T>` 并校验外部输入
- [ ] 新公开枚举 `#[non_exhaustive]`;公开结构体字段默认私有
- [ ] 破坏性变更 CHANGELOG 以 **BREAKING** 开头;弃用用 `#[deprecated(since, note)]`
- [ ] 随包分发的新文件已加入 `include`;`cargo publish -p <名字> --dry-run` 通过
- [ ] 根 `[workspace.dependencies]` 有 `{ path, version }` 条目;消费方用 `.workspace = true`;无依赖环
