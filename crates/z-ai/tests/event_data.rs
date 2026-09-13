//! 全部事件类型、共享实时 partial 与终态一致性测试，无异步运行时。
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod message_support;

use std::sync::{Arc, RwLock};

use z_ai::{
    Error,
    event::{AssistantMessageEvent, DoneEvent, DoneReason, ErrorEvent, ErrorReason},
    message::{AssistantContent, AssistantMessage, StopReason, TextContent},
};

#[test]
fn old_events_observe_later_changes_through_shared_partial_handles() {
    let partial = Arc::new(RwLock::new(message_support::assistant()));
    let start = AssistantMessageEvent::Start {
        partial: Arc::clone(&partial),
    };
    let delta = AssistantMessageEvent::TextDelta {
        content_index: 0,
        delta: "增量".into(),
        partial: Arc::clone(&partial),
    };
    let replacement = AssistantMessage::new(
        message_support::identity(),
        vec![AssistantContent::Text(
            TextContent::new("后续状态".into()).with_text_signature("signed".into()),
        )],
        message_support::usage(),
        1.into(),
    );
    *partial.write().unwrap() = replacement;
    assert!(Arc::ptr_eq(
        start.partial().unwrap(),
        delta.partial().unwrap()
    ));
    let old_view = start.partial().unwrap().read().unwrap();
    assert_eq!(
        old_view.content(),
        [AssistantContent::Text(
            TextContent::new("后续状态".into()).with_text_signature("signed".into())
        )]
    );
    assert_eq!(start.content_index(), None);
    assert_eq!(delta.content_index(), Some(0));
}

#[test]
fn stores_all_block_events_with_interleaving_and_authoritative_ends() {
    let events = [
        AssistantMessageEvent::TextStart {
            content_index: 2,
            partial: 7,
        },
        AssistantMessageEvent::ThinkingStart {
            content_index: 0,
            partial: 7,
        },
        AssistantMessageEvent::ToolCallStart {
            content_index: 1,
            partial: 7,
        },
        AssistantMessageEvent::TextDelta {
            content_index: 2,
            delta: "片段".into(),
            partial: 7,
        },
        AssistantMessageEvent::ThinkingDelta {
            content_index: 0,
            delta: "思".into(),
            partial: 7,
        },
        AssistantMessageEvent::ToolCallDelta {
            content_index: 1,
            delta: r#"{"query":"未完成"#.into(),
            partial: 7,
        },
        AssistantMessageEvent::ThinkingEnd {
            content_index: 0,
            content: "权威思考".into(),
            partial: 7,
        },
        AssistantMessageEvent::TextEnd {
            content_index: 2,
            content: "权威文本，不等于片段".into(),
            partial: 7,
        },
        AssistantMessageEvent::ToolCallEnd {
            content_index: 1,
            tool_call: message_support::tool_call(),
            partial: 7,
        },
    ];
    let indices: Vec<_> = events
        .iter()
        .map(AssistantMessageEvent::content_index)
        .collect();
    assert_eq!(
        indices,
        [
            Some(2),
            Some(0),
            Some(1),
            Some(2),
            Some(0),
            Some(1),
            Some(0),
            Some(2),
            Some(1)
        ]
    );
    for event in &events {
        assert_eq!(event.partial(), Some(&7));
    }
    assert!(
        matches!(&events[3], AssistantMessageEvent::TextDelta { delta, .. } if delta == "片段")
    );
    assert!(
        matches!(&events[4], AssistantMessageEvent::ThinkingDelta { delta, .. } if delta == "思")
    );
    assert!(
        matches!(&events[5], AssistantMessageEvent::ToolCallDelta { delta, .. } if delta == r#"{"query":"未完成"#)
    );
    assert!(
        matches!(&events[6], AssistantMessageEvent::ThinkingEnd { content, .. } if content == "权威思考")
    );
    assert!(
        matches!(&events[7], AssistantMessageEvent::TextEnd { content, .. } if content == "权威文本，不等于片段")
    );
    assert!(
        matches!(&events[8], AssistantMessageEvent::ToolCallEnd { tool_call, .. } if tool_call.thought_signature() == Some("opaque-thought"))
    );
}

#[test]
fn accepts_exactly_four_success_reasons_and_retains_terminal_values() {
    for reason in [
        DoneReason::Stop,
        DoneReason::Length,
        DoneReason::ToolUse,
        DoneReason::Deferred,
    ] {
        let message = message_support::assistant().with_stop_reason(reason.into());
        let record = DoneEvent::new(reason, message.clone()).unwrap();
        assert_eq!(record.reason(), reason);
        assert_eq!(record.message(), &message);
        assert_eq!(record.into_message(), message);
        let event: AssistantMessageEvent<()> =
            AssistantMessageEvent::done(reason, message).unwrap();
        assert_eq!(event.partial(), None);
        assert_eq!(event.content_index(), None);
        assert!(matches!(event, AssistantMessageEvent::Done(_)));
    }
}

#[test]
fn setup_can_fail_directly_and_failure_preserves_partial_usage_and_signatures() {
    for reason in [ErrorReason::Error, ErrorReason::Aborted] {
        let message = message_support::assistant()
            .with_stop_reason(reason.into())
            .with_error_message("失败".into());
        let record = ErrorEvent::new(reason, message.clone()).unwrap();
        assert_eq!(record.reason(), reason);
        assert_eq!(record.message().usage().total_tokens(), 999);
        assert_eq!(record.message().usage().reasoning(), Some(2));
        assert!(
            matches!(record.message().content(), [AssistantContent::Text(text)] if text.text_signature() == Some("legacy-id"))
        );
        assert_eq!(record.into_message(), message);
        let event: AssistantMessageEvent<()> =
            AssistantMessageEvent::error(reason, message).unwrap();
        assert!(matches!(event, AssistantMessageEvent::Error(_)));
        assert_eq!(event.partial(), None);
    }
}

#[test]
fn rejects_all_mismatched_terminal_reasons_including_pending() {
    let stops = [
        StopReason::Pending,
        StopReason::Stop,
        StopReason::Length,
        StopReason::ToolUse,
        StopReason::Error,
        StopReason::Aborted,
        StopReason::Deferred,
    ];
    for stop in stops {
        for reason in [
            DoneReason::Stop,
            DoneReason::Length,
            DoneReason::ToolUse,
            DoneReason::Deferred,
        ] {
            let result =
                DoneEvent::new(reason, message_support::assistant().with_stop_reason(stop));
            assert_eq!(result.is_ok(), StopReason::from(reason) == stop);
            if let Err(error) = result {
                assert!(matches!(error, Error::InvalidInput(_)));
            }
        }
        for reason in [ErrorReason::Error, ErrorReason::Aborted] {
            let result =
                ErrorEvent::new(reason, message_support::assistant().with_stop_reason(stop));
            assert_eq!(result.is_ok(), StopReason::from(reason) == stop);
        }
    }
    let error = DoneEvent::new(DoneReason::Stop, message_support::assistant()).unwrap_err();
    assert_eq!(
        error.to_string(),
        "参数错误: 成功事件原因与消息停止原因必须一致"
    );
    let error = ErrorEvent::new(ErrorReason::Error, message_support::assistant()).unwrap_err();
    assert_eq!(
        error.to_string(),
        "参数错误: 失败事件原因与消息停止原因必须一致"
    );
}

#[test]
fn a_thread_safe_host_handle_makes_events_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<AssistantMessageEvent<Arc<RwLock<AssistantMessage>>>>();
}
