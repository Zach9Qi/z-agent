//! `cargo xtask new <名字> [--bin]`:在 crates/ 下生成一个符合本仓库约定的 crate。
//!
//! 模板仓库初始状态 crates/ 为空,约定(继承工作区元数据与 lint、统一错误类型、tracing 日志、
//! 二进制集成测试)通过这里的生成模板落地,而不是靠示例目录。
//!
//! - 库 crate(默认):面向 crates.io 发布——`src/lib.rs` + `src/error.rs`(thiserror 错误枚举 + `Result` 别名)
//!   + 自己的 `README.md`(crates.io 页面)+ docs.rs 元数据,并以 `{ path, version }` 形式登记到根 Cargo.toml 的
//!     `[workspace.dependencies]`(发布到 crates.io 的 crate 之间必须带版本号),其他成员直接 `<名字>.workspace = true`
//! - 二进制 crate(`--bin`):`src/main.rs`(clap + tracing 装配 + 统一错误出口)+ `tests/cli.rs`,
//!   `publish = false`;可执行文件名与 crate 名相同

use std::path::Path;

use anyhow::{Context, bail, ensure};

use crate::version::{self, CARGO_TOML};

const USAGE: &str =
    "用法:cargo xtask new <名字> [--bin]\n  名字用小写字母、数字与连字符,如 my-core / my-cli";

/// 生成的 crate 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Lib,
    Bin,
}

fn parse_args(args: &[String]) -> anyhow::Result<(String, Kind)> {
    let mut name = None;
    let mut kind = Kind::Lib;
    for arg in args {
        match arg.as_str() {
            "--bin" => kind = Kind::Bin,
            "--lib" => kind = Kind::Lib,
            flag if flag.starts_with('-') => bail!("未知参数 {flag}\n{USAGE}"),
            value if name.is_none() => name = Some(value.to_owned()),
            extra => bail!("多余的参数 {extra}\n{USAGE}"),
        }
    }
    let name = name.with_context(|| format!("缺少 crate 名字\n{USAGE}"))?;
    validate_name(&name)?;
    Ok((name, kind))
}

/// crate 名只允许 kebab-case:小写字母开头,其后小写字母 / 数字 / 连字符,不能以连字符结尾
fn validate_name(name: &str) -> anyhow::Result<()> {
    let mut chars = name.chars();
    let valid_first = chars.next().is_some_and(|c| c.is_ascii_lowercase());
    let valid_rest = name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    ensure!(
        valid_first && valid_rest && !name.ends_with('-') && !name.contains("--"),
        "crate 名 {name} 不合法\n{USAGE}"
    );
    Ok(())
}

/// 在 `[workspace.dependencies]` 段头(及紧随其后的注释行)之后插入
/// `<名字> = { path = "crates/<名字>", version = "<工作区版本>" }`;version 由 `cargo xtask release` 同步
fn register_workspace_dependency(root: &Path, name: &str, version: &str) -> anyhow::Result<()> {
    let path = root.join(CARGO_TOML);
    let toml = std::fs::read_to_string(&path).with_context(|| format!("读取 {CARGO_TOML} 失败"))?;
    let section = version::locate_section(&toml, "[workspace.dependencies]")?;
    let body = &toml[section.clone()];

    let entry_prefix = format!("{name} =");
    if body
        .lines()
        .any(|l| l.trim_start().starts_with(&entry_prefix))
    {
        println!("  {CARGO_TOML} 的 [workspace.dependencies] 已有 {name},跳过登记");
        return Ok(());
    }

    // 跳过段头后的连续注释行(空行即停),让新条目排在「内部 crate」注释之下、第三方依赖之前
    let mut insert_at = section.start;
    for line in body.split_inclusive('\n') {
        if line.trim_start().starts_with('#') {
            insert_at += line.len();
        } else {
            break;
        }
    }
    let entry = format!("{name} = {{ path = \"crates/{name}\", version = \"{version}\" }}\n");
    let updated = format!("{}{}{}", &toml[..insert_at], entry, &toml[insert_at..]);
    std::fs::write(&path, updated).with_context(|| format!("写入 {CARGO_TOML} 失败"))?;
    println!(
        "  已登记到 {CARGO_TOML} [workspace.dependencies]:{}",
        entry.trim_end()
    );
    Ok(())
}

fn write_file(root: &Path, rel: &str, content: &str) -> anyhow::Result<()> {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("创建目录 {} 失败", parent.display()))?;
    }
    std::fs::write(&path, content).with_context(|| format!("写入 {rel} 失败"))?;
    println!("  创建 {rel}");
    Ok(())
}

/// `cargo xtask new` 主流程
pub fn run(root: &Path, args: &[String]) -> anyhow::Result<()> {
    let (name, kind) = parse_args(args)?;
    let dir = format!("crates/{name}");
    ensure!(
        !root.join(&dir).exists(),
        "{dir} 已存在,不会覆盖;换个名字或先删除该目录"
    );

    println!(
        "生成{} crate {name} → {dir}",
        match kind {
            Kind::Lib => "库",
            Kind::Bin => "二进制",
        }
    );
    match kind {
        Kind::Lib => {
            write_file(root, &format!("{dir}/Cargo.toml"), &lib_manifest(&name))?;
            write_file(root, &format!("{dir}/src/lib.rs"), &lib_rs(&name))?;
            write_file(root, &format!("{dir}/src/error.rs"), ERROR_RS)?;
            write_file(root, &format!("{dir}/README.md"), &lib_readme(&name))?;
            let ws_version = version::read_workspace_version(root)?.to_string();
            register_workspace_dependency(root, &name, &ws_version)?;
        }
        Kind::Bin => {
            write_file(root, &format!("{dir}/Cargo.toml"), &bin_manifest(&name))?;
            write_file(root, &format!("{dir}/src/main.rs"), &main_rs(&name))?;
            write_file(root, &format!("{dir}/tests/cli.rs"), &cli_test_rs(&name))?;
        }
    }

    println!("\n接下来:");
    match kind {
        Kind::Lib => println!(
            "  1. 编辑 {dir}/Cargo.toml 里的 description / keywords / categories,以及 {dir}/README.md(都会显示在 crates.io)"
        ),
        Kind::Bin => println!("  1. 编辑 {dir}/Cargo.toml 里的 description"),
    }
    println!(
        "  2. cargo build -p {name}   # 同时把新成员写进 Cargo.lock(CI 以 --locked 运行,记得一并提交)"
    );
    println!("  3. cargo xtask ci          # 确认 fmt / clippy / test / doc 全部通过");
    if kind == Kind::Bin {
        println!(
            "  4. 要走 release.yml 自动打包发版,把 .github/workflows/release.yml 顶部的 PKG_NAME / BIN_NAME 都填为 {name}"
        );
    }
    Ok(())
}

fn lib_manifest(name: &str) -> String {
    format!(
        r#"[package]
name = "{name}"
# crates.io 必填,会显示在搜索结果与 crate 页面顶部
description = "TODO:一句话说明这个 crate 负责什么"
# 【发布前必改】crates.io 搜索关键词(最多 5 个)与分类(slug 见 https://crates.io/category_slugs),留空不影响发布
keywords = []
categories = []
readme = "README.md"
documentation = "https://docs.rs/{name}"
version.workspace = true
authors.workspace = true
repository.workspace = true
license.workspace = true
edition.workspace = true
rust-version.workspace = true
# 只把源码、清单与 README 打进 crate 包,测试夹具 / 基准等大文件不上传
include = ["src/**/*", "Cargo.toml", "README.md"]

[dependencies]
thiserror.workspace = true
tracing.workspace = true

[lints]
workspace = true

# docs.rs 构建时开启全部 feature,让每个可选 API 都出现在文档里
[package.metadata.docs.rs]
all-features = true
"#
    )
}

fn lib_readme(name: &str) -> String {
    format!(
        r"# {name}

TODO:一句话说明这个 crate 负责什么。这个文件会显示在 crates.io 的 crate 页面。

## 安装

```bash
cargo add {name}
```

## 用法

```rust
// TODO:一个最小可运行示例
```

## 最低支持的 Rust 版本 (MSRV)

见工作区根 `Cargo.toml` 的 `rust-version`;MSRV 提升视为次版本变更并写入 CHANGELOG。

## 许可证

MIT
"
    )
}

fn lib_rs(name: &str) -> String {
    format!(
        r"//! `{name}`:TODO 一句话说明这个 crate 负责什么。
//!
//! 设计约束:
//! - 不做 IO、不读环境变量、不打印;输入输出全部通过函数参数与返回值,方便单测与多前端复用
//! - 可失败的公开函数一律返回 [`Result<T>`],错误类型是 [`Error`]
//! - 日志只用 `tracing` 宏,由调用方(二进制 crate)决定订阅器与输出级别
//!
//! 新增领域模块时按「一个领域一个文件」组织,在此处 `pub mod` 并 re-export 常用类型。

mod error;

pub use error::{{Error, Result}};
"
    )
}

const ERROR_RS: &str = r#"//! 本 crate 的统一错误类型。新增变体时保持中文文案口径,文案面向终端用户直接展示。

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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn display_is_user_facing_chinese() {
        let err = Error::InvalidInput("示例".into());
        assert_eq!(err.to_string(), "参数错误: 示例");
    }
}
"#;

fn bin_manifest(name: &str) -> String {
    format!(
        r#"[package]
name = "{name}"
description = "TODO:一句话说明这个可执行程序做什么"
# 可执行程序通过 GitHub Release 分发,不发布到 crates.io;同时允许对工作区内 crate 使用不带版本号的 path 依赖(见 deny.toml)
publish = false
version.workspace = true
authors.workspace = true
repository.workspace = true
license.workspace = true
edition.workspace = true
rust-version.workspace = true

[[bin]]
# 用户实际敲的命令名;改它时同步 tests/cli.rs 的 cargo_bin(...) 与 release.yml 的 BIN_NAME
name = "{name}"
path = "src/main.rs"

[dependencies]
anyhow.workspace = true
clap.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true

[dev-dependencies]
assert_cmd.workspace = true
predicates.workspace = true

[lints]
workspace = true
"#
    )
}

fn main_rs(name: &str) -> String {
    format!(
        r#"//! `{name}` 可执行入口:解析参数 → 装配日志 → 执行 → 统一错误出口。
//!
//! 这里是整个二进制里唯一允许 `println!` / `eprintln!` 的地方(正式输出与错误提示);
//! 诊断信息一律走 `tracing`,由 `--verbose` / `RUST_LOG` 控制级别。
//! 业务逻辑放到库 crate(`cargo xtask new <名字>`)里,这里只做参数整理与结果打印。

// 正式输出集中在本文件,单独放开 print 系列 lint;文档注释会成为 --help 文本,不能为 rustdoc 加反引号
#![allow(clippy::print_stdout, clippy::print_stderr, clippy::doc_markdown)]

use std::process::ExitCode;

use clap::Parser;

/// 命令行参数。`version` / `about` 从 Cargo.toml 自动读取;
/// 需要子命令时加 `#[command(subcommand)] command: Command` 并定义 `#[derive(Subcommand)] enum Command`。
#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {{
    /// 输出 debug 级日志到 stderr(等价于 RUST_LOG=debug;显式设置 RUST_LOG 时以后者为准)
    #[arg(short, long)]
    verbose: bool,
}}

fn main() -> ExitCode {{
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    match run(&cli) {{
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {{
            // `{{err:#}}` 把 anyhow 的 context 链平铺成一行,用户看到「做什么: 为什么」
            eprintln!("错误: {{err:#}}");
            ExitCode::FAILURE
        }}
    }}
}}

/// 装配 tracing 订阅器:`RUST_LOG` 优先;未设置时 `-v` 开 debug,否则只输出 warn 及以上。
/// 日志写到 stderr,不污染 stdout 的正式输出,方便管道使用。
fn init_tracing(verbose: bool) {{
    use tracing_subscriber::EnvFilter;

    let default_level = if verbose {{ "debug" }} else {{ "warn" }};
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_target(false)
        .init();
}}

/// 程序主体:整理参数 → 调用库 crate → 打印结果。可失败步骤用 `.context("做什么")?` 补充说明。
// 骨架里还没有可失败的步骤,先放行 unnecessary_wraps;接入业务逻辑后删掉这行
#[allow(clippy::unnecessary_wraps)]
fn run(cli: &Cli) -> anyhow::Result<()> {{
    tracing::debug!(verbose = cli.verbose, "启动");
    println!("{name} 就绪:在 src/main.rs 的 run 里接入业务逻辑");
    Ok(())
}}
"#
    )
}

fn cli_test_rs(name: &str) -> String {
    format!(
        r#"//! 二进制集成测试:以子进程方式运行编译出的可执行文件,校验退出码与输出。
//! 与单元测试的区别:这里覆盖参数解析、日志装配、错误出口整条链路。

#![allow(clippy::unwrap_used, clippy::expect_used)]

use assert_cmd::Command;
use predicates::prelude::*;

/// 定位当前 crate 编译出的二进制(名字来自 Cargo.toml 的 [[bin]])
fn bin() -> Command {{
    Command::cargo_bin("{name}").expect("应当能找到 {name} 可执行文件")
}}

#[test]
fn runs_successfully_without_log_noise() {{
    bin()
        .env_remove("RUST_LOG")
        .assert()
        .success()
        .stdout(predicate::str::contains("就绪"))
        .stderr(predicate::str::is_empty());
}}

#[test]
fn verbose_flag_emits_debug_log_to_stderr() {{
    bin()
        .arg("--verbose")
        // 测试进程可能继承开发者环境里的 RUST_LOG,清掉保证 -v 生效
        .env_remove("RUST_LOG")
        .assert()
        .success()
        .stderr(predicate::str::contains("启动"));
}}

#[test]
fn version_matches_cargo_metadata() {{
    bin()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}}
"#
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn accepts_kebab_case_only() {
        for ok in ["core", "my-core", "app2", "a-b-c"] {
            assert!(validate_name(ok).is_ok(), "{ok} 应当合法");
        }
        for bad in ["", "Core", "my_core", "-x", "x-", "a--b", "1abc", "中文"] {
            assert!(validate_name(bad).is_err(), "{bad:?} 应当被拒绝");
        }
    }

    #[test]
    fn parses_kind_flag() {
        let args = |l: &[&str]| l.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        assert_eq!(
            parse_args(&args(&["x"])).unwrap(),
            ("x".to_owned(), Kind::Lib)
        );
        assert_eq!(
            parse_args(&args(&["x", "--bin"])).unwrap(),
            ("x".to_owned(), Kind::Bin)
        );
        assert!(parse_args(&args(&[])).is_err());
        assert!(parse_args(&args(&["x", "y"])).is_err());
    }

    #[test]
    fn registers_after_leading_comments_and_is_idempotent() {
        let dir = std::env::temp_dir().join(format!("xtask-new-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(CARGO_TOML),
            "[workspace.dependencies]\n# 内部 crate\nserde = \"1\"\n\n[profile.release]\nlto = true\n",
        )
        .unwrap();

        register_workspace_dependency(&dir, "my-core", "0.1.0").unwrap();
        register_workspace_dependency(&dir, "my-core", "0.1.0").unwrap();
        let toml = std::fs::read_to_string(dir.join(CARGO_TOML)).unwrap();
        assert_eq!(
            toml,
            "[workspace.dependencies]\n# 内部 crate\nmy-core = { path = \"crates/my-core\", version = \"0.1.0\" }\nserde = \"1\"\n\n[profile.release]\nlto = true\n"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
