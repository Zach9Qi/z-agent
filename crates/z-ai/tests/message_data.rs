//! 三种消息、全部元数据及角色内容的外部 API 语义测试。
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod message_support;

use std::sync::Arc;

use z_ai::{
    identity::{ToolCallId, UnixMillis},
    message::{
        AssistantContent, AssistantMessage, AssistantMessageDiagnostic, Context, DeferredHandle,
        DiagnosticCode, DiagnosticErrorInfo, ImageContent, Message, StopReason, TextContent,
        TextPhase, TextSignatureV1, ThinkingContent, ToolResultMessage, UserContent,
        UserContentBlock, UserMessage,
    },
    tool::Tool,
    value::{JsonNumber, JsonObject, JsonSchema, JsonValue},
};

#[test]
fn preserves_user_text_or_ordered_text_image_blocks() {
    let text =
        TextContent::new("说明".into()).with_text_signature(r#"{"v":1,"id":"opaque"}"#.into());
    let image = ImageContent::new("AA==".into(), "image/png".into());
    let blocks = vec![
        UserContentBlock::Text(text.clone()),
        UserContentBlock::Image(image.clone()),
    ];
    let user = UserMessage::new(UserContent::Blocks(blocks.clone()), UnixMillis::new(42));
    assert_eq!(user.content(), &UserContent::Blocks(blocks));
    assert_eq!(user.timestamp().as_i64(), 42);
    assert_eq!(text.text(), "说明");
    assert_eq!(text.text_signature(), Some(r#"{"v":1,"id":"opaque"}"#));
    assert_eq!(image.data(), "AA==");
    assert_eq!(image.mime_type(), "image/png");
    let raw = UserMessage::new(
        UserContent::from("原始字符串".to_owned()),
        UnixMillis::new(0),
    );
    assert_eq!(raw.content(), &UserContent::Text("原始字符串".into()));
    let empty = UserContent::from(Vec::<UserContentBlock>::new());
    assert_ne!(empty, UserContent::Text(String::new()));
}

#[test]
fn preserves_thinking_redaction_tristate_and_opaque_signatures() {
    let absent = ThinkingContent::new(String::new());
    let visible = absent.clone().with_redacted(false);
    let redacted = ThinkingContent::new("[原始遮蔽内容]".into())
        .with_redacted(true)
        .with_thinking_signature("encrypted-replay".into());
    assert_eq!(absent.redacted(), None);
    assert_eq!(absent.thinking_signature(), None);
    assert_eq!(visible.redacted(), Some(false));
    assert_eq!(redacted.redacted(), Some(true));
    assert_eq!(redacted.thinking(), "[原始遮蔽内容]");
    assert_eq!(redacted.thinking_signature(), Some("encrypted-replay"));
    for phase in [TextPhase::Commentary, TextPhase::FinalAnswer] {
        let signature = TextSignatureV1::new("id-v1".into()).with_phase(phase);
        assert_eq!(signature.version(), 1);
        assert_eq!(signature.id(), "id-v1");
        assert_eq!(signature.phase(), Some(phase));
    }
    assert_eq!(TextSignatureV1::new(String::new()).phase(), None);
}

#[test]
fn preserves_tool_call_object_namespace_and_thought_signature() {
    let call = message_support::tool_call();
    assert_eq!(call.id().as_str(), "call-1");
    assert_eq!(call.name(), "search");
    assert_eq!(call.namespace(), Some("tools.local"));
    assert_eq!(call.thought_signature(), Some("opaque-thought"));
    assert_eq!(
        call.arguments().get("query"),
        Some(&JsonValue::from("最终查询"))
    );
}

#[test]
fn preserves_every_assistant_metadata_field_and_reported_total() {
    let timestamp = UnixMillis::new(-1000);
    let diagnostic_error = DiagnosticErrorInfo::new("运行时故障".into())
        .with_name("ProviderFailure".into())
        .with_stack("opaque stack".into())
        .with_code(DiagnosticCode::Number(JsonNumber::new(4.5).unwrap()));
    assert_eq!(diagnostic_error.name(), Some("ProviderFailure"));
    assert_eq!(diagnostic_error.message(), "运行时故障");
    assert_eq!(diagnostic_error.stack(), Some("opaque stack"));
    assert_eq!(
        diagnostic_error.code(),
        Some(&DiagnosticCode::Number(JsonNumber::new(4.5).unwrap()))
    );
    let string_code =
        DiagnosticErrorInfo::new(String::new()).with_code(DiagnosticCode::String("ERR_X".into()));
    assert_eq!(
        string_code.code(),
        Some(&DiagnosticCode::String("ERR_X".into()))
    );
    let diagnostic = AssistantMessageDiagnostic::new("retry".into(), timestamp)
        .with_error(diagnostic_error.clone())
        .with_details(JsonObject::new());
    assert_eq!(diagnostic.kind(), "retry");
    assert_eq!(diagnostic.timestamp(), timestamp);
    assert_eq!(diagnostic.error(), Some(&diagnostic_error));
    assert_eq!(diagnostic.details(), Some(&JsonObject::new()));
    let deferred = DeferredHandle::new(message_support::identity(), "batch:row".into())
        .with_expires_at(UnixMillis::new(3000))
        .with_poll_after_ms(0)
        .with_data(JsonValue::Null);
    assert_eq!(deferred.id(), "batch:row");
    assert_eq!(deferred.identity().provider().as_str(), "custom-provider");
    assert_eq!(deferred.identity().api().as_str(), "custom-api");
    assert_eq!(deferred.identity().model().as_str(), "requested-model");
    assert_eq!(deferred.expires_at(), Some(UnixMillis::new(3000)));
    assert_eq!(deferred.poll_after_ms(), Some(0));
    assert_eq!(deferred.data(), Some(&JsonValue::Null));
    let content = vec![
        AssistantContent::Text(TextContent::new("回答".into())),
        AssistantContent::Thinking(ThinkingContent::new("思考".into())),
        AssistantContent::ToolCall(message_support::tool_call()),
    ];
    let message = AssistantMessage::new(
        message_support::identity(),
        content.clone(),
        message_support::usage(),
        timestamp,
    )
    .with_stop_reason(StopReason::Error)
    .with_response_model("routed-model".into())
    .with_response_id("response-id".into())
    .with_provider_thinking_level("native-effort".into())
    .with_diagnostics(vec![diagnostic.clone()])
    .with_deferred(deferred.clone())
    .with_error_message("部分失败".into())
    .with_raw_stop_reason("provider-specific".into())
    .with_end_turn(false);
    assert_eq!(message.identity(), &message_support::identity());
    assert_eq!(message.content(), content);
    assert_eq!(message.timestamp(), timestamp);
    assert_eq!(message.usage().total_tokens(), 999);
    assert_eq!(message.usage().reasoning(), Some(2));
    assert_eq!(message.usage().cache_write_1h(), Some(1));
    assert_eq!(message.stop_reason(), StopReason::Error);
    assert_eq!(message.response_model(), Some("routed-model"));
    assert_eq!(message.response_id(), Some("response-id"));
    assert_eq!(message.provider_thinking_level(), Some("native-effort"));
    assert_eq!(message.diagnostics(), Some([diagnostic].as_slice()));
    assert_eq!(message.deferred(), Some(&deferred));
    assert_eq!(message.error_message(), Some("部分失败"));
    assert_eq!(message.raw_stop_reason(), Some("provider-specific"));
    assert_eq!(message.end_turn(), Some(false));
}

#[test]
fn preserves_unreported_and_explicit_empty_assistant_metadata() {
    let absent = message_support::assistant();
    assert_eq!(absent.stop_reason(), StopReason::Pending);
    assert_eq!(absent.response_model(), None);
    assert_eq!(absent.response_id(), None);
    assert_eq!(absent.provider_thinking_level(), None);
    assert_eq!(absent.diagnostics(), None);
    assert_eq!(absent.deferred(), None);
    assert_eq!(absent.error_message(), None);
    assert_eq!(absent.raw_stop_reason(), None);
    assert_eq!(absent.end_turn(), None);
    let provided = absent
        .with_response_model(String::new())
        .with_response_id(String::new())
        .with_provider_thinking_level(String::new())
        .with_diagnostics(Vec::new())
        .with_error_message(String::new())
        .with_raw_stop_reason(String::new())
        .with_end_turn(true);
    assert_eq!(provided.response_model(), Some(""));
    assert_eq!(provided.response_id(), Some(""));
    assert_eq!(provided.provider_thinking_level(), Some(""));
    assert_eq!(provided.diagnostics(), Some([].as_slice()));
    assert_eq!(provided.error_message(), Some(""));
    assert_eq!(provided.raw_stop_reason(), Some(""));
    assert_eq!(provided.end_turn(), Some(true));
    let handle = DeferredHandle::new(message_support::identity(), String::new());
    assert_eq!(handle.data(), None);
    assert_eq!(handle.poll_after_ms(), None);
    assert_eq!(handle.expires_at(), None);
}

#[test]
fn context_preserves_order_generic_host_details_and_tool_usage_isolation() {
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct HostDetails(Arc<Vec<u8>>);
    let details = HostDetails(Arc::new(vec![0, 255]));
    let image = UserContentBlock::Image(ImageContent::new("AA==".into(), "image/png".into()));
    let tool_result = ToolResultMessage::new(
        ToolCallId::new("call-1".into()).unwrap(),
        "search".into(),
        vec![image.clone()],
        UnixMillis::new(99),
    )
    .with_details(details.clone())
    .with_usage(message_support::usage())
    .with_added_tool_names(vec!["loaded".into()])
    .with_is_error(true);
    assert_eq!(tool_result.tool_call_id().as_str(), "call-1");
    assert_eq!(tool_result.tool_name(), "search");
    assert_eq!(tool_result.content(), [image]);
    assert_eq!(tool_result.details(), Some(&details));
    assert!(Arc::ptr_eq(&tool_result.details().unwrap().0, &details.0));
    assert_eq!(
        tool_result.added_tool_names(),
        Some(["loaded".to_owned()].as_slice())
    );
    assert!(tool_result.is_error());
    assert_eq!(tool_result.timestamp(), UnixMillis::new(99));
    assert_eq!(tool_result.usage().unwrap().total_tokens(), 999);
    let messages = vec![
        UserMessage::new(UserContent::Text("输入".into()), UnixMillis::new(1)).into(),
        message_support::assistant().into(),
        tool_result.into(),
    ];
    let tool = Tool::new("search".into(), "搜索".into(), JsonSchema::Bool(true));
    let context: Context<HostDetails> = Context::new(messages)
        .with_system_prompt(String::new())
        .with_tools(vec![tool.clone()]);
    assert_eq!(context.system_prompt(), Some(""));
    assert_eq!(context.tools(), Some([tool].as_slice()));
    assert!(matches!(
        context.messages(),
        [
            Message::User(_),
            Message::Assistant(_),
            Message::ToolResult(_)
        ]
    ));
    if let Message::Assistant(message) = &context.messages()[1] {
        assert_eq!(
            message.usage().total_tokens(),
            999,
            "工具报告没有合并进模型用量"
        );
    }
}

#[test]
fn preserves_empty_tools_details_and_result_metadata() {
    let context: Context = Context::new(Vec::new());
    assert_eq!(context.system_prompt(), None);
    assert_eq!(context.tools(), None);
    assert_eq!(context.with_tools(Vec::new()).tools(), Some([].as_slice()));
    let result: ToolResultMessage = ToolResultMessage::new(
        ToolCallId::new("call".into()).unwrap(),
        String::new(),
        Vec::new(),
        UnixMillis::new(0),
    );
    assert_eq!(result.details(), None);
    assert_eq!(result.usage(), None);
    assert_eq!(result.added_tool_names(), None);
    assert!(!result.is_error());
    let explicit = result
        .with_details(JsonValue::Null)
        .with_added_tool_names(Vec::new());
    assert_eq!(explicit.details(), Some(&JsonValue::Null));
    assert_eq!(explicit.added_tool_names(), Some([].as_slice()));
}

#[test]
fn messages_remain_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Context>();
    assert_send_sync::<Message>();
    assert_send_sync::<AssistantMessage>();
    assert_send_sync::<ToolResultMessage>();
}
