# z-ai 数据与多账号契约

## 1. Scope / Trigger

适用于 `crates/z-ai/` 的公开数据类型、可选 serde、账号凭据与内存存储。
本库对齐 `pi-ai` 的核心数据语义，不实现网络、OAuth 流程、存储 IO、上游 JSON 兼容或 provider 执行逻辑。
上游来源、许可证及数据边界见 `crates/z-ai/README.md` 的「许可证」「数据覆盖与安全边界」；详细 T/E/X 映射见 `.trellis/tasks/09-13-pi-ai-multi-account/research/core-scope.md`；领域术语见根 `CONTEXT.md`。
README 保留本项目 MIT 许可声明、上游来源/固定 commit、非官方说明及数据边界；用户已要求删除额外附加的上游 MIT 全文，不因旧验收记录自动恢复全文附录，也不另建替代说明文件。这是文档精简，不改变根 LICENSE、Cargo 许可声明或功能/协议边界；后续若引入受上游许可约束的代码，仍须保留适用的版权与许可声明。

## 2. Signatures

实际 API 以源码为准，以下接缝发生变更时必须同步文档和测试：

```rust
// identity.rs：各标识是不同 newtype，不传裸字符串互换身份
ProviderId::new(String) -> Result<ProviderId>
AccountId::new(String) -> Result<AccountId>

// account.rs：全部选择明确包含 provider 与本地账号
Account::new(AccountId, ProviderId, String, AccountAuth) -> Account
AccountStore::insert(&mut self, Account) -> Result<Revision>
AccountStore::snapshot(&self, &ProviderId, &AccountId) -> Result<AccountSnapshot>
AccountStore::replace_auth(&mut self, &ProviderId, &AccountId, Revision, AccountAuth) -> Result<Revision>
AccountStore::rename(&mut self, &ProviderId, &AccountId, Revision, String) -> Result<Revision>
AccountStore::delete(&mut self, &ProviderId, &AccountId, Revision) -> Result<Revision>
AccountStore::list(&self) -> impl ExactSizeIterator<Item = AccountInfo>

// event.rs：终态不允许裸原因与消息绕过一致性校验
AssistantMessageEvent::<P>::done(DoneReason, AssistantMessage) -> Result<AssistantMessageEvent<P>>
AssistantMessageEvent::<P>::error(ErrorReason, AssistantMessage) -> Result<AssistantMessageEvent<P>>
```

这些是签名摘要，不是可直接编译的 Rust 文件。

## 3. Contracts

### 账号与更新

- `AccountId` 在一个存储实例内全局唯一；ID/provider 固定，显示名独立；同一远端用户可以对应多个本地配置。
- `AccountSnapshot` 包含完整账号认证配置与该账号版本，是拥有值的稳定快照，不是已解析 HTTP 认证。
- 每次成功变更用 store 单调序号分配新版本；CAS 比较目标账号的版本，不能比较全 store 最近版本。
- 更新一次替换完整 `AccountAuth`，保留本地元数据；无 upsert，旧结果不能复活删除记录或覆盖同 ID 重建的新账号。
- 所有失败保持账号内容和序号不变；计数器先 `checked_add`，成功后才提交。
- `AccountStore` 不提供 Clone、自动 serde、重置版本或可变账号引用。版本只在同一实例连续生命周期有效。
- 本地 CAS 不协调远端 OAuth 刷新；冲突后不保证旧 token 可用。宿主协调刷新与其他写入，不自动重试旧 token。
- 删除不撤销外部快照、远端授权或在途请求；不识别复制的 OAuth 授权及共享刷新域。

### 数据与序列化

- 核心类型保留角色内容限制、毫秒单位、未提供与 `0/false/null` 的区别；usage 的 reasoning/cacheWrite1h 是子集，不重复累计，不重写报告 total。
- `AssistantMessageEvent<P>` 的 partial 由宿主句柄承载，不能偷偷改为每 delta 全量 clone；终态嵌套记录字段私有，反序列化同样核对原因。
- `serde` 默认关闭；自有 tagged JSON 值用于保留数值类别及 null，不声称原 JSON 格式兼容。扩展对象只承载数据，不执行 schema/协议。
- 有不变量的类型通过构造/`TryFrom`/自定义 Deserialize 校验。不要依赖 JSON 本身不能编码 NaN 来替代非有限浮点反序列化测试。
- 工具结果 details 的外层存在性必须独立于宿主 `D` 的序列化（`D` 可能序列化成 null）；当前实现见 `message/details.rs`。宿主类型内部自己的有损 serde 行为不由核心修复。
- 若使用 `serde_json` 要求有限浮点逐位往返，启用 `float_roundtrip`；根依赖已启用，不能只依赖简单小数测试。

### 敏感信息

- `SecretString` 无 Display/serde；凭据、账号、快照及存储不自动序列化。
- 含 headers/env/extensions/URL 的敏感父结构 Debug 整块隐藏，不只按 Authorization 等已知键过滤；错误不回显秘密输入。
- 列表白名单只有 ID/provider/label/kind/revision。调用方须把标签作为非敏感数据；显式读取原始值及任意消息内容的日志安全由调用方负责。

## 4. Validation & Error Matrix

| 条件 | 结果 | 状态 |
| --- | --- | --- |
| 空/纯空白标识 | `InvalidInput` | 不构造非法ID，不回显秘密 |
| 新增已有ID（即使provider不同） | `AlreadyExists` | 不覆盖、不消耗版本 |
| 选择不存在ID | `NotFound` | 不回退、不插入 |
| ID存在但provider错 | `ProviderMismatch` | 优先于版本比较 |
| 目标版本不匹配 | `RevisionConflict` | 旧结果不覆盖 |
| 无法分配下一版本 | `RevisionExhausted` | 内容与序号不变 |
| usage子集越界/费用非法/非有限数 | `InvalidInput`或serde错误 | 构造和反序列化一致 |
| 事件终态原因不匹配消息 | `InvalidInput`或serde错误 | 不能绕过私有终态记录 |
| key缺失、OAuth refresh为空或已过期 | 可作为数据保存/读取 | 不表示认证有效、不读环境 |

## 5. Good / Base / Bad Cases

- Base：一个 provider 的两个 API Key 与两个 OAuth 本地配置可共存，显式选择后读取各自快照。
- Good：A读取后B更新，A仍可按原版本更新；A删除重建后，旧A版本写入失败。
- Bad：拿 providerId 当账号主键；按远端用户去重；刷新结果直接覆盖；把本地CAS成功当作token远端有效。
- Bad：把缺失 details 与存在但序列化成 null 的 details 折叠；直接 derive Deserialize 绕过金额/usage/终态校验。

## 6. Tests Required

- `tests/accounts.rs`：混合账号、错误选择、重复ID、条件更新、改名/重授权、删除重建、稳定快照。
- `account.rs` 私有单元测试：版本耗尽，错误优先级，失败后内容和计数器不变。
- `tests/secret_redaction.rs`：内层/外层/列表/store/错误普通及美化 Debug 的独立秘密哨兵。
- `tests/serde_data.rs`、`model_options_serde.rs`、`value_semantics.rs`：缺失/null、宿主details、全部union、非法数值、非JSON反序列化源、有限浮点多样本逐位往返。
- `tests/event_data.rs`：共享partial可见性、权威end、残缺JSON文本、所有终态匹配/不匹配。
- gate：`cargo xtask ci`，all-features测试/clippy/rustdoc，MSRV1.85，deny与发布包README中的本项目MIT许可声明、上游来源/固定commit、非官方说明和数据边界检查；不将额外上游MIT全文附录作为当前门禁，后续引入受上游许可约束代码时仍需检查适用声明；typos缺失须明确记录跳过。

## 7. Wrong vs Correct

**Wrong**：provider单槽凭据；刷新结束直接覆盖；每个账号从0重新计版本；derive Debug打印整个headers；把JSON解析器拒绝NaN当成类型自身验证。

**Correct**：显式本地账号+provider校验；完整快照及目标版本条件写入；store单调分配防删除重建ABA；敏感配置整体脱敏；构造和serde共用验证并使用非JSON数据源验证非有限输入。
