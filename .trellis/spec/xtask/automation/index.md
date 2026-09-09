# xtask 规范(`xtask/`,仓库自动化,不发布)

> xtask 是仓库的脚本层:`cargo xtask new / ci / version check / changelog notes / release`。它替代 Makefile 与 shell 脚本,只依赖 cargo 与 git 子进程,写出来的每一行都直接影响新 crate 的形状与发版安全。

---

## 适用范围

- `xtask/` 目录下的全部代码(`xtask/Cargo.toml` `publish = false`),通过 `.cargo/config.toml:3-4` 的别名 `cargo xtask <子命令>` 调用
- 涉及 `.github/workflows/ci.yml` / `release.yml` 中调用 xtask 的步骤时,workflow 的同步修改也在本规范范围内
- 业务 crate 见 `../../crates/library/index.md` / `../../crates/binary/index.md`;xtask 虽是二进制,但**不用 clap**,不套用二进制 crate 的 `main.rs` 骨架

---

## Pre-Development Checklist

改 xtask 之前确认:

- [ ] 读过 `../../workspace/rust/index.md`「硬约束速览」与本目录 `conventions.md`
- [ ] 读过要改的模块以及它的 `mod tests`;`xtask/src/main.rs:1-13` 的 `//!` 是全部子命令的总览
- [ ] 新子命令:确定放哪个新文件、`USAGE` 文案、`parse_args` 接受哪些位置参数与 `--flag`
- [ ] 会写文件或跑 git 写操作吗 → 必须有 `--dry-run`,且只读检查在 dry-run 下照常执行
- [ ] 不引入新依赖(尤其 clap / toml / regex / chrono);确需引入先在根 `Cargo.toml` `[workspace.dependencies]` 登记并说明为什么手写不可行
- [ ] 改 `ci.rs` `STEPS` → 同一提交改 `.github/workflows/ci.yml`;改 `version check` / `changelog notes` 的参数或输出 → 同步 `release.yml:60,64`
- [ ] 改 `new_crate.rs` 的模板字符串 → 同步 `../../crates/library/crate-anatomy.md` / `../../crates/binary/cli-structure.md` 里引用的形状与行号,以及 `README.md`「仓库形态」示意
- [ ] 改了 `Cargo.toml` / `CHANGELOG.md` 的解析逻辑 → 仍是行级文本处理,且有「其余文本不变」的断言
- [ ] 只用 `rust-version = "1.85"` 已稳定的语法(`version.rs:374` 注释即一例:不能用 let-chain)

---

## Guidelines Index

| 文档 | 内容 | 何时读 |
|---|---|---|
| [conventions.md](./conventions.md) | 定位与依赖、模块结构、手写参数解析形状、子进程规则、输出与 `step()`、`--dry-run`、与 CI/release workflow 同步、文本解析而非 AST、测试要求、CHANGELOG 适用 | 任何 xtask 改动 |
| [../../workspace/rust/module-layout.md](../../workspace/rust/module-layout.md) | 文件内顺序(xtask 文件是范例)、`use` 三组、`pub(crate)`、`#[allow]` 带原因 | 新建 / 大改文件 |
| [../../workspace/rust/naming-and-api-design.md](../../workspace/rust/naming-and-api-design.md) | `root: &Path` 首参、`Options` 结构体、枚举代替 `bool`、slice pattern 分派 | 设计函数签名 |
| [../../workspace/rust/error-handling.md](../../workspace/rust/error-handling.md) | §3 `anyhow` 用法:`bail!` / `ensure!` / `.with_context`、子进程失败带 stderr 首行 | 每个 `?` 与 `bail!` |
| [../../workspace/rust/documentation.md](../../workspace/rust/documentation.md) | `//!` / `///` 中文写法;`cargo doc --document-private-items -D warnings` 对私有项链接同样生效 | 写注释 |
| [../../workspace/rust/testing.md](../../workspace/rust/testing.md) | 单测命名、成功/拒绝配对、临时目录 + `process::id()` 清理 | 补测试 |
| [../../workspace/rust/dependencies-and-changelog.md](../../workspace/rust/dependencies-and-changelog.md) | 新增依赖流程、MSRV、什么改动不记 CHANGELOG | 想加依赖 / 判断是否记日志 |

---

## Quality Check

提交前在仓库根执行:

```bash
cargo xtask ci                                  # 对 xtask 自身同样生效:fmt / clippy -D warnings / test / doc -D warnings
cargo test -p xtask                             # 只跑 xtask 单测(parse_args / 文本解析 / 临时目录用例)
cargo xtask --help                              # USAGE 与新子命令一致
cargo xtask <新子命令> --dry-run                 # 有写操作的子命令:只打印计划、不改文件
cargo xtask version check                       # 版本解析逻辑改动后仍能通过
cargo xtask changelog notes v<当前版本>          # CHANGELOG 解析逻辑改动后仍能提取段落(需该版本段落存在)
cargo +1.85 check -p xtask --all-targets        # MSRV(需 rustup toolchain install 1.85)
git diff --stat -- xtask/src/ci.rs .github/workflows/ci.yml   # 两侧要么都改、要么都不改
```

改了 `new_crate.rs` 模板后,在临时分支实际生成一次并跑门禁,确认模板产物本身通过 lint:

```bash
cargo xtask new probe-lib && cargo xtask new probe-cli --bin
cargo build && cargo xtask ci
git checkout -- Cargo.toml Cargo.lock && rm -rf crates/probe-lib crates/probe-cli
```

CI 对 xtask 与业务 crate 一视同仁(`--workspace`);`release.yml` 的 verify job 直接运行 `cargo xtask version check` / `changelog notes`(`release.yml:60,64`),这两个子命令出错会让整个发版流程在第一步停下。
