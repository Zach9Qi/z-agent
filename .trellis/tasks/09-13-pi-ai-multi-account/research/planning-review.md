# 规划审查记录

## 第一轮：多账号最小契约

只读审查员 plan-review 结论：单 crate、纯内存 store、显式账号和条件写入足够，不需要运行时、存储trait、刷新域注册表或调度层。

需要进入最终设计的修正：

1. CAS仅承诺不覆盖本地新状态。即使网络刷新single-flight，期间修改显示名仍可能令版本失效；远端轮换成功后不能保证旧token仍有效。宿主协调所有账号写入或恢复/重新授权，核心不得自动重试或强制覆盖。
2. 全局计数器只分配新revision；CAS比较目标账号revision，修改B不令A失效。新增拒绝重复、更新不upsert、删除重建版本不同。checked_add耗尽不改状态。
3. ABA保证只在同一store实例连续生命周期内；不跨新建/重置/恢复/复制。store不自动Clone/Deserialize，账号不可通过可变引用改ID/provider/revision。
4. 称AccountSnapshot而非ResolvedRequestAuth，完整保存凭据及配置。删除不撤销外部快照、远端授权或在途请求。
5. OAuth允许空refresh/不固定远端accountId；宿主提交处理后的完整凭据，核心不合并原始刷新HTTP响应。ApiKey/OAuth范围不冒称覆盖AWS/ADC。
6. 无密钥列表采用白名单ID/provider/显示名/种类/revision；headers/env/extensions/URL同样可能有秘密，连同内层类型、快照、store、错误Debug/Display纳入哨兵测试。
7. 不需要再追问D1–D4；核心类型覆盖清单可随最终规划一次确认。research/upstream-auth.md中的锁内刷新/共享刷新域属于未来宿主风险，不升级为本次核心承诺。

建议测试：2 key+2 OAuth共存、跨provider重复本地ID、A读/B改/A提交、同版本两次提交、重授权/删除/同ID重建的旧结果拒绝、失败不变、快照稳定、本地元数据保留、版本耗尽、OAuth边界、各层普通/美化Debug脱敏、serde开关。

## 第二轮：最终规划

审查员结论：无blocking，可提交用户最终范围批准，不代表批准实施。抽查源types/auth/diagnostics，未发现明确T层遗漏；D1–D4、T/E/X范围、CAS/ABA/快照/秘密保护/serde校验契约一致。

三项非阻塞修正已落实：

- PRD把「完全离线」限定为语义与账号测试；依赖获取、审计、打包的联网需求单独记录。
- 账号门禁改为`cargo test -p z-ai --test accounts`，再跑完整库测试覆盖私有单元测试，避免名称过滤导致零测试假绿。
- 提交前打包先核对文件，使用`cargo publish -p z-ai --dry-run --locked --allow-dirty`；永不实际publish。

实施提醒：默认Option<JsonValue>反序列化会折叠missing/null，必须通过字段级处理和往返测试保留区别。

检查证据：主代理PowerShell执行`task.py validate`成功，implement/check清单分别13/12个有效引用；9个Markdown文件内容/路径/空白检查通过，任务仍planning。coworker的独立gate启动器报ENOENT，未实际执行命令，不能当作校验通过；以主代理可执行环境的真实验证为准。

第三轮最终复核已完成：三项文案/命令修正均确认，无未解决项。主代理再次运行task.py validate及文档/引用/状态/未生成产品crate检查成功。可提交用户最终范围批准；审查通过不等于实施批准，状态保持planning。
