//! CHANGELOG.md(Keep a Changelog 格式)的读写。
//!
//! - 发版前:`[Unreleased]` 段必须有条目,否则拒绝发版(库的每个版本都该有说明)
//! - 发版时:把 `[Unreleased]` 下的条目搬到 `## [x.y.z] - 日期` 新段落,并维护底部的比较链接
//! - 发版后:`cargo xtask changelog notes vX.Y.Z` 输出该版本段落正文,作为 GitHub Release 的说明
//!
//! 只做行级文本处理,不引入 Markdown 解析器;日期取 UTC 当天。

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, bail, ensure};

use crate::version::Version;

/// 更新日志文件名(相对仓库根)
pub const CHANGELOG: &str = "CHANGELOG.md";
const UNRELEASED_HEADING: &str = "## [Unreleased]";

fn read(root: &Path) -> anyhow::Result<String> {
    std::fs::read_to_string(root.join(CHANGELOG)).with_context(|| format!("读取 {CHANGELOG} 失败"))
}

/// 是否为 `## [...]` 版本段标题行
fn is_version_heading(line: &str) -> bool {
    line.starts_with("## [")
}

/// 是否为底部的链接定义行,如 `[0.1.0]: https://...`
fn is_link_definition(line: &str) -> bool {
    line.starts_with('[')
        && line
            .split_once("]:")
            .is_some_and(|(label, _)| !label[1..].contains('['))
}

/// 返回标题为 `heading_prefix` 的段落正文行范围(标题行之后,到下一个段标题 / 链接定义为止)
fn section_body_range(lines: &[&str], heading_prefix: &str) -> Option<std::ops::Range<usize>> {
    let start = lines
        .iter()
        .position(|l| l.trim_end().starts_with(heading_prefix))?;
    let end = lines[start + 1..]
        .iter()
        .position(|l| is_version_heading(l) || is_link_definition(l))
        .map_or(lines.len(), |i| start + 1 + i);
    Some(start + 1..end)
}

/// 去掉首尾空行后的段落正文
fn trimmed_body(lines: &[&str]) -> Vec<String> {
    let first = lines.iter().position(|l| !l.trim().is_empty());
    let last = lines.iter().rposition(|l| !l.trim().is_empty());
    match (first, last) {
        (Some(f), Some(l)) => lines[f..=l].iter().map(|s| (*s).to_owned()).collect(),
        _ => Vec::new(),
    }
}

/// 正文里是否至少有一条列表项(`- ` / `* `)
fn has_entries(body: &[String]) -> bool {
    body.iter()
        .any(|l| l.trim_start().starts_with("- ") || l.trim_start().starts_with("* "))
}

/// 指定版本段落的正文(不含标题,已去首尾空行);没有该段落时报错
pub fn notes(root: &Path, version: &Version) -> anyhow::Result<String> {
    let text = read(root)?;
    let lines: Vec<&str> = text.lines().collect();
    let range = section_body_range(&lines, &format!("## [{version}]"))
        .with_context(|| format!("{CHANGELOG} 中没有 `## [{version}]` 段落"))?;
    let body = trimmed_body(&lines[range]);
    ensure!(
        has_entries(&body),
        "{CHANGELOG} 的 `## [{version}]` 段落没有任何条目"
    );
    Ok(body.join("\n"))
}

/// 该版本段落是否已存在(手动切过日志时 release 不再重复切)
pub fn has_section(root: &Path, version: &Version) -> anyhow::Result<bool> {
    let text = read(root)?;
    Ok(text
        .lines()
        .any(|l| l.trim_end().starts_with(&format!("## [{version}]"))))
}

/// 校验 `[Unreleased]` 段至少有一条条目;不满足时给出可操作的中文说明
pub fn assert_unreleased_has_entries(root: &Path) -> anyhow::Result<()> {
    let text = read(root)?;
    let lines: Vec<&str> = text.lines().collect();
    let range = section_body_range(&lines, UNRELEASED_HEADING)
        .with_context(|| format!("{CHANGELOG} 缺少 `{UNRELEASED_HEADING}` 段"))?;
    let body = trimmed_body(&lines[range]);
    if !has_entries(&body) {
        bail!(
            "{CHANGELOG} 的 `{UNRELEASED_HEADING}` 下没有任何条目;请先在 Added / Changed / Fixed 等小节写明本次发布的变更"
        );
    }
    Ok(())
}

/// 把 `[Unreleased]` 的条目搬到 `## [version] - date` 段落,并更新底部链接:
/// `[Unreleased]` 指向 `compare/v<version>...HEAD`,新版本指向与上一版本的比较(首个版本指向 tag 页)。
pub fn cut_release(
    root: &Path,
    version: &Version,
    repo_url: &str,
    date: &str,
) -> anyhow::Result<()> {
    let text = read(root)?;
    let lines: Vec<&str> = text.lines().collect();
    let range = section_body_range(&lines, UNRELEASED_HEADING)
        .with_context(|| format!("{CHANGELOG} 缺少 `{UNRELEASED_HEADING}` 段"))?;
    let body = trimmed_body(&lines[range.clone()]);
    ensure!(
        has_entries(&body),
        "{CHANGELOG} 的 `{UNRELEASED_HEADING}` 下没有任何条目,无法切出 {version}"
    );

    let repo = repo_url.trim_end_matches('/').trim_end_matches(".git");
    let mut out: Vec<String> = Vec::with_capacity(lines.len() + 6);
    out.extend(lines[..range.start].iter().map(|s| (*s).to_owned()));
    out.push(String::new());
    out.push(format!("## [{version}] - {date}"));
    out.push(String::new());
    out.extend(body);
    out.push(String::new());
    out.extend(lines[range.end..].iter().map(|s| (*s).to_owned()));

    // 链接定义:找 `[Unreleased]:` 行,重写并在其后插入新版本的比较链接
    let unreleased_link = format!("[Unreleased]: {repo}/compare/v{version}...HEAD");
    let previous = out
        .iter()
        .filter(|l| is_link_definition(l) && !l.starts_with("[Unreleased]:"))
        .find_map(|l| l[1..].split_once(']').map(|(label, _)| label.to_owned()));
    let version_link = match previous {
        Some(prev) => format!("[{version}]: {repo}/compare/v{prev}...v{version}"),
        None => format!("[{version}]: {repo}/releases/tag/v{version}"),
    };
    if let Some(i) = out.iter().position(|l| l.starts_with("[Unreleased]:")) {
        out[i] = unreleased_link;
        out.insert(i + 1, version_link);
    } else {
        out.push(String::new());
        out.push(unreleased_link);
        out.push(version_link);
    }

    let mut updated = out.join("\n");
    updated.push('\n');
    std::fs::write(root.join(CHANGELOG), updated).with_context(|| format!("写入 {CHANGELOG} 失败"))
}

/// UTC 当天日期 `YYYY-MM-DD`
pub fn today_utc() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    civil_from_days(i64::try_from(secs / 86_400).unwrap_or(0))
}

/// 1970-01-01 起的天数 → `YYYY-MM-DD`(Howard Hinnant 的 civil-from-days 算法,不依赖时间库)
fn civil_from_days(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

/// `cargo xtask changelog notes <vX.Y.Z>`:打印该版本段落正文,供 release.yml 生成 Release 说明
pub fn notes_cli(root: &Path, tag: &str) -> anyhow::Result<()> {
    let version = Version::from_tag(tag)?;
    println!("{}", notes(root, &version)?);
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
# 更新日志

说明文字。

## [Unreleased]

### Added

- 新功能 A

### Fixed

- 修复 B

## [0.1.0] - 2026-01-01

### Added

- 初始版本

[Unreleased]: https://github.com/o/r/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/o/r/releases/tag/v0.1.0
";

    fn temp_repo(label: &str, content: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("xtask-changelog-{}-{label}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(CHANGELOG), content).unwrap();
        dir
    }

    #[test]
    fn extracts_notes_for_released_version() {
        let dir = temp_repo("notes", SAMPLE);
        let v: Version = "0.1.0".parse().unwrap();
        assert_eq!(notes(&dir, &v).unwrap(), "### Added\n\n- 初始版本");
        assert!(notes(&dir, &"9.9.9".parse().unwrap()).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn cut_release_moves_entries_and_rewrites_links() {
        let dir = temp_repo("cut", SAMPLE);
        let v: Version = "0.2.0".parse().unwrap();
        cut_release(&dir, &v, "https://github.com/o/r.git", "2026-02-02").unwrap();
        let text = std::fs::read_to_string(dir.join(CHANGELOG)).unwrap();
        assert_eq!(
            text,
            "\
# 更新日志

说明文字。

## [Unreleased]

## [0.2.0] - 2026-02-02

### Added

- 新功能 A

### Fixed

- 修复 B

## [0.1.0] - 2026-01-01

### Added

- 初始版本

[Unreleased]: https://github.com/o/r/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/o/r/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/o/r/releases/tag/v0.1.0
"
        );
        // 切完之后 Unreleased 为空,再切必须被拒绝;新段落可被 notes 读到
        assert!(assert_unreleased_has_entries(&dir).is_err());
        assert!(has_section(&dir, &v).unwrap());
        assert_eq!(
            notes(&dir, &v).unwrap(),
            "### Added\n\n- 新功能 A\n\n### Fixed\n\n- 修复 B"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn first_release_links_to_tag_page() {
        let dir = temp_repo(
            "first",
            "# 更新日志\n\n## [Unreleased]\n\n### Added\n\n- 初始化\n\n[Unreleased]: https://github.com/o/r/commits/main\n",
        );
        let v: Version = "0.1.0".parse().unwrap();
        cut_release(&dir, &v, "https://github.com/o/r", "2026-03-03").unwrap();
        let text = std::fs::read_to_string(dir.join(CHANGELOG)).unwrap();
        assert!(text.contains("[Unreleased]: https://github.com/o/r/compare/v0.1.0...HEAD\n[0.1.0]: https://github.com/o/r/releases/tag/v0.1.0\n"));
        assert!(
            text.contains("## [Unreleased]\n\n## [0.1.0] - 2026-03-03\n\n### Added\n\n- 初始化\n")
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn empty_unreleased_is_rejected() {
        let dir = temp_repo(
            "empty",
            "# 更新日志\n\n## [Unreleased]\n\n### Added\n\n## [0.1.0] - 2026-01-01\n\n- x\n",
        );
        assert!(assert_unreleased_has_entries(&dir).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn civil_date_algorithm_matches_known_dates() {
        assert_eq!(civil_from_days(0), "1970-01-01");
        assert_eq!(civil_from_days(19_723), "2024-01-01");
        assert_eq!(civil_from_days(19_782), "2024-02-29");
        assert_eq!(civil_from_days(20_089), "2025-01-01");
        assert_eq!(today_utc().len(), 10);
    }
}
