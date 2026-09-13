//! 助手增量事件的数据契约；不创建队列、锁、执行器或事件顺序验证器。
//!
//! `P` 由宿主提供，可为共享实时 partial 句柄，不要求事件发生时的快照。
//! 常规序列先开始再更新，setup 失败可直接错误；本类型允许块按索引交错。
//! 工具增量可以是残缺 JSON，结束事件保存的权威内容不一定等于增量拼接。

use crate::{
    error::{Error, Result},
    message::{AssistantMessage, StopReason, ToolCall},
};

/// 助手增量事件；非终态保存宿主句柄，终态拥有完整消息。
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AssistantMessageEvent<P> {
    /// 响应开始。
    Start {
        /// 宿主提供的实时部分响应句柄。
        partial: P,
    },
    /// 文本块开始，常规情况下对应空文本块。
    TextStart {
        /// 内容块位置，允许与其他块交错。
        content_index: usize,
        /// 宿主提供的实时部分响应句柄。
        partial: P,
    },
    /// 文本增量，不自动拼接或改变 partial。
    TextDelta {
        /// 内容块位置。
        content_index: usize,
        /// 此次收到的原始增量。
        delta: String,
        /// 宿主提供的实时部分响应句柄。
        partial: P,
    },
    /// 文本块结束，保存权威文本而非假定的增量拼接。
    TextEnd {
        /// 内容块位置。
        content_index: usize,
        /// 最终权威文本；签名仍可从 partial 中对应内容取得。
        content: String,
        /// 宿主提供的实时部分响应句柄。
        partial: P,
    },
    /// 思考块开始；被遮蔽的块可以此时已完整且没有后续增量。
    ThinkingStart {
        /// 内容块位置。
        content_index: usize,
        /// 宿主提供的实时部分响应句柄。
        partial: P,
    },
    /// 思考文本增量。
    ThinkingDelta {
        /// 内容块位置。
        content_index: usize,
        /// 原始思考增量。
        delta: String,
        /// 宿主提供的实时部分响应句柄。
        partial: P,
    },
    /// 思考块结束。
    ThinkingEnd {
        /// 内容块位置。
        content_index: usize,
        /// 权威思考文本；签名及遮蔽标记仍保存在 partial。
        content: String,
        /// 宿主提供的实时部分响应句柄。
        partial: P,
    },
    /// 工具调用开始；初始参数形态由提供方决定。
    ToolCallStart {
        /// 内容块位置。
        content_index: usize,
        /// 宿主提供的实时部分响应句柄。
        partial: P,
    },
    /// 工具参数增量，允许不完整或尚不可解析的 JSON 文本。
    ToolCallDelta {
        /// 内容块位置。
        content_index: usize,
        /// 原始 JSON 增量，不在此解析或修复。
        delta: String,
        /// 宿主提供的实时部分响应句柄。
        partial: P,
    },
    /// 工具调用结束；完整参数仍不代表通过 schema 验证。
    ToolCallEnd {
        /// 内容块位置。
        content_index: usize,
        /// 权威工具调用，包含签名与命名空间。
        tool_call: ToolCall,
        /// 宿主提供的实时部分响应句柄。
        partial: P,
    },
    /// 原因与完整消息一致的成功终态。
    Done(DoneEvent),
    /// 原因与部分或完整消息一致的失败终态，可直接出现在 setup 阶段。
    Error(ErrorEvent),
}

impl<P> AssistantMessageEvent<P> {
    /// 校验并构造成功终态，不检查此前是否收到开始事件。
    ///
    /// # Errors
    /// 原因与消息的停止原因不一致时返回 [`Error::InvalidInput`]。
    pub fn done(reason: DoneReason, message: AssistantMessage) -> Result<Self> {
        Ok(Self::Done(DoneEvent::new(reason, message)?))
    }

    /// 校验并构造失败终态，允许保留失败前的内容及报告用量。
    ///
    /// # Errors
    /// 原因与消息的停止原因不一致时返回 [`Error::InvalidInput`]。
    pub fn error(reason: ErrorReason, message: AssistantMessage) -> Result<Self> {
        Ok(Self::Error(ErrorEvent::new(reason, message)?))
    }

    /// 借用非终态的宿主句柄；终态不含 partial 句柄。
    #[must_use]
    pub fn partial(&self) -> Option<&P> {
        match self {
            Self::Start { partial }
            | Self::TextStart { partial, .. }
            | Self::TextDelta { partial, .. }
            | Self::TextEnd { partial, .. }
            | Self::ThinkingStart { partial, .. }
            | Self::ThinkingDelta { partial, .. }
            | Self::ThinkingEnd { partial, .. }
            | Self::ToolCallStart { partial, .. }
            | Self::ToolCallDelta { partial, .. }
            | Self::ToolCallEnd { partial, .. } => Some(partial),
            Self::Done(_) | Self::Error(_) => None,
        }
    }

    /// 块更新关联的位置；开始及终态事件没有内容索引。
    #[must_use]
    pub fn content_index(&self) -> Option<usize> {
        match self {
            Self::TextStart { content_index, .. }
            | Self::TextDelta { content_index, .. }
            | Self::TextEnd { content_index, .. }
            | Self::ThinkingStart { content_index, .. }
            | Self::ThinkingDelta { content_index, .. }
            | Self::ThinkingEnd { content_index, .. }
            | Self::ToolCallStart { content_index, .. }
            | Self::ToolCallDelta { content_index, .. }
            | Self::ToolCallEnd { content_index, .. } => Some(*content_index),
            Self::Start { .. } | Self::Done(_) | Self::Error(_) => None,
        }
    }
}

/// 成功终态；私有字段防止原因与消息停止原因不一致。
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "DoneEventData"))]
pub struct DoneEvent {
    reason: DoneReason,
    message: Box<AssistantMessage>,
}

impl DoneEvent {
    /// 验证终态原因并拥有完整消息。
    ///
    /// # Errors
    /// 原因与消息的停止原因不一致时返回 [`Error::InvalidInput`]。
    pub fn new(reason: DoneReason, message: AssistantMessage) -> Result<Self> {
        if StopReason::from(reason) != message.stop_reason() {
            return Err(Error::InvalidInput(
                "成功事件原因与消息停止原因必须一致".into(),
            ));
        }
        Ok(Self {
            reason,
            message: Box::new(message),
        })
    }
    /// 成功终态原因。
    #[must_use]
    pub fn reason(&self) -> DoneReason {
        self.reason
    }
    /// 完整消息，不依赖随后变化的宿主 partial。
    #[must_use]
    pub fn message(&self) -> &AssistantMessage {
        &self.message
    }
    /// 取回完整消息的所有权。
    #[must_use]
    pub fn into_message(self) -> AssistantMessage {
        *self.message
    }
}

/// 失败终态；保留已经生成的内容、用量和诊断。
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "ErrorEventData"))]
pub struct ErrorEvent {
    reason: ErrorReason,
    message: Box<AssistantMessage>,
}

impl ErrorEvent {
    /// 验证失败原因并拥有部分或完整消息。
    ///
    /// # Errors
    /// 原因与消息的停止原因不一致时返回 [`Error::InvalidInput`]。
    pub fn new(reason: ErrorReason, message: AssistantMessage) -> Result<Self> {
        if StopReason::from(reason) != message.stop_reason() {
            return Err(Error::InvalidInput(
                "失败事件原因与消息停止原因必须一致".into(),
            ));
        }
        Ok(Self {
            reason,
            message: Box::new(message),
        })
    }
    /// 失败终态原因。
    #[must_use]
    pub fn reason(&self) -> ErrorReason {
        self.reason
    }
    /// 已生成的消息和用量。
    #[must_use]
    pub fn message(&self) -> &AssistantMessage {
        &self.message
    }
    /// 取回终态消息的所有权。
    #[must_use]
    pub fn into_message(self) -> AssistantMessage {
        *self.message
    }
}

/// 成功事件允许的四种原因，不接受待完成或失败原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DoneReason {
    /// 正常结束。
    Stop,
    /// 达到长度限制。
    Length,
    /// 需要工具执行。
    ToolUse,
    /// 转入后台响应。
    Deferred,
}

impl From<DoneReason> for StopReason {
    fn from(reason: DoneReason) -> Self {
        match reason {
            DoneReason::Stop => Self::Stop,
            DoneReason::Length => Self::Length,
            DoneReason::ToolUse => Self::ToolUse,
            DoneReason::Deferred => Self::Deferred,
        }
    }
}

/// 失败事件允许的两种原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ErrorReason {
    /// 请求错误。
    Error,
    /// 请求中止。
    Aborted,
}

impl From<ErrorReason> for StopReason {
    fn from(reason: ErrorReason) -> Self {
        match reason {
            ErrorReason::Error => Self::Error,
            ErrorReason::Aborted => Self::Aborted,
        }
    }
}

#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
struct DoneEventData {
    reason: DoneReason,
    message: AssistantMessage,
}

#[cfg(feature = "serde")]
impl TryFrom<DoneEventData> for DoneEvent {
    type Error = Error;
    fn try_from(data: DoneEventData) -> Result<Self> {
        Self::new(data.reason, data.message)
    }
}

#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
struct ErrorEventData {
    reason: ErrorReason,
    message: AssistantMessage,
}

#[cfg(feature = "serde")]
impl TryFrom<ErrorEventData> for ErrorEvent {
    type Error = Error;
    fn try_from(data: ErrorEventData) -> Result<Self> {
        Self::new(data.reason, data.message)
    }
}
