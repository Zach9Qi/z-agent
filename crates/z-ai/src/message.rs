//! 聊天上下文及角色消息；保留顺序、可选状态、报告用量和宿主工具详情。
//! 消息文本及诊断可包含敏感信息，`Debug` 和可选 serde 输出不保证脱敏。

mod content;
#[cfg(feature = "serde")]
mod details;
mod metadata;

use crate::{identity::UnixMillis, model::Usage, tool::Tool, value::JsonValue};

pub use content::{
    AssistantContent, ImageContent, TextContent, TextPhase, TextSignatureV1, ThinkingContent,
    ToolCall, UserContent, UserContentBlock,
};
pub use metadata::{
    AssistantMessageDiagnostic, DeferredHandle, DiagnosticCode, DiagnosticErrorInfo,
    ResponseIdentity,
};

/// 聊天上下文；系统提示独立于消息，工具的缺失与空列表不同。
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Context<D = JsonValue> {
    system_prompt: Option<String>,
    messages: Vec<Message<D>>,
    tools: Option<Vec<Tool>>,
}

impl<D> Context<D> {
    /// 保存有序历史，不添加系统消息或默认工具。
    #[must_use]
    pub fn new(messages: Vec<Message<D>>) -> Self {
        Self {
            system_prompt: None,
            messages,
            tools: None,
        }
    }
    /// 提供系统提示，空字符串与未提供不同。
    #[must_use]
    pub fn with_system_prompt(mut self, prompt: String) -> Self {
        self.system_prompt = Some(prompt);
        self
    }
    /// 提供工具列表，空列表与未提供不同。
    #[must_use]
    pub fn with_tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = Some(tools);
        self
    }
    /// 可选系统提示。
    #[must_use]
    pub fn system_prompt(&self) -> Option<&str> {
        self.system_prompt.as_deref()
    }
    /// 有序历史，工具详情保留宿主类型。
    #[must_use]
    pub fn messages(&self) -> &[Message<D>] {
        &self.messages
    }
    /// 可选工具定义。
    #[must_use]
    pub fn tools(&self) -> Option<&[Tool]> {
        self.tools.as_deref()
    }
}

/// 聊天消息的三种角色；工具详情可为非 JSON 的宿主数据。
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Message<D = JsonValue> {
    /// 用户原始输入。
    User(UserMessage),
    /// 助手输出；装箱避免枚举被完整元数据撑大。
    Assistant(Box<AssistantMessage>),
    /// 工具执行结果；工具用量不会自动并入模型用量。
    ToolResult(Box<ToolResultMessage<D>>),
}

impl<D> From<UserMessage> for Message<D> {
    fn from(message: UserMessage) -> Self {
        Self::User(message)
    }
}

impl<D> From<AssistantMessage> for Message<D> {
    fn from(message: AssistantMessage) -> Self {
        Self::Assistant(Box::new(message))
    }
}

impl<D> From<ToolResultMessage<D>> for Message<D> {
    fn from(message: ToolResultMessage<D>) -> Self {
        Self::ToolResult(Box::new(message))
    }
}

/// 用户消息，保留纯文本与文本/图片分块表示。
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UserMessage {
    content: UserContent,
    timestamp: UnixMillis,
}

impl UserMessage {
    /// 保存用户内容和显式时间戳。
    #[must_use]
    pub fn new(content: UserContent, timestamp: UnixMillis) -> Self {
        Self { content, timestamp }
    }
    /// 原始纯文本或有序内容块。
    #[must_use]
    pub fn content(&self) -> &UserContent {
        &self.content
    }
    /// 宿主传入的 Unix 毫秒。
    #[must_use]
    pub fn timestamp(&self) -> UnixMillis {
        self.timestamp
    }
}

/// 助手响应，允许待完成、失败部分响应及后台句柄，不执行生成过程。
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AssistantMessage {
    identity: ResponseIdentity,
    content: Vec<AssistantContent>,
    usage: Usage,
    stop_reason: StopReason,
    timestamp: UnixMillis,
    response_model: Option<String>,
    response_id: Option<String>,
    provider_thinking_level: Option<String>,
    diagnostics: Option<Vec<AssistantMessageDiagnostic>>,
    deferred: Option<DeferredHandle>,
    error_message: Option<String>,
    raw_stop_reason: Option<String>,
    end_turn: Option<bool>,
}

impl AssistantMessage {
    /// 保存来源、有序内容、报告用量和时间，初始原因为待完成。
    #[must_use]
    pub fn new(
        identity: ResponseIdentity,
        content: Vec<AssistantContent>,
        usage: Usage,
        timestamp: UnixMillis,
    ) -> Self {
        Self {
            identity,
            content,
            usage,
            stop_reason: StopReason::Pending,
            timestamp,
            response_model: None,
            response_id: None,
            provider_thinking_level: None,
            diagnostics: None,
            deferred: None,
            error_message: None,
            raw_stop_reason: None,
            end_turn: None,
        }
    }
    /// 保存当前停止原因；不清除部分内容或重算用量。
    #[must_use]
    pub fn with_stop_reason(mut self, reason: StopReason) -> Self {
        self.stop_reason = reason;
        self
    }
    /// 保存提供方报告的实际模型字符串，不将其当成本地模型目录查询。
    #[must_use]
    pub fn with_response_model(mut self, model: String) -> Self {
        self.response_model = Some(model);
        self
    }
    /// 保存提供方响应标识。
    #[must_use]
    pub fn with_response_id(mut self, id: String) -> Self {
        self.response_id = Some(id);
        self
    }
    /// 保存实际使用的提供方原生思考级别。
    #[must_use]
    pub fn with_provider_thinking_level(mut self, level: String) -> Self {
        self.provider_thinking_level = Some(level);
        self
    }
    /// 保存有序诊断，空列表与未提供不同。
    #[must_use]
    pub fn with_diagnostics(mut self, diagnostics: Vec<AssistantMessageDiagnostic>) -> Self {
        self.diagnostics = Some(diagnostics);
        self
    }
    /// 保存后台响应句柄，不执行续取或推断账号。
    #[must_use]
    pub fn with_deferred(mut self, handle: DeferredHandle) -> Self {
        self.deferred = Some(handle);
        self
    }
    /// 保存提供方或宿主报告的错误文本。
    #[must_use]
    pub fn with_error_message(mut self, message: String) -> Self {
        self.error_message = Some(message);
        self
    }
    /// 保存无法映射到标准枚举的原始停止原因。
    #[must_use]
    pub fn with_raw_stop_reason(mut self, reason: String) -> Self {
        self.raw_stop_reason = Some(reason);
        self
    }
    /// 保存是否显式结束轮次，`false` 与未报告不同。
    #[must_use]
    pub fn with_end_turn(mut self, end_turn: bool) -> Self {
        self.end_turn = Some(end_turn);
        self
    }
    /// 提供方、协议及请求模型。
    #[must_use]
    pub fn identity(&self) -> &ResponseIdentity {
        &self.identity
    }
    /// 有序助手内容。
    #[must_use]
    pub fn content(&self) -> &[AssistantContent] {
        &self.content
    }
    /// 独立报告的模型用量，不累计子集或工具用量。
    #[must_use]
    pub fn usage(&self) -> &Usage {
        &self.usage
    }
    /// 当前停止原因。
    #[must_use]
    pub fn stop_reason(&self) -> StopReason {
        self.stop_reason
    }
    /// 宿主传入的 Unix 毫秒。
    #[must_use]
    pub fn timestamp(&self) -> UnixMillis {
        self.timestamp
    }
    /// 提供方报告的实际模型。
    #[must_use]
    pub fn response_model(&self) -> Option<&str> {
        self.response_model.as_deref()
    }
    /// 提供方报告的响应标识。
    #[must_use]
    pub fn response_id(&self) -> Option<&str> {
        self.response_id.as_deref()
    }
    /// 提供方实际使用的原生思考级别。
    #[must_use]
    pub fn provider_thinking_level(&self) -> Option<&str> {
        self.provider_thinking_level.as_deref()
    }
    /// 可选的有序诊断数据。
    #[must_use]
    pub fn diagnostics(&self) -> Option<&[AssistantMessageDiagnostic]> {
        self.diagnostics.as_deref()
    }
    /// 可选后台响应句柄。
    #[must_use]
    pub fn deferred(&self) -> Option<&DeferredHandle> {
        self.deferred.as_ref()
    }
    /// 可选错误文本，不保证脱敏。
    #[must_use]
    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }
    /// 可选原始停止原因。
    #[must_use]
    pub fn raw_stop_reason(&self) -> Option<&str> {
        self.raw_stop_reason.as_deref()
    }
    /// 显式结束轮次的报告标记。
    #[must_use]
    pub fn end_turn(&self) -> Option<bool> {
        self.end_turn
    }
}

/// 助手当前状态或停止原因，包括尚未结束及后台响应。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StopReason {
    /// 尚未获得终态。
    Pending,
    /// 正常结束。
    Stop,
    /// 达到输出长度限制。
    Length,
    /// 需要执行工具。
    ToolUse,
    /// 发生错误，消息可以保留部分内容和用量。
    Error,
    /// 已中止，消息可以保留部分内容和用量。
    Aborted,
    /// 转入后台等待后续取回。
    Deferred,
}

/// 工具结果，详情的默认类型为自有 JSON 值，也可保存宿主对象。
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(bound(
        serialize = "D: serde::Serialize",
        deserialize = "D: serde::Deserialize<'de>"
    ))
)]
pub struct ToolResultMessage<D = JsonValue> {
    tool_call_id: crate::identity::ToolCallId,
    tool_name: String,
    content: Vec<UserContentBlock>,
    #[cfg_attr(
        feature = "serde",
        serde(default = "Option::default", with = "details")
    )]
    details: Option<D>,
    usage: Option<Usage>,
    added_tool_names: Option<Vec<String>>,
    is_error: bool,
    timestamp: UnixMillis,
}

impl<D> ToolResultMessage<D> {
    /// 保存调用关联、名称、内容和时间；初始为非错误、详情及用量未提供。
    #[must_use]
    pub fn new(
        tool_call_id: crate::identity::ToolCallId,
        tool_name: String,
        content: Vec<UserContentBlock>,
        timestamp: UnixMillis,
    ) -> Self {
        Self {
            tool_call_id,
            tool_name,
            content,
            details: None,
            usage: None,
            added_tool_names: None,
            is_error: false,
            timestamp,
        }
    }
    /// 保存宿主工具详情，不隐式转为 JSON 或丢弃数据。
    #[must_use]
    pub fn with_details(mut self, details: D) -> Self {
        self.details = Some(details);
        self
    }
    /// 保存工具执行自身用量，不自动并入主模型用量。
    #[must_use]
    pub fn with_usage(mut self, usage: Usage) -> Self {
        self.usage = Some(usage);
        self
    }
    /// 保存新可用工具名称；空列表与未提供不同。
    #[must_use]
    pub fn with_added_tool_names(mut self, names: Vec<String>) -> Self {
        self.added_tool_names = Some(names);
        self
    }
    /// 保存工具执行是否失败，不改变内容。
    #[must_use]
    pub fn with_is_error(mut self, is_error: bool) -> Self {
        self.is_error = is_error;
        self
    }
    /// 关联的工具调用标识。
    #[must_use]
    pub fn tool_call_id(&self) -> &crate::identity::ToolCallId {
        &self.tool_call_id
    }
    /// 工具名称。
    #[must_use]
    pub fn tool_name(&self) -> &str {
        &self.tool_name
    }
    /// 工具返回的有序文本与图片块。
    #[must_use]
    pub fn content(&self) -> &[UserContentBlock] {
        &self.content
    }
    /// 保留宿主类型的可选详情。
    #[must_use]
    pub fn details(&self) -> Option<&D> {
        self.details.as_ref()
    }
    /// 工具执行自身的可选用量。
    #[must_use]
    pub fn usage(&self) -> Option<&Usage> {
        self.usage.as_ref()
    }
    /// 新可用工具名称。
    #[must_use]
    pub fn added_tool_names(&self) -> Option<&[String]> {
        self.added_tool_names.as_deref()
    }
    /// 工具执行是否失败。
    #[must_use]
    pub fn is_error(&self) -> bool {
        self.is_error
    }
    /// 宿主传入的 Unix 毫秒。
    #[must_use]
    pub fn timestamp(&self) -> UnixMillis {
        self.timestamp
    }
}
