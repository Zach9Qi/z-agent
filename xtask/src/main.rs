//! 仓库自动化工具(xtask 模式):用 Rust 写脚本,不引入额外的脚本语言与运行时。
//!
//! 用法(经 .cargo/config.toml 的别名):
//!
//! ```text
//! cargo xtask new <名字> [--bin]                     # 在 crates/ 下生成符合约定的库 / 二进制 crate
//! cargo xtask ci                                    # 本地跑一遍与 CI 一致的门禁
//! cargo xtask version check [vX.Y.Z]                # 校验各处版本号一致,可选与 tag 比对(含更新日志段落)
//! cargo xtask changelog notes <vX.Y.Z>              # 输出该版本的更新日志段落(供 GitHub Release 说明)
//! cargo xtask release <x.y.z | patch | minor | major> [--dry-run] [--no-push]
//! ```
//!
//! 所有子进程调用都传数组参数,不拼 shell 字符串;错误一律带中文说明返回给用户。

// xtask 本身就是终端工具,正式输出直接 print
#![allow(clippy::print_stdout, clippy::print_stderr)]

mod changelog;
mod ci;
mod git;
mod new_crate;
mod release;
mod version;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, bail};

const USAGE: &str = "\
用法:cargo xtask <子命令>

子命令:
  new <名字> [--bin]                  在 crates/ 下生成符合仓库约定的 crate(默认库,--bin 为可执行程序)
  ci                                 本地运行 fmt / clippy / test / doc 门禁(与 CI 一致)
  version check [vX.Y.Z]             校验工作区版本号一致;传入 tag 时同时校验 tag 与版本一致、CHANGELOG 含该段落
  changelog notes <vX.Y.Z>           输出 CHANGELOG.md 中该版本段落的正文(release.yml 用它生成 Release 说明)
  release <x.y.z|patch|minor|major>  发版:安全检查 → 写版本号 → 刷新 Cargo.lock → commit → tag → push
      [--dry-run]                    只打印计划,不写文件、不执行 git 写操作
      [--no-push]                    本地 commit + tag 后停下,不 push";

/// 仓库根目录(xtask/ 的上一级);所有相对路径都以它为基准
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("xtask 中止:{err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> anyhow::Result<()> {
    let root = repo_root();
    let (command, rest) = args
        .split_first()
        .with_context(|| format!("缺少子命令\n{USAGE}"))?;

    match command.as_str() {
        "new" => new_crate::run(&root, rest),
        "ci" => ci::run(&root, rest),
        "version" => match rest {
            [sub, tag @ ..] if sub == "check" && tag.len() <= 1 => {
                version::check_cli(&root, tag.first().map(String::as_str))
            }
            _ => bail!("用法:cargo xtask version check [vX.Y.Z]"),
        },
        "changelog" => match rest {
            [sub, tag] if sub == "notes" => changelog::notes_cli(&root, tag),
            _ => bail!("用法:cargo xtask changelog notes <vX.Y.Z>"),
        },
        "release" => release::run(&root, rest),
        "-h" | "--help" | "help" => {
            println!("{USAGE}");
            Ok(())
        }
        other => bail!("未知子命令 {other}\n{USAGE}"),
    }
}
