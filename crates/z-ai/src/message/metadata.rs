//! 消息来源、后台响应句柄与诊断数据；这些数据不是认证解析结果。

use crate::{
    identity::{ApiId, ModelId, ProviderId, UnixMillis},
    value::{JsonNumber, JsonObject, JsonValue},
};

/// 消息的提供方、协议及请求模型，不包含本地账号身份。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ResponseIdentity {
    provider: ProviderId,
    api: ApiId,
    model: ModelId,
}

impl ResponseIdentity {
    /// 保存来源三元组；同一模型可由不同显式账号使用。
    #[must_use]
    pub fn new(provider: ProviderId, api: ApiId, model: ModelId) -> Self {
        Self {
            provider,
            api,
            model,
        }
    }
    /// 服务提供方标识。
    #[must_use]
    pub fn provider(&self) -> &ProviderId {
        &self.provider
    }
    /// 聊天协议标识。
    #[must_use]
    pub fn api(&self) -> &ApiId {
        &self.api
    }
    /// 请求模型标识。
    #[must_use]
    pub fn model(&self) -> &ModelId {
        &self.model
    }
}

/// 提供方后台响应句柄，仅保存续取数据，不执行轮询、取消或时间读取。
///
/// 未来宿主必须另行记录本地账号来源，不能从此句柄推断账号。
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DeferredHandle {
    identity: ResponseIdentity,
    id: String,
    expires_at: Option<UnixMillis>,
    poll_after_ms: Option<u64>,
    data: Option<JsonValue>,
}

impl DeferredHandle {
    /// 保存提供方的原始后台响应标识，不推断本地账号或执行协议。
    #[must_use]
    pub fn new(identity: ResponseIdentity, id: String) -> Self {
        Self {
            identity,
            id,
            expires_at: None,
            poll_after_ms: None,
            data: None,
        }
    }
    /// 保存绝对过期时间，单位为 Unix 毫秒。
    #[must_use]
    pub fn with_expires_at(mut self, timestamp: UnixMillis) -> Self {
        self.expires_at = Some(timestamp);
        self
    }
    /// 保存建议等待毫秒数，不在本库等待；零与未报告不同。
    #[must_use]
    pub fn with_poll_after_ms(mut self, milliseconds: u64) -> Self {
        self.poll_after_ms = Some(milliseconds);
        self
    }
    /// 保存重建响应所需数据；显式 `JsonValue::Null` 与未提供不同。
    #[must_use]
    pub fn with_data(mut self, data: JsonValue) -> Self {
        self.data = Some(data);
        self
    }
    /// 提供方、协议及模型来源。
    #[must_use]
    pub fn identity(&self) -> &ResponseIdentity {
        &self.identity
    }
    /// 提供方原始响应标识。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    /// 绝对过期时间。
    #[must_use]
    pub fn expires_at(&self) -> Option<UnixMillis> {
        self.expires_at
    }
    /// 建议等待毫秒数。
    #[must_use]
    pub fn poll_after_ms(&self) -> Option<u64> {
        self.poll_after_ms
    }
    /// 可选重建数据，不保证脱敏。
    #[must_use]
    pub fn data(&self) -> Option<&JsonValue> {
        self.data.as_ref()
    }
}

/// 消息诊断中的错误数据，不是本库的操作错误类型，也不保证内容脱敏。
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DiagnosticErrorInfo {
    name: Option<String>,
    message: String,
    stack: Option<String>,
    code: Option<DiagnosticCode>,
}

impl DiagnosticErrorInfo {
    /// 保存诊断错误消息，不解析宿主异常对象。
    #[must_use]
    pub fn new(message: String) -> Self {
        Self {
            name: None,
            message,
            stack: None,
            code: None,
        }
    }
    /// 保存可选错误名称。
    #[must_use]
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }
    /// 保存可选原始堆栈文本。
    #[must_use]
    pub fn with_stack(mut self, stack: String) -> Self {
        self.stack = Some(stack);
        self
    }
    /// 保存字符串或数值错误码。
    #[must_use]
    pub fn with_code(mut self, code: DiagnosticCode) -> Self {
        self.code = Some(code);
        self
    }
    /// 可选错误名称。
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
    /// 原始错误消息。
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
    /// 可选原始堆栈。
    #[must_use]
    pub fn stack(&self) -> Option<&str> {
        self.stack.as_deref()
    }
    /// 可选错误码。
    #[must_use]
    pub fn code(&self) -> Option<&DiagnosticCode> {
        self.code.as_ref()
    }
}

/// 诊断错误码保留字符串与数值的区别。
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DiagnosticCode {
    /// 原始字符串错误码。
    String(String),
    /// 整数或有限浮点错误码。
    Number(JsonNumber),
}

/// 助手消息的运行时诊断，由宿主提供类别及时间；不保证已脱敏。
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AssistantMessageDiagnostic {
    kind: String,
    timestamp: UnixMillis,
    error: Option<DiagnosticErrorInfo>,
    details: Option<JsonObject>,
}

impl AssistantMessageDiagnostic {
    /// 保存诊断类别与显式时间戳，不读取实际时钟。
    #[must_use]
    pub fn new(kind: String, timestamp: UnixMillis) -> Self {
        Self {
            kind,
            timestamp,
            error: None,
            details: None,
        }
    }
    /// 附带错误诊断数据。
    #[must_use]
    pub fn with_error(mut self, error: DiagnosticErrorInfo) -> Self {
        self.error = Some(error);
        self
    }
    /// 附带任意对象详情，空对象与未提供不同。
    #[must_use]
    pub fn with_details(mut self, details: JsonObject) -> Self {
        self.details = Some(details);
        self
    }
    /// 诊断类别，对应上游的 `type` 字段。
    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }
    /// 宿主传入的 Unix 毫秒。
    #[must_use]
    pub fn timestamp(&self) -> UnixMillis {
        self.timestamp
    }
    /// 可选诊断错误。
    #[must_use]
    pub fn error(&self) -> Option<&DiagnosticErrorInfo> {
        self.error.as_ref()
    }
    /// 可选对象详情，不保证脱敏。
    #[must_use]
    pub fn details(&self) -> Option<&JsonObject> {
        self.details.as_ref()
    }
}
