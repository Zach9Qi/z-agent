//! 认证凭据与完整账号配置的数据契约，不执行登录、刷新或请求认证解析。
//! 所有认证类型均不自动序列化，调试输出对敏感容器整体脱敏；显式读取后由宿主管理秘密。

use std::fmt;

use crate::{
    identity::UnixMillis,
    options::{ProviderEnv, ProviderHeaders},
    value::JsonObject,
};

/// 只能通过明确命名的方法读取的秘密字符串。
///
/// `Debug` 不输出内容，不实现 `Display` 或自动序列化；不承诺内存清零、加密或阻止调用方主动泄露。
/// 空字符串合法，例如某些 OAuth 凭据的刷新令牌为空。
///
/// # Examples
///
/// ```
/// use z_ai::auth::SecretString;
/// let secret = SecretString::new("fictional-key".to_owned());
/// assert_eq!(secret.expose_secret(), "fictional-key");
/// assert!(!format!("{secret:?}").contains("fictional-key"));
/// ```
///
/// 不能通过通用展示接口意外输出秘密：
///
/// ```compile_fail
/// use z_ai::auth::SecretString;
/// let secret = SecretString::new("fictional-key".to_owned());
/// let output = format!("{secret}");
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct SecretString(String);

impl SecretString {
    /// 接收由调用方拥有的秘密内容，不执行格式验证或配置解析。
    #[must_use]
    pub fn new(value: String) -> Self {
        Self(value)
    }

    /// 显式借用秘密原文；调用方不得把返回值写入不受保护的日志。
    #[must_use]
    pub fn expose_secret(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretString([REDACTED])")
    }
}

/// 可用于无密钥摘要的认证种类，不表明凭据当前有效或可调用。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum CredentialKind {
    /// API Key 配置，允许尚未提供密钥。
    ApiKey,
    /// OAuth 凭据，不代表本库执行 OAuth 流程。
    OAuth,
}

/// API Key 凭据及其 provider 配置数据。
///
/// 缺失密钥与空密钥不同，二者均可表达；缺失时不会读取环境或自动补齐。
/// `env` 保留上游存储凭据携带的配置；本库不合并它与账号层配置。
#[derive(Clone, PartialEq, Eq)]
pub struct ApiKeyCredential {
    key: Option<SecretString>,
    env: Option<ProviderEnv>,
}

impl ApiKeyCredential {
    /// 构造显式存在或缺失密钥的凭据，不判断其远端有效性。
    #[must_use]
    pub fn new(key: Option<SecretString>) -> Self {
        Self { key, env: None }
    }

    /// 设置凭据携带的 provider 配置；空映射与未提供不同。
    #[must_use]
    pub fn with_env(mut self, env: ProviderEnv) -> Self {
        self.env = Some(env);
        self
    }

    /// 借用可选密钥，读取原文仍需显式调用 [`SecretString::expose_secret`]。
    #[must_use]
    pub fn key(&self) -> Option<&SecretString> {
        self.key.as_ref()
    }

    /// 显式借用凭据配置；返回的映射可能包含秘密，不应直接记录日志。
    #[must_use]
    pub fn env(&self) -> Option<&ProviderEnv> {
        self.env.as_ref()
    }
}

impl fmt::Debug for ApiKeyCredential {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ApiKeyCredential")
            .field("key", &"[REDACTED]")
            .field("env", &"[REDACTED]")
            .finish()
    }
}

/// 宿主已经处理好的完整 OAuth 凭据。
///
/// 允许空刷新令牌，不固定远端账号字段。`expires` 是宿主提供的 Unix 毫秒时间，
/// 本库不读取时钟或判断过期。若原始刷新响应省略刷新令牌，宿主必须自行保留旧值再提交。
#[derive(Clone, PartialEq)]
pub struct OAuthCredential {
    access: SecretString,
    refresh: SecretString,
    expires: UnixMillis,
    extensions: JsonObject,
}

impl OAuthCredential {
    /// 构造完整令牌数据；不执行登录、刷新或原始 HTTP 响应合并。
    #[must_use]
    pub fn new(access: SecretString, refresh: SecretString, expires: UnixMillis) -> Self {
        Self {
            access,
            refresh,
            expires,
            extensions: JsonObject::new(),
        }
    }

    /// 设置 provider 扩展，允许保存远端用户、组织、端点等数据而不将其视为本地身份。
    #[must_use]
    pub fn with_extensions(mut self, extensions: JsonObject) -> Self {
        self.extensions = extensions;
        self
    }

    /// 借用访问令牌，读取原文仍需显式调用 [`SecretString::expose_secret`]。
    #[must_use]
    pub fn access(&self) -> &SecretString {
        &self.access
    }

    /// 借用刷新令牌，允许其秘密原文为空。
    #[must_use]
    pub fn refresh(&self) -> &SecretString {
        &self.refresh
    }

    /// 返回宿主提供的过期时间，不执行当前时间比较。
    #[must_use]
    pub fn expires(&self) -> UnixMillis {
        self.expires
    }

    /// 显式借用 provider 扩展；键和值均可能含秘密，不应直接记录日志。
    #[must_use]
    pub fn extensions(&self) -> &JsonObject {
        &self.extensions
    }
}

impl fmt::Debug for OAuthCredential {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OAuthCredential")
            .field("access", &"[REDACTED]")
            .field("refresh", &"[REDACTED]")
            .field("expires", &self.expires)
            .field("extensions", &"[REDACTED]")
            .finish()
    }
}

/// 同一 provider 下可以混合共存的凭据类型。
#[derive(Clone, PartialEq)]
#[non_exhaustive]
pub enum Credential {
    /// API Key 凭据及可选配置。
    ApiKey(ApiKeyCredential),
    /// 宿主处理后的完整 OAuth 凭据。
    OAuth(OAuthCredential),
}

impl Credential {
    /// 返回不包含秘密的认证种类。
    #[must_use]
    pub fn kind(&self) -> CredentialKind {
        match self {
            Self::ApiKey(_) => CredentialKind::ApiKey,
            Self::OAuth(_) => CredentialKind::OAuth,
        }
    }
}

impl fmt::Debug for Credential {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Credential")
            .field("kind", &self.kind())
            .finish_non_exhaustive()
    }
}

/// 与完整凭据一起原子替换的账号认证配置。
///
/// headers、端点、环境数据与扩展都可能包含秘密，因此 `Debug` 整块脱敏。
/// 映射只保存数据，不解析环境、不执行合并，也不将 OAuth 自动转换为 HTTP 认证。
#[derive(Clone, PartialEq)]
pub struct AccountAuth {
    credential: Credential,
    headers: Option<ProviderHeaders>,
    base_url: Option<String>,
    env: Option<ProviderEnv>,
    extensions: JsonObject,
}

impl AccountAuth {
    /// 构造仅含完整凭据的账号认证配置。
    #[must_use]
    pub fn new(credential: Credential) -> Self {
        Self {
            credential,
            headers: None,
            base_url: None,
            env: None,
            extensions: JsonObject::new(),
        }
    }

    /// 设置完整账号 headers；缺键不覆盖，值为 `None` 表示删除该 header。
    #[must_use]
    pub fn with_headers(mut self, headers: ProviderHeaders) -> Self {
        self.headers = Some(headers);
        self
    }

    /// 设置账号端点；不验证或访问 URL。
    #[must_use]
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = Some(base_url);
        self
    }

    /// 设置账号级环境或 provider 配置数据，不读取进程环境。
    #[must_use]
    pub fn with_env(mut self, env: ProviderEnv) -> Self {
        self.env = Some(env);
        self
    }

    /// 设置完整账号扩展，不执行其中的 provider 行为。
    #[must_use]
    pub fn with_extensions(mut self, extensions: JsonObject) -> Self {
        self.extensions = extensions;
        self
    }

    /// 借用完整凭据，不执行刷新或认证解析。
    #[must_use]
    pub fn credential(&self) -> &Credential {
        &self.credential
    }

    /// 显式借用账号 headers；键和值均可能包含秘密。
    #[must_use]
    pub fn headers(&self) -> Option<&ProviderHeaders> {
        self.headers.as_ref()
    }

    /// 显式借用账号端点；URL 可能包含秘密。
    #[must_use]
    pub fn base_url(&self) -> Option<&str> {
        self.base_url.as_deref()
    }

    /// 显式借用账号环境数据；返回值不自动脱敏。
    #[must_use]
    pub fn env(&self) -> Option<&ProviderEnv> {
        self.env.as_ref()
    }

    /// 显式借用账号扩展；返回值不自动脱敏。
    #[must_use]
    pub fn extensions(&self) -> &JsonObject {
        &self.extensions
    }
}

impl fmt::Debug for AccountAuth {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AccountAuth")
            .field("credential", &self.credential)
            .field("headers", &"[REDACTED]")
            .field("base_url", &"[REDACTED]")
            .field("env", &"[REDACTED]")
            .field("extensions", &"[REDACTED]")
            .finish()
    }
}

/// 宿主为模型请求准备的认证数据，不代表本库已经解析选定账号。
///
/// 保留可选 API Key、headers 和端点；本库不提供从 OAuth 到此类型的转换器。
#[derive(Clone, PartialEq, Eq, Default)]
pub struct ModelAuth {
    api_key: Option<SecretString>,
    headers: Option<ProviderHeaders>,
    base_url: Option<String>,
}

impl ModelAuth {
    /// 构造所有认证字段均未提供的数据。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置显式的模型请求密钥。
    #[must_use]
    pub fn with_api_key(mut self, api_key: SecretString) -> Self {
        self.api_key = Some(api_key);
        self
    }

    /// 设置模型请求 headers，保留缺失、设置和删除的区别。
    #[must_use]
    pub fn with_headers(mut self, headers: ProviderHeaders) -> Self {
        self.headers = Some(headers);
        self
    }

    /// 设置模型请求端点，不访问或验证 URL。
    #[must_use]
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = Some(base_url);
        self
    }

    /// 借用可选的模型请求密钥。
    #[must_use]
    pub fn api_key(&self) -> Option<&SecretString> {
        self.api_key.as_ref()
    }

    /// 显式借用模型请求 headers；返回值可能包含秘密。
    #[must_use]
    pub fn headers(&self) -> Option<&ProviderHeaders> {
        self.headers.as_ref()
    }

    /// 显式借用模型请求端点；返回值可能包含秘密。
    #[must_use]
    pub fn base_url(&self) -> Option<&str> {
        self.base_url.as_deref()
    }
}

impl fmt::Debug for ModelAuth {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ModelAuth")
            .field("api_key", &"[REDACTED]")
            .field("headers", &"[REDACTED]")
            .field("base_url", &"[REDACTED]")
            .finish()
    }
}

/// 宿主已经准备的模型认证、环境配置与来源描述的纯数据结果。
///
/// 来源描述也整体脱敏，因为它可能包含文件路径或其他敏感信息。
/// 本库不执行凭据解析、默认选择、环境回退或网络认证。
#[derive(Clone, PartialEq, Eq)]
pub struct AuthResult {
    auth: ModelAuth,
    env: Option<ProviderEnv>,
    source: Option<String>,
}

impl AuthResult {
    /// 保存宿主提供的模型认证数据。
    #[must_use]
    pub fn new(auth: ModelAuth) -> Self {
        Self {
            auth,
            env: None,
            source: None,
        }
    }

    /// 设置宿主准备的 provider 环境数据，不读取进程环境。
    #[must_use]
    pub fn with_env(mut self, env: ProviderEnv) -> Self {
        self.env = Some(env);
        self
    }

    /// 设置可供宿主显式展示的来源描述，不自动写入调试输出。
    #[must_use]
    pub fn with_source(mut self, source: String) -> Self {
        self.source = Some(source);
        self
    }

    /// 借用宿主提供的模型认证数据。
    #[must_use]
    pub fn auth(&self) -> &ModelAuth {
        &self.auth
    }

    /// 显式借用 provider 环境数据；返回值可能包含秘密。
    #[must_use]
    pub fn env(&self) -> Option<&ProviderEnv> {
        self.env.as_ref()
    }

    /// 显式借用来源描述；调用方决定是否适合展示。
    #[must_use]
    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }
}

impl fmt::Debug for AuthResult {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthResult")
            .field("auth", &self.auth)
            .field("env", &"[REDACTED]")
            .field("source", &"[REDACTED]")
            .finish()
    }
}
