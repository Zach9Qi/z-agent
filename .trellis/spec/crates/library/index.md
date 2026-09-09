# 库 crate 规范(`crates/<名字>`,发布到 crates.io)

> 库 crate 是纯逻辑层:输入从参数进、结果从返回值出,不碰 IO / 环境变量 / 终端。它是仓库里唯一发布到 crates.io 的东西,公开 API 一旦发布不可撤销。

---

## 适用范围

- 由 `cargo xtask new <名字>`(不带 `--bin`)生成、位于 `crates/<名字>/` 且**没有** `publish = false` 的 crate
- 模板初始状态 `crates/` 为空,约定全部由 `xtask/src/new_crate.rs:157-267` 的模板函数落地
- 二进制 crate 见 `../binary/index.md`;xtask 见 `../../xtask/automation/index.md`

---

## Pre-Development Checklist

开始写库代码之前确认:

- [ ] 读过 `../../workspace/rust/index.md`「硬约束速览」与本目录 `crate-anatomy.md`
- [ ] 新 crate 用 `cargo xtask new <名字>` 生成,而不是复制现有目录或手写 `Cargo.toml`
- [ ] 这段逻辑**确实**不需要 IO / 环境变量 / 打印——需要的话它属于二进制 crate
- [ ] 想清楚新功能属于哪个领域文件(`src/<领域>.rs`),还是要新开一个;不新建 `utils.rs`
- [ ] 新公开函数的失败情形能映射到现有 `Error` 变体,还是需要新变体(`../../workspace/rust/error-handling.md` §2.2)
- [ ] 这次改动是否触碰公开 API(删 / 改签名 / 加 pub 字段 / 提升 MSRV)→ 需要 **BREAKING** 条目与 minor bump
- [ ] 新依赖已在根 `Cargo.toml` `[workspace.dependencies]` 登记且许可证在 `deny.toml` 白名单
- [ ] 只用 `rust-version = "1.85"`(根 `Cargo.toml:21`)已稳定的语法与 API

### 「这段逻辑该放哪」速判

| 特征 | 归属 |
|---|---|
| 输入是 `&str` / 结构体,输出是值或 `Result`,不碰文件、网络、环境 | 库 crate(本层) |
| 要读文件 / 读 `std::env` / 调子进程 / `println!` | 二进制 crate `run` 或 handler;读完转成值再调库 |
| 只在仓库开发流程里用(生成代码、发版、门禁) | `xtask/`(`../../xtask/automation/index.md`) |

---

## Guidelines Index

| 文档 | 内容 | 何时读 |
|---|---|---|
| [crate-anatomy.md](./crate-anatomy.md) | 目录骨架、`Cargo.toml` 元数据、分层禁令、TODO 占位清单、semver 与弃用流程、内部依赖登记、发布约束 | 新建库 crate、改公开 API、发布前 |
| [../../workspace/rust/module-layout.md](../../workspace/rust/module-layout.md) | 文件内顺序、`use` 三组、可见性、`#[allow]` 用法 | 每次新建 / 大改文件 |
| [../../workspace/rust/naming-and-api-design.md](../../workspace/rust/naming-and-api-design.md) | 命名、参数类型、构造、必须实现的 trait、`#[non_exhaustive]` / newtype | 设计公开函数与类型 |
| [../../workspace/rust/error-handling.md](../../workspace/rust/error-handling.md) | `error.rs` 形状、何时加变体、`# Errors` 小节 | 新增可失败函数 |
| [../../workspace/rust/documentation.md](../../workspace/rust/documentation.md) | `///` / `//!` 写法、doctest、README 四节 | 写公开项文档 |
| [../../workspace/rust/testing.md](../../workspace/rust/testing.md) | 单元 / 集成 / doctest 三层,库 crate 专项 §4 | 补测试 |
| [../../workspace/rust/dependencies-and-changelog.md](../../workspace/rust/dependencies-and-changelog.md) | 新增依赖流程、feature、MSRV、CHANGELOG 写法 | 加依赖、发版前 |

---

## Quality Check

提交前在仓库根执行:

```bash
cargo xtask ci                                        # typos(如已安装)/ fmt / clippy -D warnings / test / doc -D warnings
grep -rn "TODO" crates/<名字>                          # 首次发布前必须为空
RUSTDOCFLAGS="-D warnings" cargo doc -p <名字> --no-deps --open   # 看 docs.rs 上会长什么样,检查 intra-doc link
cargo test -p <名字> --doc                             # README / lib.rs 示例能编译运行
cargo publish -p <名字> --dry-run                      # include / 元数据 / 内部依赖 version 打包演练
cargo xtask version check                             # 内部依赖 version 与工作区版本一致
cargo tree -p <名字> -e normal                         # 依赖方向只有「库 → 更底层的库」
```

CI(`.github/workflows/ci.yml`)额外做 MSRV 编译检查(`:125`)、`cargo deny check` 与 `cargo publish --workspace --dry-run --locked`(`:144`);本地能过 `cargo xtask ci` 但 CI 失败,通常是 `Cargo.lock` 没一起提交或用了高于 1.85 的语法。
