# xtask 约定:子命令、参数解析、子进程、输出、dry-run 与 CI 同步

> xtask 是「用 Rust 写的仓库脚本」:替代 Makefile / shell,不引入第二种语言与运行时。它的正确性直接决定发版是否安全。
> 规则来源:`xtask/src/*.rs` 既有代码、`xtask/Cargo.toml`、`.cargo/config.toml`、`.github/workflows/ci.yml` / `release.yml`。

---

## 1. 定位与依赖

- 入口别名 `xtask = "run --package xtask --"`(`.cargo/config.toml:3-4`),所以 `cargo xtask <子命令>` 每次都会编译 xtask。根 `Cargo.toml` 目前**没有**设 `default-members`(`crates/` 为空时 glob 匹配不到会报错,见 `Cargo.toml:7-9` 注释),所以根目录 `cargo build` / `cargo test` 也会连 xtask 一起编;有了业务 crate 后可按那段注释加 `default-members = ["crates/*"]` 让日常构建跳过它。
- `publish = false`(`xtask/Cargo.toml:5`);`description` 已填,其余元数据 `.workspace = true`,`[lints] workspace = true`——**与业务 crate 相同的 lint 基线**。
- 依赖**只有** `anyhow` / `serde` / `serde_json`(`xtask/Cargo.toml:13-16`)。`serde_json` 只用来解析 `cargo metadata` 输出(`version.rs:325-336`)。**刻意不用 clap**:xtask 的参数形态简单,手写解析 ≈ 20 行,换来更快的冷编译;不要为 xtask 引入 clap / regex / toml / chrono 之类的依赖(`changelog.rs:163` 自己实现了 civil-from-days 就是为了不引时间库)。
- xtask 是二进制,但**不遵守** `../../crates/binary/cli-structure.md` 的 clap 约定;其余 workspace 规范(模块布局、命名、错误处理、文档、测试)照常适用。

---

## 2. 模块结构

```text
xtask/src/
├── main.rs        # USAGE 常量、repo_root()、main()、run() 分派表;不含任何子命令逻辑
├── new_crate.rs   # 子命令 new(含全部生成模板)
├── ci.rs          # 子命令 ci
├── release.rs     # 子命令 release
├── version.rs     # 领域模块:版本号解析 / 读写 / 一致性校验(含 version check 子命令的 check_cli)
├── changelog.rs   # 领域模块:CHANGELOG 读写(含 changelog notes 子命令的 notes_cli)
└── git.rs         # 领域模块:git 子进程封装
```

| 规则 | 依据 |
|---|---|
| 一个子命令一个文件,对外只暴露 `pub(crate) fn run(root: &Path, args: &[String]) -> anyhow::Result<()>`(或 `xxx_cli`);现有文件写的是 `pub fn`,改到时顺手改成 `pub(crate)`(`../../workspace/rust/module-layout.md` §4) | `main.rs:66-85` 的 `match command.as_str()` 直接转调 |
| 领域模块(`version` / `changelog` / `git`)被多个子命令共享,不含参数解析 | `release.rs:14-16` 同时 `use` 三者 |
| `main.rs` 用 `match` + slice pattern 分派多级子命令 | `main.rs:69-78`:`[sub, tag @ ..] if sub == "check" && tag.len() <= 1` |
| `repo_root()` 取 `CARGO_MANIFEST_DIR` 的上一级 | `main.rs:43-47`;所有路径以它为基准,不依赖 `current_dir` |
| **触碰仓库文件或子进程的函数,第一个参数是 `root: &Path`**;纯函数(`parse_args`、`locate_section`、`civil_from_days`)不需要 | `new_crate.rs:61,107`、`release.rs:75,172`、`git.rs:9,30`、`version.rs:278,293,304` |
| 文件名 / 常量集中定义 | `CARGO_TOML` / `CARGO_LOCK`(`version.rs:19-22`)、`CHANGELOG`(`changelog.rs:17`),其他模块 `use` 它们而不是重写字符串 |

新增子命令 = 新文件 + `main.rs` 加 `mod`、加 `match` 分支、加 `USAGE` 一行(`main.rs:30-40`)。

---

## 3. 参数解析(手写模式)

固定形状(`release.rs:25-58`、`new_crate.rs:18-43`):

```rust
const USAGE: &str = "用法:cargo xtask release <x.y.z | patch | minor | major> [--dry-run] [--no-push]";

fn parse_args(args: &[String]) -> anyhow::Result<Options> {
    let (mut spec, mut dry_run, mut no_push) = (None, false, false);
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
    Ok(Options { spec, dry_run, no_push })
}
```

- 结果放进 `#[derive(Debug, PartialEq, Eq)] struct Options`(`release.rs:29-37`),每个字段有 `///`;两个以内的结果可用元组(`new_crate.rs:28` 返回 `(String, Kind)`)。
- 布尔选择用枚举,不用 `bool`:`enum Kind { Lib, Bin }`(`new_crate.rs:23-26`)。
- 错误文案末尾拼 `\n{USAGE}`,用户不用再敲 `--help`。只有一个可选 flag 时可直接 slice pattern(`ci.rs:74-78`)。
- 值校验紧跟解析:`validate_name` 在 `parse_args` 内调用(`new_crate.rs:41`),后续逻辑信任输入。

---

## 4. 子进程

| 规则 | 写法 | 范例 |
|---|---|---|
| **只传数组参数,不拼 shell 字符串** | `Command::new("git").args(args)` | `git.rs:10-11`、`ci.rs:88-89`、`version.rs:340-341` |
| 只读查询:捕获 stdout | `git::query(root, &["status", "--porcelain"])` → `anyhow::Result<String>`,失败时把 **stderr 首行**放进错误 | `git.rs:8-27` |
| 写操作:直通终端 | `git::run(root, &["commit", "--quiet", "-m", msg])`,stdio 继承,用户看到 git 自己的提示 | `git.rs:29-44` |
| 必须检查 `status.success()` | 非零退出转成带命令名与退出码的错误 | `git.rs:16-23,36-42` |
| `stdin(Stdio::null())` | 防止子进程等待交互输入卡死 | `git.rs:13`、`version.rs:343` |
| `.current_dir(root)` | 不依赖调用者所在目录 | 全部 `Command` 调用 |
| 可选工具 | `Step { optional: true }`,`ErrorKind::NotFound` 时打印「[跳过]」继续 | `ci.rs:12-19,95-102`(typos) |
| cargo 子命令 | `cargo metadata --no-deps --format-version 1`(`version.rs:339-347`)、`cargo update --workspace --offline`(`version.rs:413-431`);加 `--offline` / `--no-deps` 避免意外联网与解析全图 | — |

不要用 `sh -c` / `cmd /C`;需要管道时在 Rust 里处理 `output.stdout`。

---

## 5. 输出与错误

- xtask 与二进制 crate 的 `main.rs` 是仅有的允许 `println!` / `eprintln!` 的地方:`main.rs:15-16` 的 `#![allow(clippy::print_stdout, clippy::print_stderr)]` 带原因注释。
- 进度 / 结果 → `println!`;失败原因 → 由 `main` 统一 `eprintln!("xtask 中止:{err:#}")`(`main.rs:54`)。子命令内部只 `bail!` / `.with_context`,不自己打印错误再返回。
- 计划性输出用 `step(dry_run, msg)`(`release.rs:162-169`):dry-run 时前缀 `[dry-run] 将执行:`,否则 `→ `;不要各处手拼前缀。
- 安全检查等「多项失败要一次全列出」的场景:先收集 `Vec<String>`,统一打印后再 `bail!`(`release.rs:75-144,201-212`;`ci.rs --keep-going` 同理 `ci.rs:106-122`)。
- 文案中文、不带句尾标点、写「做什么 + 对象 + 失败原因」;错误规则见 `../../workspace/rust/error-handling.md` §3。

---

## 6. `--dry-run` 约定

任何**写文件或执行 git 写操作**的子命令必须提供 `--dry-run`(`release.rs`):

- 只读查询照常执行,让检查结果真实(`safety_checks` 在 dry-run 下也跑 `git fetch` / `status`,`release.rs:73-75`);
- 每一步写操作都是 `step(dry_run, ..)` 打印计划 + `if !dry_run { 真正执行 }` 成对出现(`release.rs:294-321,327-339`);
- dry-run 下检查未通过不立即中止,跑完全部计划后再以错误退出(`release.rs:210-212,236-241`),让用户一次看到所有问题;
- 结束语明确「未改动任何文件」(`release.rs:242-251`)。

`new` 没有 `--dry-run`,因为它拒绝覆盖已存在目录(`new_crate.rs:110-113`)且不碰 git。新子命令若同时写文件和跑 git,必须加。

---

## 7. 与 CI 同步

`ci.rs` 的 `STEPS`(`ci.rs:21-70`)必须与 `.github/workflows/ci.yml` 的命令一致(注释 `ci.rs:21`;`../../workspace/rust/index.md`「硬约束速览」第 9 条):

| 步骤 | `ci.rs` | `ci.yml` |
|---|---|---|
| typos | `typos`(optional,`:23-29`) | `crate-ci/typos@v1.50.1`(`:40`) |
| fmt | `cargo fmt --all --check`(`:30-36`) | `:59` |
| clippy | `cargo clippy --workspace --all-targets -- -D warnings`(`:37-50`) | `:92`(加 `--locked`) |
| test | `cargo test --workspace`(`:51-57`) | `:95`(加 `--locked`) |
| doc | `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps --document-private-items`(`:58-69`) | `:62`(加 `--locked`) |

允许的差异只有两处(`ci.rs:3-5`、`ci.yml:3-4`):本地不加 `--locked`(允许边改依赖边跑);typos 本地可选、CI 必跑。CI 独有的 `msrv` / `package` / `deny` job 不进 `STEPS`(需要额外工具链或联网)。改任一侧都要在同一提交里改另一侧。

`release.yml` 依赖 `cargo xtask version check <tag>`(`release.yml:60`)与 `cargo xtask changelog notes <tag>`(`release.yml:64`);改这两个子命令的参数或输出格式要同步 workflow。

---

## 8. 文本解析而非 AST 重写

`version.rs` / `changelog.rs` 对 `Cargo.toml` / `Cargo.lock` / `CHANGELOG.md` 全部做**行级文本定位与切片替换**,不引入 `toml_edit` / Markdown 解析器:

- `locate_section(toml, "[workspace.dependencies]")` 返回段体字节范围(`version.rs:190-205`);`key_value_in_line(line, "version")` 返回引号内范围(`version.rs:231-238`);`write_workspace_version` 收集所有范围后**从后往前** `replace_range`(`version.rs:304-323`)。
- `changelog.rs:7` 注释即规则:「只做行级文本处理,不引入 Markdown 解析器」;`section_body_range`(`changelog.rs:38-47`)按 `## [` 标题与 `[x]:` 链接定义切段。
- `Cargo.lock` 按「`name = "x"` 下一行是 `version = "..."`」相邻行匹配(`version.rs:372-386`)。

**原因**:根 `Cargo.toml` 里的中文注释与 `【新项目必改】` 标记是模板的一部分,AST 重写会丢注释、改格式;同时避免为一个脚本引入解析器依赖。代价是格式假设(段头独占一行、`key = "value"` 单行),测试用 `write_preserves_everything_else` 断言其余文本逐字节不变(`version.rs:608-628`)。新增解析逻辑沿用这套 helper,不要引入解析库。

`Cargo.lock` 内容由 `cargo update --workspace --offline` 生成(`version.rs:413`),脚本只读不写。

---

## 9. 测试

- 单元测试在文件末尾,固定头 `#[cfg(test)] #[allow(clippy::unwrap_used, clippy::expect_used)] mod tests`(`release.rs:343-345`、`new_crate.rs:418-420`)。
- **每个新子命令至少两条 `parse_args` 测试**:成功路径断言整个 `Options`(`release.rs:353-363` `parses_spec_and_flags`),拒绝路径覆盖「缺参 / 未知 flag / 多余参数」(`release.rs:366-370`;`new_crate.rs:434-446` `parses_kind_flag`)。
- 需要文件系统的测试建独立临时目录并清理:`std::env::temp_dir().join(format!("xtask-<模块>-{}", std::process::id()))`(`release.rs:393`、`version.rs:609`、`new_crate.rs:450`;多用例加 label,`changelog.rs:213-218` `temp_repo`),结尾 `remove_dir_all`。
- 涉及 git 的流程只测「在触发 git 之前就被拦下」的分支(`release.rs:391-403` `downgrade_is_rejected_before_any_git_call`);不在单测里跑真实 git。
- 纯函数(`actions_url`、`civil_from_days`、`locate_section`)用表驱动断言精确值。通用规则见 `../../workspace/rust/testing.md`。

---

## 10. CHANGELOG 与规范适用

- 修改 xtask 同样遵守 `../../workspace/rust/` 全部规范(模块布局、命名、错误、文档、测试、依赖)。
- 纯 xtask 内部改动(重构、修 bug、改提示文案)**不记** CHANGELOG(`../../workspace/rust/dependencies-and-changelog.md` §4.1「不记」)。
- 例外:新增 / 删除 `cargo xtask` 子命令、改变子命令参数,且 README「常用开发指令速查」有登记 → 记 `Added` / `Changed`,并同步 `README.md` 表格与 `main.rs` `USAGE`。

---

## 11. 自检清单

- [ ] 新子命令是独立文件,`main.rs` 有 `mod` / `match` 分支 / `USAGE` 行三处同步
- [ ] `parse_args` 按 §3 形状:`match arg.as_str()`、未知 flag 与多余参数都 `bail!` 并附 `USAGE`
- [ ] 没有引入 clap 或其他新依赖;触碰文件 / 子进程的函数首参 `root: &Path`
- [ ] 子进程全部 `Command::new(..).args([..])`,检查 `status.success()`,只读用 `git::query`、写操作用 `git::run`
- [ ] 写文件 / git 写操作的子命令有 `--dry-run`,每步 `step(dry_run, ..)` + `if !dry_run` 成对
- [ ] 改了 `ci.rs` `STEPS` 或 `ci.yml` 命令,另一侧同一提交里已改;`version check` / `changelog notes` 接口变化已同步 `release.yml`
- [ ] 解析 `Cargo.toml` / `CHANGELOG.md` 沿用 `locate_section` / `key_value_in_line` / `section_body_range`,有「其余文本不变」的断言
- [ ] 单测:`parse_args` 成功 + 拒绝;临时目录带 `process::id()` 并清理;不跑真实 git
- [ ] 若新增了用户可见的 `cargo xtask` 子命令:README 速查表、`CHANGELOG.md` `Added` 都已更新
