# pi-ai 核心语义调研

## 基线与方法

- 源仓库：`C:/Users/chenziqi/Desktop/codes/js/pi`；下文路径相对该根。
- 包：`packages/ai`，npm `@earendil-works/pi-ai`，manifest `0.84.4`。
- HEAD：`1148f7958c260188b03488496044dcfa852ef580`，分支 main；主代理执行 `git status --short --untracked-files=no` 无输出。未把当前 HEAD 等同正式发布包。
- 源根 LICENSE 为 MIT，Copyright (c) 2025 Mario Zechner；实质移植需保留许可/版权通知。
- 只读调研源码、README、测试；未执行源项目测试或读取实际凭据。

## 类型与不变量

| 主题 | 必须关注的语义 | 源码锚点 |
| --- | --- | --- |
| 标识与模型 | Api/ProviderId 可扩展字符串；Model 包含端点、能力、价格阶梯、thinking/sampling/compat；模型相等比较 provider+id，历史签名重放还比较 api | `packages/ai/src/types.ts:17-76,843`；`packages/ai/src/models.ts:951`；`packages/ai/src/api/transform-messages.ts:95` |
| 消息 | user/assistant/toolResult；systemPrompt 在 Context；毫秒时间戳；工具结果支持图片/details/自身 usage/addedToolNames | `packages/ai/src/types.ts:422-470,524` |
| 内容与元数据 | textSignature、thinkingSignature、redacted、thoughtSignature、namespace、responseModel/Id、providerThinkingLevel、diagnostics、rawStopReason、endTurn 不应丢失 | `packages/ai/src/types.ts:345-450`；`packages/ai/src/api/openai-responses-shared.ts:49-72` |
| 流终态 | start/updates/done；setup 可直接 error；终态携带部分 assistant；complete 是 stream.result；stopReason 含 pending/stop/length/toolUse/error/aborted/deferred | `packages/ai/src/types.ts:406,530-565`；`packages/ai/src/models.ts:687-709` |
| 事件所有权 | partial 是共享累积对象而非快照；事件按 contentIndex 关联且可交错；Rust 需明确是否有意改变所有权契约，避免每增量全量 clone 的二次方成本 | `packages/ai/src/utils/assistant-message-frame.ts:139,230-261,372`；`packages/ai/test/assistant-message-frame.test.ts:230,268,489,545` |
| 工具与 schema | 工具执行不在 pi-ai；toolcall_end 不代表 schema 有效；验证含克隆、optional null 处理、coercion；strict prefer/require 有别；grammar 和 strict schema 仅支持受限子集 | `packages/ai/src/utils/validation.ts:302-351`；`packages/ai/src/utils/json-parse.ts:32,85,104`；`packages/ai/src/api/constrained-sampling.ts:11,115,188-260` |
| usage | reasoning 是 output 子集；cacheWrite1h 是 cacheWrite 子集；价格美元/百万 token；阶梯按总输入严格超过阈值，应用整个请求；tool usage 不并入 LLM context | `packages/ai/src/types.ts:383-404,457,825-840`；`packages/ai/src/models.ts:891-910` |
| reasoning | thinking map 缺失/null/字符串不同；xhigh/max 显式 opt-in；安全余量、输出上限与预算映射均是行为 | `packages/ai/src/models.ts:915-943`；`packages/ai/src/api/simple-options.ts:12-95` |
| 历史转换 | 跨模型清签名；丢弃 redacted thinking；规范化 tool ID 并同步 result；补孤立调用结果；跳过 error/aborted assistant | `packages/ai/src/api/transform-messages.ts:95-219` |

后台响应 DeferredHandle 与延迟工具加载是不同概念。前者有类型和 fetch/cancel 路由，但此次未发现内置 API 具体实现，不承诺所有 provider 支持：`packages/ai/src/types.ts:272-282,410`、`packages/ai/src/models.ts:711-745`、`packages/ai/src/api/lazy.ts:68-94`。

## 规模与边界

- 全包 178 个 src TS 文件、142 个 test 文件；10 个已知聊天 API、40 个内置 provider 工厂（39 静态 wrapper、Radius 动态）。计数包含外围，不是净移植工作量。
- 还有图片生成、动态模型目录、faux provider、SSE/WebSocket、连接恢复、retry/timeout/cancel、诊断和 legacy compat。
- 当前主入口无内置 catalog/SDK 注册副作用；Models 按 model.provider 分派，provider 按 model.api 选协议。旧全局 registry 在 compat.ts：`packages/ai/src/index.ts:4-48`、`packages/ai/src/models.ts:641-684,775-806`、`packages/ai/src/compat.ts:100-138,178-214`。
- 候选分期（未获用户选择）：纯核心优先；核心+指定协议的可用纵切；全包分批对齐。
- IO 候选：HTTP/SSE/WebSocket、SDK认证链、环境读取、时钟/等待、动态目录、OAuth、持久化。纯核心可包括消息/事件/价格/reasoning/schema/历史转换。

## 兼容素材与风险

- 优先离线 fixtures：`assistant-message-frame.test.ts`、`faux-provider.test.ts`、`models-runtime.test.ts`、`validation.test.ts`、`constrained-sampling.test.ts`、`openai-responses-terminal-event.test.ts`、`anthropic-cache-write-1h-cost.test.ts`、`context-estimate.test.ts`（均在 packages/ai/test）。
- 后续可人工 fixture → TS golden → Rust 差分，对时间戳/随机 ID 显式归一化；不得直接跑含真实凭据/网络的完整上游测试集。
- README 与源码有漂移：README `:708,1337-1370` 的非视觉图片、跨模型 thinking、aborted 历史处理描述与共享转换 `:12-55,95-115,195` 不符。以固定 commit 源码/测试为核对依据，差异需记录。
- 本地缺生成的 `packages/ai/src/providers/data/.manifest.json`（被 gitignore）；没有准确模型数量/目录生成时点，重新在线生成会漂移。
- JS UTF-16 长度、Date.now、partial JSON repair 的语义不能机械映射为 Rust 字节长度/全量 JSON parser。
- 多账号影响传输缓存、模型权限、后台句柄及请求绑定；细节见 `upstream-auth.md`。
