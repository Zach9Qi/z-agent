//! 核心消息、事件自有格式往返及反序列化边界；不承诺上游 JSON 兼容。
#![cfg(feature = "serde")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod message_support;

use serde::{Deserialize, Serialize};
use serde_json::json;

use z_ai::{
    event::{AssistantMessageEvent, DoneReason, ErrorReason},
    identity::{ToolCallId, UnixMillis},
    message::{
        AssistantContent, AssistantMessage, AssistantMessageDiagnostic, Context, DeferredHandle,
        DiagnosticCode, DiagnosticErrorInfo, ImageContent, Message, StopReason, TextContent,
        TextPhase, TextSignatureV1, ThinkingContent, ToolCall, ToolResultMessage, UserContent,
        UserContentBlock, UserMessage,
    },
    tool::Tool,
    value::{JsonObject, JsonSchema, JsonValue},
};

#[track_caller]
fn round_trip<T>(value: &T)
where
    T: Serialize + for<'de> Deserialize<'de> + PartialEq + std::fmt::Debug,
{
    let text = serde_json::to_string(value).unwrap();
    let result: T = serde_json::from_str(&text).unwrap();
    assert_eq!(&result, value);
}

#[test]
fn round_trips_every_message_field_and_nested_explicit_null() {
    let diagnostic = AssistantMessageDiagnostic::new("retry".into(), UnixMillis::new(0))
        .with_error(
            DiagnosticErrorInfo::new("message".into())
                .with_name("name".into())
                .with_stack("stack".into())
                .with_code(DiagnosticCode::Number(42_i64.into())),
        )
        .with_details(JsonObject::from([("null".into(), JsonValue::Null)]));
    let assistant = AssistantMessage::new(
        message_support::identity(),
        vec![
            AssistantContent::Text(
                TextContent::new("text".into()).with_text_signature("opaque-v0".into()),
            ),
            AssistantContent::Thinking(
                ThinkingContent::new("redacted".into())
                    .with_thinking_signature("encrypted".into())
                    .with_redacted(true),
            ),
            AssistantContent::ToolCall(message_support::tool_call()),
        ],
        message_support::usage(),
        UnixMillis::new(i64::MIN),
    )
    .with_stop_reason(StopReason::Deferred)
    .with_response_model("routed".into())
    .with_response_id(String::new())
    .with_provider_thinking_level("native".into())
    .with_diagnostics(vec![diagnostic])
    .with_error_message(String::new())
    .with_raw_stop_reason("unknown".into())
    .with_end_turn(false)
    .with_deferred(
        DeferredHandle::new(message_support::identity(), "response".into())
            .with_data(JsonValue::Null)
            .with_poll_after_ms(0)
            .with_expires_at(UnixMillis::new(i64::MAX)),
    );
    let result = ToolResultMessage::new(
        ToolCallId::new("call-1".into()).unwrap(),
        "tool".into(),
        vec![
            UserContentBlock::Text(TextContent::new("result".into())),
            UserContentBlock::Image(ImageContent::new("AA==".into(), "image/png".into())),
        ],
        UnixMillis::new(4),
    )
    .with_details(JsonValue::Null)
    .with_usage(message_support::usage())
    .with_added_tool_names(vec!["loaded".into()])
    .with_is_error(true);
    let context: Context = Context::new(vec![
        UserMessage::new(UserContent::Text("user".into()), UnixMillis::new(0)).into(),
        UserMessage::new(UserContent::Blocks(Vec::new()), UnixMillis::new(1)).into(),
        assistant.into(),
        result.into(),
    ])
    .with_system_prompt(String::new())
    .with_tools(vec![Tool::new(
        "name".into(),
        "description".into(),
        JsonSchema::Bool(false),
    )]);
    round_trip(&context);
    let restored: Context =
        serde_json::from_str(&serde_json::to_string(&context).unwrap()).unwrap();
    assert!(
        matches!(&restored.messages()[3], Message::ToolResult(result) if result.details() == Some(&JsonValue::Null))
    );
}

#[test]
fn distinguishes_missing_details_and_deferred_data_from_null() {
    let empty: ToolResultMessage = ToolResultMessage::new(
        ToolCallId::new("call".into()).unwrap(),
        String::new(),
        Vec::new(),
        UnixMillis::new(0),
    );
    round_trip(&empty);
    round_trip(&empty.clone().with_details(JsonValue::Null));
    let mut encoded = serde_json::to_value(&empty).unwrap();
    encoded.as_object_mut().unwrap().remove("details");
    let restored: ToolResultMessage = serde_json::from_value(encoded).unwrap();
    assert_eq!(restored.details(), None);
    let handle = DeferredHandle::new(message_support::identity(), String::new());
    round_trip(&handle);
    round_trip(&handle.with_data(JsonValue::Null));
    for redacted in [None, Some(false), Some(true)] {
        let content = match redacted {
            Some(value) => ThinkingContent::new(String::new()).with_redacted(value),
            None => ThinkingContent::new(String::new()),
        };
        round_trip(&content);
    }
}

#[test]
fn serializes_generic_host_details_only_when_the_host_type_supports_it() {
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct HostDetails {
        bytes: Vec<u8>,
        generation: u64,
    }
    let result = ToolResultMessage::new(
        ToolCallId::new("call".into()).unwrap(),
        String::new(),
        Vec::new(),
        UnixMillis::new(0),
    )
    .with_details(HostDetails {
        bytes: vec![0, 255],
        generation: u64::MAX,
    });
    let context: Context<HostDetails> = Context::new(vec![result.into()]);
    round_trip(&context);
}

#[test]
fn preserves_present_host_details_that_serialize_to_null() {
    let missing: ToolResultMessage<Option<JsonValue>> = ToolResultMessage::new(
        ToolCallId::new("call".into()).unwrap(),
        String::new(),
        Vec::new(),
        UnixMillis::new(0),
    );
    let present_null = missing.clone().with_details(None);
    let present_json_null = missing.clone().with_details(Some(JsonValue::Null));
    round_trip(&missing);
    round_trip(&present_null);
    round_trip(&present_json_null);
    assert_ne!(
        serde_json::to_string(&missing).unwrap(),
        serde_json::to_string(&present_null).unwrap()
    );
    let encoded = serde_json::to_string(&present_null).unwrap();
    let restored: ToolResultMessage<Option<JsonValue>> = serde_json::from_str(&encoded).unwrap();
    assert_eq!(restored.details(), Some(&None));
}

#[test]
fn round_trips_all_nonterminal_events_and_authoritative_values() {
    let events = [
        AssistantMessageEvent::Start { partial: 8_u64 },
        AssistantMessageEvent::TextStart {
            content_index: 1,
            partial: 8,
        },
        AssistantMessageEvent::TextDelta {
            content_index: 1,
            delta: "text".into(),
            partial: 8,
        },
        AssistantMessageEvent::TextEnd {
            content_index: 1,
            content: "final text".into(),
            partial: 8,
        },
        AssistantMessageEvent::ThinkingStart {
            content_index: 2,
            partial: 8,
        },
        AssistantMessageEvent::ThinkingDelta {
            content_index: 2,
            delta: "thinking".into(),
            partial: 8,
        },
        AssistantMessageEvent::ThinkingEnd {
            content_index: 2,
            content: "final thought".into(),
            partial: 8,
        },
        AssistantMessageEvent::ToolCallStart {
            content_index: 0,
            partial: 8,
        },
        AssistantMessageEvent::ToolCallDelta {
            content_index: 0,
            delta: r#"{"x":"#.into(),
            partial: 8,
        },
        AssistantMessageEvent::ToolCallEnd {
            content_index: 0,
            tool_call: message_support::tool_call(),
            partial: 8,
        },
    ];
    for event in events {
        round_trip(&event);
    }
}

#[test]
fn terminal_deserialization_checks_reason_and_nested_message_invariants() {
    for reason in [
        DoneReason::Stop,
        DoneReason::Length,
        DoneReason::ToolUse,
        DoneReason::Deferred,
    ] {
        let event: AssistantMessageEvent<()> = AssistantMessageEvent::done(
            reason,
            message_support::assistant().with_stop_reason(reason.into()),
        )
        .unwrap();
        round_trip(&event);
        let mut invalid = serde_json::to_value(&event).unwrap();
        invalid["Done"]["message"]["stop_reason"] = json!("Pending");
        assert!(serde_json::from_value::<AssistantMessageEvent<()>>(invalid).is_err());
    }
    for reason in [ErrorReason::Error, ErrorReason::Aborted] {
        let event: AssistantMessageEvent<()> = AssistantMessageEvent::error(
            reason,
            message_support::assistant().with_stop_reason(reason.into()),
        )
        .unwrap();
        round_trip(&event);
        let mut invalid = serde_json::to_value(&event).unwrap();
        invalid["Error"]["message"]["stop_reason"] = json!("Stop");
        assert!(serde_json::from_value::<AssistantMessageEvent<()>>(invalid).is_err());
    }
    let event: AssistantMessageEvent<()> = AssistantMessageEvent::done(
        DoneReason::Stop,
        message_support::assistant().with_stop_reason(StopReason::Stop),
    )
    .unwrap();
    let mut invalid = serde_json::to_value(&event).unwrap();
    invalid["Done"]["message"]["usage"]["reasoning"] = json!(9);
    assert!(serde_json::from_value::<AssistantMessageEvent<()>>(invalid).is_err());
}

#[test]
fn nested_ids_role_content_and_signature_version_cannot_bypass_validation() {
    let mut invalid = serde_json::to_value(message_support::assistant()).unwrap();
    invalid["identity"]["model"] = json!(" ");
    assert!(serde_json::from_value::<AssistantMessage>(invalid).is_err());
    let mut invalid = serde_json::to_value(message_support::tool_call()).unwrap();
    invalid["arguments"] = json!([]);
    assert!(serde_json::from_value::<ToolCall>(invalid).is_err());
    let image = json!({"Image": {"data":"AA==", "mime_type":"image/png"}});
    assert!(serde_json::from_value::<AssistantContent>(image).is_err());
    let thinking =
        json!({"Thinking": {"thinking":"x", "thinking_signature":null, "redacted":null}});
    assert!(serde_json::from_value::<UserContentBlock>(thinking).is_err());
    for phase in [TextPhase::Commentary, TextPhase::FinalAnswer] {
        round_trip(&TextSignatureV1::new("id".into()).with_phase(phase));
    }
    assert!(
        serde_json::from_value::<TextSignatureV1>(json!({"v":2,"id":"id","phase":null})).is_err()
    );
    assert!(
        serde_json::from_value::<TextSignatureV1>(json!({"v":0,"id":"id","phase":null})).is_err()
    );
}
