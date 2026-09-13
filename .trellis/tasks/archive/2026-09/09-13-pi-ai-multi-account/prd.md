# Rust pi-ai 语义移植与多账号认证设计

## Goal

在当前工作区交付一个独立 Rust 纯核心库：保留 pi-ai 核心聊天数据的含义与约束，支持同一 provider 的多个独立本地账号配置，为后续运行时提供稳定基础。本次不交付能直接访问模型服务的完整客户端。

## Background

- 源仓库：`C:/Users/chenziqi/Desktop/codes/js/pi`；源包 `@earendil-works/pi-ai`，manifest `0.84.4`。调研基线 HEAD `1148f7958c260188b03488496044dcfa852ef580`，当时受跟踪文件工作树干净；当前源码不等同正式发布包，行为以固定源码与测试为准。
- 单账号限制贯穿核心 CredentialStore/Models.login/logout 和上层 AuthStorage：源 `packages/ai/src/auth/types.ts:36-93`、`packages/ai/src/models.ts:570-626`、`packages/coding-agent/src/core/auth-storage.ts:17,449-478`。
- 认证数据不只是 key，还包括 headers/baseUrl、env/source；这些内容可能随账号变化：源 `packages/ai/src/auth/types.ts:3-19,104-112`。
- 源包还有 10 个已知聊天 API、40 个内置 provider 工厂及图片生成/动态目录等外围；本次仅取明确列出的核心范围。
- 当前 Rust 仓库没有业务 crate；现行库规范禁止 IO，纯核心边界与其一致。

## Confirmed Decisions

- D1：用户选择核心基础库，不实现真实服务调用。
- D2：必须显式指定账号；不提供默认账号、自动选择或跨账号切换。
- D3：只保证数据语义一致，不要求上游 JSON 字段名、标签或编码格式兼容。
- D4：账号指独立本地命名凭据配置；同一远端用户可以对应多个配置；凭据更新不改变本地账号身份。
- 用户已在最终规划摘要后回复「ok」，明确批准下列范围、设计和执行计划；本任务进入实施。
- 后续用户批准文档精简：删除 crate 的三个独立说明文件；最新要求为删除 README 额外附加的上游 MIT 全文，保留本项目 MIT 许可声明、上游来源/固定 commit、非官方说明与数据边界，详细类型映射保留在 `research/core-scope.md`。不改根 LICENSE、Cargo 许可声明、Rust 实现或功能/协议边界，不新增替代独立文档；后续若引入受上游许可约束的代码，仍须保留适用的版权与许可声明。此前合并全文的验证仅为历史，不作为恢复附录的要求。

## Requirements

### R1 核心聊天数据

保留以下类别的可表达性、顺序、可选状态与含义，采用 Rust 惯用表示：

- 开放 provider/API/model 标识，模型能力、费率与阶梯、thinking 映射、采样及兼容扩展数据。
- Context、user/assistant/toolResult 三种消息，text/image/thinking/toolCall 内容、签名、工具参数及结果附加信息。
- Usage/Cost、七种 StopReason、毫秒时间、错误/诊断、响应模型/ID、后台响应句柄等消息元数据。
- 工具 JSON schema、约束采样描述、通用请求选项及 provider 专有数据的扩展承载。
- assistant 增量事件及完成/错误事件的类型契约，包括部分响应、contentIndex 和未完成的工具 JSON 文本。

精确字段映射与「强类型／扩展数据承载／不实现行为」区别见 `research/core-scope.md` 与 `design.md`。不把保存数据宣称为实现对应协议或算法。

### R2 多账号与显式选择

- 同一 provider 可同时有多个 API Key 账号、多个 OAuth 账号，并允许两种认证方式混合存在。
- 本地身份独立于 key/token、显示名、远端用户/组织/project 等信息；不按远端身份归并账号。
- 新增、查找、更新、删除及列表不相互覆盖；重复本地账号身份明确失败。
- 需要目标账号的操作必须显式指定；不存在或与目标 provider 不匹配时明确失败，不回退到其他账号或环境凭据。

### R3 更新一致性

- 一次账号快照包含该账号的完整凭据与相关配置；后续更新不改变已有快照。
- 凭据更新不丢失本地名称等账号元数据；账号身份和 provider 归属不可经更新偷偷改变。
- 同一账号的过期更新结果不得覆盖较新状态；删除后不得由旧结果复活；同身份删除重建后仍拒绝旧结果。
- 修改 B 不影响 A 的更新有效性；失败操作不改变账号内容。
- 仅保证同一内存存储实例生命周期内的本地一致性；不保证远端 token 轮换、跨进程一致性或取消在途请求。

### R4 纯核心边界与信息保护

- 不执行网络、文件、进程、环境读取或实际时钟读取；宿主传入所需数据及时间。
- OAuth 在本次只表示凭据与条件更新，不执行登录/刷新、处理原始 token endpoint 响应或自动刷新。
- 列表仅含非凭据摘要。凭据、账号级配置、快照和存储的常规 Debug/错误输出不得泄露 key、token、headers/env/扩展中的秘密。
- 不为凭据或账号存储自动提供序列化；允许宿主显式读取凭据，不承诺内存清零、加密或阻止调用方主动泄露。

### R5 Rust 集成与质量

- 单个独立库 crate；中文公开文档、可执行示例；README 保留本项目 MIT 许可声明、上游来源/固定 commit、非官方说明与数据边界，不要求额外上游 MIT 全文附录；`research/core-scope.md` 保留详细源码语义映射。
- 核心数据支持可选 serde；序列化仅承诺自有格式往返，不承诺兼容上游或脱敏。
- 语义与账号测试完全离线，不依赖真实账号；通过工作区规定的质量门禁及 feature 组合检查。依赖获取、审计及打包检查的联网需求单独记录，不访问真实模型服务或使用真实凭据。

## Acceptance Criteria

- AC1 → R1：逐字段/变体 fixtures 覆盖三类消息、角色内容限制、模型、工具、选项和事件；可选值与 0/false/null 等不同语义不被折叠。
- AC2 → R1：thinking/text/tool 签名、redacted、响应元数据、后台句柄、失败部分响应、usage 子集关系均有明确测试；不重复计算 reasoning/cacheWrite1h，不自动改写报告的 total。
- AC3 → R2：同一 provider 同时存在两组 API Key、两组 OAuth；同远端用户的独立配置允许共存；查找/更新各自隔离，重复 ID、无效选择和 provider 不匹配明确失败。
- AC4 → R3：旧结果、重复提交、更新中删除、删除后重建、另一个账号变更等确定性序列可验证条件更新规则，版本耗尽失败不改数据。
- AC5 → R3/R4：空 OAuth refresh 等上游边界可表达；快照保留完整凭据及配置，更新保留本地元数据；删除不宣称撤销已发出的快照或远端授权。
- AC6 → R4：秘密哨兵测试覆盖无密钥列表、各层 Debug 和错误输出，不产生已知敏感字段明文泄漏。
- AC7 → R5：默认、no-default-features、serde/all-features 构建测试与文档通过；所有有不变量的数据经构造与反序列化均不能绕过校验。
- AC8 → R5：`cargo xtask ci` 通过；CHANGELOG、含本项目 MIT 许可声明/上游来源与固定 commit/非官方说明/数据边界的 README 和任务内 `research/core-scope.md` 语义映射齐全；后续引入受上游许可约束的代码时仍需保留适用声明。

## Out of Scope

- HTTP/SSE/WebSocket、provider payload 转换、SDK、动态模型目录、图片生成接口；聊天图片数据仍在范围内。
- OAuth 交互登录/网络刷新、文件存储、环境读取、外部取密命令、CLI/UI、AWS/ADC 等额外认证链。
- 默认账号、自动轮询、优先级调度、限流切换、故障转移；重复 OAuth 授权识别与跨账号共享刷新域。
- JSON schema 执行/coercion、partial JSON 修复、历史转换、计价计算算法、reasoning 参数计算、完整异步事件流运行时；本次只保留相应核心数据。
- 上游 JSON 兼容、旧 auth.json 导入导出/迁移、跨进程存储协议。

## Research

- `research/upstream-core.md`：上游基线、行为语义、离线测试与范围风险。
- `research/upstream-auth.md`：原认证链路、provider 特例与上层并发风险；其中网络锁/共享刷新域建议不属于本次核心承诺。
- `research/rust-workspace.md`：规范、依赖/MSRV、验证约束。
- `research/core-scope.md`：本次精确类型范围与兼容层级。

## Planning Status

D1–D4 及最终规划已获用户批准；执行任务启动与实施，重大范围变化仍须回到规划评审。
