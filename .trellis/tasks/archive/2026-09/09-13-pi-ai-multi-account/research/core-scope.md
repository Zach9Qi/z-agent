# 首版纯核心类型覆盖矩阵

## 依据与层级

源根 `C:/Users/chenziqi/Desktop/codes/js/pi/packages/ai`，下文 src/test 相对此目录；基线见 upstream-core.md。

- **T（强类型）**：核心数据含义由 Rust 类型/校验表达；不要求 TS 名称或 JSON 形状一致。
- **E（扩展承载）**：保留专有数据的完整 JSON 值，不声称对应 SDK 类型、语义验证或执行行为已移植。
- **X（排除）**：本次不提供该行为/运行时。保留其数据不代表执行其行为。
- 上游 `unknown/any` 的配置扩展在 Rust 限定为 JSON 值；工具结果 details 可保留宿主泛型。函数、SDK对象、循环引用不转换为 JSON，也不假称可无损移植。

## T：核心数据

| 领域 | 覆盖字段/语义 | 源码 | 拟建测试 |
| --- | --- | --- | --- |
| 标识 | ProviderId/ApiId/ModelId 开放字符串，不限定内置目录；模型身份是 provider+modelId，不把结构相等等同身份相等 | src/types.ts:17-76,843；src/models.ts:947-957 | model_data.rs |
| Context | systemPrompt 独立于消息；有序 messages；tools 缺失/空列表可区分 | src/types.ts:524-528 | message_data.rs |
| user | 字符串或 text/image blocks；Unix 毫秒 | src/types.ts:422-426 | message_data.rs |
| assistant | text/thinking/toolCall 有序块，不含 image；api/provider/model；responseModel/Id、providerThinkingLevel、diagnostics、usage、stopReason、deferred、errorMessage、rawStopReason、endTurn、timestamp | src/types.ts:428-450 | message_data.rs |
| toolResult | call ID/name、text/image、宿主 details、独立 usage、addedToolNames、isError、timestamp | src/types.ts:452-470 | message_data.rs |
| 内容与签名 | Text、Thinking、Image、ToolCall；opaque text/thinking/thought signatures；redacted 三态；tool namespace；arguments 必须是 JSON object | src/types.ts:345-381 | message_data.rs |
| TextSignatureV1 | v=1/id/phase 数据可表达；不强迫 opaque signature 解析为 V1，旧字符串与 JSON 外观原样保留 | src/types.ts:345-349；src/api/openai-responses-shared.ts:49-72 | message_data.rs |
| Usage/Cost | input/output/cacheRead/cacheWrite/totalTokens；optional reasoning/cacheWrite1h；各费用与total；缺失与0不同，不重算total | src/types.ts:383-404 | model_data.rs |
| 计数解释 | reasoning 是 output 子集，cacheWrite1h 是 cacheWrite 子集；tool usage 不并入主 LLM usage | src/types.ts:388-394,457-458 | model_data.rs |
| StopReason | pending/stop/length/toolUse/error/aborted/deferred；保留 unknown rawStopReason 不硬塞标准枚举 | src/types.ts:406 | event_data.rs |
| 后台句柄 | provider/modelId/api/id、expiresAt、pollAfterMs、data（缺失/null不同）；不提供fetch/cancel | src/types.ts:410-420 | message_data.rs |
| diagnostics | type/timestamp/error/details；error name/message/stack/code；code可字符串/数值；内容不天然适合日志 | src/utils/diagnostics.ts:1-13 | message_data.rs |
| Model | id/name/api/provider/baseUrl/reasoning/input/cost/contextWindow/maxTokens；thinking map/sampling/headers/compat | src/types.ts:843-871 | model_data.rs |
| thinking | minimal/low/medium/high/xhigh/max，模型档另有off；map缺key=默认，Unsupported=null语义，Mapped=原生字符串 | src/types.ts:83-97,850-854 | model_data.rs |
| 费率 | 4类美元/百万token费率；inputTokensAbove阶梯及optional tiers；只存数据，不执行calculateCost | src/types.ts:825-840；src/models.ts:891-910 | model_data.rs |
| Tool/schema | name/description/parameters/constrainedSampling；schema保存完整文档含$ref/$defs/扩展关键字，不执行或检查provider支持 | src/types.ts:493-522 | options_tools.rs |
| 约束采样 | 缺失、false、json_schema prefer/require、grammar的可选lark/regex；空variants仍可表达 | src/types.ts:496-515 | options_tools.rs |
| 通用请求数据 | temperature/samplingParams/maxTokens/transport/cacheRetention/sessionId/websocketConnectTimeoutMs/metadata；timeout/retry字段；env纯数据；headers的未提供/设置/删除不同 | src/types.ts:113-225 | options_tools.rs |
| Simple选项 | toolChoice仅auto/none；reasoning；deferred false/true/object及window；thinkingBudgets四档 | src/types.ts:314-322 | options_tools.rs |
| 响应/续取数据 | response status/headers、deferred wait毫秒；无网络方法 | src/types.ts:118-121,227-236 | options_tools.rs |
| 事件 | Start；text/thinking/toolcall 的 start/delta/end；contentIndex/partial；end的权威content或ToolCall；Done四种成功原因；Error两种失败原因与部分AssistantMessage | src/types.ts:530-562 | event_data.rs |

## E：显式扩展承载，而非十套 SDK 类型重写

- `Model.compat` 按 API/已知家族携带 JSON object；保留上游所有已知字段与扩展，不在首版逐字段强类型化四套兼容结构。
- 对应 OpenAI Completions `src/types.ts:568-643`，Responses/Anthropic/Bedrock `:646-735`，routing/fallback `:307-311,744-823`；动态 chatTemplateKwargs `:86-97` 也保留值结构。
- fixtures 至少覆盖非默认布尔、null、数组、嵌套对象、模型fallback、百分位约束、字符串/数值price、chat-template变量及omitWhenOff，不能扁平为字符串映射。
- provider 专有 options 使用 `ApiId + JsonObject` 携带；不递归复制 SDK toolChoice/serviceTier 联合。普通选项数据不包含通用 apiKey 绕过口；认证材料来自显式账号。
- 专有样本：Anthropic effort/display/budget/toolChoice (`src/api/anthropic-messages.ts:216-286`)，OpenAI effort/summary/serviceTier (`openai-responses.ts:94-100`)，Codex nullable summary (`openai-codex-responses.ts:72-79`)，Vertex project/location 与 Google budget=-1 (`google-vertex.ts:46-55`、`google-shared.ts:28`)，其余参见 `src/types.ts:243-262`。
- 扩展对象仍是宿主数据，可含秘密；保留不代表执行或允许其改变已选账号，未来适配器须约束认证覆盖。

## 事件语义边界

- 上游 partial 为共享累积对象而非历史快照：`src/types.ts:530-544`、`src/utils/assistant-message-frame.ts:135-139`。
- 使用 `AssistantMessageEvent<P>` 保存宿主提供的 partial 句柄；P可为共享对象，库不固定队列/锁/executor；终态携带完整AssistantMessage。
- 测试用共享句柄证明旧事件可观察后续修改，不逐delta克隆完整消息。
- 类型保存事件数据；不提供事件顺序验证器。约定允许setup直接Error、其余Start后更新再终态，块可按contentIndex交错。
- ToolCallDelta保存残缺JSON字符串，不能使用全量JSON解析器强制解析。End可有权威值不同于delta拼接。
- 上游离线案例：`test/assistant-message-frame.test.ts:41,74,108,359,382,424,489,515`；仅取数据语义，不移植修复器/frame encoder/reducer。

## Rust 值域与序列化

- 自有 JsonValue/JsonObject/JsonSchema，公开API不暴露serde_json内部类型；JSON number能保留有符号/无符号整数及有限浮点。
- 工具结果 details 通过泛型保留非JSON宿主数据；默认JsonValue；messages/context相应携带该泛型。
- count/duration采用非负整数，UnixMillis保留毫秒单位，费用/浮点参数拒绝NaN/Infinity；计数字段的整数限制作为明确Rust契约记录。
- 私有字段+构造校验；可选serde经同一校验路径，不能派生Deserialize绕过已声明不变量；serde_json仅测试/内部按需使用。
- schema是保存的文档，不自动转化或限制其合法关键字；Image只保存base64及MIME，不要求解码依赖。

## X：首版不执行的能力

网络/SDK/OAuth流程、环境及文件、provider payload转换、schema验证/coercion、strict-schema变换、JSON修复、价格计算、reasoning clamp/预算计算、历史转换、图片生成、动态catalog、完整异步stream/result运行时。

## 账号接缝

- Account不进入Context/Message的固有身份；相同Model可用于不同显式账号。
- 本库提供账号快照与provider匹配检查，不提供将OAuth转换为HTTP请求的解析器。
- DeferredHandle仍只有上游数据；未来宿主必须保存其本地账号来源并显式选择，不能从provider/model/远端ID推断本地账号。
