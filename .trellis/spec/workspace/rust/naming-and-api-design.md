# 命名与 API 设计

> 目标:调用方一眼看懂签名、改内部实现时不破坏调用方。
> 来源:Rust API Guidelines(标注 `C-XXX` 条目号,可在 <https://rust-lang.github.io/api-guidelines/checklist.html> 查原文)、rust-analyzer 《Style》、本仓库 `xtask/` 既有代码。

---

## 1. 命名(C-CASE / C-CONV / C-GETTER / C-ITER / C-WORD-ORDER)

| 项 | 写法 | 例 |
|---|---|---|
| crate | kebab-case,`cargo xtask new` 只接受 `^[a-z][a-z0-9-]*$` 且不以 `-` 结尾、无 `--`(`xtask/src/new_crate.rs` `validate_name`) | `my-core`、`my-cli` |
| 模块 / 函数 / 方法 / 局部变量 / 字段 | `snake_case` | `read_workspace_version` |
| 类型 / trait / 枚举变体 | `UpperCamelCase`;缩写词当普通单词(`Uuid` 不是 `UUID`,`Stdin` 不是 `StdIn`) | `Version`、`Bump::Patch` |
| 常量 / 静态量 | `SCREAMING_SNAKE_CASE` | `CARGO_TOML`、`USAGE` |
| feature | 不带 `use-` / `with-` 等占位词,直接用启用的东西命名(C-FEATURE) | `serde`、`std`,不是 `use-serde` |
| 一致的词序 | 同一组名字用同样的「动词-对象-错误」顺序(C-WORD-ORDER) | `ParseError` / `WriteError`,不要混 `ErrorParse` |

**转换方法前缀(C-CONV)**:

| 前缀 | 开销 | 所有权 | 例 |
|---|---|---|---|
| `as_` | 免费 | 借用 → 借用 | `str::as_bytes` |
| `to_` | 昂贵 | 借用 → 借用 / 借用 → 拥有 / 拥有 → 拥有(Copy 类型) | `str::to_lowercase`、`Path::to_path_buf` |
| `into_` | 可变 | 拥有 → 拥有(非 Copy) | `String::into_bytes` |

**getter(C-GETTER)**:不加 `get_` 前缀,直接 `fn name(&self) -> &str`。只有 `get` 后面跟着可变的「取哪一个」语义时才用 `get`(`Cell::get`、`<[T]>::get(i)`)。返回借用型:`&str` 而不是 `String` 或 `&String`,`Option<&T>` 而不是 `&Option<T>`,`&[T]` 而不是 `&Vec<T>`,`&Path` 而不是 `&PathBuf`。

**迭代器(C-ITER / C-ITER-TY)**:`iter()` / `iter_mut()` / `into_iter()` 三件套;返回的迭代器类型名与方法名对应(`Iter` / `IterMut` / `IntoIter`)。

**局部变量**:用「无聊而长」的名字,默认是类型名的小写(`snapshot: Snapshot`);允许的固定缩写只有 `ctx`、`db`、`acc`、`res`(函数最终结果)、`it`(不在意名字)、`n_foos`(数量)、`foo_idx`(下标)。与关键字冲突用 `krate` / `ty` / `func`,不用 `r#`。拼写用美式英语(`color`、`behavior`);`typos` 默认 `locale = "en"` 同时接受英美拼写,所以这条靠 review(要机器强制就在 `typos.toml` 加 `[default] locale = "en-us"`)。

**布尔变量给条件命名**:多行条件先抽成 `let is_release_branch = ...;` 再 `if`,便于调试与阅读(rust-analyzer "Helper Variables")。范例:`xtask/src/new_crate.rs` `validate_name` 里的 `valid_first` / `valid_rest`。

---

## 2. 参数与返回类型

### 2.1 左边永远优于右边

```text
&[T]        而不是  &Vec<T>
&str        而不是  &String
Option<&T>  而不是  &Option<T>
&Path       而不是  &PathBuf
impl Iterator<Item = T>  而不是  Vec<T>(当调用方只是遍历时)
```

范例:`xtask/src/git.rs` `query(root: &Path, args: &[&str])`;`xtask/src/main.rs` `run(args: &[String])`。

### 2.2 前置条件写进类型,不在函数内部判空

```rust
// GOOD:调用方负责保证有值,控制流在调用处可见
fn cut_release(changelog: &str, version: &Version) -> anyhow::Result<String>

// BAD:把 if 藏进函数,调用方不知道 None 会发生什么
fn cut_release(changelog: Option<&str>, version: Option<&Version>) -> anyhow::Result<String>
```

同理不要把 `if cond { f() }` 写成 `fn f() { if !cond { return } ... }`——**把 if 推给调用方,把 for 推进函数**。

### 2.3 用类型代替 `bool` / `Option` 参数(C-CUSTOM-TYPE)

```rust
// GOOD
enum Kind { Lib, Bin }
fn scaffold(root: &Path, name: &str, kind: Kind)

// BAD:调用处 scaffold(root, name, true) 没人知道 true 是什么
fn scaffold(root: &Path, name: &str, is_bin: bool)
```

范例:`xtask/src/new_crate.rs` `enum Kind`、`xtask/src/version.rs` `enum Bump`。

如果某个 `bool` / `Option` 参数在所有调用点都是字面量 `true` / `false` / `Some(..)` / `None`,**拆成两个函数**,公共部分抽 helper。

### 2.4 参数多于 4 个 → `Config` / `Options` 结构体

```rust
#[derive(Debug, PartialEq, Eq)]
struct Options { spec: String, dry_run: bool, no_push: bool }
fn parse_args(args: &[String]) -> anyhow::Result<Options>
```

范例:`xtask/src/release.rs:29-37` `Options`(每个字段都有 `///` 说明含义)。这类配置结构体**不要实现 `Default`**(调用方比库更清楚默认值)、**不要存进状态对象**,每次显式传入。

### 2.5 上下文参数放最前

贯穿多层调用不变的参数(`root: &Path`、`db`、`ctx`)放在参数表最前面。xtask 里所有**触碰仓库文件或子进程**的函数都是 `fn xxx(root: &Path, ...)`(`git.rs`、`changelog.rs` `notes`、`version.rs` `snapshot`);纯函数(`parse_args`、`locate_section`、`civil_from_days`)没有这个参数。

### 2.6 分配推到调用方(Push Allocations to the Call Site)

需要拥有所有权时直接收 `String` / `Vec<T>`,不要收 `&str` 再 `to_owned()`;这样调用方已有所有权时零成本,且成本可见。反过来只读就收 `&str` / `&[T]`。

### 2.7 避免为了「通用」而泛型化

`fn f(path: &Path)` 而不是 `fn f(path: impl AsRef<Path>)`;`&mut dyn FnMut()` 而不是把一大段逻辑塞进 `impl FnMut()` 的泛型函数。泛型只在**跨 crate 公开 API 且确有多种调用形态**时使用(C-GENERIC 的适用前提),内部代码一律具体类型——单态化拖慢编译,而编译时间不遵守 80/20。

---

## 3. 构造与默认值(C-CTOR / rust-analyzer "Constructors")

- 无参构造用 `#[derive(Default)]` 或手动 `impl Default`。**二进制 / xtask 内部类型不再写零参 `new()`**(rust-analyzer 规则:一种写法就够);**库 crate 的公开类型两者都提供**且行为一致(C-COMMON-TRAITS:下游习惯 `Foo::new()`,而泛型代码需要 `Default`),`new` 直接 `Self::default()`。
- 有参构造用 `new(..)`;需要校验时返回 `Result<Self>`(见 `error-handling.md` §2)。
- 从别的类型转换用 `From` / `TryFrom` / `FromStr`,不自造与它们语义相同的 `from_xxx` / `parse`(C-CONV-TRAITS)。范例:`xtask/src/version.rs:116` `impl FromStr for Version`。语义**不同**的领域构造器可以保留自己的名字:同文件 `Version::from_tag`(`:88`,先剥 `v` 前缀再解析)是合理的;`Bump::parse`(`:40`)返回 `Option` 而非 `Result`,按本条应改为 `FromStr`,改到时顺手改。
- **不要造「假默认值」**:类型没有合理空值时不要为了 `Default` 塞一个哨兵状态,让调用方决定初始值。
- `Vec::new()` 而不是 `vec![]`;`String::new()` 而不是 `"".to_owned()`。

**函数优于「doer 对象」**:对外提供 `do_thing(a, b)`,不要提供 `ThingDoer::new(a, b).run()`。实现内部可以有 `struct Ctx`,但它是私有细节。范例:`xtask/src/release.rs` 对外只有 `run(root, args)`。

---

## 4. 公开类型必须实现的 trait(C-COMMON-TRAITS / C-DEBUG)

**derive 顺序固定**:`Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default`,serde 的 `Serialize, Deserialize` 放最后。范例 `xtask/src/version.rs:25` `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`。

> 业内没有统一顺序:BurntSushi 系(regex / globset / jiff)按字母序 `Clone, Copy, Debug, Eq, Hash, PartialEq`,clap / toml / cargo_metadata 系 `Debug` 领头。本仓库选 `Debug` 领头的一派与现有 xtask 代码一致;共识部分是语义配对相邻(`Clone, Copy` / `PartialEq, Eq` / `PartialOrd, Ord`)。属性分行:`#[derive(..)]` 一行,`#[non_exhaustive]` 下一行,条件派生 `#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]` 再下一行(模板 `error.rs` 与 clap `ErrorKind` 的写法)。

| trait | 何时必须 |
|---|---|
| `Debug` | **所有**公开类型(C-DEBUG);输出不能为空(C-DEBUG-NONEMPTY)。包含敏感字段(密钥、token)时手写 `Debug` 打码。这是事实标准:regex / jiff / tracing / tokio 都开 `#![warn(missing_debug_implementations)]`,没有 derive 的公开类型(regex `Match`、jiff `Date`、semver `Version`、clap `Arg`)也都手写了 `impl Debug`(见 `references.md` §2) |
| `Clone` | 值语义类型都实现;持有资源句柄的类型谨慎 |
| `Copy` | 只有小的纯数据(≤ 2 个机器字)且未来不会加 `String` 字段时 |
| `PartialEq, Eq` | 能比较就实现;有浮点字段只实现 `PartialEq` |
| `PartialOrd, Ord, Hash` | 要当 `BTreeMap` / `HashMap` 键时 |
| `Default` | 有自然空值时 |
| `Display` | 面向用户展示的类型(错误、版本号、ID);实现 `Display` 不要同时用 `Debug` 派生做展示 |
| `Send + Sync` | 公开类型默认应满足;加一条 `const _: () = { fn assert<T: Send + Sync>() {} let _ = assert::<MyType>; };` 之类的编译期断言防止回退(C-SEND-SYNC) |
| `Serialize / Deserialize` | 用于配置 / 持久化 / 网络的数据结构,放在 `serde` feature 后面(C-SERDE) |

> 当前 `[workspace.lints]` 没有开 `missing_debug_implementations`;新增公开类型时靠自检清单保证。若后续在根 `Cargo.toml` 开启该 lint,本条自动变为编译期约束。

---

## 5. 面向未来的封装(C-STRUCT-PRIVATE / C-NEWTYPE / C-SEALED / non_exhaustive)

- **库 crate 公开结构体字段默认私有**,只有「任意值都合法且未来不会加不变量」的纯数据才 `pub`。私有字段允许后续加字段、改表示不破坏语义化版本。
- **公开枚举(尤其错误枚举)加 `#[non_exhaustive]`**,给自己留加变体的余地;模板 `error.rs` 已如此(`xtask/src/new_crate.rs` `ERROR_RS`)。
- **newtype 区分同类型不同语义**:`struct CrateName(String)`、`struct UserId(u64)`,让 `fn publish(name: CrateName)` 无法误传路径。newtype 也用来隐藏第三方类型(C-NEWTYPE-HIDE),避免依赖升级变成破坏性变更。
- **不希望下游实现的 trait 用 sealed 模式**(私有 supertrait)。
- **只有智能指针才实现 `Deref`**(C-DEREF);不要用 `Deref` 模拟继承。
- **不要在结构体定义上重复派生 trait 的约束**(C-STRUCT-BOUNDS):`struct Foo<T> { .. }` 不写 `where T: Debug`,把约束放在需要它的 `impl` 上。

---

## 6. 控制流与表达式(rust-analyzer 《Style》)

| 规则 | GOOD | BAD |
|---|---|---|
| 早返回 | `if !ok { return None; } Some(..)` | `if ok { Some(..) } else { None }` |
| 抛错用 `return Err(e)` | `return Err(e);` | `Err(e)?;`(类型不受约束,编译器无法标记死代码) |
| `if let` 带 `else` 就用 `match` | `match x { Some(v) => .., None => .. }` | `if let Some(v) = x { .. } else { .. }`(纯布尔条件的 `if a { .. } else { .. }` 不在此列,照常写) |
| 比较用 `<` / `<=` | `lo <= x && x <= hi` | `x >= lo && x <= hi` |
| 不用 `ref` | `Some(v) => v.len()`(match ergonomics) | `Some(ref v) =>` |
| 空分支 | `Ok(_) => (),` | `Ok(_) => {}` |
| 组合子不要硬凑 | 遇到 `?` / `.await` / 分支就换回 `for` / `if` / `match` | `Some(x).filter(\|it\| it.ok())`、`cond.then(..)` 链 |
| 类型标注优先于 turbofish | `let v: Vec<String> = iter.collect();` | `iter.collect::<Vec<_>>()`、`let v: Vec<_> =` |
| 单次使用的 helper 不抽函数 | 用块 `let buf = { ..; buf };` | `fn prepare_buf(..)` 只被调用一次 |
| 局部嵌套函数放函数末尾 | `return go(..); fn go(..) {..}` | 先定义 `fn go` 再调用 |
| 穷尽匹配 | 对自己的枚举 `match` 写全变体 | `_ => {}` 兜底(新变体会被静默吞掉) |

范例:`xtask/src/version.rs` `FromStr` 里的 `match s.split_once('-')`;`xtask/src/main.rs` `run` 的 slice pattern `[sub, tag @ ..] if ...`。

> 现有 xtask 代码并未全部遵守本表:`release.rs:96,101,119,137` 与 `version.rs:198` 用 `=> {}`;`changelog.rs:143-149` 有 `if let ... else`;`release.rs:82-86`、`new_crate.rs:435` 用 `.collect::<Vec<_>>()`;`release.rs:268-274` `bump_and_commit` 有 5 个参数。这些是新代码的规则,改到旧代码时顺手改,不专门刷。

---

## 7. 自检清单

- [ ] 新公开函数的参数是 `&str` / `&[T]` / `&Path` 而非 owned 借用;上下文参数在最前
- [ ] 没有 `bool` / `Option` 参数在所有调用点都是字面量;参数 ≥ 5 个已收成结构体
- [ ] 公开类型 derive 了 `Debug`(以及能实现的 `Clone / PartialEq / Eq`),顺序符合 §4
- [ ] 库 crate 公开结构体字段私有;公开枚举 `#[non_exhaustive]`
- [ ] 零参构造用 `Default`;转换用 `From` / `TryFrom` / `FromStr`
- [ ] 没有 `get_` 前缀 getter;转换方法前缀符合 `as_` / `to_` / `into_`
- [ ] `if let ... else` 已改 `match`;`Err(e)?` 已改 `return Err(e)`
