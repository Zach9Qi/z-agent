# z-ai 执行计划

> 用户已在最新最终规划摘要后回复「ok」批准实施，task.py start 已成功，状态为 in_progress。下列清单按实际验证证据更新；任务创建同意、D1–D4答复本身不替代最终批准。

初次实施、验收、独立审查及命令退出码证据见 [validation.md](./validation.md)。产品验收已完成；用户随后批准仅做文档精简，最新要求是删除README额外附加的上游MIT全文，保留本项目MIT许可声明、上游来源/固定commit、非官方说明与数据边界。不改根LICENSE、Cargo许可声明、Rust实现或功能/协议边界。本轮文档、CI与打包复核已由主代理通过；此前合并全文的结果仅作历史，不作为恢复附录的要求。提交、归档和日志记录仍待用户确认提交计划后按流程执行。

## 0. 启动与质量基线

- [x] 重读prd/design/core-scope及implement.jsonl；确认源JS HEAD未偏离记录，若变化不偷偷切换语义基线。
- [x] 检查git工作树并保留用户改动；只在当前Rust仓库工作，不修改源JS项目，不读取真实凭据或运行上游全量测试。
- [x] 用户批准后运行 `python .trellis/scripts/task.py start .trellis/tasks/09-13-pi-ai-multi-account`。若会话识别失败，使用已有PI_SESSION_ID映射TRELLIS_CONTEXT_ID，不另建任务。
- [x] 读取trellis-before-dev与各层spec；确认Rust 1.85、许可白名单和当前已解析依赖。
- [x] 运行 `cargo xtask new z-ai`，遵循生成器登记；补crate元数据而非复制上游manifest。不提前生成任何产品文件。
- [x] 按依赖最小化原则增thiserror、可选serde与serde_json测试用途，根依赖先登记，成员workspace=true；更新锁文件。

回滚点：只有新crate/根依赖/锁文件，尚未变更其他业务crate。不要删除此前存在的用户改动。

## 1. 值域与数据基础

- [x] identity、value、唯一error：开放ID、不混用ID类型、毫秒时间、JSON完整值域、schema文档、中文错误。
- [x] 对空/纯空白ID、有限数值、整数/浮点边界、缺失/null建测试；公开不变量不能通过enum暴露字段或serde绕过。
- [x] 角色专属消息/内容、泛型details、Context、signatures、diagnostics/deferred数据。
- [x] Model/Usage/Cost/tiers/thinking映射；保留总量、子集和optional语义，区分模型身份相等与结构相等。
- [x] Tool/schema/constrained sampling与通用options；compat/provider options为E层JSON承载，fixture保留所有已知专有形状，不能宣称SDK类型全覆盖。
- [x] 写 `tests/message_data.rs`、`model_data.rs`、`options_tools.rs`、`value_semantics.rs`；覆盖矩阵每行可追溯到至少一项构造/观察或失败用例，避免只测derive自动相等。

门禁：`cargo test -p z-ai --no-default-features`、公开API外部集成测试和doctest可用；不引入任何真实IO。

## 2. 事件数据

- [x] 定义AssistantMessageEvent<P>、限制的成功/失败原因及终态消息一致性校验。
- [x] Start/text/thinking/toolcall所有变体保留index/partial/delta/权威end数据；tool delta保持未完成JSON字符串。
- [x] 写 `tests/event_data.rs`：共享P的旧事件观察新partial、块交错可表达、setup直接错误、四类成功/两类错误、终态部分usage、签名和权威end。
- [x] 文档明确无事件队列、异步result、顺序验证器或partial JSON修复器，不以类型测试冒称算法移植。

门禁：针对event的单元/集成/doctest；具体宿主共享句柄满足Send+Sync（测试可以用标准库Arc/RwLock）。

## 3. 认证数据与显式账号

- [x] auth：SecretString、API Key/OAuth、完整账号认证配置、ModelAuth/AuthResult数据；不实现解析成HTTP认证的provider逻辑。
- [x] account：Account、白名单AccountInfo、AccountSnapshot、AccountStore；ID/provider不可变，显示名独立。
- [x] 新增拒绝重复ID；快照显式provider+account并检查匹配；过期OAuth仍可读取供宿主处理，无隐式fallback。
- [x] 所有写入&mut self，条件更新/改名/删除比较目标账号revision；全局单调序号只用于分配新版本。
- [x] 认证与账号相关配置一次替换，本地身份/名称不随刷新丢失；不暴露绕过revision的可变引用，不派生store Clone/serde。
- [x] checked_add并先校验后变更；无upsert/无计数器重置；同ID删除重建拒绝旧快照。
- [x] 写 `tests/accounts.rs`：2key+2OAuth同provider、混合、同远端用户多配置、ID跨provider冲突、各失败路径、快照稳定、B更新不影响A、同旧revision两次提交、改名/重授权冲突、删除/重建ABA。
- [x] 私有单元测试覆盖revision耗尽，所有失败验证数据和版本不变。
- [x] 文档与测试边界：核心CAS不防远端double-refresh、不保证刷新失败后旧token有效、不撤销外部快照/远端授权/在途请求。

门禁：`cargo test -p z-ai --test accounts`明确运行账号集成目标，再用`cargo test -p z-ai`覆盖版本耗尽等私有单元测试；reviewer独立审查身份隔离与版本状态机，不用sleep/网络伪装并发验证。

## 4. 脱敏与可选序列化

- [x] 手写敏感内层类型、快照/store Debug；已知headers/env/extensions/URL容器整块脱敏；错误不拼秘密输入。
- [x] 列表白名单仅ID/provider/显示名/种类/revision；不把到期信息或配置当作已脱敏摘要返回。
- [x] 写 `tests/secret_redaction.rs`：不同哨兵覆盖key/access/refresh/headers/env/URL/extensions，普通/美化Debug、inner/outer/store、错误Debug/Display；检查非敏感摘要仍可用。
- [x] serde仅覆盖适合交换的核心数据，不自动覆盖secret/auth/account快照/store。公开显式读取secret记录风险，不添加Display。
- [x] 写 `tests/serde_data.rs`：自有格式往返、nested JsonValue、missing/null/false/0区分，受约束ID/数值/终态/usage反序列化无效输入失败；泛型details可序列化时成立。
- [x] 文档指出序列化≠脱敏，不承诺上游JSON兼容或zeroize。

feature门禁：

```text
cargo check -p z-ai --no-default-features --all-targets
cargo test -p z-ai
cargo test -p z-ai --no-default-features
cargo test -p z-ai --features serde
cargo test -p z-ai --all-features
cargo clippy -p z-ai --all-targets --all-features -- -D warnings
cargo test -p z-ai --doc --all-features
```

## 5. 文档、合规与最终验证

- [x] 生成可运行README四节/公开中文文档，包含最小消息、显式账号、条件更新失败示例；配置值均虚构。
- [x] README的「许可证」「数据覆盖与安全边界」承载本项目MIT许可声明、上游来源/固定commit、非官方说明与范围摘要；不要求额外上游MIT全文附录，后续引入受上游许可约束代码时仍需保留适用声明。详细T/E/X映射保留在 `research/core-scope.md`，不再维护三个独立说明文件或新建替代文档。
- [x] 历史验证（第一次文档精简，非现行全文要求）：文档精简后，主代理复核README来源/许可证全文/数据边界及三个文件删除，并通过文档、CI与打包检查；执行证据见validation.md。
- [x] 历史验证（第一次文档精简）：主代理实际统计确认crate为28文件、发布包为20文件，打包README与源码README逐字一致且包含完整MIT文本；不是仅按删除数量推算。
- [x] 最新调整：删除README额外上游MIT版权/许可全文，本项目MIT声明与来源保留，三个旧文件仍不存在；主代理通过本轮CI、all-features doctest/rustdoc、20文件dry-run隔离编译及包内README逐字一致检查。根LICENSE未改，workspace许可仍MIT；真实执行结果及有限来源核查局限见validation.md，不将历史MSRV/deny结果称为本轮重跑。
- [x] CHANGELOG记录新增z-ai及纯核心限制，不修改工作区统一版本。
- [x] reviewer检查API字段/泛型/错误/serde不变量/敏感输出与全部AC映射；缺陷修复后由同一reviewer复审，不能只凭实现代理自报完成。
- [x] 按trellis-check执行最终全范围验证；主代理检查命令真实退出码。

必须通过：

```text
cargo build --workspace
cargo xtask version check
cargo xtask ci
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

PowerShell文档额外检查：

```powershell
$env:RUSTDOCFLAGS = '-D warnings'
cargo doc --workspace --no-deps --document-private-items
cargo doc -p z-ai --no-deps --all-features
# 完成后恢复原RUSTDOCFLAGS
```

环境具备时单独检查并记录未安装/外部失败，不伪称运行成功：

```text
cargo +1.85 check -p z-ai --all-targets --all-features
cargo deny check
cargo publish -p z-ai --dry-run --locked --allow-dirty
cargo tree -p z-ai -e normal
```

提交前先检查拟打包文件，再使用带`--allow-dirty`的dry-run以允许本次未提交的新crate；始终保留`--dry-run`，绝不执行真正publish。依赖获取、审计和打包检查可能访问registry，其网络需求单独记录；语义/账号测试仍不访问真实服务或凭据。不安装工具/不联网更新模型目录来掩盖验证差异。工作区CI或feature检查真实失败必须修复或明确阻塞后再汇报。

## 6. 收尾与回滚

- [x] 按trellis-update-spec完成稳定契约同步：新增`z-ai-core-contracts.md`并更新相关索引，具体记录见validation.md。
- [x] 最终git diff检查通过，未提交凭据、未改源JS项目；完整验收与真实门禁证据已记录。
- [x] 实施未发生重大范围变化；今后如需重大范围或公开契约调整，仍须先同步prd/design/implement并重新获得最终评审批准。
- [ ] 用户已明确要求将当前变更提交到Git，按已展示的Phase 3.4范围执行功能提交；不推送，提交结果在收尾时记录。
- [ ] 提交完成后按Trellis流程归档任务；完整AC已达到，但不提前执行archive。
- [ ] 提交及归档后记录会话日志；当前保持待办。

回滚范围：本次新crate、根依赖/锁文件、CHANGELOG、包文档；不撤销无关文件或用户改动。无数据库/认证文件迁移，无部署回滚。
