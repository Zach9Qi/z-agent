# 模块、导入与文件内顺序

> 让第一次打开文件的人自上而下读一遍就能抓住主线;让 `git blame` / diff 干净。
> 规则来源:本仓库 `xtask/src/*.rs` 的既有写法 + rust-analyzer 《Style》(Order of Imports / Import Style / Order of Items)。

---

## 1. 文件骨架(自上而下)

每个 `.rs` 文件按下面顺序组织,段之间空一行。`xtask/src/main.rs`、`xtask/src/release.rs`、`xtask/src/git.rs` 是**头部与 import 分组**的范例(条目顺序见下方说明)。

```text
//! 模块级文档:这个文件负责什么、关键约束是什么(1-5 行)
#![allow(...)]            // 仅 crate 根;必须带原因注释(见 §5)

mod a;                    // 子模块声明,按「建议阅读顺序」排列,放在 use 之前
mod b;

use std::...;             // 第 1 组:std / core / alloc

use anyhow::...;          // 第 2 组:外部 crate(含工作区内其他 crate)
use serde::Deserialize;

use crate::version::...;  // 第 3 组:本 crate(优先 crate::,少用 super::)

pub use ...;              // re-export 视为「定义项」而非导入,放在导入之后;库 crate 根才用

pub const ...;            // 常量:先 pub 后私有
const USAGE: &str = ...;

pub struct / pub enum     // 类型定义:先公开后私有;父类型在子类型之前(自顶向下)
struct Helper { .. }

impl ...                  // impl 紧跟自己的类型;同一类型的 impl 不要散落到多个文件

pub fn run(..)            // 函数:先公开入口,后私有辅助;辅助函数放在调用者之后
fn helper(..)

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests { use super::*; ... }   // 永远在文件末尾
```

**判断依据**:把所有函数体折叠后,文件应该像一份按重要性排列的 API 文档。全是私有项只有一个公开项时,公开项永远在最上面。

**硬性要求**只有三条:`//!` / `#![allow]` / `mod` / 三组 `use` 的头部顺序;`impl` 紧跟自己的类型;`mod tests` 在文件末尾。「公开在前、辅助函数在调用者之后」是**新文件**的目标写法(rust-analyzer 的 top-down 规则)。

> 现有 xtask 代码是 bottom-up 的:`release.rs:172` / `new_crate.rs:107` 的 `pub fn run` 在一串私有 helper 之后,`main.rs:43` 的 `repo_root` 在 `main` 之前,`version.rs:325-336` 的私有 `Metadata` 夹在函数之间。它们的头部与 import 分组是范例,条目顺序不是;改到这些文件时**不必**为了顺序重排(diff 噪音),新文件按 top-down 写。

参考文件(头部与 import 分组):
- `xtask/src/main.rs:1-28`:`//!` → `#![allow]`(带原因)→ `mod` × 6 → `use std` → 空行 → `use anyhow`
- `xtask/src/release.rs:10-16`:三组 import 用空行隔开;`:19-37` 常量在类型之前,`Options` 类型紧跟其 `///`

---

## 2. 导入(`use`)规则

| 规则 | 写法 | 原因 |
|---|---|---|
| 三组分隔 | std → 外部 crate → `crate::`,组间空行,组内 rustfmt 自动排序(`rustfmt.toml` 已开 `reorder_imports`) | 一眼看出本文件依赖了哪些层;新增外部依赖在 diff 里显眼 |
| 一 crate 一条 `use` | `use anyhow::{Context, bail, ensure};` 而不是三行 | rustfmt **不会**自动合并(`imports_granularity` 是 nightly 选项),靠人工;`reorder_imports` 只负责组内排序 |
| 实现 `fmt` / `ops` trait 时导入模块 | `use std::fmt;` + `impl fmt::Display for Version` | 一眼区分「实现 trait」和「使用 trait」;少打字。范例 `xtask/src/version.rs:11,159` |
| 常见名冲突的模块用限定路径 | `use crate::version;` 然后 `version::check_cli(..)`;`use crate::changelog::{self, CHANGELOG};` | 避免多个模块都有 `run` / `Error` 时靠 IDE 才知道来源。范例 `xtask/src/release.rs:14-16` |
| 优先 `crate::` 路径 | 不用 `super::`(测试模块的 `use super::*;` 例外)、不用 `self::` | 在文件移动、模块拆分时路径依然成立 |
| 禁止局部 glob | 不写 `use MyEnum::*;`、不写 `use some_module::*;`(`prelude` 除外) | 读者找不到符号出处;新变体会悄悄遮蔽局部名 |
| trait 实现不用内联全路径 | `use std::fmt;` 后 `impl fmt::Display`,不写 `impl std::fmt::Display` | 见上一条。**函数与类型**的少量使用允许内联全路径(`std::fs::write(..)`、`std::ops::Range<usize>`、`std::env::temp_dir()`,xtask 里到处如此),同一文件用到 3 次以上再 `use`;已 `use` 的项不要再写全路径(`unused_qualifications` 会报) |

反例(禁止):

```rust
impl std::fmt::Display for Foo { .. }           // 内联全路径
use crate::a::*;                                // glob
use super::super::config::Config;               // 用 crate:: 代替
```

---

## 3. 模块拆分与文件布局

- **一个领域一个文件**:`lib.rs` 模板注释即此约定(`xtask/src/new_crate.rs` `lib_rs()` 生成的 `//!`)。xtask 的 `changelog.rs` / `version.rs` / `release.rs` / `git.rs` 就是按职责切的,而不是按「utils / helpers / types」这类技术维度切。
- **小 crate 平铺单文件**(`src/lib.rs` + `src/error.rs` + 每个领域一个 `.rs`),globset / walkdir / semver / anyhow 都是这样;不要一上来就建目录。
- **确实需要子模块目录时用 `foo.rs` + `foo/bar.rs`**,不用 `foo/mod.rs`。业内两种都常见(clap / tracing / jiff 用 `mod.rs`,indexmap / std 用 `foo.rs`),本仓库统一后者:编辑器 tab 和 `git log --stat` 里不会出现一排 `mod.rs`。
- **禁止 `utils.rs` / `helpers.rs` / `common.rs` 垃圾桶模块**。放不下的函数说明它缺一个领域名;实在通用的两三个函数,直接放在唯一调用者所在文件的末尾。
- **`mod` 声明按阅读顺序**排列,不必按字母序(`rustfmt.toml` 的 `reorder_modules = true` 只作用于连续的 `mod` 块,想分组就用空行/注释隔开)。
- 库 crate 根 `lib.rs` 只做三件事:`//!` 文档、`mod` 声明、`pub use` re-export;不在 `lib.rs` 里写业务函数(`new_crate.rs` 的 `lib_rs()` 模板就是这个形状)。

---

## 4. 可见性

| 需求 | 写法 |
|---|---|
| 公开 API(库 crate) | `pub`,必须有 `///` 文档(`missing_docs = "warn"`,CI 下即报错) |
| crate 内共享、不对外 | `pub(crate)`。二进制 crate 与 xtask 里所有跨模块共享的项都用 `pub(crate)` 而不是 `pub` —— 二进制没有外部使用者,`pub` 是误导 |
| 只在本文件用 | 不加修饰 |
| 字段 | 有不变量 → 私有 + 借用型 getter(`fn name(&self) -> &str`),**不提供 setter**;任何值都合法 → 直接 `pub` 字段。xtask 内部的 `Version`(`xtask/src/version.rs:55-64`)字段 `pub`、只在 `FromStr`(`:116`)入口校验,`pre: Some("")` 这类非法值靠「不从别处构造」约定——二进制内部可以接受;**库 crate 的公开类型**有这种不变量必须私有字段 + getter,不能靠约定 |

> 注意:xtask 现有代码在 `changelog.rs` / `version.rs` 中用的是 `pub fn`(因为 `main.rs` 是 crate 根,`pub` 与 `pub(crate)` 在二进制里效果相同)。新代码统一用 `pub(crate)`,改到旧代码时顺手改,不必专门刷一遍。

**re-export 要克制**:库 crate 根 `pub use error::{Error, Result};` 是标准做法(模板已生成);除此之外每加一个 `pub use` 都要问「用户为什么需要从这里拿它」。二进制 / xtask 不做 re-export。

---

## 5. `#[allow]` 的用法

- **crate 级 `#![allow]` 只允许出现在 `main.rs` / `lib.rs` 顶部,且必须紧跟一行 `//` 说明原因**。范例 `xtask/src/main.rs:15-16`、`new_crate.rs` `main_rs()` 模板。
- **项级 `#[allow]` 必须写原因,并且能说明「什么时候删」**,如模板里的
  `// 骨架里还没有可失败的步骤,先放行 unnecessary_wraps;接入业务逻辑后删掉这行`。
- 测试模块统一 `#[cfg(test)] #[allow(clippy::unwrap_used, clippy::expect_used)]`,这是唯一不需要写原因的 allow。
- 发现某条 pedantic lint 在整个仓库都是噪音 → 改根 `Cargo.toml` `[workspace.lints.clippy]` 并写注释,不要在十个文件里各放一次。

---

## 6. 自检清单

- [ ] 文件顶部有 `//!`,说明职责与关键约束
- [ ] `mod` 在 `use` 之前;`use` 分三组、组间空行;没有 glob、没有 `super::`(测试除外)
- [ ] `impl` 紧跟类型;`mod tests` 在末尾;新文件:公开项在私有项之前、辅助函数在调用者之后
- [ ] 二进制 / xtask 里没有多余的 `pub`(应为 `pub(crate)`)
- [ ] 没有新的 `utils` / `helpers` / `common` 模块
- [ ] 每个 `#[allow]` 都带原因注释
