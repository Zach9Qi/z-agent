//! 本地质量门禁:与 .github/workflows/ci.yml 跑同一组命令,提交前先在本地过一遍。
//!
//! 顺序按「快 → 慢」排列,首个失败即停止并给出原因;`--keep-going` 时全部跑完再汇总。
//! 与 ci.yml 的差别:这里不加 `--locked`,允许本地边改依赖边跑;CI 上锁文件过时会直接失败。
//! typos 拼写检查需要本机安装 typos-cli(`cargo install typos-cli`),未安装时跳过并提示;CI 上总是执行。

use std::path::Path;
use std::process::Command;

use anyhow::{Context, bail};

/// 一条门禁步骤:名字 + 程序 + 参数 + 附加环境变量;`optional` 表示程序不存在时跳过而非失败
struct Step {
    name: &'static str,
    program: &'static str,
    args: &'static [&'static str],
    env: &'static [(&'static str, &'static str)],
    optional: bool,
}

/// 与 ci.yml 保持同步:改这里的命令时同时改 workflow
const STEPS: &[Step] = &[
    Step {
        name: "拼写检查(typos)",
        program: "typos",
        args: &[],
        env: &[],
        optional: true,
    },
    Step {
        name: "格式检查(cargo fmt)",
        program: "cargo",
        args: &["fmt", "--all", "--check"],
        env: &[],
        optional: false,
    },
    Step {
        name: "Clippy(警告视为错误)",
        program: "cargo",
        args: &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
        env: &[],
        optional: false,
    },
    Step {
        name: "单元测试与集成测试(cargo test)",
        program: "cargo",
        args: &["test", "--workspace"],
        env: &[],
        optional: false,
    },
    Step {
        name: "文档构建(rustdoc 警告视为错误)",
        program: "cargo",
        args: &[
            "doc",
            "--workspace",
            "--no-deps",
            "--document-private-items",
        ],
        env: &[("RUSTDOCFLAGS", "-D warnings")],
        optional: false,
    },
];

/// `cargo xtask ci [--keep-going]`
pub fn run(root: &Path, args: &[String]) -> anyhow::Result<()> {
    let keep_going = match args {
        [] => false,
        [flag] if flag == "--keep-going" => true,
        _ => bail!("用法:cargo xtask ci [--keep-going]"),
    };

    let mut failed = Vec::new();
    for step in STEPS {
        println!(
            "\n==> {}\n    {} {}",
            step.name,
            step.program,
            step.args.join(" ")
        );
        let status = match Command::new(step.program)
            .args(step.args)
            .envs(step.env.iter().copied())
            .current_dir(root)
            .status()
        {
            Ok(status) => status,
            Err(err) if step.optional && err.kind() == std::io::ErrorKind::NotFound => {
                println!(
                    "    [跳过] 本机没有 {} 命令(cargo install {}-cli 可安装);CI 上会执行",
                    step.program, step.program
                );
                continue;
            }
            Err(err) => {
                return Err(err).with_context(|| format!("无法启动 {}", step.program));
            }
        };
        if status.success() {
            println!("    [通过] {}", step.name);
        } else {
            println!("    [失败] {}", step.name);
            failed.push(step.name);
            if !keep_going {
                break;
            }
        }
    }

    if failed.is_empty() {
        println!("\n全部门禁通过");
        Ok(())
    } else {
        bail!("门禁未通过:{}", failed.join(",  "));
    }
}
