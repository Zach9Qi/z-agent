# z-ai

`z-ai` 是一个与 `pi-ai` 核心聊天数据语义对齐的 Rust 纯核心库，并提供显式多账号与凭据条件更新能力。
输入输出均通过参数和返回值，不访问网络、文件、环境变量或实际时钟。

## 安装

本仓库的其他 workspace 成员可以直接使用：

```toml
[dependencies]
z-ai.workspace = true
```

其他本地项目可用路径依赖接入当前源码：

```text
cargo add z-ai --path /path/to/z-agent/crates/z-ai
```

默认不启用可选功能。
需要核心数据的自有格式序列化时，在依赖上启用 `features = ["serde"]`。
若宿主使用 `serde_json` 并要求有限浮点逐位往返一致，还应为自己的 `serde_json` 依赖启用 `float_roundtrip`，避免默认解析器出现尾数舍入偏差：

```toml
[dependencies]
z-ai = { path = "/path/to/z-agent/crates/z-ai", features = ["serde"] }
serde_json = { version = "1", features = ["float_roundtrip"] }
```

认证材料、账号与存储不自动序列化，也不导入原项目的 `auth.json`。

## 用法

### 开放标识

不同语义的标识不可混用，允许自定义 provider/API/model。
空或纯空白标识返回错误，不自动修改大小写或首尾空白。

```rust
use z_ai::identity::{AccountId, ProviderId};

let provider = ProviderId::new("example-provider".into())?;
let account = AccountId::new("work".into())?;
assert_eq!(provider.as_str(), "example-provider");
assert_eq!(account.as_str(), "work");
# Ok::<(), z_ai::Error>(())
```

### 同一 provider 的两个账号

本地账号是一份独立凭据配置，不等同于服务商侧的唯一用户。
两个账号可以对应同一远端用户，也可以使用不同 API Key 或 OAuth 授权。
示例中的凭据均为虚构值。

```rust
use z_ai::account::{Account, AccountStore};
use z_ai::auth::{AccountAuth, ApiKeyCredential, Credential, SecretString};
use z_ai::identity::{AccountId, ProviderId};

let provider = ProviderId::new("example-provider".into())?;
let work = AccountId::new("work".into())?;
let personal = AccountId::new("personal".into())?;
let mut accounts = AccountStore::new();

for (id, label, key) in [
    (work.clone(), "工作", "example-work-key"),
    (personal.clone(), "个人", "example-personal-key"),
] {
    let credential = ApiKeyCredential::new(Some(SecretString::new(key.into())));
    let auth = AccountAuth::new(Credential::ApiKey(credential));
    accounts.insert(Account::new(id, provider.clone(), label.into(), auth))?;
}

let work_snapshot = accounts.snapshot(&provider, &work)?;
let personal_snapshot = accounts.snapshot(&provider, &personal)?;
assert_ne!(work_snapshot.revision(), personal_snapshot.revision());
assert_eq!(accounts.list().len(), 2);
# Ok::<(), z_ai::Error>(())
```

`AccountId` 在一个存储实例内全局唯一，即使 provider 不同也不能重复。
所有账号相关操作必须显式指定身份；没有默认账号、环境回退、轮询或自动故障转移。
读取过期 OAuth 配置是允许的，因为宿主可能需要读取它再执行刷新；读取成功不代表已完成认证或可调用服务。

### 条件更新拒绝过期结果

快照包含一个版本的完整凭据和配置。
更新认证与相关配置需要提供读取时的版本；同一旧版本不能提交两次，也不能覆盖删除后同 ID 重建的账号。

```rust
use z_ai::account::{Account, AccountStore};
use z_ai::auth::{AccountAuth, ApiKeyCredential, Credential, SecretString};
use z_ai::identity::{AccountId, ProviderId};
use z_ai::Error;

let provider = ProviderId::new("example-provider".into())?;
let id = AccountId::new("work".into())?;
let mut accounts = AccountStore::new();
let auth = AccountAuth::new(Credential::ApiKey(ApiKeyCredential::new(Some(
    SecretString::new("example-initial-key".into()),
))));
accounts.insert(Account::new(id.clone(), provider.clone(), "工作".into(), auth))?;
let snapshot = accounts.snapshot(&provider, &id)?;

let replacement = AccountAuth::new(Credential::ApiKey(ApiKeyCredential::new(Some(
    SecretString::new("example-replacement-key".into()),
))));
accounts.replace_auth(&provider, &id, snapshot.revision(), replacement.clone())?;
assert!(matches!(
    accounts.replace_auth(&provider, &id, snapshot.revision(), replacement),
    Err(Error::RevisionConflict)
));
# Ok::<(), z_ai::Error>(())
```

版本保护仅对同一存储实例的连续生命周期有效；不是跨进程事务协议。
本地条件更新不协调远端 OAuth 刷新。
若远端 token 已轮换而本地提交冲突，旧 token 可能已失效，宿主需要协调写入或恢复/重新授权。
删除账号不会撤销已交给调用方的快照、远端授权或在途请求。

### 数据覆盖与安全边界

- 包含模型、消息、工具/schema 描述、usage/cost、通用选项及 assistant 事件数据；具体字段和约束见各公开类型的 API 文档。
- 对齐数据含义，不保证上游 JSON 字段名、标签或编码格式兼容；provider 专有兼容配置与 options 通过 JSON 扩展承载，不等于完整 SDK 类型或协议行为移植。
- text/thinking/tool 签名原样保留；reasoning 是 output 的子集，cacheWrite1h 是 cacheWrite 的子集，不重复累计，也不重算服务端报告的 total；工具用量独立。
- 事件的部分响应由宿主提供句柄，可保持共享累积语义；本库不实现队列、异步流运行时、事件顺序检查或 partial JSON 修复。
- 时间为 Unix 毫秒，计数/时长非负；浮点拒绝非有限数，费用非负；保留未报告与 `0`/`false`/显式 JSON null 的区别。
- 凭据及已知敏感配置的 `Debug` 默认脱敏；账号摘要中的 ID/provider/名称应由调用方作为非敏感标签管理。
- 消息、诊断、任意 JSON 及 `serde` 输出不是自动脱敏日志；显式读取秘密之后的使用由宿主负责。
- 不提供凭据加密、内存清零或操作系统密钥环；不包含网络、OAuth 登录/刷新、文件存储、计价算法、schema 执行或图片生成接口，聊天中的图片数据仍在范围内。

## 最低支持的 Rust 版本 (MSRV)

Rust **1.85**，edition 2024。
默认与 `serde` 特性均遵循同一 MSRV。

## 许可证

MIT。
核心数据语义参考 [earendil-works/pi 的 packages/ai](https://github.com/earendil-works/pi/tree/1148f7958c260188b03488496044dcfa852ef580/packages/ai)，本地多账号模型独立设计。
参考源码基线为 `1148f7958c260188b03488496044dcfa852ef580`，包 manifest 版本为 `0.84.4`；不表示与该正式发布包或后续版本完全等价。
本库不是上游官方 Rust SDK，也不是完整客户端替代品。
