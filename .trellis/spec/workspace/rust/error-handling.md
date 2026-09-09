# 错误处理

> 库 crate 用 `thiserror` 给调用方**可匹配的强类型错误**;二进制 / xtask 用 `anyhow` 给终端用户**可读的因果链**。
> 来源:[index.md](./index.md)「硬约束速览」第 2 条、`cargo xtask new` 生成的 `error.rs` / `main.rs` 模板(`xtask/src/new_crate.rs`)、Rust API Guidelines C-GOOD-ERR / C-FAILURE / C-VALIDATE、BurntSushi 系 crate(`ignore` / `globset` / `regex`)与 `clap_builder` 的错误类型形态。

---

## 1. 两种世界,一条边界

| | 库 crate(`crates/<name>`,发 crates.io) | 二进制 crate / xtask |
|---|---|---|
| 错误类型 | 自己的 `Error` 枚举(`src/error.rs`,thiserror)+ `pub type Result<T, E = Error>` | `anyhow::Result<T>`;不定义错误类型 |
| 谁看这个错误 | 下游 Rust 代码(要 `match`)+ 最终用户(要 `Display`) | 只有终端用户 |
| 添加上下文 | 用变体字段携带(文件名、参数名) | `.with_context(\|\| format!("做什么 {对象}"))` |
| 出口 | 返回给调用方 | `main` 统一 `eprintln!("错误: {err:#}")`(xtask 用 `xtask 中止:{err:#}`)后 `ExitCode::FAILURE` |
| `unwrap` / `expect` / `panic!` | **禁止**(`unwrap_used` / `expect_used` 已开;库 crate 不应 panic) | 同样禁止;`main.rs` 也不例外 |

**边界规则**:二进制调用库时,库错误经 `?` 自动转成 `anyhow::Error`(`Error: std::error::Error + Send + Sync + 'static` 即可),不需要 `map_err`。库 **不能** 依赖 `anyhow`;二进制 **不应** 定义 thiserror 枚举(它没有需要 `match` 的调用方)。

---

## 2. 库 crate:`error.rs` 的形状

模板(`xtask/src/new_crate.rs` `ERROR_RS`)生成的即是标准形状:

```rust
//! 本 crate 的统一错误类型。新增变体时保持中文文案口径,文案面向终端用户直接展示。

/// 本 crate 的统一 `Result` 别名。
pub type Result<T, E = Error> = core::result::Result<T, E>;

/// 统一错误类型:可失败函数一律返回 [`Result<T>`],`Display` 输出用户可读的中文文案。
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// 调用方传入的参数不合法(外部输入不可信,校验放在公开函数的边界)
    #[error("参数错误: {0}")]
    InvalidInput(String),

    /// 文件系统等输入输出错误
    #[error("输入输出错误: {0}")]
    Io(#[from] std::io::Error),
}
```

### 2.1 必须遵守

- **一个 crate 一个 `Error`**,放 `src/error.rs`,`lib.rs` 里 `pub use error::{Error, Result};`。不要每个模块一个错误类型——调用方只想 `match` 一处。
- **形态升级路线**:枚举变体都需要携带同一组上下文(如文件路径、行号、原始输入)时,改成 `pub struct Error { path: Option<PathBuf>, kind: ErrorKind }` + `#[non_exhaustive] pub enum ErrorKind` + `fn kind(&self) -> &ErrorKind` 访问器。这是 globset(`Error { glob, kind }`)、clap_builder(`Error { inner: Box<ErrorInner> }` + `kind()`)、walkdir(`Error { depth, inner }`)的做法;仍然是**一个**公开错误类型,thiserror 对 struct 同样可用(`#[error("{kind}")]` + 字段)。不要走到「多个公开错误类型」。
- **`#[non_exhaustive]`**:允许日后加变体不算破坏性变更。下游匹配时必须写 `_ =>`,这是有意为之。
- **`#[derive(Debug, thiserror::Error)]`**,并且满足 C-GOOD-ERR:`Debug + Display + Send + Sync + 'static`。带 `Rc` / `RefCell` / 裸指针的字段会破坏 `Send + Sync`,禁止放进错误。
- **`#[error("...")]` 文案是中文、面向用户、不带句尾标点、以名词短语或「做什么失败」开头**;不复述类型名(`Display` 不写 "Error: "),让 `main` 的 `错误: {err:#}` 拼出「错误: 参数错误: xxx」这样的一句话。
- **每个变体都有 `///` 文档**说明何时产生(`missing_docs` 对枚举变体同样生效)。
- **`#[from]` 只给「整个 crate 只有一种来源」的底层错误**(`std::io::Error`、`serde_json::Error`)。同一底层错误在不同场景语义不同时,用 `#[source]` + 显式字段,并在变体里带上上下文:

```rust
/// 读取配置文件失败
#[error("读取配置 {path} 失败")]
ReadConfig {
    /// 配置文件路径
    path: std::path::PathBuf,
    /// 底层 IO 错误
    #[source]
    source: std::io::Error,
},
```

- **公开函数边界校验外部输入**(C-VALIDATE):不合法立即 `return Err(Error::InvalidInput(...))`,文案说明「哪个参数、为什么不合法、期望什么」。内部私有函数信任已校验的输入,不重复校验。范例(anyhow 版,思路相同):`xtask/src/new_crate.rs` `validate_name` 在 `parse_args` 边界拒绝非 kebab-case 名字,后续文件生成不再检查。
- **不要用 `Box<dyn Error>` 作为库的公开错误类型**,也不要 `String` / `()` 当错误——下游无法匹配(C-GOOD-ERR)。不要实现已弃用的 `Error::description()`(老库 ignore / regex 仍保留是为了兼容,新库 toml / jiff / camino 都不写)。
- **不要在库里 `tracing::error!` 之后再返回同一个错误**(重复上报);库用 `debug!` / `info!` / `warn!` 记录内部状态与可恢复的异常(`README.md`「错误与日志」),不可恢复的失败只通过返回值上报,谁处理错误谁决定是否记 `error!`。

### 2.2 何时加新变体、何时复用 `InvalidInput`

| 情况 | 做法 |
|---|---|
| 调用方传参不合法(空字符串、越界、格式错) | `InvalidInput(String)`,文案带参数名 |
| 底层库返回错误且只有一种来源 | 新变体 + `#[from]` |
| 同一底层错误多处产生、需要区分 | 新变体 + 结构体字段 + `#[source]` |
| 业务状态不允许(重复、冲突、未找到) | 新的语义变体(`NotFound { name }`、`AlreadyExists { name }`),**不要**塞进 `InvalidInput` 的字符串里——下游需要 `match` 它 |

### 2.3 文档:`# Errors` 小节

每个返回 `Result` 的公开函数,`///` 里写 `# Errors` 小节,列出**会返回哪些变体、在什么条件下**(C-FAILURE)。有 `# Errors` 小节后 `clippy::missing_errors_doc` 才安静。

```rust
/// 解析工作区版本号。
///
/// # Errors
///
/// - `toml` 里没有 `[workspace.package]` 段或缺少 `version` 字段时返回 [`Error::InvalidInput`]
/// - `version` 的值不是合法 semver 时返回 [`Error::InvalidInput`],文案含原始字符串
pub fn parse_version(toml: &str) -> Result<Version>
```

`# Panics` 小节:库不应 panic,所以一般不写;`missing_panics_doc` 已在工作区 allow。真的有 `assert!` 前置条件(不是外部输入)时才写。

---

## 3. 二进制 / xtask:`anyhow` 的用法

范例:`xtask/src/main.rs`、`xtask/src/git.rs`、`xtask/src/changelog.rs`。

| 场景 | 写法 | 范例 |
|---|---|---|
| 给底层错误加「在做什么」 | `.with_context(\|\| format!("读取 {CHANGELOG} 失败"))?` | `changelog.rs:21` |
| 上下文是常量字符串 | `.context("...")?`(不要 `with_context(\|\| "...")`) | — |
| 条件不满足直接失败 | `ensure!(cond, "文案 {var}")` | `new_crate.rs:52` |
| 无条件失败 | `bail!("未知子命令 {other}\n{USAGE}")` | `main.rs:84` |
| 构造错误值但不立刻返回 | `anyhow!("...")`(或 rust-analyzer 偏好的 `format_err!`,二选一,仓库内统一用 `anyhow!`) | `version.rs:120` |
| 返回类型 | 写全 `anyhow::Result<T>`,不 `use anyhow::Result` 后写裸 `Result` | 全部 xtask 文件 |
| `Option` → 错误 | `.with_context(\|\| ...)?`(anyhow 给 `Option` 实现了 `Context`) | `main.rs:62-64` |

**文案约定**(与库一致):中文、不带句尾标点、「做什么 + 对象 + 失败/原因」,`{err:#}` 会把 context 链用 `: ` 连接成一行,所以每一层只描述自己这一层。用法错误把 `USAGE` 拼在文案末尾,让用户不用再敲 `--help`。

**退出码**:`main` 只区分 `ExitCode::SUCCESS` / `ExitCode::FAILURE`;需要更多退出码(例如 lint 工具的「发现问题」≠「运行失败」)时定义 `enum` + `impl From<..> for ExitCode`,不要散落魔法数字。

**子进程失败**:检查 `status.success()`,失败时把命令与 stderr 首行放进错误(`git.rs:16-23`),不要只报「命令失败」。

---

## 4. 禁止事项一览

| 禁止 | 替代 |
|---|---|
| 库里 `unwrap()` / `expect()` / `panic!` / `unreachable!` / 索引 `v[i]` 越界风险 | `?`、`ok_or(Error::..)`、`get(i)`;测试模块顶部 `#[allow(clippy::unwrap_used, clippy::expect_used)]` 放开 |
| `Result<T, String>` / `Result<T, Box<dyn Error>>` 作为公开 API | 库:`Result<T>`(自己的 `Error`);二进制:`anyhow::Result<T>` |
| `map_err(\|e\| Error::Io(e))` | `#[from]` + `?` |
| `map_err(\|e\| anyhow!("{e}"))`(丢掉 source 链) | `.with_context(..)` |
| `Err(e)?` 抛错 | `return Err(e);`(类型受约束,死代码可被编译器发现) |
| 忽略错误 `let _ = fallible();` | 显式 `match` / `if let Err(err)` 并 `tracing::warn!` 说明为什么可忽略 |
| `#[should_panic]` 测试 | 断言 `is_err()` 并检查变体 / 文案 |
| 错误文案里写 "Error" / "错误:" 前缀 | 前缀由 `main` 统一加 |

---

## 5. 自检清单

- [ ] 库 crate:新增可失败公开函数返回 `Result<T>`,`///` 有 `# Errors` 小节列出变体
- [ ] 新变体有 `///` 文档、中文 `#[error]` 文案、不带句尾标点;底层错误用 `#[from]` 或 `#[source]`
- [ ] 外部输入在公开函数第一行附近校验,失败返回 `InvalidInput`(文案含参数名与期望)
- [ ] 二进制 / xtask:每个 `?` 前有 `.with_context` / `.context`,或错误本身已含足够信息
- [ ] 没有新的 `unwrap` / `expect` / `panic!`(`cargo clippy --workspace --all-targets -- -D warnings` 通过)
- [ ] 测试覆盖至少一个错误路径,并断言 `Display` 文案(模板 `display_is_user_facing_chinese` 即范例)
