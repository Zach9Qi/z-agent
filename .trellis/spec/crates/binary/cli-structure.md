# 二进制 crate 结构:`main.rs` 骨架、clap 约定、输出契约与集成测试

> 二进制 crate 是「壳」:解析参数、装配日志、调库、打印。业务逻辑一行都不该出现在这里。
> 规则来源:`cargo xtask new <名字> --bin` 生成模板 `bin_manifest()` / `main_rs()` / `cli_test_rs()`(`xtask/src/new_crate.rs:269-417`)、`../../workspace/rust/index.md`「硬约束速览」第 1 条、`README.md`「错误与日志」、根 `Cargo.toml` `[profile.release]`、`.github/workflows/release.yml:21-24`。

---

## 1. `Cargo.toml`(`new_crate.rs:269-302`)

| 段 | 内容 | 规则 |
|---|---|---|
| `[package]` | `description = "TODO:…"`(`:273`)、`publish = false`(`:275`)、其余 `.workspace = true` | `publish = false` 让 `cargo publish --workspace` 跳过它,也允许对内部 crate 只写 `path`(`deny.toml:46` `allow-wildcard-paths`);`description` 是 `--help` 的 about 文本,必改 |
| `[[bin]]` | `name = "<名字>"`、`path = "src/main.rs"`(`:283-286`) | `name` 是用户敲的命令;改它要同步 `tests/cli.rs` 的 `cargo_bin(..)` 与 `release.yml` 的 `BIN_NAME` |
| `[dependencies]` | `anyhow` / `clap` / `tracing` / `tracing-subscriber` `.workspace = true`(`:288-292`) | 调库时加 `<库名>.workspace = true`;不加 `thiserror` |
| `[dev-dependencies]` | `assert_cmd` / `predicates`(`:294-296`) | 子进程集成测试 |
| `[lints]` | `workspace = true`(`:298-299`) | 不得改成局部表 |

`[profile.release]` 在根 `Cargo.toml:67-71`:`lto = "fat"`、`codegen-units = 1`、`strip = true`、`panic = "abort"`。
**`panic = "abort"` 意味着 release 构建没有栈展开**:不要依赖 `std::panic::catch_unwind`、不要用 `Drop` 做「panic 时也要执行」的清理;本就不该 panic(`unwrap_used` 已开)。

---

## 2. `main.rs` 固定骨架(`new_crate.rs:304-368`)

自上而下顺序**固定**,与 `../../workspace/rust/module-layout.md` §1 一致:

```text
//! `<名字>` 可执行入口:解析参数 → 装配日志 → 执行 → 统一错误出口。(:306-310)
// 正式输出集中在本文件…;文档注释会成为 --help 文本,不能为 rustdoc 加反引号
#![allow(clippy::print_stdout, clippy::print_stderr, clippy::doc_markdown)]   (:312-313)

use std::process::ExitCode;
use clap::Parser;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli { #[arg(short, long)] verbose: bool }                              (:321-327)

fn main() -> ExitCode                 // parse → init_tracing → run → 错误出口   (:329-341)
fn init_tracing(verbose: bool)        // EnvFilter + stderr + with_target(false) (:345-356)
fn run(cli: &Cli) -> anyhow::Result<()>                                        (:358-365)
```

| 函数 | 只做 | 不做 |
|---|---|---|
| `main` | `Cli::parse()` → `init_tracing(cli.verbose)` → `match run(&cli)`;`Err` 时 `eprintln!("错误: {err:#}")` + `ExitCode::FAILURE`(`:337-338`) | 任何业务分支、任何 `?` |
| `init_tracing` | `EnvFilter::try_from_default_env()` 失败才用 `-v ? "debug" : "warn"`(`:349-350`);`.with_writer(std::io::stderr).with_target(false).init()`(`:352-355`) | 读其他环境变量、写文件日志 |
| `run` | 整理参数 → 调库 crate → `println!` 结果(`main.rs` 是唯一允许 print 的文件);每个可失败步骤 `.with_context(|| "做什么")?` | 定义错误类型、循环解析文本、算法 |

模板 `run` 上方有 `#[allow(clippy::unnecessary_wraps)]` 与注释「接入业务逻辑后删掉这行」(`:359-360`)——**第一个 `?` 进入 `run` 时就删**,否则 pedantic 不会再提醒你。

---

## 3. 薄化规则:何时拆文件

`main.rs` 只做参数解析、日志装配、子命令分派、结果打印(`../../workspace/rust/index.md`「硬约束速览」第 1 条)。业务逻辑一律在库 crate,二进制通过 `<库名>.workspace = true` 调用。

触发拆分的信号(满足任一):`run` 超过约 50 行;出现第二个子命令;`Cli` 上的字段超过一屏。拆成:

```text
src/
├── main.rs          # 仍只有:#![allow] / mod / main / init_tracing;不再有 run 的业务分支
├── args.rs          # Cli 结构体 + #[derive(Subcommand)] enum Command + ValueEnum 类型(全是 pub(crate))
└── commands/
    ├── build.rs     # pub(crate) fn run(args: &BuildArgs) -> anyhow::Result<Report>
    └── check.rs
```

- `main.rs` 里 `match cli.command { Command::Build(args) => commands::build::run(&args)?, .. }`,拿到返回值后在 `main.rs` 里 `println!("{report}")`——分派表 + 统一打印就是 `main` 的全部逻辑。
- 每个 handler 收**类型化参数**(`&BuildArgs`,不是 `&Cli` 整体或 `&[String]`),**返回可打印的值**(`String`、实现了 `Display` 的结构体;没有正式输出的 handler 返回 `()`,`main` 不打印),自己不 `println!`。这样 `#![allow(clippy::print_stdout, clippy::print_stderr)]` 始终只在 `main.rs` 一处(`../../workspace/rust/index.md`「硬约束速览」第 3 条:正式输出只允许出现在 `main.rs` 与 xtask);需要边跑边输的长任务,handler 收一个 `&mut dyn std::io::Write` 由 `main.rs` 传入 `stdout().lock()`(内部代码用 `dyn` 而非泛型,见 `../../workspace/rust/naming-and-api-design.md` §2.7)。
- `args.rs` 里不 `use` 任何库 crate 类型做字段——参数类型是终端的事,进 handler 后再转成库类型。
- 二进制 crate 内跨模块共享一律 `pub(crate)`(`../../workspace/rust/module-layout.md` §4)。

---

## 4. clap derive 约定

| 需求 | 写法 | 说明 |
|---|---|---|
| 帮助文本 | 字段 / 结构体的 `///` | 会原样出现在 `--help`,写给终端用户、**不加反引号**;因此顶部 `#![allow(clippy::doc_markdown)]`(`../../workspace/rust/documentation.md` §5)。范例 `:324` |
| 短 / 长 flag | `#[arg(short, long)]` | 模板 `verbose`(`:325`);短名冲突时显式 `short = 'x'` |
| 子命令 | `#[command(subcommand)] command: Command` + `#[derive(Debug, Subcommand)] enum Command { Build(BuildArgs), .. }` | 模板 `Cli` 文档注释已预留说明(`:320`);变体名 UpperCamelCase 自动映射到 kebab-case 子命令 |
| 枚举取值 | `#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)] enum Format { Json, Text }` + `#[arg(long, value_enum)]` | 不用 `String` 再手动匹配 |
| 默认值 | `#[arg(long, default_value_t = 8)]`(类型化)或 `default_value = "text"`(字符串) | 默认值会出现在 `--help`,`///` 里不必重复 |
| 环境变量 | `#[arg(long, env = "MY_APP_TOKEN")]` | **需要 clap 的 `env` feature**,根 `Cargo.toml:35` 目前只开了 `derive`,用前先在根 `[workspace.dependencies]` 的 clap 条目加 `"env"`;只有在 `///` 里写明该变量时才用;敏感值加 `hide_env_values = true` |
| 版本 / 描述 | `#[command(author, version, about, long_about = None)]`(`:322`) | 全部来自 `Cargo.toml`;**禁止**在代码里硬编码版本字符串(`tests/cli.rs` 的 `version_matches_cargo_metadata` 会对比 `CARGO_PKG_VERSION`) |
| 结构体派生 | `#[derive(Debug, Parser)]`(`:321`);子结构体 `#[derive(Debug, Args)]` | `Debug` 便于 `tracing::debug!(?cli)` |

---

## 5. stdout / stderr 契约

| 流 | 内容 | 保证 |
|---|---|---|
| stdout | **仅**正式输出(结果、报表、JSON) | 可安全 `| jq` / `> file`;`println!` 只在 `main.rs`(§3 的统一打印处也在 `main.rs`) |
| stderr | `tracing` 日志 + `错误: …` 一行 | 默认级别 `warn`,成功时 stderr **为空**——`runs_successfully_without_log_noise` 断言 `stderr(predicate::str::is_empty())`(`:386-393`) |

级别优先级:`RUST_LOG` 显式设置 > `-v/--verbose`(debug)> 默认 warn(`:349-350`)。测试里 `.env_remove("RUST_LOG")` 避免继承开发者 shell(`:400`)。
业务代码里的进度 / 诊断一律 `tracing::info!` / `debug!`,不用 `eprintln!`——否则用户无法用 `RUST_LOG=off` 静音。

---

## 6. 退出码

- `main` 返回 `ExitCode`:`SUCCESS`(0)/ `FAILURE`(1)(`:333-338`)。
- 参数错误、`--help`、`--version` 由 clap 自己处理:用法错误退出 **2**,`--help` / `--version` 退出 0,`Cli::parse()` 内部完成,不经过 `run`。
- 需要更多退出码(如「检查发现问题」≠「运行失败」)时:`enum Exit { Ok, Findings, Failure }` + `impl From<Exit> for ExitCode`,`main` 里 `match` 转换;不散落 `ExitCode::from(3)` 魔法数字(`../../workspace/rust/error-handling.md` §3)。

---

## 7. 错误处理

只用 `anyhow`;不定义 thiserror 枚举(二进制没有需要 `match` 的调用方)。每个可失败步骤 `.with_context(|| format!("读取 {path}"))?` / `.context("…")?`,让 `{err:#}` 输出「错误: 读取 x: 权限不足」这样的因果链。库错误经 `?` 自动转 `anyhow::Error`,不 `map_err`。全部规则见 `../../workspace/rust/error-handling.md` §3。

---

## 8. 集成测试 `tests/cli.rs`(`new_crate.rs:370-417`)

模板三条是最低要求(详见 `../../workspace/rust/testing.md` §5):

| 测试 | 位置 | 保证 |
|---|---|---|
| `runs_successfully_without_log_noise` | `:386` | 默认成功路径 stdout 有内容、stderr 为空 |
| `verbose_flag_emits_debug_log_to_stderr` | `:396` | `--verbose` 生效且日志走 stderr |
| `version_matches_cargo_metadata` | `:407` | `--version` 输出含 `CARGO_PKG_VERSION`(版本单一来源) |

新增每个子命令至少补三条:成功(`.success()` + stdout 关键片段)、用户错误(`.code(1)` + stderr 含 `错误: `)、参数校验(缺参 / 未知 flag → `.code(2)`)。模板里 stdout 断言的 `"就绪"` 字样来自 `run` 的占位输出(`:363`),接入真实逻辑后一起改。

**参数定义本身也要一条单元测试**(clap 官方教程 `05_01_assert.rs` 与 typos-cli `args.rs` 都这么做):在 `main.rs` / `args.rs` 末尾

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn cli_definition_is_consistent() {
        use clap::CommandFactory;
        super::Cli::command().debug_assert();
    }
}
```

它在编译期查不出的 clap 配置错误(短名冲突、必填参数带默认值、子命令重名)在 `cargo test` 时立即暴露,而不是第一次跑到那条参数时 panic。

`fn bin()` 用 `Command::cargo_bin("<名字>")`(`:381-383`),字符串必须等于 `[[bin]] name`。重命名可执行文件时三处同步:`Cargo.toml` `[[bin]] name`、`tests/cli.rs` `cargo_bin(..)`、`.github/workflows/release.yml` 顶部 `BIN_NAME`(`PKG_NAME` 是包名,两者要么都填要么都空,`release.yml:40-48`)。`cargo xtask new --bin` 结束时的第 4 条提示即此(`new_crate.rs:149-153`)。

---

## 9. 自检清单

- [ ] `Cargo.toml`:`publish = false`、`[[bin]] name` 与 `cargo_bin(..)` 一致、`description` 已替换 TODO
- [ ] `main.rs` 顶部 `#![allow(clippy::print_stdout, clippy::print_stderr, clippy::doc_markdown)]` 带原因;`unnecessary_wraps` 的 allow 已删
- [ ] `main` 只有 parse / init_tracing / run / 统一打印 / 错误出口;业务逻辑在库 crate,`run` 或 handler 里只有「整理参数 → 调库 → 返回可打印值」;`println!` 只出现在 `main.rs`
- [ ] 第二个子命令出现时已拆 `args.rs` + `commands/<名字>.rs`,handler 收类型化参数、返回可打印值(`String` / 实现 `Display` 的类型),自己不 `println!`
- [ ] clap `///` 无反引号;`version` / `about` 来自 `Cargo.toml`;`env = ".."` 的字段在帮助文本里写明了变量名
- [ ] 成功路径 stderr 为空;日志用 `tracing`,不用 `eprintln!`
- [ ] 每个 `?` 前有 `.context` / `.with_context`
- [ ] `tests/cli.rs` 覆盖每个子命令的成功 / 用户错误 / 参数错误三类;`.env_remove("RUST_LOG")`
- [ ] 有 `Cli::command().debug_assert()` 单元测试
- [ ] 改了 `[[bin]] name` 时 `release.yml` 的 `PKG_NAME` / `BIN_NAME` 已同步
