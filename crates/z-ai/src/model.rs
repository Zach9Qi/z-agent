//! 模型能力、费率、思考映射及用量数据；不包含目录、计价或推理预算算法。
//! 可选 serde 仅承诺自有格式往返；序列化会包含原始配置，不等于脱敏。

mod usage;

use std::{collections::BTreeMap, fmt};

use crate::{
    error::Result,
    identity::{ApiId, ModelId, ProviderId},
    value::JsonObject,
};

pub use usage::{TokenAmounts, Usage, UsageCost, UsageCounts};

/// 模型描述，与具体本地账号无关；敏感配置在常规调试输出中整体隐藏。
#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Model {
    identity: ModelIdentity,
    base_url: String,
    capabilities: ModelCapabilities,
    cost: ModelCost,
    thinking_level_map: Option<ThinkingLevelMap>,
    sampling_params: Option<JsonObject>,
    headers: Option<BTreeMap<String, String>>,
    compat: Option<ModelCompat>,
}

impl Model {
    /// 保存模型的身份、端点、能力及费率，不解析端点或自动发现兼容性。
    #[must_use]
    pub fn new(
        identity: ModelIdentity,
        base_url: String,
        capabilities: ModelCapabilities,
        cost: ModelCost,
    ) -> Self {
        Self {
            identity,
            base_url,
            capabilities,
            cost,
            thinking_level_map: None,
            sampling_params: None,
            headers: None,
            compat: None,
        }
    }

    /// 保存思考映射；空映射与未提供映射不同。
    #[must_use]
    pub fn with_thinking_level_map(mut self, map: ThinkingLevelMap) -> Self {
        self.thinking_level_map = Some(map);
        self
    }
    /// 保存任意 JSON 采样参数，不执行与请求参数的合并。
    #[must_use]
    pub fn with_sampling_params(mut self, params: JsonObject) -> Self {
        self.sampling_params = Some(params);
        self
    }
    /// 保存模型默认请求头；这里不执行认证或头部合并。
    #[must_use]
    pub fn with_headers(mut self, headers: BTreeMap<String, String>) -> Self {
        self.headers = Some(headers);
        self
    }
    /// 保存有 API 归属的完整兼容扩展，不进行 SDK 字段验证。
    #[must_use]
    pub fn with_compat(mut self, compat: ModelCompat) -> Self {
        self.compat = Some(compat);
        self
    }
    /// 模型标识。
    #[must_use]
    pub fn id(&self) -> &ModelId {
        self.identity.id()
    }
    /// 显示名称。
    #[must_use]
    pub fn name(&self) -> &str {
        self.identity.name()
    }
    /// 协议标识。
    #[must_use]
    pub fn api(&self) -> &ApiId {
        self.identity.api()
    }
    /// 提供方标识。
    #[must_use]
    pub fn provider(&self) -> &ProviderId {
        self.identity.provider()
    }
    /// 原始端点，可能包含秘密，显式读取后由宿主管理。
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
    /// 模型是否声明支持推理。
    #[must_use]
    pub fn reasoning(&self) -> bool {
        self.capabilities.reasoning()
    }
    /// 有序输入模态。
    #[must_use]
    pub fn input(&self) -> &[InputModality] {
        self.capabilities.input()
    }
    /// 费率及阶梯数据。
    #[must_use]
    pub fn cost(&self) -> &ModelCost {
        &self.cost
    }
    /// 上下文窗口 token 数。
    #[must_use]
    pub fn context_window(&self) -> u64 {
        self.capabilities.context_window()
    }
    /// 最大输出 token 数。
    #[must_use]
    pub fn max_tokens(&self) -> u64 {
        self.capabilities.max_tokens()
    }
    /// 思考映射；缺键、显式不支持和原生字符串不同。
    #[must_use]
    pub fn thinking_level_map(&self) -> Option<&ThinkingLevelMap> {
        self.thinking_level_map.as_ref()
    }
    /// 完整默认采样参数。
    #[must_use]
    pub fn sampling_params(&self) -> Option<&JsonObject> {
        self.sampling_params.as_ref()
    }
    /// 默认请求头，可能含秘密。
    #[must_use]
    pub fn headers(&self) -> Option<&BTreeMap<String, String>> {
        self.headers.as_ref()
    }
    /// 有 API 归属的兼容扩展。
    #[must_use]
    pub fn compat(&self) -> Option<&ModelCompat> {
        self.compat.as_ref()
    }
    /// 仅按提供方与模型 ID 比较身份，不比较协议、名称、端点或配置。
    #[must_use]
    pub fn same_identity(&self, other: &Self) -> bool {
        self.provider() == other.provider() && self.id() == other.id()
    }
}

impl fmt::Debug for Model {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Model([已脱敏])")
    }
}

/// 模型的开放标识与显示名称；结构相等不等同于模型身份相等。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ModelIdentity {
    id: ModelId,
    name: String,
    api: ApiId,
    provider: ProviderId,
}

impl ModelIdentity {
    /// 保存已校验的开放标识及不作归一化的显示名称。
    #[must_use]
    pub fn new(id: ModelId, name: String, api: ApiId, provider: ProviderId) -> Self {
        Self {
            id,
            name,
            api,
            provider,
        }
    }
    /// 模型标识。
    #[must_use]
    pub fn id(&self) -> &ModelId {
        &self.id
    }
    /// 显示名称。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    /// API 协议标识。
    #[must_use]
    pub fn api(&self) -> &ApiId {
        &self.api
    }
    /// 提供方标识。
    #[must_use]
    pub fn provider(&self) -> &ProviderId {
        &self.provider
    }
}

/// 模型声明的能力；只保存非负整数上限，不执行合理性推断或限制算法。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ModelCapabilities {
    reasoning: bool,
    input: Vec<InputModality>,
    context_window: u64,
    max_tokens: u64,
}

impl ModelCapabilities {
    /// 保存模型声明，保留输入顺序、空列表与零值。
    #[must_use]
    pub fn new(
        reasoning: bool,
        input: Vec<InputModality>,
        context_window: u64,
        max_tokens: u64,
    ) -> Self {
        Self {
            reasoning,
            input,
            context_window,
            max_tokens,
        }
    }
    /// 是否声明支持推理。
    #[must_use]
    pub fn reasoning(&self) -> bool {
        self.reasoning
    }
    /// 输入模态列表。
    #[must_use]
    pub fn input(&self) -> &[InputModality] {
        &self.input
    }
    /// 上下文窗口 token 数。
    #[must_use]
    pub fn context_window(&self) -> u64 {
        self.context_window
    }
    /// 最大输出 token 数。
    #[must_use]
    pub fn max_tokens(&self) -> u64 {
        self.max_tokens
    }
}

/// 模型可接受的聊天输入模态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InputModality {
    /// 文本输入。
    Text,
    /// 图片输入。
    Image,
}

/// 四类美元每百万 token 费率；不执行实际计价。
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ModelCostRates {
    amounts: TokenAmounts,
}

impl ModelCostRates {
    /// 保存四类费率，单位为美元每百万 token。
    ///
    /// # Errors
    /// 任一费率为负数或非有限值时返回 [`crate::Error::InvalidInput`]。
    pub fn new(input: f64, output: f64, cache_read: f64, cache_write: f64) -> Result<Self> {
        Ok(Self {
            amounts: TokenAmounts::new(input, output, cache_read, cache_write)?,
        })
    }
    /// 输入费率。
    #[must_use]
    pub fn input(&self) -> f64 {
        self.amounts.input()
    }
    /// 输出费率。
    #[must_use]
    pub fn output(&self) -> f64 {
        self.amounts.output()
    }
    /// 缓存读取费率。
    #[must_use]
    pub fn cache_read(&self) -> f64 {
        self.amounts.cache_read()
    }
    /// 缓存写入费率。
    #[must_use]
    pub fn cache_write(&self) -> f64 {
        self.amounts.cache_write()
    }
}

/// 一档请求级费率，阈值只保存，不自动选择或排序。
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ModelCostTier {
    input_tokens_above: u64,
    rates: ModelCostRates,
}

impl ModelCostTier {
    /// 保存严格超过的输入阈值和该档费率。
    #[must_use]
    pub fn new(input_tokens_above: u64, rates: ModelCostRates) -> Self {
        Self {
            input_tokens_above,
            rates,
        }
    }
    /// 请求输入必须严格超过的阈值，单位为 token。
    #[must_use]
    pub fn input_tokens_above(&self) -> u64 {
        self.input_tokens_above
    }
    /// 此档四类费率。
    #[must_use]
    pub fn rates(&self) -> &ModelCostRates {
        &self.rates
    }
}

/// 基础费率与可选的有序阶梯；缺失阶梯与空阶梯不同。
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ModelCost {
    rates: ModelCostRates,
    tiers: Option<Vec<ModelCostTier>>,
}

impl ModelCost {
    /// 保存基础费率，阶梯初始为未提供。
    #[must_use]
    pub fn new(rates: ModelCostRates) -> Self {
        Self { rates, tiers: None }
    }
    /// 原样保存阶梯顺序与阈值，不执行排序或计价。
    #[must_use]
    pub fn with_tiers(mut self, tiers: Vec<ModelCostTier>) -> Self {
        self.tiers = Some(tiers);
        self
    }
    /// 基础四类费率。
    #[must_use]
    pub fn rates(&self) -> &ModelCostRates {
        &self.rates
    }
    /// 有序阶梯，未提供与空列表不同。
    #[must_use]
    pub fn tiers(&self) -> Option<&[ModelCostTier]> {
        self.tiers.as_deref()
    }
}

/// 通用请求的六档思考强度；不计算提供方预算。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ThinkingLevel {
    /// 最小强度。
    Minimal,
    /// 低强度。
    Low,
    /// 中强度。
    Medium,
    /// 高强度。
    High,
    /// 超高强度。
    XHigh,
    /// 最大强度。
    Max,
}

/// 模型思考映射的档位；除六档外还可单独描述关闭推理。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ModelThinkingLevel {
    /// 关闭推理。
    Off,
    /// 最小强度。
    Minimal,
    /// 低强度。
    Low,
    /// 中强度。
    Medium,
    /// 高强度。
    High,
    /// 超高强度。
    XHigh,
    /// 最大强度。
    Max,
}

impl From<ThinkingLevel> for ModelThinkingLevel {
    fn from(level: ThinkingLevel) -> Self {
        match level {
            ThinkingLevel::Minimal => Self::Minimal,
            ThinkingLevel::Low => Self::Low,
            ThinkingLevel::Medium => Self::Medium,
            ThinkingLevel::High => Self::High,
            ThinkingLevel::XHigh => Self::XHigh,
            ThinkingLevel::Max => Self::Max,
        }
    }
}

/// 思考映射值；缺键由映射容器表示，不等同于显式不支持。
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ThinkingMapping {
    /// 显式不支持，对应上游 `null`。
    Unsupported,
    /// 提供方原生值，包括空字符串，原样保存。
    Mapped(String),
}

/// 思考档位到原生值的映射；缺键表示由未来适配器使用默认值。
pub type ThinkingLevelMap = BTreeMap<ModelThinkingLevel, ThinkingMapping>;

/// 有 API 归属的完整兼容扩展，保留嵌套 JSON，不强类型重写 SDK 配置。
#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ModelCompat {
    api: ApiId,
    values: JsonObject,
}

impl ModelCompat {
    /// 保存协议归属与兼容字段；不检查、解释或执行提供方字段。
    #[must_use]
    pub fn new(api: ApiId, values: JsonObject) -> Self {
        Self { api, values }
    }
    /// 此扩展所声明的协议。
    #[must_use]
    pub fn api(&self) -> &ApiId {
        &self.api
    }
    /// 完整兼容字段，可能含秘密；显式读取由宿主管理。
    #[must_use]
    pub fn values(&self) -> &JsonObject {
        &self.values
    }
}

impl fmt::Debug for ModelCompat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ModelCompat([已脱敏])")
    }
}
