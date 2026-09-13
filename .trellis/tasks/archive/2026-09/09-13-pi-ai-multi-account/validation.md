# z-ai 实施与验证记录

## 提交与归档结果

- 用户明确要求提交当前变更，工作提交为`0bf5190a500672075457352d7ec9eff3bcf5e49d`：`feat(ai): 新增核心数据模型与显式多账号支持`。
- 提交前再次执行`cargo xtask ci`通过，typos因本机未安装跳过；`git diff --check`与任务上下文清单验证通过。
- 任务已归档至`.trellis/tasks/archive/2026-09/09-13-pi-ai-multi-account/`，task.json状态completed，记录工作commit；同步修复归档后研究清单和spec引用。
- 未push、未真正publish；会话journal按`.trellis/config.yaml`与`.gitignore`仅保存在本地，不强制跟踪。
- 下方为实施各阶段的历史记录，其中尚待提交/待授权等表述描述当时状态，以本节及归档task.json为最终状态。

## 历史：交付状态（提交前）

- 用户在最终规划摘要后回复「ok」，任务已从planning启动为in_progress。
- 工作分支：`feat/z-ai-core`；源JS基线仍为`1148f7958c260188b03488496044dcfa852ef580`，未修改源项目或读取真实凭据。
- `cargo xtask new z-ai`生成单库；core/model/accounts分别实现基础/消息/事件、模型/工具/选项、认证/账号。
- 初次验收时新库31个文件，包括实现、10个集成测试文件、README/SEMANTICS/NOTICE/上游MIT文本与清单；这是文档精简前的历史统计，不代表当前文件数量。
- 产品实现与初次验收已完成；用户随后批准仅做文档精简，最新要求是删除README额外附加的上游MIT全文，保留本项目MIT许可声明、上游来源/固定commit、非官方说明与数据边界。不改根LICENSE、Cargo许可声明、Rust实现或功能/协议边界；本轮实际验证见「当前调整」段。用户已明确要求将当前变更提交到Git；本轮按已展示范围执行功能提交与Trellis归档，不推送或真正publish，提交结果在收尾时补记。

## 验收映射

| 条件 | 证据 |
| --- | --- |
| AC1 核心类型 | identity/value/message/model/tool/options/event及各自公开API测试覆盖T矩阵；E层保留专有JSON值，X行为不实现 |
| AC2 不丢关键语义 | message_data/event_data/model_data/serde_data覆盖opaque签名、redaction三态、响应元数据、失败partial、usage子集/total、shared-live partial |
| AC3 多账号 | accounts.rs的2key+2OAuth同provider、同远端用户多配置、跨provider重复ID及显式选择错误 |
| AC4 条件写入 | accounts.rs、account私有耗尽单测；adversarial_contracts.rs额外36场景逐次检查全store/版本不变，以及4次整体配置替换与5代快照 |
| AC5 OAuth/快照边界 | 空refresh、缺key配置可保存不代表已认证；旧快照不变，删除不撤销外部值；README的账号更新说明/数据边界及任务research/core-scope.md明确宿主责任 |
| AC6 秘密保护 | secret_redaction.rs检查内层/外层/列表/存储/错误的普通与美化Debug；无自动凭据serde/SecretString Display/Store Clone |
| AC7 构造与serde | ID/finite/费用/usage/终态校验，Option泛型details存在性，非JSON反序列化输入，所有feature门禁 |
| AC8 工作区质量 | 初次门禁各命令真实退出0；当前文档入口为README本项目MIT许可声明/上游来源与固定commit/非官方说明/数据边界、任务research/core-scope.md与spec契约；本轮实际门禁见「当前调整」，不要求额外上游MIT全文附录 |

## 初次验收真实门禁（文档精简前）

由主代理PowerShell前台执行并检查每条退出码，以下均exit=0。保留当时文件清单及数量，不将旧结果改写为精简后验证：

| 命令 | 结果 |
| --- | --- |
| `cargo build --workspace --locked` | 通过 |
| `cargo xtask ci` | fmt、默认clippy、workspace测试、rustdoc通过；typos未安装，脚本明确跳过 |
| `cargo test -p z-ai --no-default-features --locked` | 62个unit/integration + 7个doctest（含2个compile-fail） |
| `cargo test -p z-ai --all-features --locked` | 84个unit/integration + 7个doctest；当前唯一非默认feature为serde |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | 通过 |
| `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps --all-features --document-private-items --locked` | 通过；执行后恢复环境值 |
| `cargo +1.85.0 check --workspace --all-targets --all-features --locked --offline` | 已安装的最低Rust 1.85.0通过 |
| `cargo xtask version check` | 根/z-ai/xtask及内部依赖统一0.1.0 |
| `cargo package -p z-ai --list --allow-dirty --locked` | 含README/SEMANTICS/NOTICE/LICENSE-PI-AI与所有src |
| `cargo publish -p z-ai --dry-run --locked --allow-dirty` | 打包23文件，隔离编译通过，明确aborting upload due to dry run；未上传 |
| `cargo deny check` | advisories/bans/licenses/sources全部ok |
| `cargo tree -p z-ai -e normal --locked`及all-features | 默认仅thiserror，serde可选；无网络/运行时依赖 |
| `git diff --check` | 通过 |
| `python .trellis/scripts/task.py validate .trellis/tasks/09-13-pi-ai-multi-account` | implement/check清单13/12条有效引用 |

默认workspace还运行xtask原有23个测试，全部通过；不要把它们计入上述z-ai数字。
默认feature下serde专用测试目标报告0 tests是条件编译预期，在all-features中真实执行。
打包提示未包含集成测试是清单白名单的有意行为，不是省略了本地测试。
依赖获取/审计/打包可能访问公开registry或advisory database；语义/账号测试不访问模型服务或真实账号。

## 历史：第一次文档精简后验证完成

以下记录保留当时将上游MIT全文并入README的检查结果，仅作为历史，不代表当前README内容或恢复全文附录的要求；后续删除附录的决定与验证见「当前调整」。

- 用户批准删除 `crates/z-ai/LICENSE-PI-AI`、`NOTICE.md`、`SEMANTICS.md` 三个独立文件；来源、版权及完整MIT文本并入现有README的「许可证」，范围摘要保留在「数据覆盖与安全边界」，详细T/E/X映射仍见任务 `research/core-scope.md`。不创建替代独立文档，不改变功能或协议边界。
- 此段先记录待验证事项，现已依据主代理提供的真实执行结果补齐；本次文档同步代理未运行cargo，避免争锁，也不冒称亲自执行下列门禁。
- 主代理在删除前逐字确认原LICENSE全文仍在README中；精简未修改Rust实现。精简后的实际crate文件数为28、发布包为20，以下包含统计和打包证据，不是仅按删除数量推算。

| 本次检查 | 主代理提供的实际结果 |
| --- | --- |
| `cargo xtask ci` | 前台exit=0；默认z-ai 62个unit/integration + 7个doctest，xtask原有23个测试通过；typos未安装，照旧明确跳过 |
| all-features doctest | 7个通过 |
| all-features rustdoc，`RUSTDOCFLAGS=-D warnings` | 通过 |
| `cargo package --list` | 实际20文件，三个旧文件名均不在包内 |
| `cargo publish --dry-run` | 实际打包20文件、隔离编译通过，明确aborting upload due to dry run；未发布 |
| Python文件及包内容检查 | 源crate实际28文件；三个文件确实不存在；manifest include仅src、Cargo.toml、README；包内README与源文件逐字一致并保留上游MIT版权及全文 |
| `git diff --check` | 通过 |

此前MSRV、deny及其他未列入本次重跑项的结果仍只属于上方初次门禁历史，不冒称本次重新执行。主代理最终扫描确认：crates/z-ai无三个旧文件名引用，.trellis仅本文件的初次历史及删除变更记录提及旧文件，不存在现行链接；Trellis task validate的implement/check清单13/12条通过，git diff --check通过。

## 当前调整：删除README额外上游MIT全文

### 范围与现行要求

- 用户最新要求为「不是必要的就删掉吧」。本轮删除README的额外上游MIT全文附录，保留本项目MIT许可声明、上游来源/固定commit `1148f7958c260188b03488496044dcfa852ef580`、非官方说明与数据边界；不修改根LICENSE、Cargo许可声明、Rust代码或产品边界，不新增许可/来源文件，不提交或归档。
- `.trellis/spec/crates/library/z-ai-core-contracts.md`及当前任务prd/design/implement的现行要求同步为上述边界，避免根据旧验证记录自动恢复全文附录。旧验证事实和数字保留并标为历史。
- 此次调整仅针对当前额外全文附录，不豁免适用的许可义务。后续若引入受上游许可约束的代码，仍须保留适用的版权与许可声明。

### 有限来源核查及局限

以下为主代理提供的只读核查结果，由文档同步代理记录；文档同步代理未独立重复来源核查。

- 就当前检查范围而言，更符合参考接口和数据语义进行Rust独立建模；未发现非平凡实现、大段注释或复杂fixtures直接移入。
- `event.rs`中关于redacted在start已完整提供、没有delta，以及工具初始参数由provider决定的短功能注释，与上游`packages/ai/src/types.ts:542-544`契约相应。
- 简短Lark示例`start: /[a-z]+/`相同，但属于常见示例，测试主体用途不同。
- 账号CAS实现不是源provider的Promise队列算法翻译。
- 核查仅覆盖当前可见源码及上述对应关系，未做创作历史取证，不构成法律结论，也不对所有可能表达复用或未来引入的内容作排除保证。

### 本轮实际验证

主代理前台执行并提供以下实际结果；文档同步代理未运行cargo，避免争锁，不冒称亲自执行这些门禁。

| 本次检查 | 主代理提供的实际结果 |
| --- | --- |
| `cargo xtask ci` | exit=0；z-ai默认62个unit/integration + 7个doctest、xtask原有23个测试通过；typos缺失，照旧明确跳过 |
| all-features doctest | 7个通过 |
| all-features rustdoc，`RUSTDOCFLAGS=-D warnings` | 通过 |
| `cargo publish --dry-run` | 打包20文件、隔离编译通过，明确aborting upload due to dry run；未发布 |
| Python README及包内容检查 | README额外上游版权/许可正文已删除，本项目MIT字样与来源仍在；三个旧文件仍不存在；包内README与源码逐字一致 |
| Python根许可检查 | 根LICENSE仍含Zach9Qi版权；workspace license仍为MIT |
| `git diff --exit-code -- LICENSE` | exit=0，根LICENSE未修改 |
| `git diff --check` | exit=0 |

文档同步代理另行执行五份文档的现行要求/历史记录/空白自检、Trellis task validate（implement/check为13/12条）及`git diff --check`，均通过。自检初次因PowerShell管道中文编码导致断言失败，显式设置UTF-8并恢复原环境后重跑通过；不是产品门禁失败。

MSRV、deny及其他未列入本轮的命令仍只属于先前历史，未在本轮重新执行。语义/账号测试不访问真实模型服务或凭据；dry-run可能访问公开registry，但未上传。提交、归档仍须主代理按用户授权另行处理。

## 历史：独立审查与测试

- 持久reviewer先审稳定model/tool，再逐文件全审src/tests/manifest/docs，后复审spec和新增adversarial测试；最终无具体缺陷或未解决阻塞。执行门禁证据由主代理提供，不冒称reviewer亲自执行。
- 独立tester因工具白名单仅允许TS测试路径，未绕过限制。其完整Rust黑盒测试源码通过报告提交，由主代理正常写入`tests/adversarial_contracts.rs`并格式化。
- tester随后亲自以`--locked --offline`运行默认/serde目标2个测试及两种feature的目标clippy，均通过；没有触发实现缺陷，不宣称经历RED→GREEN修复。
- 这2个测试已包括在最终62/84总数中，不额外重复累计。

## 已解决问题与知识沉淀

- 初期worker模块已完成但基础lib/feature尚未接线，曾产生编译失败；接线后全部重跑通过，不能使用早期失败当最终状态。
- 确定性浮点样本发现serde_json默认解析出现1 ULP差异。根依赖开启`float_roundtrip`，无版本升级；1024个位模式及边界逐位往返通过，README提醒宿主也需正确配置后端。
- 新增`.trellis/spec/crates/library/z-ai-core-contracts.md`，7节记录真实签名、CAS/ABA/serde/敏感信息契约、错误矩阵和测试；库索引与通用Rust索引更新真实代码入口。

## 环境限制与后续

- 本机无typos，未安装或伪称已做拼写检查；远端CI仍需执行该项。
- 后台命令包装器曾返回exit0但0秒/空日志，未作为证据；所有相关命令随后在前台真实重跑并取得输出。
- coworker独立gate启动器曾报ENOENT，主代理使用可执行的PowerShell路径验证；不把未启动的gate当通过。
- 根repository元数据仍保留模板URL，此任务没有猜测真实远端或修改发布身份；真正首次发布前应核对。此次仅dry-run，不发布。
- 提交须用户确认。提交后再按Trellis工作流archive与记录journal；不提前归档或生成提交。
