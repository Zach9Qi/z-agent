//! git 子进程封装:只读查询返回 stdout,写操作直通终端。

use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, bail};

/// 执行只读 git 查询,返回去掉末尾换行的 stdout(保留 `status --porcelain` 的首列空格)。
pub fn query(root: &Path, args: &[&str]) -> anyhow::Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("无法启动 git {}", args.join(" ")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "git {} 失败:{}",
            args.join(" "),
            stderr.lines().next().unwrap_or("(无输出)")
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_owned())
}

/// 执行会改动仓库的 git 命令;输出直通终端让用户看到 git 自己的提示。
pub fn run(root: &Path, args: &[&str]) -> anyhow::Result<()> {
    let status = Command::new("git")
        .args(args)
        .current_dir(root)
        .status()
        .with_context(|| format!("无法启动 git {}", args.join(" ")))?;
    if !status.success() {
        bail!(
            "git {} 执行失败(退出码 {:?})",
            args.join(" "),
            status.code()
        );
    }
    Ok(())
}
