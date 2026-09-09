# 二进制 crate 规范(`crates/<名字>`,`publish = false`)

> 二进制 crate 只做四件事:解析参数、装配 tracing、分派子命令、打印结果。所有业务逻辑都在库 crate 里,这里每个分支都是「整理参数 → 调库 → 返回结果」,由 `main` 统一打印。

---

## 适用范围

- 由 `cargo xtask new <名字> --bin` 生成、位于 `crates/<名字>/`、`Cargo.toml` 有 `publish = false` 与 `[[bin]]` 的 crate
- 约定由 `xtask/src/new_crate.rs:269-417` 的 `bin_manifest()` / `main_rs()` / `cli_test_rs()` 模板落地
- 可选地由 `.github/workflows/release.yml` 打成五目标安装包(顶部 `PKG_NAME` / `BIN_NAME`,`release.yml:21-24`)
- 库 crate 见 `../library/index.md`;`xtask/` 虽然也是二进制,但遵守 `../../xtask/automation/index.md`(不用 clap)

---

## Pre-Development Checklist

开始改二进制代码之前确认:

- [ ] 读过 `../../workspace/rust/index.md`「硬约束速览」、`README.md`「错误与日志」与本目录 `cli-structure.md`
- [ ] 要写的逻辑是不是业务逻辑?是 → 先去库 crate 实现并测试,二进制只调它
- [ ] 需要调用的库 crate 已在 `[dependencies]` 用 `<库名>.workspace = true` 登记
- [ ] 新增子命令 / flag:确定名字(kebab-case)、短名是否冲突、默认值、是否读环境变量(要在帮助文本里写明)
- [ ] `run` 是否已经超过约 50 行或将出现第二个子命令 → 按 `cli-structure.md` §3 拆 `args.rs` + `commands/`
- [ ] 想清楚新输出走 stdout(正式结果)还是 stderr(日志 / 错误);成功时 stderr 必须为空
- [ ] 是否需要非 0/1 的退出码 → 先定义 `enum` + `From<..> for ExitCode`
- [ ] 改了 `[[bin]] name` → 同步 `tests/cli.rs` `cargo_bin(..)` 与 `release.yml` `PKG_NAME` / `BIN_NAME`
- [ ] 使用者可见的行为变化(新子命令、默认值、输出格式)已在 `CHANGELOG.md` `[Unreleased]` 记一条

---

## Guidelines Index

| 文档 | 内容 | 何时读 |
|---|---|---|
| [cli-structure.md](./cli-structure.md) | `Cargo.toml` / `main.rs` 骨架、拆文件时机与布局、clap derive 约定、stdout/stderr 契约、退出码、`tests/cli.rs` 要求、`release.yml` 同步 | 新建二进制、加子命令、改输出 |
| [../../workspace/rust/module-layout.md](../../workspace/rust/module-layout.md) | 文件内顺序、`use` 分组、二进制里用 `pub(crate)`、`#[allow]` 带原因 | 新建 / 大改文件 |
| [../../workspace/rust/naming-and-api-design.md](../../workspace/rust/naming-and-api-design.md) | 命名、参数类型(`&Path` / `&str`)、用枚举代替 `bool` 参数、`Options` 结构体 | 设计 handler 签名与参数类型 |
| [../../workspace/rust/error-handling.md](../../workspace/rust/error-handling.md) | §3 `anyhow` 用法、context 文案、退出码、子进程失败处理 | 每个 `?` |
| [../../workspace/rust/documentation.md](../../workspace/rust/documentation.md) | §5 `--help` 文本写法(无反引号、给终端用户看) | 写 clap `///` |
| [../../workspace/rust/testing.md](../../workspace/rust/testing.md) | §5 二进制专项:`tests/cli.rs` 三条基线 + 每个子命令补什么 | 补集成测试 |
| [../../workspace/rust/dependencies-and-changelog.md](../../workspace/rust/dependencies-and-changelog.md) | 依赖登记、MSRV、CHANGELOG 写法 | 加依赖、记日志 |
| [../library/crate-anatomy.md](../library/crate-anatomy.md) | 库 crate 的边界——判断「这段逻辑该放哪」 | 拿不准分层时 |

---

## Quality Check

提交前在仓库根执行:

```bash
cargo xtask ci                                    # typos(如已安装)/ fmt / clippy -D warnings / test / doc -D warnings
cargo test -p <名字>                               # 只跑本 crate 的 tests/cli.rs
cargo run -p <名字> -- --help                      # 人眼检查帮助文本:无反引号、描述来自 Cargo.toml description
cargo run -p <名字> -- <子命令> 2>/dev/null         # 成功时 stdout 干净、可管道;再看一次 2>&1 确认 stderr 为空
RUST_LOG=debug cargo run -p <名字> -- <子命令>       # RUST_LOG 覆盖 -v 与默认级别
cargo build --release -p <名字>                    # 验证 panic = "abort" / lto 配置下能编译
grep -n "unnecessary_wraps" crates/<名字>/src/main.rs   # 接入真实逻辑后应为空
```

改了 `[[bin]] name` 时额外核对:

```bash
grep -n "cargo_bin" crates/<名字>/tests/cli.rs
grep -n "BIN_NAME\|PKG_NAME" .github/workflows/release.yml
```

CI 在 Linux / macOS / Windows 三平台跑 `cargo test --workspace --locked`(`.github/workflows/ci.yml:92-95`);`tests/cli.rs` 里的路径与换行断言要跨平台成立。
