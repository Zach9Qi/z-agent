# 认证与多账号调研

源根：`C:/Users/chenziqi/Desktop/codes/js/pi`。下文 `ai/` 与 `coding-agent/` 相对源根 `packages/`。只读源码/测试，未读取实际 auth.json 或 secrets。

## 单账号限制的实际位置

- `ai/src/auth/types.ts:36-93` 明确 one credential per provider；CredentialStore 的 read/modify/delete 都仅以 providerId 为键，CredentialInfo 无本地账号 ID。
- `ai/src/auth/credential-store.ts:9-25` 内存 Map 与串行队列均以 provider 为键。
- `ai/src/models.ts:570-626` login 覆盖 provider 槽，logout 删除槽；同 provider 的 API key/OAuth 也互相替换。
- `coding-agent/src/core/auth-storage.ts:17,449-478` 使用 Record<string, Credential>，按 provider 写回文件。
- `coding-agent/src/core/runtime-credentials.ts:4-35` 临时 key 同样按 provider；选择器 `coding-agent/src/modes/interactive/components/oauth-selector.ts:14-19,47-54` 只有 providerId/authType。
- 底层 OAuth.refresh/toAuth 接受具体凭据，请求可携带具体认证，并非协议本身禁止多账号。问题是同一集合内没有账号命名/共存/选择/隔离契约。

认证链：ModelRuntime → RuntimeCredentials/AuthStorage → Models.getAuth → resolveProviderAuth → key/env/ambient 或锁内 OAuth refresh → AuthResult → 请求 model/options → provider/API。

## 不能只返回一个 key 字符串

- 当前接口是 Models.getAuth、OAuthAuth.toAuth；旧 oauth.ts 仅兼容导出。扩展 getApiKey 经 provider-composer 转为 toAuth：`coding-agent/src/core/provider-composer.ts:245-263`。
- ModelAuth 包含 apiKey/headers/baseUrl；AuthResult 还带 env/source：`ai/src/auth/types.ts:3-19,104-112`。
- Kimi OAuth 返回 Authorization header；Copilot 返回 key 和账号相关 baseUrl：`ai/src/auth/oauth/kimi-coding.ts:293-295`、`github-copilot.ts:500-505`。
- 请求级 options 目前逐字段覆盖解析结果，transformHeaders 最后执行：`ai/src/models.ts:641-669`；多账号不能混用 A 的 token 与 B 的 header/endpoint/project。

## 领域候选（尚未批准）

- ProviderDefinition：服务/协议/模型及认证方法，不是用户身份。
- AccountRecord：稳定本地 AccountId、ProviderId、显示名、凭据、账号级配置、revision；独立于远端身份字段。
- Credential：ApiKey/OAuth；是否覆盖 ambient/external sources 取决于范围确认。
- RequestAccountSelection：显式账号或明确默认/策略，策略尚待用户决定。
- ResolvedRequestAuth：绑定 AccountId 与 revision 的请求认证/配置快照。

拟议最低不变量：账号稳定绑定 provider；更新/注销 A 不动 B；API key 与 OAuth 可共存；显式选择失败不隐式换身份；请求配置来源一致；同刷新域串行且锁内重读；旧刷新结果不能复活已删除或覆盖重登录账号；列表与 Debug 不泄露凭据。本地标签/ID 不能塞进 refresh 返回值，多个 provider 会重建 OAuth credential 导致丢失。

若同一 OAuth 授权被复制到两条本地账号记录，仅按 AccountId 锁不够；须拒绝重复授权或共享刷新域。尚未定义支持多进程与否。

## provider 特例

| Provider | 语义 | 实证 |
| --- | --- | --- |
| Codex | accountId 是 JWT 内远端 chatgpt_account_id，登录/刷新都提取；不能充当本地 ID | `ai/src/auth/oauth/openai-codex.ts:396-415`；`ai/src/api/openai-codex-responses.ts:1583-1612` |
| Copilot | refresh 字段实际保存 GitHub token；短期 token、enterpriseUrl、availableModelIds、proxy-ep 端点均随账号 | `ai/src/auth/oauth/github-copilot.ts:64-86,312-366`；`ai/src/providers/github-copilot.ts:19-26` |
| Cloudflare | key 外还有 account/gateway ID，env 可按字段补齐；Gateway header 特殊 | `ai/src/providers/cloudflare-auth.ts:10-48,75-100` |
| Vertex | API key/ADC/service account；project/location/keyFilename 是资源与配置，不是登录身份 | `ai/src/providers/google-vertex.ts:15-85`；`ai/src/api/google-vertex.ts:420-453` |
| Bedrock | bearer 或 AWS credentials/profile，不是普通 OAuth | `ai/src/providers/amazon-bedrock.ts:11-78`；`ai/src/api/bedrock-converse-stream.ts:144-206` |
| Radius/Kimi | gateway/issuer 在 provider 配置或环境；多环境账号需隔离授权服务器配置 | `ai/src/auth/oauth/radius.ts:133-139,357-396`；`kimi-coding.ts:36-38,288-290` |
| OpenRouter/xAI | OpenRouter 可为永久 key、空 refresh；xAI 刷新响应可不返回新 refresh token | `ai/src/auth/oauth/openrouter.ts:127-132,305-309`；`xai.ts:128-142` |

当前没有通用 tenantId 契约；Azure 当前实现为 API key，不能擅自宣称已支持 tenant OAuth。

## 优先级与竞争风险

- 典型优先级：请求显式 key → 上层 runtime key → stored credential → 扩展/models.json fallback → provider env/ambient。但 stored key 的缺字段可从 env 补齐；OAuth 刷新失败不退到 env：`ai/src/auth/resolve.ts:73-153`；`ai/test/models-runtime.test.ts:858,927`。
- 请求认证提前至少 5 分钟刷新，15 秒超时；部分 provider 保存 expires 时另减 5 分钟。目录刷新路径不同：`ai/src/models.ts:452-477`。
- 内存 provider 锁与上层整文件锁不同；AuthStorage 持有文件锁执行网络回调，多账号可能互相阻塞：`coding-agent/src/core/auth-storage.ts:115-190`。
- 服务端轮换成功后取消/本地写失败，旧 token 可能已不可用；取消不等于回滚远端。当前测试只承诺旧本地值未改：`ai/test/models-runtime.test.ts:735-775`。
- Codex WebSocket 已按 session + 远端 accountId 缓存，但本地账号/URL/revision 仍需审查；fallback/debug 状态按 session，目录缓存按 provider，DeferredHandle 没有本地账号：`ai/src/api/openai-codex-responses.ts:876-878,1142-1170`；`ai/src/models.ts:380-390`；`ai/src/types.ts:410`；`ai/test/openai-codex-stream.test.ts:1375`。
- AuthStorage read 会解析 $ENV/!command；命令缓存按字符串而非账号：`coding-agent/src/core/auth-storage.ts:441-446`；`resolve-config-value.ts:9,145-150,208-223`。
- 原文件明文 JSON，尝试 0600/0700，但写入并非此处可见的原子替换；不等同可靠加密/跨平台 ACL 方案：`coding-agent/src/core/auth-storage.ts:24-25,56-65,185-187`。
- Kimi unauthorized 注释声称清理凭据，但实现/测试保留凭据并报错；应以实现与测试核对。

## 待用户决定

首版移植范围；账号是命名凭据配置还是远端自然人；显式选择/默认/自动调度；IO与持久化归属；必需 providers；JSON与旧auth迁移；并发及注销语义。推荐不默认跨账号重试，但尚未获批准。
