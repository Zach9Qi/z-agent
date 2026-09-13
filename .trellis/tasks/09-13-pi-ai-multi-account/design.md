# z-ai 纯核心库技术设计

> 本设计已随最终规划获得用户「ok」批准，任务已启动实施。重大范围变化仍须重新评审。

## 1. 交付边界

- 新库 `crates/z-ai/`，Rust import 为 `z_ai`，由 `cargo xtask new z-ai` 生成；名称及范围已获批准。
- 一个 crate、一个集成交付。内部聊天数据与账号模块共享标识、JSON值与错误约定，按步骤实施，不另建运行时或 adapter crate。
- 只包含数据、校验和内存账号操作，无网络/文件/环境/进程/时钟IO，无tokio/futures/reqwest/SDK依赖。
- 使用 `CONTEXT.md` 的领域术语。Account是本地凭据配置，不是远端用户；Provider不是账号；Model不按账号复制。
- 兼容范围以 `research/core-scope.md` 的 T/E/X 矩阵为准：核心字段强类型，provider compat/options保留JSON扩展，行为工具和运行时排除。

## 2. 模块与依赖

建议模块按领域拆分，必要时以领域目录分文件，不建utils/helpers：

| 模块 | 职责 |
| --- | --- |
| identity | ProviderId、ApiId、ModelId、AccountId、ToolCallId等不同语义的ID；UnixMillis/Revision在所属领域定义 |
| value | JsonValue/JsonObject/JsonNumber/JsonSchema，明确的动态值边界 |
| message | Context、各角色消息与内容、签名、diagnostics、deferred handle |
| model | Model、能力、thinking map、usage/费用与阶梯数据 |
| tool | schema描述及约束采样数据，不执行schema |
| options | 通用请求/采样/响应选项、header删除语义、provider扩展 |
| event | AssistantMessageEvent<P>及其成功/失败原因；无异步运行时 |
| auth | SecretString、API Key/OAuth凭据、账号认证配置、ModelAuth/AuthResult纯数据 |
| account | Account/AccountInfo/AccountSnapshot/AccountStore与条件更新 |
| error | crate唯一Error/Result，中文错误，不包含秘密原文 |

依赖：必需thiserror；自有JSON值不公开serde_json类型；serde作为可选feature且默认关闭，serde_json供往返测试或必要的私有实现。所有新依赖/用途注释按根workspace管理，不引入密码学、OAuth或时间/随机库。本项目许可证保持MIT；README记录上游来源及非官方关系。后续若引入受上游许可约束的代码，仍须保留适用的版权与许可声明。

## 3. 核心数据契约

### 3.1 标识与值域

- ProviderId/ApiId/ModelId/AccountId是开放字符串newtype，不绑定已知provider目录。基础ID构造拒绝空或纯空白，不自动trim/大小写归一化；禁止把不同ID类型互换。
- 库不生成AccountId；调用方提供。AccountId在一个store内全局唯一，不以远端用户/邮箱/token为键。显示名独立，可重复；ID/provider归属在记录存在期间不可更新。
- UnixMillis采用明确毫秒单位的有符号整数；duration/count采用非负整数；费用及JSON浮点只接受有限值。原TS number到整数值域的收窄写入README与类型文档。
- JsonValue保存null/bool/string/number/array/object；JsonNumber保留i64/u64整数及有限浮点的可表达范围。字段未提供用Option区分显式null，不能用null同时表示缺失。
- JsonSchema保存完整JSON schema文档（包括允许的布尔文档），不执行验证、解析引用或强制object schema。ToolCall.arguments是JSON object，但无需在构造时满足某个schema。
- 宿主ToolResult details支持泛型，默认JsonValue；Context/Message携带相同details类型，不把非JSON宿主数据默默丢弃。

### 3.2 消息、模型与usage

- 角色专属content枚举限制组合：user/toolResult支持text/image；assistant支持text/thinking/toolCall。systemPrompt只在Context。
- 原可选字段保留未报告与false/0/空值区别；签名为opaque字符串，不强制解析V1，也不重写redacted内容。
- stopReason七种完整保留；未知provider原因存rawStopReason。可表示pending、带部分内容/usage的error/aborted以及deferred句柄。
- Usage保存服务端报告的各字段与total；reasoning<=output、cacheWrite1h<=cacheWrite为已声明子集不变量，不重复累计；不强制total等于可见项之和，不将工具usage并入主usage。
- 费用保存有限非负数；阶梯保存阈值与顺序，不实现选择阶梯/重算价格算法。若源fixture出现与这里校验冲突的数据，回到规划核对，不静默丢弃。
- Model的结构相等与身份相等分开：身份比较只用provider+id；模型api/端点/配置仍为数据，不执行协议。
- thinking map用「缺键／Unsupported／Mapped」区分上游缺失/null/string。所有六档thinking及off可表达；专有negative budget在扩展JSON保留，不被通用无符号预算误收。

### 3.3 选项与扩展

- 通用数据选项保留原可选性及header未提供/设置/删除区别；环境仅为显式传入的数据。
- Provider专有options、Model.compat/routing采用有API归属的JsonObject扩展，不宣称四套compat或十套SDK选项已逐字段强类型化；样本覆盖见范围矩阵。
- 不复制signal/fetch/SDK client/telemetry callbacks等宿主对象；不在普通RequestOptions暴露通用apiKey覆盖选定账号的入口。
- 扩展数据可含认证信息；本库不执行它，也不能保证未来适配器不会误用。未来宿主需要验证它不会覆盖选定账号的身份。
- ModelAuth/AuthResult仅保存上游apiKey/headers/baseUrl、env/source的含义；本库不将OAuth转换为它们，也不把AccountSnapshot命名为已解析认证。

### 3.4 事件与所有权

- `AssistantMessageEvent<P>`：P为宿主提供的partial句柄，非终态事件携带P；终态拥有完整AssistantMessage。
- 不强制P为快照，不在每个delta克隆完整消息。示例/测试使用`Arc<RwLock<AssistantMessage>>`保留上游shared-live-partial语义，但库不创建锁/队列/异步executor。
- Start、三类block start/delta/end、Done、Error字段全部保留。contentIndex允许块交错；tool delta是残缺JSON字符串；End持有权威内容，不假定始终等于delta拼接。
- Done原因只允许stop/length/toolUse/deferred，Error只允许error/aborted；构造时校验终态原因与消息stopReason一致。
- 不实现事件顺序状态机。文档说明setup可直接Error，常规Start→updates→终态；类型本身不保证整个序列合法。
- 不提供后台fetch/cancel；未来宿主应将DeferredHandle与本地账号来源绑定，不能从其provider/model/id推断本地身份。

## 4. 多账号模型

```text
ProviderId ← AccountId（一个store内唯一，归属不可变）
                 ├─ 本地显示名
                 └─ AccountAuth
                      ├─ Credential::ApiKey | Credential::OAuth
                      └─ 账号级headers/baseUrl/env/extensions

AccountStore → 显式(providerId, accountId)查找 → AccountSnapshot
                                                  ├─ 身份/本地元数据
                                                  ├─ revision
                                                  └─ 完整AccountAuth
```

- ApiKeyCredential保留key可选与provider配置数据的能力；缺key的配置记录可存，但不代表可调用，也不会读取环境补齐。
- OAuthCredential保存access/refresh/expires与provider扩展；允许空refresh，不要求通用远端accountId；到期时间由宿主传入/判断，store不检查真实时钟。
- 核心接收宿主处理后的完整OAuth凭据，不合并原始HTTP刷新响应；provider省略新refresh token时由宿主保留旧值。
- 远端扩展随凭据更新；本地ID/provider/显示名放外层，不随OAuth返回值丢失。独立授权对应同一远端用户不去重。
- 凭据和相关headers/endpoint/env可在一次条件更新中整体替换，避免库内部形成A版token+B版endpoint。
- 不识别同一refresh token被多条记录复制；宿主承担共享刷新域/重复授权协调，首版没有防远端重复刷新承诺。

## 5. AccountStore 条件更新契约

### 5.1 API形状（概念级）

- 新增：接收已校验Account记录，重复AccountId拒绝，即使provider不同也拒绝。
- 快照：必须同时给出provider/account，校验归属，返回拥有值的AccountSnapshot；即使OAuth已过期仍可读取供宿主刷新。
- 摘要列表：仅ID/provider/显示名/credential kind/revision；不返回credentials/headers/baseUrl/env/extensions，不执行任何配置解析。
- 更新认证：显式选择 + expected revision + 完整新AccountAuth；成功保留本地身份与名称。
- 修改显示名：同样走expected revision，不提供可绕过版本的可变引用。
- 删除：显式选择 + expected revision；不存在、归属错或版本冲突明确失败，不幂等吞掉错误。
- 所有修改使用`&mut self`；多线程宿主自行加互斥。没有async storage trait、网络锁或全局singleton。

### 5.2 版本分配与ABA

- store私有u64单调序号为每次成功变更分配新Revision。CAS比较的是目标账号当前Revision，而非store最新序号。
- 新增/修改/删除先完成校验与checked_add，再变更数据。版本耗尽时返回明确错误，数据与计数器均保持不变。
- 删除同ID再新增取得新Revision，旧快照不能覆盖新记录；修改B不使A的Revision变化。
- 错误时不改任何账号。更新不是upsert，删除后旧结果不可能复活账号。
- 版本只在同一store实例连续生命周期内有效；不跨重建/恢复/重置。store不实现Clone/Serialize/Deserialize，不提供计数器重置或可变账号迭代器。
- 快照是值副本，删除/更新不会撤销外部已持有的凭据。删除只影响后续查找/更新，不取消在途请求、撤销远端授权或清除调用方内存。

### 5.3 OAuth宿主调用流程与失败边界

1. 宿主显式取AccountSnapshot并记录revision。
2. 宿主自行决定是否刷新，执行provider特定网络流程和single-flight；core不参与。
3. 宿主提交完整AccountAuth与原revision。
4. core成功替换或返回冲突，不自动重试，不强制覆盖。

**重要限制**：CAS只是本地冲突保护。远端轮换成功后，如果期间有改名/重新授权/删除导致冲突，旧token可能已失效；core不能恢复远端状态。宿主需要协调刷新与该账号所有写入，或承担冲突后的恢复/重新授权。首版不拆多个revision、不引入刷新事务/租约协议。

## 6. 安全、校验与序列化

- SecretString不实现Display/Serialize/Deserialize，Debug固定占位；提供明确命名的显式读取方法供宿主使用，不承诺zeroize。
- Credential/AccountAuth/ModelAuth/AuthResult及其快照/store的Debug不输出secret或完整配置；headers/env/extensions/URL整体视为敏感容器。其他数据类型如Model/Options中的已知敏感配置字段也脱敏。
- 摘要白名单的ID/provider/显示名由调用方作为非敏感标签管理；如果用户把秘密填入标签，不承诺自动识别。
- 不只依赖外层Debug：测试内层类型直接格式化、普通/美化Debug与错误Debug/Display。InvalidInput等错误只描述问题，不拼接非法凭据原文。
- 消息/诊断/任意JSON内容不保证已脱敏；可选serde输出可能含敏感内容，不适合自动日志。serde特性针对聊天/模型/选项数据，不覆盖认证/store/快照。
- 私有不变量经所有公开入口检查；有验证规则的Deserialize使用中间数据+TryFrom等路径，不允许派生绕过。默认/no-default/serde各组合独立检查。

## 7. 兼容、文档与回滚

- 不提供上游JSON兼容或旧auth.json导入；自有序列化仅提供往返能力。
- README的「许可证」保留本项目MIT许可声明、上游来源/固定commit及非官方说明，不再要求额外附加上游MIT全文；「数据覆盖与安全边界」说明数据与行为边界。详细语义矩阵及源路径保留在 `research/core-scope.md`，E层专有数据不做强类型验证、X层不实现。
- 用户批准删除三个独立说明文件，并在后续要求删除README额外附加的上游MIT全文；不新增替代独立文档，不修改根LICENSE、Cargo许可声明、Rust实现或功能/协议边界。先前将全文并入README的验证保留为历史，不据此恢复附录；后续引入受上游许可约束代码时仍需保留适用声明。
- 新库README保留用途、离线示例、错误/账号更新边界及feature说明；CHANGELOG记录用户可感知新增，不改工作区版本号。
- 回滚单位为尚未发布的新crate及对应根依赖/lock/CHANGELOG改动，不改源JS仓库。不撤销用户已有修改；若类型范围重大调整先回到规划。
- 无产品迁移/部署步骤，无凭据文件变更；当前已完成纯核心实施，未执行任何真实认证或模型服务网络测试。依赖审计与打包验证记录见 [validation.md](./validation.md)。
