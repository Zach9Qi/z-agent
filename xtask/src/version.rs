//! 版本号读写与一致性校验。
//!
//! 版本号只在根 Cargo.toml 的 `[workspace.package]` 维护一处,成员 crate 通过
//! `version.workspace = true` 继承;Cargo.lock 也会记录每个成员的版本。本模块负责:
//! - `read_workspace_version` / `write_workspace_version`:纯文本读写根 Cargo.toml,保留注释与其余内容
//! - `refresh_cargo_lock`:Cargo.toml 改完后刷新 Cargo.lock 里成员 crate 的版本,不下载不编译
//! - `assert_consistent`:校验各成员与工作区版本一致,可选与 tag 比对
//! - `check_cli`:`cargo xtask version check [vX.Y.Z]`,供 release.yml 的 verify job 与本地自检复用

use std::cmp::Ordering;
use std::fmt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::str::FromStr;

use anyhow::{Context, anyhow, bail, ensure};
use serde::Deserialize;

/// 根清单文件名(相对仓库根)
pub const CARGO_TOML: &str = "Cargo.toml";
/// 锁文件名(相对仓库根);由 cargo 刷新,脚本不手写
pub const CARGO_LOCK: &str = "Cargo.lock";

/// 版本递增类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bump {
    /// x.y.Z
    Patch,
    /// x.Y.0
    Minor,
    /// X.0.0
    Major,
}

impl Bump {
    /// 全部递增类型名,供参数解析与用法提示使用
    pub const NAMES: [&'static str; 3] = ["patch", "minor", "major"];

    /// 从命令行单词解析;不是递增类型返回 `None`
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "patch" => Some(Self::Patch),
            "minor" => Some(Self::Minor),
            "major" => Some(Self::Major),
            _ => None,
        }
    }
}

/// 语义化版本:`x.y.z`,可带 `-预发布后缀`(如 `1.2.0-beta.1`)。刻意不实现 build metadata(`+xxx`)。
///
/// 排序规则:先比 `major.minor.patch`;相同时带预发布后缀的小于正式版,两个预发布后缀按字典序比较
/// (不实现 semver 的数字段特殊规则,只用于阻止明显的版本回退)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    /// 主版本
    pub major: u64,
    /// 次版本
    pub minor: u64,
    /// 修订版本
    pub patch: u64,
    /// 预发布后缀(不含 `-`);正式版为 `None`
    pub pre: Option<String>,
}

impl Version {
    /// 是否为预发布版本(带 `-` 后缀)
    pub fn is_prerelease(&self) -> bool {
        self.pre.is_some()
    }

    /// 基于当前版本递增。预发布后缀会被丢弃再递增(0.2.0-beta.1 patch → 0.2.1),
    /// 刻意不实现 npm semver 那套「预发布转正」语义,发布预发布版请直接传完整版本号。
    pub fn bump(&self, kind: Bump) -> Self {
        let (major, minor, patch) = match kind {
            Bump::Major => (self.major + 1, 0, 0),
            Bump::Minor => (self.major, self.minor + 1, 0),
            Bump::Patch => (self.major, self.minor, self.patch + 1),
        };
        Self {
            major,
            minor,
            patch,
            pre: None,
        }
    }

    /// 去掉 tag 的 `v` 前缀解析版本号;格式不合法时报错
    pub fn from_tag(tag: &str) -> anyhow::Result<Self> {
        tag.strip_prefix('v')
            .unwrap_or(tag)
            .parse()
            .with_context(|| format!("tag {tag} 不是合法格式,期望 vX.Y.Z(可带 -预发布后缀)"))
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch)
            .cmp(&(other.major, other.minor, other.patch))
            .then_with(|| match (&self.pre, &other.pre) {
                (None, None) => Ordering::Equal,
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (Some(a), Some(b)) => a.cmp(b),
            })
    }
}

impl FromStr for Version {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = || anyhow!("版本号 {s} 不是合法的 semver(期望 x.y.z,可带 -预发布后缀)");
        let (core, pre) = match s.split_once('-') {
            Some((core, pre)) => (core, Some(pre)),
            None => (s, None),
        };
        let mut parts = core.split('.');
        let mut next_num = || -> anyhow::Result<u64> {
            let part = parts.next().ok_or_else(invalid)?;
            // 拒绝空串与前导零以外的所有非数字;"0" 本身合法
            ensure!(
                !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()),
                invalid()
            );
            ensure!(part == "0" || !part.starts_with('0'), invalid());
            part.parse().map_err(|_| invalid())
        };
        let major = next_num()?;
        let minor = next_num()?;
        let patch = next_num()?;
        ensure!(parts.next().is_none(), invalid());

        if let Some(pre) = pre {
            ensure!(
                !pre.is_empty()
                    && pre
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-'),
                invalid()
            );
        }
        Ok(Self {
            major,
            minor,
            patch,
            pre: pre.map(str::to_owned),
        })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(pre) = &self.pre {
            write!(f, "-{pre}")?;
        }
        Ok(())
    }
}

/// 一次校验得到的快照
#[derive(Debug)]
pub struct Snapshot {
    /// 根 Cargo.toml `[workspace.package].version`
    pub workspace: Version,
    /// 各成员 crate 的 (名字, cargo metadata 报告的版本)
    pub members: Vec<(String, Version)>,
    /// 各成员在 Cargo.lock 中的版本;`None` 表示锁文件里没有该条目
    pub locked: Vec<(String, Option<Version>)>,
    /// 根 Cargo.toml `[workspace.dependencies]` 里带 `path = "crates/..."` 的内部 crate 条目及其 `version`
    /// (发布到 crates.io 的 crate 之间必须写版本号,发版时要与工作区版本同步)
    pub internal_deps: Vec<(String, Version)>,
}

/// 读取仓库内文件(UTF-8);IO 错误带上路径说明
fn read_repo_file(root: &Path, rel: &str) -> anyhow::Result<String> {
    std::fs::read_to_string(root.join(rel)).with_context(|| format!("读取 {rel} 失败"))
}

/// 定位 TOML 文本中指定段(如 `[workspace.package]`),返回段体的字节范围(不含段头行)。
/// 段尾取下一个以 `[` 开头的段头,没有则到文件末尾。
pub fn locate_section(toml: &str, header: &str) -> anyhow::Result<std::ops::Range<usize>> {
    let mut pos = 0;
    let mut start = None;
    for line in toml.split_inclusive('\n') {
        let trimmed = line.trim();
        match start {
            None if trimmed == header => start = Some(pos + line.len()),
            Some(s) if trimmed.starts_with('[') => return Ok(s..pos),
            _ => {}
        }
        pos += line.len();
    }
    start
        .map(|s| s..toml.len())
        .ok_or_else(|| anyhow!("{CARGO_TOML} 缺少 {header} 段"))
}

/// 根 Cargo.toml 的 `[workspace.package]` 段体范围
fn locate_workspace_package(toml: &str) -> anyhow::Result<std::ops::Range<usize>> {
    locate_section(toml, "[workspace.package]")
}

/// 在一段 TOML 文本里找首个 `version = "..."` 行,返回引号内内容的字节范围(相对该段)。
/// `version.workspace = true` 这类点号键不会匹配。
fn locate_version_value(section: &str) -> Option<std::ops::Range<usize>> {
    let mut pos = 0;
    for line in section.split_inclusive('\n') {
        if let Some(range) = version_value_in_line(line) {
            return Some(pos + range.start..pos + range.end);
        }
        pos += line.len();
    }
    None
}

/// 单行内 `version = "x"` 的引号内范围;不是该形式返回 `None`
fn version_value_in_line(line: &str) -> Option<std::ops::Range<usize>> {
    key_value_in_line(line, "version")
}

/// 单行内 `<key> = "x"` 的引号内范围(行首键,允许缩进);不是该形式返回 `None`
fn key_value_in_line(line: &str, key: &str) -> Option<std::ops::Range<usize>> {
    let rest = line.trim_start().strip_prefix(key)?.trim_start();
    let inner = rest.strip_prefix('=')?.trim_start().strip_prefix('"')?;
    let len = inner.find('"')?;
    // inner 是 line 的后缀切片,长度差即为其在 line 中的起点
    let start = line.len() - inner.len();
    Some(start..start + len)
}

/// 内部 crate 依赖行:`<名字> = { path = "crates/<名字>", version = "x.y.z" }`。
/// 返回 (名字, 行内 `version` 值的字节范围);不是内部依赖行或没写 version 返回 `None`
fn internal_dep_in_line(line: &str) -> Option<(String, std::ops::Range<usize>)> {
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') || !trimmed.contains("path = \"crates/") {
        return None;
    }
    let name = trimmed.split_once('=')?.0.trim();
    // 只在内联表 `{ ... }` 内部找 version,避免把名字里的 "version" 当键
    let brace = line.find('{')?;
    let table = &line[brace..];
    let idx = table.find("version")?;
    let range = key_value_in_line(&table[idx..], "version")?;
    let start = brace + idx + range.start;
    Some((name.to_owned(), start..start + range.len()))
}

/// `[workspace.dependencies]` 段里全部内部 crate 条目:(名字, 版本, 版本值的绝对字节范围)
fn internal_deps(toml: &str) -> anyhow::Result<Vec<(String, Version, std::ops::Range<usize>)>> {
    let Ok(section) = locate_section(toml, "[workspace.dependencies]") else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    let mut pos = section.start;
    for line in toml[section].split_inclusive('\n') {
        if let Some((name, range)) = internal_dep_in_line(line) {
            let abs = pos + range.start..pos + range.end;
            let version = toml[abs.clone()]
                .parse()
                .with_context(|| format!("{CARGO_TOML} 中内部依赖 {name} 的 version 无法解析"))?;
            out.push((name, version, abs));
        }
        pos += line.len();
    }
    Ok(out)
}

/// 读取根 Cargo.toml `[workspace.package]` 段的字符串字段(如 `repository`)
pub fn read_workspace_field(root: &Path, key: &str) -> anyhow::Result<String> {
    let toml = read_repo_file(root, CARGO_TOML)?;
    let range = locate_workspace_package(&toml)?;
    let section = &toml[range];
    let mut pos = 0;
    for line in section.split_inclusive('\n') {
        if let Some(r) = key_value_in_line(line, key) {
            return Ok(section[pos + r.start..pos + r.end].to_owned());
        }
        pos += line.len();
    }
    bail!("{CARGO_TOML} 的 [workspace.package] 段找不到 {key} 字段")
}

/// 读取根 Cargo.toml `[workspace.package].version`
pub fn read_workspace_version(root: &Path) -> anyhow::Result<Version> {
    let toml = read_repo_file(root, CARGO_TOML)?;
    let range = locate_workspace_package(&toml)?;
    let section = &toml[range.clone()];
    let value = locate_version_value(section)
        .ok_or_else(|| anyhow!("{CARGO_TOML} 的 [workspace.package] 段找不到 version 字段"))?;
    section[value].parse()
}

/// 把根 Cargo.toml 的 `[workspace.package].version` 与 `[workspace.dependencies]` 里内部 crate 的 `version`
/// 全部改为 `version`,其余文本(含注释)原样保留;不触碰 Cargo.lock
pub fn write_workspace_version(root: &Path, version: &Version) -> anyhow::Result<()> {
    let toml = read_repo_file(root, CARGO_TOML)?;
    let range = locate_workspace_package(&toml)?;
    let value = locate_version_value(&toml[range.clone()])
        .ok_or_else(|| anyhow!("{CARGO_TOML} 的 [workspace.package] 段找不到 version 字段"))?;

    let mut ranges: Vec<std::ops::Range<usize>> = internal_deps(&toml)?
        .into_iter()
        .map(|(_, _, r)| r)
        .collect();
    ranges.push(range.start + value.start..range.start + value.end);
    // 从后往前替换,前面的字节偏移不受影响
    ranges.sort_by_key(|r| std::cmp::Reverse(r.start));
    let mut updated = toml;
    for r in ranges {
        updated.replace_range(r, &version.to_string());
    }
    std::fs::write(root.join(CARGO_TOML), updated)
        .with_context(|| format!("写入 {CARGO_TOML} 失败"))
}

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<MetadataPackage>,
    workspace_members: Vec<String>,
}

#[derive(Deserialize)]
struct MetadataPackage {
    id: String,
    name: String,
    version: String,
}

/// 通过 `cargo metadata --no-deps` 列出工作区全部成员的 (名字, 版本);不解析依赖,也不会改写 Cargo.lock
pub fn workspace_members(root: &Path) -> anyhow::Result<Vec<(String, Version)>> {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(root)
        .stdin(Stdio::null())
        .output()
        .context("无法启动 cargo metadata")?;
    if !output.status.success() {
        bail!(
            "cargo metadata 失败:{}",
            String::from_utf8_lossy(&output.stderr)
                .lines()
                .next()
                .unwrap_or("(无输出)")
        );
    }
    let metadata: Metadata =
        serde_json::from_slice(&output.stdout).context("解析 cargo metadata 输出失败")?;
    metadata
        .packages
        .into_iter()
        .filter(|p| metadata.workspace_members.contains(&p.id))
        .map(|p| {
            let version = p
                .version
                .parse()
                .with_context(|| format!("成员 {} 的版本 {} 无法解析", p.name, p.version))?;
            Ok((p.name, version))
        })
        .collect()
}

/// 读取 Cargo.lock 中指定 crate 的版本;lock 里 name 与 version 相邻成行,直接按行匹配
fn locked_version(lock: &str, crate_name: &str) -> Option<Version> {
    let needle = format!("name = \"{crate_name}\"");
    // 不用 let-chain:它自 1.88 才稳定,而工作区声明的 MSRV 是 1.85
    let mut lines = lock.lines().map(str::trim_end);
    while let Some(line) = lines.next() {
        if line != needle {
            continue;
        }
        let value = lines
            .next()
            .and_then(|next| next.strip_prefix("version = \""))
            .and_then(|s| s.strip_suffix('"'));
        return value.and_then(|v| v.parse().ok());
    }
    None
}

/// 读取工作区版本、各成员版本与 Cargo.lock 中的记录
pub fn snapshot(root: &Path) -> anyhow::Result<Snapshot> {
    let workspace = read_workspace_version(root)?;
    let members = workspace_members(root)?;
    let lock = read_repo_file(root, CARGO_LOCK)?;
    let locked = members
        .iter()
        .map(|(name, _)| (name.clone(), locked_version(&lock, name)))
        .collect();
    let internal_deps = internal_deps(&read_repo_file(root, CARGO_TOML)?)?
        .into_iter()
        .map(|(name, version, _)| (name, version))
        .collect();
    Ok(Snapshot {
        workspace,
        members,
        locked,
        internal_deps,
    })
}

/// 刷新 Cargo.lock 中成员 crate 的版本条目。
/// 用 `cargo update --workspace --offline`:只重新解析工作区成员,不下载、不编译,也不会顺带升级依赖。
/// 失败时返回带原因的错误:release.yml 以 `--locked` 编译,带着过时的 Cargo.lock 发布必定在 CI 失败,不如就地中止。
pub fn refresh_cargo_lock(root: &Path) -> anyhow::Result<()> {
    let output = Command::new("cargo")
        .args(["update", "--workspace", "--offline"])
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .output()
        .with_context(|| format!("无法启动 cargo 刷新 {CARGO_LOCK}"))?;
    if !output.status.success() {
        bail!(
            "刷新 {CARGO_LOCK} 失败:{}",
            String::from_utf8_lossy(&output.stderr)
                .lines()
                .next()
                .unwrap_or("(无输出)")
        );
    }
    Ok(())
}

/// 校验各成员版本与工作区版本一致;传入 tag(vX.Y.Z 或 X.Y.Z)时同时校验 tag 与版本一致。
/// 通过返回快照,不通过返回带中文说明的错误。
/// Cargo.lock 不在这里判定,由 [`check_cli`] 根据场景决定是警告还是错误。
pub fn assert_consistent(root: &Path, tag: Option<&str>) -> anyhow::Result<Snapshot> {
    let snap = snapshot(root)?;
    let mismatched: Vec<String> = snap
        .members
        .iter()
        .filter(|(_, v)| *v != snap.workspace)
        .map(|(name, v)| format!("{name}={v}"))
        .collect();
    if !mismatched.is_empty() {
        bail!(
            "成员 crate 版本与工作区版本 {} 不一致:{}。请确认成员 Cargo.toml 使用 `version.workspace = true`",
            snap.workspace,
            mismatched.join(",")
        );
    }
    let stale_deps: Vec<String> = snap
        .internal_deps
        .iter()
        .filter(|(_, v)| *v != snap.workspace)
        .map(|(name, v)| format!("{name}={v}"))
        .collect();
    if !stale_deps.is_empty() {
        bail!(
            "{CARGO_TOML} [workspace.dependencies] 里内部 crate 的 version 与工作区版本 {} 不一致:{}。请运行 cargo xtask release 同步,或手动改成一致",
            snap.workspace,
            stale_deps.join(",")
        );
    }
    if let Some(tag) = tag {
        let expected = Version::from_tag(tag)?;
        if expected != snap.workspace {
            bail!(
                "tag {tag} 与代码版本 {} 不一致。请先运行 cargo xtask release {expected} 同步版本号,再推送 tag",
                snap.workspace
            );
        }
        // 发版必须有对应的更新日志段落,Release 说明从这里生成
        crate::changelog::notes(root, &expected)
            .with_context(|| format!("tag {tag} 缺少更新日志"))?;
    }
    Ok(snap)
}

/// `cargo xtask version check [tag]` 的实现:打印结果,不一致时返回错误。
///
/// Cargo.lock 与代码版本不一致时:本地自检(无 tag)只给警告,cargo build 会自动修正;
/// 带 tag(release.yml 的 verify job)时视为错误,因为后续 build 以 `--locked` 编译必定失败,不如在这里就地拦下。
pub fn check_cli(root: &Path, tag: Option<&str>) -> anyhow::Result<()> {
    let snap = assert_consistent(root, tag).context("版本校验失败")?;
    let names: Vec<&str> = snap.members.iter().map(|(n, _)| n.as_str()).collect();
    println!(
        "版本一致:{}({CARGO_TOML} [workspace.package] / {})",
        snap.workspace,
        names.join(" / ")
    );
    if !snap.internal_deps.is_empty() {
        let deps: Vec<&str> = snap.internal_deps.iter().map(|(n, _)| n.as_str()).collect();
        println!(
            "  [workspace.dependencies] 内部 crate 版本一致:{}",
            deps.join(" / ")
        );
    }
    if let Some(tag) = tag {
        println!(
            "tag {tag} 与代码版本一致,{} 含对应段落",
            crate::changelog::CHANGELOG
        );
    }
    let stale: Vec<String> = snap
        .locked
        .iter()
        .filter(|(_, v)| v.as_ref() != Some(&snap.workspace))
        .map(|(name, v)| {
            format!(
                "{name}={}",
                v.as_ref()
                    .map_or_else(|| "(未找到)".to_owned(), ToString::to_string)
            )
        })
        .collect();
    if !stale.is_empty() {
        let detail = format!(
            "{CARGO_LOCK} 中的版本与代码版本 {} 不一致({}),请执行 cargo update --workspace 后一并提交",
            snap.workspace,
            stale.join(",")
        );
        if tag.is_some() {
            bail!("{detail};release.yml 以 --locked 编译,这个 tag 无法发布");
        }
        eprintln!("警告:{detail}");
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_displays_semver() {
        let v: Version = "1.2.3".parse().unwrap();
        assert_eq!(
            (v.major, v.minor, v.patch, v.pre.as_deref()),
            (1, 2, 3, None)
        );
        assert_eq!(v.to_string(), "1.2.3");

        let pre: Version = "0.2.0-beta.1".parse().unwrap();
        assert_eq!(pre.pre.as_deref(), Some("beta.1"));
        assert!(pre.is_prerelease());
        assert_eq!(pre.to_string(), "0.2.0-beta.1");
    }

    #[test]
    fn rejects_malformed_versions() {
        for bad in ["1.2", "1.2.3.4", "01.2.3", "a.b.c", "1.2.3-", "1.2.3-β", ""] {
            assert!(bad.parse::<Version>().is_err(), "{bad:?} 应当被拒绝");
        }
    }

    #[test]
    fn orders_by_core_then_prerelease() {
        let parse = |s: &str| s.parse::<Version>().unwrap();
        assert!(parse("0.1.0") < parse("0.2.0"));
        assert!(parse("0.2.0") < parse("1.0.0"));
        assert!(parse("1.0.0-beta.1") < parse("1.0.0"));
        assert!(parse("1.0.0-alpha") < parse("1.0.0-beta"));
        assert_eq!(parse("1.0.0"), parse("1.0.0"));
    }

    #[test]
    fn bump_drops_prerelease() {
        let v: Version = "0.2.0-beta.1".parse().unwrap();
        assert_eq!(v.bump(Bump::Patch).to_string(), "0.2.1");
        assert_eq!(v.bump(Bump::Minor).to_string(), "0.3.0");
        assert_eq!(v.bump(Bump::Major).to_string(), "1.0.0");
    }

    #[test]
    fn from_tag_strips_v_prefix() {
        assert_eq!(Version::from_tag("v1.0.0").unwrap().to_string(), "1.0.0");
        assert_eq!(Version::from_tag("1.0.0").unwrap().to_string(), "1.0.0");
        assert!(Version::from_tag("release-1").is_err());
    }

    const SAMPLE: &str = "\
[workspace]
members = [\"crates/*\"]

[workspace.package]
# 注释要保留
version = \"0.1.0\"
edition = \"2024\"

[workspace.dependencies]
# 内部 crate
my-core = { path = \"crates/my-core\", version = \"0.1.0\" }
serde = { version = \"1\", features = [\"derive\"] }
";

    #[test]
    fn locates_workspace_package_version_only() {
        let range = locate_workspace_package(SAMPLE).unwrap();
        let section = &SAMPLE[range];
        assert!(section.contains("# 注释要保留"));
        assert!(!section.contains("[workspace.dependencies]"));
        let value = locate_version_value(section).unwrap();
        assert_eq!(&section[value], "0.1.0");
    }

    #[test]
    fn write_preserves_everything_else() {
        let dir = std::env::temp_dir().join(format!("xtask-version-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(CARGO_TOML), SAMPLE).unwrap();

        write_workspace_version(&dir, &"0.2.0".parse().unwrap()).unwrap();
        let updated = std::fs::read_to_string(dir.join(CARGO_TOML)).unwrap();
        // 工作区版本与内部依赖的 version 都被改写,serde 的 version = "1" 不受影响
        assert_eq!(
            updated,
            SAMPLE.replace("version = \"0.1.0\"", "version = \"0.2.0\"")
        );
        assert!(updated.contains("serde = { version = \"1\""));
        assert_eq!(read_workspace_version(&dir).unwrap().to_string(), "0.2.0");
        let deps = internal_deps(&updated).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "my-core");
        assert_eq!(deps[0].1.to_string(), "0.2.0");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn internal_dep_line_parsing() {
        let line = "my-core = { path = \"crates/my-core\", version = \"0.1.0\" }";
        let (name, range) = internal_dep_in_line(line).unwrap();
        assert_eq!(name, "my-core");
        assert_eq!(&line[range], "0.1.0");
        // 没写 version 的 path 依赖、第三方依赖、注释都不算
        assert!(internal_dep_in_line("my-bin = { path = \"crates/my-bin\" }").is_none());
        assert!(internal_dep_in_line("serde = { version = \"1\" }").is_none());
        assert!(internal_dep_in_line("# x = { path = \"crates/x\", version = \"1\" }").is_none());
    }

    #[test]
    fn reads_workspace_field() {
        let dir = std::env::temp_dir().join(format!("xtask-field-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(CARGO_TOML),
            "[workspace.package]\nversion = \"0.1.0\"\nrepository = \"https://github.com/o/r\"\n",
        )
        .unwrap();
        assert_eq!(
            read_workspace_field(&dir, "repository").unwrap(),
            "https://github.com/o/r"
        );
        assert!(read_workspace_field(&dir, "homepage").is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn reads_locked_version_by_adjacent_lines() {
        let lock = "\
[[package]]
name = \"anyhow\"
version = \"1.0.0\"

[[package]]
name = \"my-core\"
version = \"0.1.0\"
";
        assert_eq!(
            locked_version(lock, "my-core").unwrap().to_string(),
            "0.1.0"
        );
        assert!(locked_version(lock, "missing").is_none());
    }
}
