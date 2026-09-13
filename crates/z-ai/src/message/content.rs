//! 角色专属内容块与原样签名；不解码图片，不解释或改写签名。

#[cfg(feature = "serde")]
use crate::error::{Error, Result};
use crate::{identity::ToolCallId, value::JsonObject};

/// 用户输入保留原字符串与分块表示的区别。
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum UserContent {
    /// 未分块的原始文本。
    Text(String),
    /// 有序文本及图片块，允许空列表。
    Blocks(Vec<UserContentBlock>),
}

impl From<String> for UserContent {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

impl From<Vec<UserContentBlock>> for UserContent {
    fn from(blocks: Vec<UserContentBlock>) -> Self {
        Self::Blocks(blocks)
    }
}

/// 用户消息和工具结果允许的内容；不能插入思考或工具调用。
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum UserContentBlock {
    /// 文本及可选不透明签名。
    Text(TextContent),
    /// 原始编码图片数据。
    Image(ImageContent),
}

/// 助手允许的内容；图片不属于助手聊天输出块。
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AssistantContent {
    /// 文本及可选不透明签名。
    Text(TextContent),
    /// 思考及可选重放信息。
    Thinking(ThinkingContent),
    /// 工具调用及完整 JSON 对象参数。
    ToolCall(ToolCall),
}

/// 文本块，签名可为旧字符串或 JSON 外观文本，均原样保存。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TextContent {
    text: String,
    text_signature: Option<String>,
}

impl TextContent {
    /// 保存文本，初始未提供签名。
    #[must_use]
    pub fn new(text: String) -> Self {
        Self {
            text,
            text_signature: None,
        }
    }
    /// 保存不透明文本签名，不尝试解析版本。
    #[must_use]
    pub fn with_text_signature(mut self, signature: String) -> Self {
        self.text_signature = Some(signature);
        self
    }
    /// 原始文本。
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
    /// 可选的原始文本签名。
    #[must_use]
    pub fn text_signature(&self) -> Option<&str> {
        self.text_signature.as_deref()
    }
}

/// 思考块，保留 `redacted` 未报告、否、是三种状态。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ThinkingContent {
    thinking: String,
    thinking_signature: Option<String>,
    redacted: Option<bool>,
}

impl ThinkingContent {
    /// 原样保存思考内容，即使内容已经被提供方遮蔽。
    #[must_use]
    pub fn new(thinking: String) -> Self {
        Self {
            thinking,
            thinking_signature: None,
            redacted: None,
        }
    }
    /// 保存不透明思考签名或加密载荷。
    #[must_use]
    pub fn with_thinking_signature(mut self, signature: String) -> Self {
        self.thinking_signature = Some(signature);
        self
    }
    /// 保存已报告的遮蔽标记，不根据此值改写内容。
    #[must_use]
    pub fn with_redacted(mut self, redacted: bool) -> Self {
        self.redacted = Some(redacted);
        self
    }
    /// 原始思考内容。
    #[must_use]
    pub fn thinking(&self) -> &str {
        &self.thinking
    }
    /// 原始重放签名。
    #[must_use]
    pub fn thinking_signature(&self) -> Option<&str> {
        self.thinking_signature.as_deref()
    }
    /// 未报告与显式 `false` 不同。
    #[must_use]
    pub fn redacted(&self) -> Option<bool> {
        self.redacted
    }
}

/// 图片内容，仅保存 base64 文本与 MIME 类型，不进行解码或类型探测。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ImageContent {
    data: String,
    mime_type: String,
}

impl ImageContent {
    /// 保存原始图片编码和声明的 MIME 类型。
    #[must_use]
    pub fn new(data: String, mime_type: String) -> Self {
        Self { data, mime_type }
    }
    /// 原始 base64 文本。
    #[must_use]
    pub fn data(&self) -> &str {
        &self.data
    }
    /// 声明的 MIME 类型。
    #[must_use]
    pub fn mime_type(&self) -> &str {
        &self.mime_type
    }
}

/// 完整工具调用；参数必须是对象，但不在此验证它是否满足工具 schema。
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ToolCall {
    id: ToolCallId,
    name: String,
    arguments: JsonObject,
    thought_signature: Option<String>,
    namespace: Option<String>,
}

impl ToolCall {
    /// 保存关联标识、原始名称与参数对象。
    #[must_use]
    pub fn new(id: ToolCallId, name: String, arguments: JsonObject) -> Self {
        Self {
            id,
            name,
            arguments,
            thought_signature: None,
            namespace: None,
        }
    }
    /// 保存提供方思考上下文签名，不进行解释。
    #[must_use]
    pub fn with_thought_signature(mut self, signature: String) -> Self {
        self.thought_signature = Some(signature);
        self
    }
    /// 保存命名空间，允许空字符串以保留已报告状态。
    #[must_use]
    pub fn with_namespace(mut self, namespace: String) -> Self {
        self.namespace = Some(namespace);
        self
    }
    /// 工具调用关联标识。
    #[must_use]
    pub fn id(&self) -> &ToolCallId {
        &self.id
    }
    /// 工具原始名称。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    /// 完整参数对象。
    #[must_use]
    pub fn arguments(&self) -> &JsonObject {
        &self.arguments
    }
    /// 提供方不透明思考签名。
    #[must_use]
    pub fn thought_signature(&self) -> Option<&str> {
        self.thought_signature.as_deref()
    }
    /// 可选命名空间。
    #[must_use]
    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }
}

/// 文本签名第一版的结构化数据；不用于强迫普通签名符合此格式。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "TextSignatureV1Data"))]
pub struct TextSignatureV1 {
    v: u8,
    id: String,
    phase: Option<TextPhase>,
}

impl TextSignatureV1 {
    /// 保存第一版签名标识，版本固定为一。
    #[must_use]
    pub fn new(id: String) -> Self {
        Self {
            v: 1,
            id,
            phase: None,
        }
    }
    /// 保存已报告的输出阶段。
    #[must_use]
    pub fn with_phase(mut self, phase: TextPhase) -> Self {
        self.phase = Some(phase);
        self
    }
    /// 签名版本，固定为一。
    #[must_use]
    pub fn version(&self) -> u8 {
        self.v
    }
    /// 原始签名标识，不强加工具或模型 ID 的约束。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    /// 已报告的输出阶段。
    #[must_use]
    pub fn phase(&self) -> Option<TextPhase> {
        self.phase
    }
}

/// 第一版文本签名的输出阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TextPhase {
    /// 过程说明。
    Commentary,
    /// 最终答复。
    FinalAnswer,
}

#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
struct TextSignatureV1Data {
    v: u8,
    id: String,
    phase: Option<TextPhase>,
}

#[cfg(feature = "serde")]
impl TryFrom<TextSignatureV1Data> for TextSignatureV1 {
    type Error = Error;
    fn try_from(data: TextSignatureV1Data) -> Result<Self> {
        if data.v != 1 {
            return Err(Error::InvalidInput("文本签名版本必须为一".into()));
        }
        Ok(Self {
            v: 1,
            id: data.id,
            phase: data.phase,
        })
    }
}
