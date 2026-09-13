//! 消息和事件测试共用的纯数据夹具，不连接服务或读取时钟。

use z_ai::{
    identity::{ApiId, ModelId, ProviderId, ToolCallId, UnixMillis},
    message::{AssistantContent, AssistantMessage, ResponseIdentity, TextContent, ToolCall},
    model::{TokenAmounts, Usage, UsageCost, UsageCounts},
    value::{JsonObject, JsonValue},
};

#[track_caller]
pub fn usage() -> Usage {
    Usage::new(
        UsageCounts::new(10, 8, 3, 4),
        999,
        UsageCost::new(TokenAmounts::new(0.1, 0.2, 0.3, 0.4).unwrap(), 5.0).unwrap(),
    )
    .with_reasoning(2)
    .unwrap()
    .with_cache_write_1h(1)
    .unwrap()
}

#[track_caller]
pub fn identity() -> ResponseIdentity {
    ResponseIdentity::new(
        ProviderId::new("custom-provider".into()).unwrap(),
        ApiId::new("custom-api".into()).unwrap(),
        ModelId::new("requested-model".into()).unwrap(),
    )
}

pub fn assistant() -> AssistantMessage {
    AssistantMessage::new(
        identity(),
        vec![AssistantContent::Text(
            TextContent::new("部分回答".into()).with_text_signature("legacy-id".into()),
        )],
        usage(),
        UnixMillis::new(-1000),
    )
}

#[track_caller]
pub fn tool_call() -> ToolCall {
    ToolCall::new(
        ToolCallId::new("call-1".into()).unwrap(),
        "search".into(),
        JsonObject::from([("query".into(), JsonValue::from("最终查询"))]),
    )
    .with_namespace("tools.local".into())
    .with_thought_signature("opaque-thought".into())
}
