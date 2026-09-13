//! 通用请求与响应选项，只保存数据，不执行传输、重试或环境读取。
//! 所有超时和 token 预算使用非负整数；扩展对象只承载 JSON，不绕过显式账号选择。
//! 可选 serde 仅承诺自有格式往返，序列化可能含秘密；适配器须阻止扩展覆盖账号身份。

use std::{collections::BTreeMap, fmt};

use crate::{
    error::{Error, Result},
    identity::ApiId,
    model::ThinkingLevel,
    value::JsonObject,
};

/// 请求头覆盖：缺键不覆盖，`Some` 设置值，`None` 删除默认请求头。
///
/// 原始映射可能含秘密，调用方显式读取或格式化映射时自行承担泄漏风险。
pub type ProviderHeaders = BTreeMap<String, Option<String>>;

/// 显式传入的提供方环境数据；本库不会读取或修改进程环境。
///
/// 原始映射可能含秘密，调用方显式读取或格式化映射时自行承担泄漏风险。
pub type ProviderEnv = BTreeMap<String, String>;

// 同形的可选纯数据字段共用展开，字段名称和中文语义仍在各类型旁显式列出。
macro_rules! optional_copy {
    ($setter:ident, $getter:ident, $ty:ty, $doc:literal) => {
        #[doc = $doc]
        #[must_use]
        pub fn $setter(mut self, value: $ty) -> Self {
            self.$getter = Some(value);
            self
        }
        #[doc = $doc]
        #[must_use]
        pub fn $getter(&self) -> Option<$ty> {
            self.$getter
        }
    };
}

macro_rules! optional_ref {
    ($setter:ident, $getter:ident, $ty:ty, $doc:literal) => {
        #[doc = $doc]
        #[must_use]
        pub fn $setter(mut self, value: $ty) -> Self {
            self.$getter = Some(value);
            self
        }
        #[doc = $doc]
        #[must_use]
        pub fn $getter(&self) -> Option<&$ty> {
            self.$getter.as_ref()
        }
    };
}

/// 请求级环境、头部及重试数据；空值只表示未提供，不代表提供方的默认配置。
///
/// 不包含通用 API key 覆盖入口、回调、取消信号或传输客户端。
#[derive(Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RequestOptions {
    env: Option<ProviderEnv>,
    headers: Option<ProviderHeaders>,
    timeout_ms: Option<u64>,
    max_retries: Option<u64>,
    max_retry_delay_ms: Option<u64>,
}

impl RequestOptions {
    /// 创建所有字段均未提供的请求数据，不读取提供方默认配置。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    optional_ref!(
        with_env,
        env,
        ProviderEnv,
        "显式环境数据；不读取进程环境，空映射与未提供不同。"
    );
    optional_ref!(
        with_headers,
        headers,
        ProviderHeaders,
        "请求头覆盖；缺键、设置及删除三态独立保留。"
    );
    optional_copy!(
        with_timeout_ms,
        timeout_ms,
        u64,
        "请求超时毫秒数；零与未提供不同。"
    );
    optional_copy!(
        with_max_retries,
        max_retries,
        u64,
        "客户端最大重试次数；只保存数据，不执行重试。"
    );
    optional_copy!(
        with_max_retry_delay_ms,
        max_retry_delay_ms,
        u64,
        "服务器重试延迟上限毫秒数；零表达关闭上限，不在本库执行。"
    );
}

impl fmt::Debug for RequestOptions {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RequestOptions([已脱敏])")
    }
}

/// 通用流式请求数据；不包含流运行时、负载合并或传输实现。
#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamOptions {
    request: RequestOptions,
    temperature: Option<FiniteParameter>,
    sampling_params: Option<JsonObject>,
    max_tokens: Option<u64>,
    transport: Option<Transport>,
    cache_retention: Option<CacheRetention>,
    session_id: Option<String>,
    websocket_connect_timeout_ms: Option<u64>,
    metadata: Option<JsonObject>,
    provider_options: Option<ProviderOptions>,
}

impl StreamOptions {
    /// 保存请求级配置，其余流式选项初始为未提供。
    #[must_use]
    pub fn new(request: RequestOptions) -> Self {
        Self {
            request,
            temperature: None,
            sampling_params: None,
            max_tokens: None,
            transport: None,
            cache_retention: None,
            session_id: None,
            websocket_connect_timeout_ms: None,
            metadata: None,
            provider_options: None,
        }
    }

    /// 请求级配置；其调试输出隐藏整个敏感容器。
    #[must_use]
    pub fn request(&self) -> &RequestOptions {
        &self.request
    }
    /// 保存温度，不施加提供方专属范围限制。
    ///
    /// # Errors
    /// 非有限温度返回 [`Error::InvalidInput`]。
    pub fn with_temperature(mut self, temperature: f64) -> Result<Self> {
        self.temperature = Some(FiniteParameter::new(temperature)?);
        Ok(self)
    }
    /// 可选温度；零与未提供不同。
    #[must_use]
    pub fn temperature(&self) -> Option<f64> {
        self.temperature.as_ref().map(|value| value.0)
    }
    /// 保存会话标识，不生成或规范化会话。
    #[must_use]
    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }
    /// 原始会话标识，空字符串与未提供不同。
    #[must_use]
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    optional_ref!(
        with_sampling_params,
        sampling_params,
        JsonObject,
        "完整 JSON 采样参数；不执行与模型参数的覆盖合并。"
    );
    optional_copy!(
        with_max_tokens,
        max_tokens,
        u64,
        "最大输出 token 数，限定为非负整数。"
    );
    optional_copy!(
        with_transport,
        transport,
        Transport,
        "首选传输数据；本库不建立连接或执行回退。"
    );
    optional_copy!(
        with_cache_retention,
        cache_retention,
        CacheRetention,
        "缓存保留偏好；缺失不自动补成短期。"
    );
    optional_copy!(
        with_websocket_connect_timeout_ms,
        websocket_connect_timeout_ms,
        u64,
        "`WebSocket` 建连超时毫秒数，不是连接后空闲超时。"
    );
    optional_ref!(
        with_metadata,
        metadata,
        JsonObject,
        "完整请求元数据，可保留嵌套对象、显式 null、false 和零。"
    );
    optional_ref!(
        with_provider_options,
        provider_options,
        ProviderOptions,
        "有 API 归属的专有扩展，不解释或执行认证覆盖。"
    );
}

impl fmt::Debug for StreamOptions {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("StreamOptions([已脱敏])")
    }
}

/// 通用简化请求数据，不执行思考预算计算或后台任务提交。
#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SimpleStreamOptions {
    stream: StreamOptions,
    tool_choice: Option<ToolChoice>,
    reasoning: Option<ThinkingLevel>,
    deferred: Option<DeferredMode>,
    thinking_budgets: Option<ThinkingBudgets>,
}

impl SimpleStreamOptions {
    /// 保存通用流式配置，简化选项初始为未提供。
    #[must_use]
    pub fn new(stream: StreamOptions) -> Self {
        Self {
            stream,
            tool_choice: None,
            reasoning: None,
            deferred: None,
            thinking_budgets: None,
        }
    }
    /// 通用流式配置。
    #[must_use]
    pub fn stream(&self) -> &StreamOptions {
        &self.stream
    }

    optional_copy!(
        with_tool_choice,
        tool_choice,
        ToolChoice,
        "通用工具选择，只表达自动与禁用，不复制 SDK 专有联合。"
    );
    optional_copy!(
        with_reasoning,
        reasoning,
        ThinkingLevel,
        "六档通用思考强度，不计算或裁剪预算。"
    );
    optional_ref!(
        with_deferred,
        deferred,
        DeferredMode,
        "后台请求模式，未提供、false、true 与窗口对象独立保留。"
    );
    optional_ref!(
        with_thinking_budgets,
        thinking_budgets,
        ThinkingBudgets,
        "四档显式 token 预算；专有负预算应置于提供方 JSON 扩展。"
    );
}

impl fmt::Debug for SimpleStreamOptions {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SimpleStreamOptions([已脱敏])")
    }
}

/// 提供方专有选项的完整 JSON 对象；不声称具备相应 SDK 字段验证。
#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProviderOptions {
    api: ApiId,
    values: JsonObject,
}

impl ProviderOptions {
    /// 保存协议归属和完整扩展，宿主负责确保它不改变选定账号。
    #[must_use]
    pub fn new(api: ApiId, values: JsonObject) -> Self {
        Self { api, values }
    }
    /// 声明的 API 归属。
    #[must_use]
    pub fn api(&self) -> &ApiId {
        &self.api
    }
    /// 原始扩展字段，可能含秘密，显式读取由宿主管理。
    #[must_use]
    pub fn values(&self) -> &JsonObject {
        &self.values
    }
}

impl fmt::Debug for ProviderOptions {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ProviderOptions([已脱敏])")
    }
}

/// 传输偏好，不代表已经建立连接。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Transport {
    /// 服务器发送事件。
    Sse,
    /// `WebSocket` 连接。
    WebSocket,
    /// 缓存 `WebSocket` 连接。
    WebSocketCached,
    /// 由未来适配器选择。
    Auto,
}

/// 缓存保留偏好，不承诺具体提供方支持。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CacheRetention {
    /// 不保留缓存。
    None,
    /// 短期缓存。
    Short,
    /// 长期缓存。
    Long,
}

/// 提供方中立的工具选择。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ToolChoice {
    /// 自动决定是否使用工具。
    Auto,
    /// 禁用工具调用。
    None,
}

/// 后台请求的三种显式形状；外层 `Option` 表示未提供。
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DeferredMode {
    /// 显式关闭，对应 `false`。
    Disabled,
    /// 显式启用，对应 `true`。
    Enabled,
    /// 对象形状；窗口可未提供。
    Config(DeferredConfig),
}

/// 后台请求窗口对象；空对象有意义，不等同于布尔值。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DeferredConfig {
    window: Option<DeferredWindow>,
}

impl DeferredConfig {
    /// 创建未指定窗口的对象，不填入服务端默认窗口。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    optional_copy!(
        with_window,
        window,
        DeferredWindow,
        "后台处理窗口；只保存期限选择。"
    );
}

/// 后台请求可选择的处理窗口。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DeferredWindow {
    /// 十五分钟。
    Minutes15,
    /// 一小时。
    Hour1,
    /// 二十四小时。
    Hours24,
}

/// 四档可选的 token 预算；空对象表示没有显式覆盖，非算法默认预算。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ThinkingBudgets {
    minimal: Option<u64>,
    low: Option<u64>,
    medium: Option<u64>,
    high: Option<u64>,
}

impl ThinkingBudgets {
    /// 创建四档均未提供的预算对象。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    optional_copy!(
        with_minimal,
        minimal,
        u64,
        "最小档 token 预算，零与缺失不同。"
    );
    optional_copy!(with_low, low, u64, "低档 token 预算。");
    optional_copy!(with_medium, medium, u64, "中档 token 预算。");
    optional_copy!(with_high, high, u64, "高档 token 预算。");
}

/// 提供方响应的状态及头部；不持有网络响应或读取响应体。
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProviderResponse {
    status: u64,
    headers: BTreeMap<String, String>,
}

impl ProviderResponse {
    /// 保存宿主传入的非负整数状态和响应头，不验证 HTTP 状态范围。
    #[must_use]
    pub fn new(status: u64, headers: BTreeMap<String, String>) -> Self {
        Self { status, headers }
    }
    /// 原始状态值。
    #[must_use]
    pub fn status(&self) -> u64 {
        self.status
    }
    /// 原始响应头，可能含秘密。
    #[must_use]
    pub fn headers(&self) -> &BTreeMap<String, String> {
        &self.headers
    }
}

impl fmt::Debug for ProviderResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ProviderResponse([已脱敏])")
    }
}

/// 后台响应续取选项；不提供网络续取方法。
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DeferredFetchOptions {
    request: RequestOptions,
    wait_ms: Option<u64>,
}

impl DeferredFetchOptions {
    /// 保存请求配置，长轮询时长初始未提供。
    #[must_use]
    pub fn new(request: RequestOptions) -> Self {
        Self {
            request,
            wait_ms: None,
        }
    }
    /// 请求级配置。
    #[must_use]
    pub fn request(&self) -> &RequestOptions {
        &self.request
    }
    optional_copy!(
        with_wait_ms,
        wait_ms,
        u64,
        "最长轮询等待毫秒数；零表达仅检查一次，不在本库执行。"
    );
}

impl fmt::Debug for DeferredFetchOptions {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("DeferredFetchOptions([已脱敏])")
    }
}

/// 后台取消只复用请求配置，不执行取消或撤销远端任务。
pub type DeferredCancelOptions = RequestOptions;

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "f64", try_from = "f64"))]
struct FiniteParameter(f64);

impl FiniteParameter {
    fn new(value: f64) -> Result<Self> {
        if !value.is_finite() {
            return Err(Error::InvalidInput("temperature 必须是有限数值".into()));
        }
        Ok(Self(value))
    }
}

#[cfg(feature = "serde")]
impl TryFrom<f64> for FiniteParameter {
    type Error = Error;
    fn try_from(value: f64) -> Result<Self> {
        Self::new(value)
    }
}

#[cfg(feature = "serde")]
impl From<FiniteParameter> for f64 {
    fn from(value: FiniteParameter) -> Self {
        value.0
    }
}
