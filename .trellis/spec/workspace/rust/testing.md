# 测试

> 测试是唯一不会过期的文档。每个改动都要回答:哪条测试证明它对?哪条测试在它坏掉时会红?
> 来源:`xtask/src/*.rs` 的 `mod tests`、`cargo xtask new` 生成的 `error.rs` / `tests/cli.rs` 模板、rust-analyzer 《Style》(Minimal Tests / `#[should_panic]` / `#[ignore]`)、axum CONTRIBUTING(每个 API 至少一个 doctest)、`.trellis/spec/guides/index.md` 的「同义反复测试」判定。

---

## 1. 三层测试与放置位置

| 层 | 放哪 | 测什么 | 范例 |
|---|---|---|---|
| 单元测试 | 同文件末尾 `#[cfg(test)] mod tests { use super::*; }` | 私有函数、边界条件、错误路径、`Display` 文案 | `xtask/src/version.rs:530+`、`xtask/src/new_crate.rs:418+` |
| 集成测试 | `crates/<name>/tests/*.rs`(每个文件是独立 crate,只能用公开 API) | 库:公开 API 组合使用;二进制:子进程跑可执行文件,校验退出码 / stdout / stderr | 模板 `tests/cli.rs`(`new_crate.rs` `cli_test_rs()`) |
| doctest | `///` / `//!` 里的 `` ``` `` 块 | 公开 API 的最小用法;同时是文档 | 见 `documentation.md` §3 |

**测试模块固定头部**(唯一允许不写原因的 `#[allow]`):

```rust
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    ...
}
```

`tests/*.rs` 集成测试文件顶部同样 `#![allow(clippy::unwrap_used, clippy::expect_used)]` + `//!` 说明这组测试覆盖什么。

---

## 2. 测试命名与结构

- **函数名是一句话陈述被验证的行为**,snake_case,不加 `test_` 前缀:
  `accepts_kebab_case_only`、`rejects_malformed_versions`、`registers_after_leading_comments_and_is_idempotent`、`verbose_flag_emits_debug_log_to_stderr`。看 `cargo test` 输出就知道坏了什么。
- **一个测试一个行为**;同一行为的多个输入用 `for case in [...]` 循环 + 带上下文的断言消息:
  ```rust
  for bad in ["", "Core", "my_core", "-x"] {
      assert!(validate_name(bad).is_err(), "{bad:?} 应当被拒绝");
  }
  ```
  (`new_crate.rs` `accepts_kebab_case_only`)。断言消息用中文说明期望,失败时直接可读。
- **成功路径和失败路径都要有**:`parses_spec_and_flags` + `rejects_missing_unknown_or_extra_args`(`release.rs`)是标准配对。
- **断言精确值,不只断言 `is_ok()`**:`assert_eq!(err.to_string(), "参数错误: 示例")`(模板 `display_is_user_facing_chinese`);比较结构体用 `#[derive(PartialEq, Eq)]` 后整体 `assert_eq!`(`release.rs` 对 `Options`),而不是逐字段。
- **fixture 最小化**:测试用的输入只保留说明问题所需的部分;多行文本用无缩进的原始字符串 `"\` 或 `r"..."`(`changelog.rs` `SAMPLE`)。
- **共享准备逻辑抽成测试模块内的私有 fn**(`args(&[...])`、`temp_repo(label, content)`),不要抽成正式代码里的 `pub` 函数。内部会 `unwrap` / `assert` 的测试辅助函数加 `#[track_caller]`,失败时 panic 位置指向调用它的测试行而不是辅助函数内部(semver `tests/util/mod.rs`、clap 内部都这么做)。
- **测试专用的 impl**(如 `impl quickcheck::Arbitrary`)用 `#[cfg(test)]` 门控放在正文类型旁边,不放进 `mod tests`。
- **同一个模块的测试超过 ~300 行**时外置:`#[cfg(test)] mod tests;` + `src/foo/tests.rs`(indexmap `map/tests.rs`、walkdir `src/tests/` 的做法),不要让测试比实现还长地挤在同一文件。

---

## 3. 禁止 / 避免

| 禁止 | 原因 / 替代 |
|---|---|
| `#[should_panic]` | 库不该 panic;显式断言 `is_err()` 并检查变体或 `Display` |
| `#[ignore]` | 会永久被忘记;测不过就断言当前(错误的)行为并加 `// FIXME(issue):` 说明,修好时测试会红提醒你改回来 |
| 同义反复测试 | 心里把被测功能删掉,测试还能过 → 它没测任何东西(`guides/index.md`「Verifying AI Cross-Review」) |
| 测试依赖执行顺序 / 共享可变全局 | 每个测试自己建临时目录(`temp_dir().join(format!("xtask-x-{}-{label}", process::id()))`)并在结尾清理(`changelog.rs` `temp_repo`) |
| 测试里读真实环境 | 子进程测试 `.env_remove("RUST_LOG")`(模板 `tests/cli.rs`),避免继承开发者 shell 的变量 |
| 在测试里 `println!` 调试后留下 | `cargo test -- --nocapture` 临时看,提交前删;`print_stdout` lint 在测试目标同样生效 |
| 只测 happy path | 每个 `Error` 变体至少被一条测试触发 |
| 用 `sleep` 等待 | 用确定性的信号 / 条件;实在需要时间用注入的时钟 |

---

## 4. 库 crate 专项

- **纯逻辑、无 IO**([index.md](./index.md)「硬约束速览」第 1 条)意味着单元测试不需要临时文件、不需要 mock:输入 `&str` / 结构体,断言输出。做不到说明函数边界划错了(IO 应在二进制层)。
- **公开 API 用集成测试**(`tests/`)从外部视角调一遍,确保 `pub use` re-export 齐全、类型能被下游命名(`Send + Sync` 编译期断言可放这里)。
- **错误文案是 API 的一部分**:每个变体的 `Display` 至少断言一次,改文案 = 改测试,防止悄悄变化。
- **属性测试**(`proptest` 或 `quickcheck`,仓库内选一个)用于解析器 / 序列化往返(`parse(display(x)) == x`),这是 regex / jiff / indexmap 的标配;先加到根 `[workspace.dependencies]`,成员在 `[dev-dependencies]` 里写 `proptest.workspace = true`。
- **`Send + Sync` 编译期断言**:`fn assert_send_sync<T: Send + Sync>() {}` + `assert_send_sync::<Error>();` 放在测试里(walkdir `src/tests/recursive.rs` 模式),防止日后加字段时悄悄丢掉 auto trait。

---

## 5. 二进制 crate 专项(`tests/cli.rs`)

模板生成的三条测试是最低要求:

| 测试 | 保证 |
|---|---|
| `runs_successfully_without_log_noise` | 默认级别 stderr 为空(日志不污染管道) |
| `verbose_flag_emits_debug_log_to_stderr` | `--verbose` 生效且日志走 stderr |
| `version_matches_cargo_metadata` | `--version` 输出含 `CARGO_PKG_VERSION`(版本单一来源) |

新增子命令时补:成功路径(退出码 0 + stdout 关键片段)、每类用户错误(退出码 1 + stderr 含「错误: 」前缀与文案)、参数校验(缺参数 / 未知 flag 时 clap 自己的退出码 2)。

用 `assert_cmd::Command::cargo_bin("<bin-name>")` 定位二进制,`predicates::str::contains` / `is_empty` 断言;`[[bin]] name` 改了要同步这里。

**不要在单元测试里测 `main` / `init_tracing`**——它们只能通过子进程验证。业务逻辑本就应在库 crate 里被单元测试覆盖。

---

## 6. 运行

```bash
cargo test --workspace                       # 全部(单元 + 集成 + doctest)
cargo test -p <crate> <过滤词>               # 单个 crate / 单个测试
cargo test --workspace --doc                 # 只跑 doctest
cargo test -- --nocapture                    # 看输出(临时调试)
cargo xtask ci                               # 提交前门禁(含 test)
```

CI 在 Linux / macOS / Windows 三平台跑 `cargo test --workspace --locked`(`.github/workflows/ci.yml`),路径分隔符、换行符、临时目录位置相关的断言要跨平台成立(用 `Path::join`,不要拼 `/`;比较文本前 `trim_end`)。

---

## 7. 自检清单

- [ ] 新函数有单元测试;新公开 API 有 doctest 或集成测试;新错误变体被触发过
- [ ] 测试名是行为陈述;成功 / 失败路径成对;断言精确值并带中文消息
- [ ] 没有 `#[should_panic]` / `#[ignore]` / 顺序依赖 / 读真实环境变量
- [ ] 临时目录带 `process::id()` 且结尾清理;子进程测试 `env_remove("RUST_LOG")`
- [ ] 在 Windows 路径与 CRLF 下断言仍成立
