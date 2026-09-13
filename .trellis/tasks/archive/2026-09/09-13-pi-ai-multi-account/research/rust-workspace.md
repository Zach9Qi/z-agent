# Rust 工作区约束调研

## 现状

- crates/ 仅 .gitkeep，无业务实现；工作区 members 为 crates/*、xtask：`Cargo.toml:4-9`。
- 根 version 0.1.0、edition 2024、resolver 3、MSRV 1.85、MIT；stable 未固定版本：`Cargo.toml:13-21`、`rust-toolchain.toml:3-5`。
- 已有 thiserror/serde/serde_json/tracing 依赖表；无 HTTP/异步 runtime/OAuth 依赖：`Cargo.toml:25-41`。
- 根元数据仍是模板；没有 CONTEXT/ADR 或相关产品历史设计，归档仅规范初始化。
- 开发身份按已有 git user.name 初始化为 ZachQ。当前会话 PI_SESSION_ID 未被本地 Trellis 自动识别，命令通过 `TRELLIS_CONTEXT_ID=pi:session:<PI_SESSION_ID>` 绑定 planning 指针；未执行 task.py start。

## 硬约束与冲突

- 新库只能 `cargo xtask new <name>`，名称小写 ASCII 开头，后续字母/数字/短横线，不允许连续或尾随短横线；生成根依赖登记与成员继承：`xtask/src/new_crate.rs:45-57,107-129,157-186`。
- 库禁止文件/网络/进程 IO、环境变量读取、打印与日志装配：`.trellis/spec/crates/library/crate-anatomy.md:70-83`；`.trellis/spec/crates/library/index.md:3,19-20,31-33`。模板 Error::Io 不是豁免。
- 纯核心方案可返回请求/动作描述、由宿主 IO 后传入结果；完整网络 client 需要先批准并更新规范。仅注入 trait、开 feature 或拆 adapter 库不能被视作自动绕过禁令。
- 配置显式传入、不存进状态对象的通用约定与长期持有 Config 的 Client 模式可能冲突，设计时需评审：`.trellis/spec/workspace/rust/naming-and-api-design.md:86`。
- 公开类型 Debug 要求对 secret 手写脱敏；字段默认私有、枚举 non_exhaustive、serde feature：同上 `:122-140`。
- 库统一 thiserror Error/Result，公开边界验证；公开中文文档、Errors 小节；依赖根登记+中文用途+最小 feature+MSRV/license 核验。

## 验证计划输入

- 必须 `cargo xtask ci`：本地 typos（可缺省）、fmt、clippy -D warnings、test、rustdoc -D warnings：`xtask/src/ci.rs:4-69`。
- CI 另有三平台测试、MSRV、打包与 deny：`.github/workflows/ci.yml:74-95,125-159`。
- 目前无 all-features/no-default-features 编译测试矩阵，若有 serde/provider feature 应显式补充验证。
- 单元/外部公开 API 集成/doctest；禁真实环境依赖、ignore、should_panic、sleep；时间注入；可补属性测试和 Send+Sync 检查：`.trellis/spec/workspace/rust/testing.md:8-24,51-72`。
- 新库还需 version check、依赖方向、README/doctest/发布包 include 与元数据检查。公开变化同步 CHANGELOG。

## 任务组织建议（未定案）

如果选择全运行时，核心兼容、多账号、认证生命周期、协议适配可拆独立验收子任务，但共同交付一个 crate；父任务负责范围和整体验收。若先纯核心可保持单个任务。复杂任务需先完成 PRD/design/implement 与有效上下文清单，再由用户批准实施。
