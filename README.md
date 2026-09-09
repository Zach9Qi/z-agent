<div align="center">

# 🦀 Rust Monorepo Starter

### 工业级 Rust 多 Crate 模板 · 开箱即用的自动化工程基座

**为发布到 crates.io 量身打造** · **不塞任何多余的示例代码** · **一条命令搞定一切**

[![Rust 2024](https://img.shields.io/badge/Rust-2024_Edition_(MSRV_1.85)-DEA584?style=for-the-badge&logo=rust&logoColor=black)](https://www.rust-lang.org/)
[![Cargo Workspace](https://img.shields.io/badge/Cargo-Workspace_Resolver_3-000000?style=for-the-badge&logo=rust&logoColor=white)](https://doc.rust-lang.org/cargo/reference/workspaces.html)
[![Clippy pedantic](https://img.shields.io/badge/Clippy-Pedantic_Strict-F74C00?style=for-the-badge)](https://doc.rust-lang.org/clippy/)
[![crates.io OIDC](https://img.shields.io/badge/crates.io-Trusted_Publishing-orange?style=for-the-badge&logo=rust)](https://crates.io/docs/trusted-publishing)
[![CI Matrix](https://img.shields.io/badge/CI-Ubuntu_|_macOS_|_Win-2088FF?style=for-the-badge&logo=githubactions&logoColor=white)](.github/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue?style=for-the-badge)](https://opensource.org/licenses/MIT)

<br/>

[✨ 点击使用此模板新建项目](https://github.com/Zach9Qi/rust-monorepo-starter/generate) · [⚡ 30秒体验](#-30-秒快速开始) · [💎 为什么选择本模板？](#-为什么选择这个模板) · [📐 架构与设计准则](#-工程架构与设计准则) · [🚀 自动化发版](#-零心智负担的发版工作流)

<br/>

</div>

---

## 💡 为什么选择这个模板？

你是否经历过以下 Rust 项目的「起步痛苦」？

- ❌ `cargo new` 只有一个单体 demo，想拆分多 crate 却要手动维护一堆 `Cargo.toml` 和版本依赖。
- ❌ 手动发布到 crates.io 总是被各种低级问题打断：漏了 `include` 文件、内部依赖忘记写 `version`、本地能编但发上去编不过。
- ❌ 长期保存的 `CARGO_REGISTRY_TOKEN` 存在严重泄露风险。
- ❌ 每次想建一个新 crate 都要拷贝粘贴配置，`error.rs`、`lib.rs`、`README.md`、`Cargo.toml` 重新写一遍。
- ❌ 跨平台 CI 编译、MSRV 校验、依赖合规审计、二进制多平台打包脚本复杂繁琐，写到怀疑人生。

> **Rust Monorepo Starter 将这些全部变成了自动化基座。**  
> 你的仓库第一天就是干净整洁的，没有 `crates/foo`、`crates/bar` 这类恶心的示例废代码。**所有的规范与最佳实践，都由内置的 `xtask` 自动化落地。**

---

## ⚡ 30 秒快速开始

无需全局安装额外的复杂工具，依靠 Rust 原生能力即刻起航：

```bash
# 1. 一键生成一个符合 crates.io 规范的发布级 Library crate
cargo xtask new my-core
# -> 自动生成规范结构、thiserror 错误类型、独立 README、继承工作区 lint，并登记到 [workspace.dependencies]

# 2. 一键生成一个 CLI 二进制 crate（附带 clap 解析、tracing 日志、集成测试）
cargo xtask new my-cli --bin

# 3. 本地运行全量 CI 门禁（格式化、拼写、极严静态检查、测试、文档）
cargo xtask ci

# 4. 发布新版本？一行命令全自动完成校验、CHANGELOG 切段、git 标签与推送到 crates.io
cargo xtask release 0.2.0
```

---

## 🌟 核心工程亮点

<table>
<tr>
<td width="50%">

### 🧱 真正的一致性工作区 (DRY)
- 根 `Cargo.toml` 集中声明 `version`、`edition`、`license` 与依赖版本表。
- 子 crate 仅需 `xxx.workspace = true` 继承。
- 升级依赖只需改动根目录 **一行代码**。

</td>
<td width="50%">

### 🧩 零样板生成器 (`cargo xtask new`)
- `crates/` **初始为空**，保持仓库纯净！
- 一条命令生成规范的 `my-core` 库，自带完整的 `description`、`keywords`、`categories`、`docs.rs` 元数据。
- 自动注册到 `[workspace.dependencies]`，下游随时引用。

</td>
</tr>
<tr>
<td width="50%">

### 🔐 零密钥泄露 (Trusted Publishing)
- 集成 crates.io 原生 **OIDC 信任凭证**。
- GitHub Actions 无需长期存储 `CARGO_REGISTRY_TOKEN`。
- CI 阶段强制执行 `publish --dry-run` 隔离编译，彻底杜绝发版翻车。

</td>
<td width="50%">

### 🛡️ 工业级代码质量门禁
- 统一启用 **`clippy::pedantic`** 严苛基线。
- 强制禁用调试宏残留 (`print_stdout`, `dbg_macro`)，拒绝 `unwrap_used`。
- `missing_docs = "warn"`，所有公开 API 必须具备文档注释。

</td>
</tr>
<tr>
<td width="50%">

### 📝 Keep a Changelog 驱动发版
- `CHANGELOG.md` 的 `[Unreleased]` 为空时**拒绝发版**，倒逼规范。
- 自动提取版本变更记录注入为 GitHub Release Notes。
- 自动补齐 GitHub commit 差异比较链接。

</td>
<td width="50%">

### 🤖 5 大目标平台二进制打包
- 推送 tag 自动矩阵编译并输出 Release 附件：
  - Windows x64 (`.zip`)
  - macOS Apple Silicon & Intel (`.tar.gz`)
  - Linux x64 & ARM64 (`.tar.gz`)
- 自动生成 `SHA256SUMS` 校验和。

</td>
</tr>
</table>

---

## 📋 新项目 3 步起步清单

点击页面上方 **`Use this template`** 创建你的新项目后，仅需花 1 分钟完成以下基础替换：

```markdown
1. [ ] Cargo.toml               -> 修改 [workspace.package] 的 authors 与 repository
2. [ ] CHANGELOG.md             -> 将底部 [Unreleased] 比较链接替换为你的新仓库地址
3. [ ] LICENSE                  -> 将版权所属人替换为你的名字/组织名
4. [ ] (可选) release.yml       -> 若需要分发二进制包，在顶部填入 PKG_NAME / BIN_NAME
```

全部修改完成后，运行门禁校验：
```bash
cargo xtask version check
cargo xtask ci
```

---

## 📂 干净规范的项目结构

```text
.
├── .cargo/config.toml             # cargo 别名 xtask 与交叉编译链接器
├── .github/
│   ├── dependabot.yml             # 依赖与 Actions 版本分组自动升级
│   └── workflows/
│       ├── ci.yml                 # 完整 CI 门禁 (fmt/clippy/test/msrv/deny/dry-run)
│       └── release.yml            # 自动化发版流水线 (OIDC -> crates.io + Release 产物)
├── crates/                        # 业务 crate 目录（初始保持干净，无垃圾示例）
│   └── .gitkeep
├── xtask/                         # 纯 Rust 实现的工程自动化任务（零外部臃肿依赖）
│   └── src/
│       ├── new_crate.rs           # cargo xtask new 模板生成器
│       ├── ci.rs                  # 本地等效 CI 门禁
│       ├── release.rs             # 6 道安全防线的一键发版器
│       ├── changelog.rs           # CHANGELOG.md 解析与维护
│       └── version.rs             # 工作区版本同步与一致性检验
├── Cargo.toml                     # 工作区统管清单 (dependencies / lints / profile)
├── CHANGELOG.md                   # 遵循 Keep a Changelog 规范的变更日志
├── deny.toml                      # 许可证合规与依赖安全审计白名单
├── rust-toolchain.toml            # 锁定稳定版工具链 (Edition 2024 / MSRV 1.85)
└── typos.toml                     # 毫秒级拼写检查配置
```

当你执行 `cargo xtask new my-lib` 后，即刻生成规范库：
```text
crates/my-lib/
├── Cargo.toml        # 预配齐全的 crates.io 详细元数据，继承 workspace lints
├── README.md         # 专属于该 crate 的说明文档
└── src/
    ├── lib.rs        # 顶层模块与文档架构
    └── error.rs      # 基于 thiserror 的强类型错误枚举
```

---

## 🛠️ 常用开发指令速查表

| 命令 | 说明 | 适用场景 |
|:---|:---|:---|
| **`cargo xtask new <name>`** | 生成标准库 crate，自动关联工作区依赖表 | 增加核心能力或业务库 |
| **`cargo xtask new <name> --bin`** | 生成可执行 CLI crate（自带 clap + tracing） | 增加命令行终端工具 |
| **`cargo xtask ci`** | 严格执行 `typos` → `fmt` → `clippy` → `test` → `doc` | 提交代码前必跑 |
| **`cargo xtask ci --keep-going`** | 遇到错误不中断，完整跑完并打印全局诊断报告 | 批量修复问题时 |
| **`cargo xtask release [patch\|minor\|major\|x.y.z]`** | 全自动执行 6 道发版安全检查并推送 tag | 版本发布上线 |
| **`cargo xtask version check`** | 校验工作区内部依赖版本的一致性 | 依赖排错与发版自检 |
| **`cargo publish --workspace --dry-run`** | 在沙箱隔离环境中演练 crates.io 打包 | 确认是否缺少 include 源码 |

---

## 🚀 零心智负担的发版工作流

不需要繁琐易错的手动切分支、改版本、打 tag，一切只需交给一条命令：

```bash
# 自动提升语义化版本 (patch / minor / major) 并完成发版
cargo xtask release minor

# 也可以直接指定目标版本或预发版
cargo xtask release 1.0.0-rc.1

# 支持安全演练 (只打印操作方案，不修改任何文件)
cargo xtask release minor --dry-run
```

<details>
<summary><b>🔍 发版背后的 6 道自动化安全防线（点击展开）</b></summary>
<br/>

1. **工作区洁净度检查**：确保没有未提交的代码或未跟踪的杂乱文件。
2. **分支与远程状态**：强制要求在 `main` 分支且已与 `origin/main` 保持完全同步。
3. **标签与版本防撞**：检查本地及远端是否已存在同名 tag，杜绝版本倒退与覆盖。
4. **CHANGELOG 校验**：`CHANGELOG.md` 必须含有有效记录，拒绝空白发布。
5. **版本一致性写入**：纯文本同步根清单与内部依赖版本，离线刷新 `Cargo.lock`。
6. **自动切段与 Git 操作**：归档 Release Notes，自动创建原子 Git Commit 与带签名附注的 Git Tag 并推送到 GitHub。

</details>

推送 Tag 后，**GitHub Actions 将自动接管流水线**：
1. **Verify**：二次交叉检验版本与测试套件。
2. **Build Matrix**：并行跨平台编译 Windows / macOS / Linux 5 大平台二进制打包（如有）。
3. **Publish to crates.io**：通过 OIDC 临时凭证按依赖顺序发布所有公开库。
4. **GitHub Release**：发布带完整 CHANGELOG 说明与二进制附件的 Release。

---

## 📐 工程架构与设计准则

1. **分层原则 (Separation of Concerns)**：
   - 库 crate 保持无状态与纯净：**禁止直接进行终端输出或硬编码环境变量**。所有输入由函数参数传递，所有错误由强类型 Result 抛出。
   - 二进制 crate 仅作流程装配：负责解析 CLI 参数、配置日志订阅者、分发子命令，不堆积核心业务逻辑。
2. **错误处理策略**：
   - 底层库使用 `thiserror` 派生高内聚的枚举错误，方便外部调用方做模式匹配与处理。
   - 上层应用（二进制/xtask）使用 `anyhow` 捕获完整调用上下文，终端统一使用 `{err:#}` 优雅打印。
3. **零调试残留**：
   - 项目严禁在生产代码保留 `println!` 或 `dbg!`，所有运行期跟踪统一使用 `tracing` 门面。
4. **文档即规范**：
   - 启用 `missing_docs = "warn"`，所有公开函数、结构体均需文档注释，保证 docs.rs 呈现工业级 API 文档。

---

## 🤝 参与贡献与许可证

欢迎 Star ⭐️ 并 Fork 使用！如果你有更好的工程化实践建议，欢迎提交 Issue 或 Pull Request。

本项目基于 [MIT License](LICENSE) 自由开源。
