//! 一键发布:安全检查 → 写版本号 → 刷新 Cargo.lock → commit → tag → push。
//!
//! - 安全检查全部通过才会动文件:工作区干净、位于 main、与 origin/main 同步、tag 不存在
//! - 版本号写根 Cargo.toml 的 `[workspace.package]` 与 `[workspace.dependencies]` 内部 crate 条目,随后刷新 Cargo.lock
//! - CHANGELOG.md 的 `[Unreleased]` 必须有条目,发版时切成 `## [x.y.z] - 日期` 段落(Release 说明由此生成)
//! - `--dry-run`:只读的 git 查询照常执行(让检查结果真实),所有写文件 / git 写操作改为打印计划
//! - `--no-push`:本地 commit + tag 后停下,由用户自行 push
//! - push 之后由 .github/workflows/release.yml 接手跨平台打包并创建 GitHub Release

use std::path::Path;

use anyhow::{Context, bail};

use crate::changelog::{self, CHANGELOG};
use crate::git;
use crate::version::{self, Bump, CARGO_LOCK, CARGO_TOML, Version};

/// 发布时受控的分支与远端;脚本只在这条链路上工作
const RELEASE_BRANCH: &str = "main";
const REMOTE: &str = "origin";

/// 版本提交只允许包含这三个文件,避免把工作区里其他改动一起带进发布提交
const RELEASE_FILES: [&str; 3] = [CARGO_TOML, CARGO_LOCK, CHANGELOG];

const USAGE: &str =
    "用法:cargo xtask release <x.y.z | patch | minor | major> [--dry-run] [--no-push]";

/// 命令行参数解析结果
#[derive(Debug, PartialEq, Eq)]
struct Options {
    /// 目标版本:具体 semver(可带 v 前缀)或递增类型 patch / minor / major
    spec: String,
    /// 只打印计划,不写文件、不执行 git 写操作
    dry_run: bool,
    /// 本地 commit + tag 后停下,不 push
    no_push: bool,
}

fn parse_args(args: &[String]) -> anyhow::Result<Options> {
    let mut spec = None;
    let mut dry_run = false;
    let mut no_push = false;
    for arg in args {
        match arg.as_str() {
            "--dry-run" => dry_run = true,
            "--no-push" => no_push = true,
            flag if flag.starts_with('-') => bail!("未知参数 {flag}\n{USAGE}"),
            value if spec.is_none() => spec = Some(value.to_owned()),
            extra => bail!("多余的参数 {extra}\n{USAGE}"),
        }
    }
    let spec = spec.with_context(|| format!("缺少目标版本\n{USAGE}"))?;
    Ok(Options {
        spec,
        dry_run,
        no_push,
    })
}

/// 把 spec(具体版本或递增类型)解析成目标版本号
fn resolve_target(spec: &str, current: &Version) -> anyhow::Result<Version> {
    if let Some(kind) = Bump::parse(spec) {
        return Ok(current.bump(kind));
    }
    Version::from_tag(spec).with_context(|| {
        format!(
            "目标版本 {spec} 无效:请传 x.y.z(可带 -预发布后缀)或 {}",
            Bump::NAMES.join(" / ")
        )
    })
}

/// 发布前安全检查,返回全部未通过项的中文说明(空表示通过)。
/// 只做只读查询(git fetch 只更新远端跟踪分支,不动工作区),dry-run 下也照常执行。
fn safety_checks(root: &Path, tag: &str) -> Vec<String> {
    let mut problems = Vec::new();

    match git::query(root, &["status", "--porcelain"]) {
        Ok(dirty) if !dirty.is_empty() => {
            let lines: Vec<&str> = dirty.lines().collect();
            let preview = lines
                .iter()
                .take(10)
                .copied()
                .collect::<Vec<_>>()
                .join("\n    ");
            let more = if lines.len() > 10 {
                format!("\n    ……共 {} 项", lines.len())
            } else {
                String::new()
            };
            problems.push(format!(
                "工作区有未提交改动(含未跟踪文件),请先提交或 stash:\n    {preview}{more}"
            ));
        }
        Ok(_) => {}
        Err(err) => problems.push(format!("无法读取工作区状态:{err:#}")),
    }

    match git::query(root, &["branch", "--show-current"]) {
        Ok(branch) if branch == RELEASE_BRANCH => {}
        Ok(branch) => problems.push(format!(
            "请在 {RELEASE_BRANCH} 分支发布(当前:{})",
            if branch.is_empty() {
                "detached HEAD"
            } else {
                &branch
            }
        )),
        Err(err) => problems.push(format!("无法读取当前分支:{err:#}")),
    }

    let sync = git::query(root, &["fetch", "--quiet", REMOTE, RELEASE_BRANCH]).and_then(|_| {
        let local = git::query(root, &["rev-parse", "HEAD"])?;
        let remote = git::query(root, &["rev-parse", &format!("{REMOTE}/{RELEASE_BRANCH}")])?;
        Ok((local, remote))
    });
    match sync {
        Ok((local, remote)) if local == remote => {}
        Ok((local, remote)) => problems.push(format!(
            "本地与远端不同步,请先 pull / push(本地 {},{REMOTE}/{RELEASE_BRANCH} {})",
            &local[..7.min(local.len())],
            &remote[..7.min(remote.len())]
        )),
        Err(err) => problems.push(format!("无法从远端 {REMOTE} 获取 {RELEASE_BRANCH}:{err:#}")),
    }

    match git::query(root, &["tag", "--list", tag]) {
        Ok(existing) if !existing.is_empty() => problems.push(format!("tag {tag} 已存在(本地)")),
        Ok(_) => match git::query(
            root,
            &["ls-remote", "--tags", REMOTE, &format!("refs/tags/{tag}")],
        ) {
            Ok(existing) if !existing.is_empty() => {
                problems.push(format!("tag {tag} 已存在(远端 {REMOTE})"));
            }
            Ok(_) => {}
            Err(err) => problems.push(format!("无法查询远端 {REMOTE} 的 tag:{err:#}")),
        },
        Err(err) => problems.push(format!("无法查询本地 tag:{err:#}")),
    }

    problems
}

/// 从 origin 的 URL 推出 GitHub Actions 页面地址;不是 GitHub 仓库时返回 `None`
fn actions_url(remote_url: &str) -> Option<String> {
    let rest = remote_url.trim();
    let idx = rest.find("github.com")?;
    let path = rest[idx + "github.com".len()..].trim_start_matches([':', '/']);
    let path = path
        .strip_suffix(".git")
        .unwrap_or(path)
        .trim_end_matches('/');
    let (owner, repo) = path.split_once('/')?;
    if owner.is_empty() || repo.is_empty() || repo.contains('/') {
        return None;
    }
    Some(format!("https://github.com/{owner}/{repo}/actions"))
}

/// 打印一条计划 / 执行日志;dry-run 下统一带「[dry-run] 将执行」前缀
fn step(dry_run: bool, message: &str) {
    if dry_run {
        println!("[dry-run] 将执行:{message}");
    } else {
        println!("→ {message}");
    }
}

/// `cargo xtask release` 主流程
pub fn run(root: &Path, args: &[String]) -> anyhow::Result<()> {
    let Options {
        spec,
        dry_run,
        no_push,
    } = parse_args(args)?;

    let current = version::read_workspace_version(root)?;
    let target = resolve_target(&spec, &current)?;
    // 拒绝版本回退:目标必须 >= 当前(等于时走下方「仅打 tag」分支)
    if target < current {
        bail!("目标版本 {target} 低于当前版本 {current},拒绝发布回退版本");
    }
    let tag = format!("v{target}");
    let commit_message = format!("chore(release): {tag}");

    println!(
        "发布 {tag}(当前 {current}){}",
        if dry_run {
            " —— dry-run,不会改动任何文件"
        } else {
            ""
        }
    );
    if target.is_prerelease() {
        println!("  版本带预发布后缀,GitHub Release 会自动标记为 prerelease");
    }

    println!("安全检查:");
    let problems = safety_checks(root, &tag);
    if problems.is_empty() {
        println!(
            "  [通过] 工作区干净 / 位于 {RELEASE_BRANCH} / 与 {REMOTE}/{RELEASE_BRANCH} 同步 / tag {tag} 不存在"
        );
    } else {
        for problem in &problems {
            eprintln!("  [失败] {problem}");
        }
        if !dry_run {
            bail!("安全检查未通过,未做任何改动");
        }
    }

    // 更新日志:已有该版本段落(手动切过)则直接用;否则 Unreleased 必须有条目,发版时切段
    let changelog_ready = changelog::has_section(root, &target)?;
    if !changelog_ready {
        changelog::assert_unreleased_has_entries(root).context("发版前必须先写更新日志")?;
    }

    if changelog_ready && version_already_bumped(root, &current, &target)? {
        // 例如手动改过版本号与日志只差打 tag:不再制造空提交
        println!("  版本号与 {CHANGELOG} 已全部为 {target},跳过写文件与 commit,仅打 tag");
    } else {
        bump_and_commit(root, &target, &commit_message, dry_run, !changelog_ready)?;
    }
    tag_and_push(root, &tag, no_push, dry_run)?;

    if let Some(url) = git::query(root, &["remote", "get-url", REMOTE])
        .ok()
        .and_then(|u| actions_url(&u))
    {
        println!("推送 tag 后 GitHub Actions 会自动打包并创建 Release,进度见:{url}");
    }

    if dry_run && !problems.is_empty() {
        bail!(
            "dry-run 结束:有 {} 项安全检查未通过,实际执行会被拒绝",
            problems.len()
        );
    }
    println!(
        "{}",
        if dry_run {
            "dry-run 结束,未改动任何文件".to_owned()
        } else {
            format!("发布 {tag} 完成")
        }
    );
    Ok(())
}

/// 根 Cargo.toml、全部成员与 Cargo.lock 是否已经全是目标版本
fn version_already_bumped(
    root: &Path,
    current: &Version,
    target: &Version,
) -> anyhow::Result<bool> {
    if current != target {
        return Ok(false);
    }
    let snap = version::snapshot(root)?;
    Ok(snap.members.iter().all(|(_, v)| v == target)
        && snap.locked.iter().all(|(_, v)| v.as_ref() == Some(target)))
}

/// 写版本号 → 刷新 Cargo.lock → 切更新日志 → 只提交这三个文件
fn bump_and_commit(
    root: &Path,
    target: &Version,
    commit_message: &str,
    dry_run: bool,
    cut_changelog: bool,
) -> anyhow::Result<()> {
    step(
        dry_run,
        &format!(
            "写入版本号 {target} → {CARGO_TOML} [workspace.package] 与 [workspace.dependencies] 内部 crate"
        ),
    );
    step(
        dry_run,
        &format!("刷新 {CARGO_LOCK}(cargo update --workspace --offline)"),
    );
    if cut_changelog {
        step(
            dry_run,
            &format!(
                "{CHANGELOG}:把 [Unreleased] 条目切到 ## [{target}] - {}",
                changelog::today_utc()
            ),
        );
    }
    if !dry_run {
        version::write_workspace_version(root, target)?;
        // Cargo.toml 已改、锁文件没跟上时不能继续提交:告诉用户如何回滚后中止
        version::refresh_cargo_lock(root).with_context(|| {
            format!("已中止,未提交任何内容;可用 git checkout -- {CARGO_TOML} 恢复版本号")
        })?;
        if cut_changelog {
            let repo = version::read_workspace_field(root, "repository")?;
            changelog::cut_release(root, target, &repo, &changelog::today_utc()).with_context(
                || {
                    format!(
                        "已中止,未提交任何内容;可用 git checkout -- {CARGO_TOML} {CARGO_LOCK} 回滚"
                    )
                },
            )?;
        }
    }

    step(dry_run, &format!("git add -- {}", RELEASE_FILES.join(" ")));
    step(dry_run, &format!("git commit -m \"{commit_message}\""));
    if !dry_run {
        let mut add = vec!["add", "--"];
        add.extend(RELEASE_FILES);
        git::run(root, &add)?;
        git::run(root, &["commit", "--quiet", "-m", commit_message])?;
    }
    Ok(())
}

/// 打附注 tag,除 `--no-push` 外随即推送分支与 tag
fn tag_and_push(root: &Path, tag: &str, no_push: bool, dry_run: bool) -> anyhow::Result<()> {
    // 用附注 tag:`git push --follow-tags` 只会带上附注 tag,轻量 tag 会被落下
    step(dry_run, &format!("git tag -a {tag} -m \"{tag}\""));
    if !dry_run {
        git::run(root, &["tag", "-a", tag, "-m", tag])?;
    }

    let push_command = format!("git push --follow-tags {REMOTE} {RELEASE_BRANCH}");
    if no_push {
        println!("已指定 --no-push,跳过推送;确认无误后手动执行:{push_command}");
        return Ok(());
    }
    step(dry_run, &push_command);
    if !dry_run {
        git::run(root, &["push", "--follow-tags", REMOTE, RELEASE_BRANCH])?;
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn parses_spec_and_flags() {
        let opts = parse_args(&args(&["patch", "--dry-run", "--no-push"])).unwrap();
        assert_eq!(
            opts,
            Options {
                spec: "patch".into(),
                dry_run: true,
                no_push: true
            }
        );
    }

    #[test]
    fn rejects_missing_unknown_or_extra_args() {
        assert!(parse_args(&args(&[])).is_err());
        assert!(parse_args(&args(&["1.0.0", "--force"])).is_err());
        assert!(parse_args(&args(&["1.0.0", "2.0.0"])).is_err());
    }

    #[test]
    fn resolves_bump_or_explicit_version() {
        let current: Version = "0.1.0".parse().unwrap();
        assert_eq!(
            resolve_target("minor", &current).unwrap().to_string(),
            "0.2.0"
        );
        assert_eq!(
            resolve_target("v1.0.0", &current).unwrap().to_string(),
            "1.0.0"
        );
        assert_eq!(
            resolve_target("1.0.0-rc.1", &current).unwrap().to_string(),
            "1.0.0-rc.1"
        );
        assert!(resolve_target("latest", &current).is_err());
    }

    #[test]
    fn downgrade_is_rejected_before_any_git_call() {
        // 用一个只有 Cargo.toml 的临时目录:如果回退检查没拦下,后续 git 查询会报另一种错误
        let dir = std::env::temp_dir().join(format!("xtask-release-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(CARGO_TOML),
            "[workspace.package]\nversion = \"0.2.0\"\n",
        )
        .unwrap();

        let err = run(&dir, &args(&["0.1.0", "--dry-run"])).unwrap_err();
        assert!(err.to_string().contains("拒绝发布回退版本"), "{err:#}");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn derives_actions_url_from_remote() {
        let expected = Some("https://github.com/foo/bar/actions".to_owned());
        assert_eq!(actions_url("git@github.com:foo/bar.git"), expected);
        assert_eq!(actions_url("https://github.com/foo/bar.git\n"), expected);
        assert_eq!(actions_url("https://github.com/foo/bar"), expected);
        assert_eq!(actions_url("https://gitlab.com/foo/bar.git"), None);
    }
}
