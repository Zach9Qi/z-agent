# 文档与注释

> rustdoc 是库 crate 的产品界面(docs.rs),`--help` 是二进制的产品界面;注释是给下一个维护者的。三者都用**中文**([index.md](./index.md)「硬约束速览」第 4 条),标识符、命令、路径保持原文。
> 来源:`missing_docs = "warn"` + `RUSTDOCFLAGS="-D warnings" cargo doc --document-private-items`(`xtask/src/ci.rs`、`.github/workflows/ci.yml`)、Rust API Guidelines C-CRATE-DOC / C-EXAMPLE / C-QUESTION-MARK / C-FAILURE / C-LINK / C-HIDDEN、rust-analyzer 《Style · Documentation》、`ignore` / `globset` / `tracing` 的 crate 文档结构。

---

## 1. 哪些东西必须有文档

| 项 | 要求 | 由谁检查 |
|---|---|---|
| 每个 `.rs` 文件 | 顶部 `//!`:职责 + 关键约束(1-5 行)。范例 `xtask/src/ci.rs:1-5`、`xtask/src/git.rs:1` | 人工 / review |
| 所有 `pub` 项(含枚举变体、结构体字段、trait 方法) | `///`,第一行是能独立成句的一句话摘要 | `missing_docs`(CI 下 `-D warnings`) |
| `pub(crate)` / 私有项 | 非自明的写一行 `///`;`cargo doc --document-private-items` 会检查其中的链接 | rustdoc `-D warnings` |
| 返回 `Result` 的公开函数 | `# Errors` 小节(见 `error-handling.md` §2.3) | `clippy::missing_errors_doc`(pedantic) |
| 会 panic 的公开函数 | `# Panics` 小节;库不应 panic,一般不需要(`missing_panics_doc` 已 allow) | 人工 |
| 库 crate `lib.rs` | crate 级文档:一句话定位 → 设计约束 → **至少一个可运行示例** → feature 说明(如有) | C-CRATE-DOC |
| clap 结构体 / 字段 | `///` 会成为 `--help` 文本:写给终端用户,不加反引号(`main.rs` 模板已 `#![allow(clippy::doc_markdown)]`) | 人工;`tests/cli.rs` 可断言 |

---

## 2. `///` 的写法

```rust
/// 在 `[workspace.dependencies]` 段头(及紧随其后的注释行)之后插入
/// `<名字> = { path = "crates/<名字>", version = "<工作区版本>" }`;version 由 `cargo xtask release` 同步
fn register_workspace_dependency(root: &Path, name: &str, version: &str) -> anyhow::Result<()>
```

- **第一段一句话说「它做什么」**,rustdoc 列表页只显示这一段;不要以「这个函数…」开头,也不要复述函数名。
- **代码、路径、命令、标识符一律反引号**:`` `Cargo.toml` `` / `` `cargo xtask release` ``。`clippy::doc_markdown`(pedantic)会要求这样做;唯一例外是 clap 帮助文本(见 §5)。
- **链接用 intra-doc link**:`` [`Error::InvalidInput`] `` / `` [`Result<T>`] ``,不要写 docs.rs URL;失效链接会被 `-D warnings` 拦住。模板 `lib.rs` / `error.rs` 已示范。
- **小节顺序**(需要时才写):摘要 → 详述 → `# Examples` → `# Errors` → `# Panics` → `# Safety`(本仓库 `unsafe_code = "forbid"`,不会出现)。
- **枚举变体 / 字段的 `///` 说明「什么时候是这个值 / 含义与单位」**,不是重复名字。范例 `xtask/src/version.rs:26-33` `Bump` 变体用 `x.y.Z` / `x.Y.0` / `X.0.0` 说明。
- 注释与文档里**不要写 TODO 而不留主体**:`// TODO(谁/issue): 什么条件下改` ;`clippy::todo` 会拦 `todo!()` 宏,但注释里的 TODO 靠 review。模板里的 `TODO:一句话说明…` 是「新项目必改」占位,创建 crate 后立刻替换。

---

## 3. 示例与 doctest(C-EXAMPLE / C-QUESTION-MARK)

- **库 crate 每个公开函数 / 类型至少一个 `# Examples`**(至少 crate 级文档一个)。doctest 随 `cargo test` 运行,是「永不过期的文档」。
- 示例里用 `?` 而不是 `unwrap()`:doctest 结尾写 `# Ok::<(), my_crate::Error>(())`(`#` 开头的行在渲染时隐藏,但会编译运行)。

```rust
//! ```
//! use my_core::{parse, Error};
//!
//! let value = parse("1.2.3")?;
//! assert_eq!(value.major, 1);
//! # Ok::<(), Error>(())
//! ```
```

- 示例要**最小**:只保留说明这个 API 所需的行;需要准备数据的行用 `#` 隐藏。
- 不能运行的示例标 `` ```no_run ``(需要网络 / 文件系统)或 `` ```text ``(不是 Rust);不要用 `ignore`(不编译、会腐烂)。范例 `xtask/src/main.rs:5-11` 用 `` ```text `` 展示命令行用法。
- 二进制 crate 一般不写 doctest;它的「示例」是 `tests/cli.rs`。

---

## 4. 行内注释(`//`)

- **写完整句子说明「为什么」**,不是「做什么」——代码已经说了做什么。中文句子不必加句号,但要成句。
  - GOOD:`// 测试进程可能继承开发者环境里的 RUST_LOG,清掉保证 -v 生效`(`cli_test_rs` 模板)
  - BAD:`// 移除 RUST_LOG`
- **每个 `#[allow]` 上方一行注释说明原因和何时可以删**(见 `module-layout.md` §5)。
- **不要注释掉代码留在仓库里**;要保留的替代方案写进 `design.md` 或 commit message。
- 与其写长注释解释一段绕的代码,先试试给条件抽变量、拆函数(`naming-and-api-design.md` §6)。
- Markdown 文件(README / CHANGELOG / spec)优先**一句一行**,不要在句中硬换行,diff 更干净。

---

## 5. 二进制 crate:`--help` 文本

clap derive 把 `///` 当作帮助文本,所以 `main.rs` / `args.rs` 里的文档注释有特殊规则(模板 `main_rs()` 已示范):

- 写给**终端用户**:说「这个选项做什么、默认值、与环境变量的关系」,不说实现。
  范例:`/// 输出 debug 级日志到 stderr(等价于 RUST_LOG=debug;显式设置 RUST_LOG 时以后者为准)`
- **不用反引号**(`--help` 里会原样显示成 `` `RUST_LOG` ``),因此该文件顶部 `#![allow(clippy::doc_markdown)]` 并注明原因。
- 字段第一行是短摘要(`-h`),空一行后的段落是长说明(`--help`)。
- 结构体级 `///` 是命令描述;`#[command(author, version, about, long_about = None)]` 从 `Cargo.toml` 读 `description` / `version`,不要在代码里重复写版本号。

---

## 6. README 与 CHANGELOG

- 库 crate 有独立 `README.md`(crates.io 页面),模板生成的结构:首段一句话说明,之后四个二级标题「安装 → 用法(最小可运行示例)→ MSRV → 许可证」。首个版本前把所有 `TODO` 替换掉。
- README 里的代码示例应与 `lib.rs` crate 文档的 doctest 保持同一份(用 `#![doc = include_str!("../README.md")]` 把 README 直接当 crate 文档是可选做法;采用时 README 示例就成了 doctest,必须能编译)。
- CHANGELOG 条目写「对使用者的影响」,不是 diff 复述(`CHANGELOG.md` 顶部约定);破坏性变更以 **BREAKING** 开头;MSRV 提升记 `Changed`。见 `dependencies-and-changelog.md`。

---

## 7. 验证

```bash
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items   # 链接 / 语法 / missing_docs
cargo test --workspace --doc                                                           # doctest
cargo clippy --workspace --all-targets -- -D warnings                                  # doc_markdown / missing_errors_doc
typos                                                                                  # 拼写(装了 typos-cli 时;CI 必跑)
```

`cargo xtask ci` 一次跑完以上全部。

---

## 8. 自检清单

- [ ] 新文件顶部有 `//!`;新 `pub` 项有 `///` 且首句独立成句
- [ ] 文档里的标识符 / 路径 / 命令都在反引号里(clap 帮助文本除外);链接用 intra-doc link
- [ ] 可失败公开函数有 `# Errors`;库公开 API 有 `# Examples` 且用 `?` 不用 `unwrap`
- [ ] 行内注释解释「为什么」;没有注释掉的代码;`TODO` 占位已替换
- [ ] `cargo xtask ci` 的 doc 步骤通过
