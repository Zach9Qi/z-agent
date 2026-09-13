//! 从公共 API 验证工具 schema、约束采样和通用／专有选项的所有数据形状。
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use z_ai::{
    Error,
    model::ThinkingLevel,
    options::{
        CacheRetention, DeferredCancelOptions, DeferredConfig, DeferredFetchOptions, DeferredMode,
        DeferredWindow, ProviderEnv, ProviderHeaders, ProviderOptions, ProviderResponse,
        RequestOptions, SimpleStreamOptions, StreamOptions, ThinkingBudgets, ToolChoice, Transport,
    },
    tool::{ConstrainedSampling, GrammarFormat, Strictness, Tool},
    value::{JsonObject, JsonSchema, JsonValue},
};

#[test]
fn tool_preserves_complete_schema_without_resolving_or_rewriting_it() {
    let document = JsonValue::Object(object([
        ("$ref", text("#/$defs/input")),
        (
            "$defs",
            JsonValue::Object(object([(
                "input",
                JsonValue::Object(object([
                    ("type", text("object")),
                    ("additionalProperties", JsonValue::Bool(false)),
                ])),
            )])),
        ),
        (
            "x-provider",
            JsonValue::Object(object([
                ("secret", text("opaque")),
                ("nullable", JsonValue::Null),
            ])),
        ),
    ]));
    let schema = JsonSchema::try_from(document.clone()).unwrap();
    let tool = Tool::new("custom.namespace/tool".into(), "完整描述".into(), schema);
    assert_eq!(tool.name(), "custom.namespace/tool");
    assert_eq!(tool.description(), "完整描述");
    let JsonSchema::Object(schema) = tool.parameters() else {
        panic!("schema 必须保留对象文档")
    };
    assert_eq!(JsonValue::Object(schema.clone()), document);
    assert_eq!(schema.get("$ref"), Some(&text("#/$defs/input")));
    assert!(tool.constrained_sampling().is_none());
    for boolean in [false, true] {
        let tool = Tool::new(String::new(), String::new(), JsonSchema::Bool(boolean));
        assert!(matches!(tool.parameters(), JsonSchema::Bool(value) if *value == boolean));
        assert_eq!(tool.name(), "", "核心不擅自执行提供方名称规则");
    }
}

#[test]
fn constrained_sampling_preserves_disabled_schema_and_every_grammar_variant() {
    let base = tool_fixture();
    assert!(base.constrained_sampling().is_none());
    let disabled = base
        .clone()
        .with_constrained_sampling(ConstrainedSampling::Disabled);
    assert!(matches!(
        disabled.constrained_sampling(),
        Some(ConstrainedSampling::Disabled)
    ));
    for strictness in [Strictness::Prefer, Strictness::Require] {
        let tool = base
            .clone()
            .with_constrained_sampling(ConstrainedSampling::JsonSchema(strictness));
        assert!(
            matches!(tool.constrained_sampling(), Some(ConstrainedSampling::JsonSchema(value)) if *value == strictness)
        );
    }
    let empty = base
        .clone()
        .with_constrained_sampling(ConstrainedSampling::Grammar(BTreeMap::new()));
    assert!(
        matches!(empty.constrained_sampling(), Some(ConstrainedSampling::Grammar(variants)) if variants.is_empty())
    );
    for (format, grammar) in [
        (GrammarFormat::OpenAiLark, "start: /[a-z]+/"),
        (GrammarFormat::OpenAiRegex, "^[a-z]+$"),
    ] {
        let tool = base
            .clone()
            .with_constrained_sampling(ConstrainedSampling::Grammar(BTreeMap::from([(
                format,
                grammar.into(),
            )])));
        let Some(ConstrainedSampling::Grammar(variants)) = tool.constrained_sampling() else {
            panic!("必须保留语法变体")
        };
        assert_eq!(variants.len(), 1);
        assert_eq!(variants.get(&format).map(String::as_str), Some(grammar));
    }
    let both = base.with_constrained_sampling(ConstrainedSampling::Grammar(BTreeMap::from([
        (GrammarFormat::OpenAiLark, String::new()),
        (GrammarFormat::OpenAiRegex, String::new()),
    ])));
    assert!(
        matches!(both.constrained_sampling(), Some(ConstrainedSampling::Grammar(variants)) if variants.len() == 2)
    );
}

#[test]
fn request_data_preserves_header_absence_set_delete_environment_and_retry_units() {
    let empty = RequestOptions::new();
    assert!(empty.env().is_none());
    assert!(empty.headers().is_none());
    assert_eq!(empty.timeout_ms(), None);
    assert_eq!(empty.max_retries(), None);
    assert_eq!(empty.max_retry_delay_ms(), None);
    let headers: ProviderHeaders = BTreeMap::from([
        ("x-set".into(), Some("value".into())),
        ("x-delete".into(), None),
        ("x-empty".into(), Some(String::new())),
    ]);
    let env: ProviderEnv = BTreeMap::from([
        ("REGION".into(), "moon-1".into()),
        ("EMPTY".into(), String::new()),
    ]);
    let request = RequestOptions::new()
        .with_env(env)
        .with_headers(headers)
        .with_timeout_ms(1_234)
        .with_max_retries(7)
        .with_max_retry_delay_ms(12_345);
    assert_eq!(
        request.env().unwrap().get("REGION").map(String::as_str),
        Some("moon-1")
    );
    assert_eq!(
        request.env().unwrap().get("EMPTY").map(String::as_str),
        Some("")
    );
    let headers = request.headers().unwrap();
    assert_eq!(headers.get("x-set"), Some(&Some("value".into())));
    assert_eq!(headers.get("x-delete"), Some(&None));
    assert_eq!(headers.get("x-empty"), Some(&Some(String::new())));
    assert_eq!(headers.get("absent"), None);
    assert_eq!(request.timeout_ms(), Some(1_234));
    assert_eq!(request.max_retries(), Some(7));
    assert_eq!(request.max_retry_delay_ms(), Some(12_345));
    let zero = RequestOptions::new()
        .with_timeout_ms(0)
        .with_max_retries(0)
        .with_max_retry_delay_ms(0)
        .with_env(BTreeMap::new())
        .with_headers(BTreeMap::new());
    assert_eq!(zero.timeout_ms(), Some(0));
    assert_eq!(zero.max_retries(), Some(0));
    assert_eq!(zero.max_retry_delay_ms(), Some(0));
    assert!(zero.headers().unwrap().is_empty());
    assert!(zero.env().unwrap().is_empty());
}

#[test]
fn stream_options_preserve_all_fields_and_do_not_merge_sampling_objects() {
    let stream = stream_fixture()
        .with_temperature(0.75)
        .unwrap()
        .with_sampling_params(object([("top_p", number(0)), ("custom", JsonValue::Null)]))
        .with_max_tokens(123)
        .with_transport(Transport::WebSocketCached)
        .with_cache_retention(CacheRetention::Long)
        .with_session_id("session-public".into())
        .with_websocket_connect_timeout_ms(456)
        .with_metadata(object([
            ("false", JsonValue::Bool(false)),
            ("nested", JsonValue::Object(object([("n", number(0))]))),
        ]))
        .with_provider_options(ProviderOptions::new(
            "custom-api".parse().unwrap(),
            object([("extra", JsonValue::Null)]),
        ));
    assert_eq!(stream.request().timeout_ms(), None);
    assert_eq!(stream.temperature().unwrap().to_bits(), 0.75_f64.to_bits());
    assert_eq!(
        stream.sampling_params().unwrap().get("custom"),
        Some(&JsonValue::Null)
    );
    assert_eq!(
        stream.sampling_params().unwrap().get("top_p"),
        Some(&number(0))
    );
    assert!(
        stream
            .sampling_params()
            .unwrap()
            .get("unsupplied")
            .is_none()
    );
    assert_eq!(stream.max_tokens(), Some(123));
    assert_eq!(stream.transport(), Some(Transport::WebSocketCached));
    assert_eq!(stream.cache_retention(), Some(CacheRetention::Long));
    assert_eq!(stream.session_id(), Some("session-public"));
    assert_eq!(stream.websocket_connect_timeout_ms(), Some(456));
    assert_eq!(
        stream.metadata().unwrap().get("false"),
        Some(&JsonValue::Bool(false))
    );
    assert!(
        matches!(stream.metadata().unwrap().get("nested"), Some(JsonValue::Object(nested)) if nested.get("n") == Some(&number(0)))
    );
    assert_eq!(
        stream.provider_options().unwrap().api().as_str(),
        "custom-api"
    );
    assert_eq!(
        stream.provider_options().unwrap().values().get("extra"),
        Some(&JsonValue::Null)
    );
}

#[test]
fn stream_omissions_zero_and_each_transport_or_cache_choice_are_distinct() {
    let empty = stream_fixture();
    assert_eq!(empty.temperature(), None);
    assert!(empty.sampling_params().is_none());
    assert_eq!(empty.max_tokens(), None);
    assert_eq!(empty.transport(), None);
    assert_eq!(empty.cache_retention(), None);
    assert_eq!(empty.session_id(), None);
    assert_eq!(empty.websocket_connect_timeout_ms(), None);
    assert!(empty.metadata().is_none());
    assert!(empty.provider_options().is_none());
    let zero = empty
        .clone()
        .with_temperature(0.0)
        .unwrap()
        .with_max_tokens(0)
        .with_session_id(String::new())
        .with_websocket_connect_timeout_ms(0)
        .with_metadata(JsonObject::new())
        .with_sampling_params(JsonObject::new());
    assert_eq!(zero.temperature().unwrap().to_bits(), 0.0_f64.to_bits());
    assert_eq!(zero.max_tokens(), Some(0));
    assert_eq!(zero.session_id(), Some(""));
    assert_eq!(zero.websocket_connect_timeout_ms(), Some(0));
    assert_eq!(zero.metadata().unwrap().iter().count(), 0);
    assert_eq!(zero.sampling_params().unwrap().iter().count(), 0);
    for transport in [
        Transport::Sse,
        Transport::WebSocket,
        Transport::WebSocketCached,
        Transport::Auto,
    ] {
        assert_eq!(
            empty.clone().with_transport(transport).transport(),
            Some(transport)
        );
    }
    for retention in [
        CacheRetention::None,
        CacheRetention::Short,
        CacheRetention::Long,
    ] {
        assert_eq!(
            empty
                .clone()
                .with_cache_retention(retention)
                .cache_retention(),
            Some(retention)
        );
    }
}

#[test]
fn temperature_constructor_rejects_nonfinite_without_imposing_provider_ranges() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let error = stream_fixture().with_temperature(value).unwrap_err();
        assert!(matches!(error, Error::InvalidInput(_)));
        assert_eq!(error.to_string(), "参数错误: temperature 必须是有限数值");
    }
    for value in [-1.0, 0.0, f64::MAX] {
        assert_eq!(
            stream_fixture()
                .with_temperature(value)
                .unwrap()
                .temperature()
                .unwrap()
                .to_bits(),
            value.to_bits()
        );
    }
}

#[test]
fn simple_options_cover_all_tool_thinking_and_deferred_shapes() {
    let empty = SimpleStreamOptions::new(stream_fixture());
    assert!(empty.tool_choice().is_none());
    assert!(empty.reasoning().is_none());
    assert!(empty.deferred().is_none());
    assert!(empty.thinking_budgets().is_none());
    assert_eq!(empty.stream().max_tokens(), None);
    for choice in [ToolChoice::Auto, ToolChoice::None] {
        assert_eq!(
            empty.clone().with_tool_choice(choice).tool_choice(),
            Some(choice)
        );
    }
    for reasoning in [
        ThinkingLevel::Minimal,
        ThinkingLevel::Low,
        ThinkingLevel::Medium,
        ThinkingLevel::High,
        ThinkingLevel::XHigh,
        ThinkingLevel::Max,
    ] {
        assert_eq!(
            empty.clone().with_reasoning(reasoning).reasoning(),
            Some(reasoning)
        );
    }
    assert!(matches!(
        empty
            .clone()
            .with_deferred(DeferredMode::Disabled)
            .deferred(),
        Some(DeferredMode::Disabled)
    ));
    assert!(matches!(
        empty
            .clone()
            .with_deferred(DeferredMode::Enabled)
            .deferred(),
        Some(DeferredMode::Enabled)
    ));
    let object = empty
        .clone()
        .with_deferred(DeferredMode::Config(DeferredConfig::new()));
    assert!(
        matches!(object.deferred(), Some(DeferredMode::Config(config)) if config.window().is_none())
    );
    for window in [
        DeferredWindow::Minutes15,
        DeferredWindow::Hour1,
        DeferredWindow::Hours24,
    ] {
        let options = empty.clone().with_deferred(DeferredMode::Config(
            DeferredConfig::new().with_window(window),
        ));
        assert!(
            matches!(options.deferred(), Some(DeferredMode::Config(config)) if config.window() == Some(window))
        );
    }
    let budgets = ThinkingBudgets::new();
    assert_eq!(budgets.minimal(), None);
    assert_eq!(budgets.low(), None);
    assert_eq!(budgets.medium(), None);
    assert_eq!(budgets.high(), None);
    let options = empty.with_thinking_budgets(
        budgets
            .with_minimal(0)
            .with_low(1)
            .with_medium(2)
            .with_high(u64::MAX),
    );
    let budgets = options.thinking_budgets().unwrap();
    assert_eq!(budgets.minimal(), Some(0));
    assert_eq!(budgets.low(), Some(1));
    assert_eq!(budgets.medium(), Some(2));
    assert_eq!(budgets.high(), Some(u64::MAX));
}

#[test]
fn provider_extensions_keep_sdk_shapes_nullable_summary_and_negative_native_budgets() {
    let anthropic = ProviderOptions::new(
        "anthropic-messages".parse().unwrap(),
        object([
            ("thinkingEnabled", JsonValue::Bool(false)),
            ("thinkingBudgetTokens", number(0)),
            ("effort", text("xhigh")),
            ("thinkingDisplay", text("omitted")),
            ("interleavedThinking", JsonValue::Bool(false)),
            (
                "toolChoice",
                JsonValue::Object(object([("type", text("tool")), ("name", text("lookup"))])),
            ),
        ]),
    );
    assert_eq!(anthropic.api().as_str(), "anthropic-messages");
    assert_eq!(
        anthropic.values().get("thinkingEnabled"),
        Some(&JsonValue::Bool(false))
    );
    assert_eq!(
        anthropic.values().get("thinkingBudgetTokens"),
        Some(&number(0))
    );
    assert_eq!(anthropic.values().get("effort"), Some(&text("xhigh")));
    assert_eq!(
        anthropic.values().get("thinkingDisplay"),
        Some(&text("omitted"))
    );
    assert!(
        matches!(anthropic.values().get("toolChoice"), Some(JsonValue::Object(choice)) if choice.get("name") == Some(&text("lookup")))
    );
    for api in ["openai-responses", "openai-codex-responses"] {
        let options = ProviderOptions::new(
            api.parse().unwrap(),
            object([
                ("reasoningEffort", text("max")),
                ("reasoningSummary", JsonValue::Null),
                ("serviceTier", text("flex")),
                ("toolChoice", text("required")),
            ]),
        );
        assert_eq!(options.api().as_str(), api);
        assert_eq!(
            options.values().get("reasoningSummary"),
            Some(&JsonValue::Null)
        );
        assert!(options.values().get("unsupplied").is_none());
        assert_eq!(options.values().get("reasoningEffort"), Some(&text("max")));
        assert_eq!(options.values().get("serviceTier"), Some(&text("flex")));
        assert_eq!(options.values().get("toolChoice"), Some(&text("required")));
    }
    let vertex = ProviderOptions::new(
        "google-vertex".parse().unwrap(),
        object([
            ("project", text("project-x")),
            ("location", text("region-y")),
            (
                "thinking",
                JsonValue::Object(object([
                    ("enabled", JsonValue::Bool(true)),
                    ("budgetTokens", number(-1)),
                    ("level", text("HIGH")),
                ])),
            ),
        ]),
    );
    assert_eq!(vertex.values().get("project"), Some(&text("project-x")));
    assert_eq!(vertex.values().get("location"), Some(&text("region-y")));
    let JsonValue::Object(thinking) = vertex.values().get("thinking").unwrap() else {
        panic!("提供方思考配置必须保留对象")
    };
    assert_eq!(thinking.get("budgetTokens"), Some(&number(-1)));
    assert_eq!(thinking.get("enabled"), Some(&JsonValue::Bool(true)));
    assert_eq!(thinking.get("level"), Some(&text("HIGH")));
}

#[test]
fn response_and_deferred_data_preserve_status_headers_and_millisecond_wait() {
    let response =
        ProviderResponse::new(429, BTreeMap::from([("retry-after".into(), "17".into())]));
    assert_eq!(response.status(), 429);
    assert_eq!(
        response.headers().get("retry-after").map(String::as_str),
        Some("17")
    );
    let request = RequestOptions::new().with_timeout_ms(100);
    let fetch = DeferredFetchOptions::new(request.clone());
    assert_eq!(fetch.request().timeout_ms(), Some(100));
    assert_eq!(fetch.wait_ms(), None);
    assert_eq!(fetch.clone().with_wait_ms(0).wait_ms(), Some(0));
    assert_eq!(fetch.with_wait_ms(5_000).wait_ms(), Some(5_000));
    let cancel: DeferredCancelOptions = request;
    assert_eq!(cancel.timeout_ms(), Some(100));
}

#[test]
fn every_sensitive_options_parent_and_response_redacts_normal_and_pretty_debug() {
    let secret = "OPTIONS-SECRET-SENTINEL";
    let request = RequestOptions::new()
        .with_headers(BTreeMap::from([(secret.into(), Some(secret.into()))]))
        .with_env(BTreeMap::from([(secret.into(), secret.into())]));
    let provider = ProviderOptions::new(
        "public-api".parse().unwrap(),
        object([(secret, text(secret))]),
    );
    let stream = StreamOptions::new(request.clone())
        .with_provider_options(provider.clone())
        .with_metadata(object([(secret, text(secret))]))
        .with_sampling_params(object([(secret, text(secret))]))
        .with_session_id(secret.into());
    let simple = SimpleStreamOptions::new(stream.clone());
    let response = ProviderResponse::new(200, BTreeMap::from([(secret.into(), secret.into())]));
    let fetch = DeferredFetchOptions::new(request.clone());
    let containers: [&dyn std::fmt::Debug; 6] =
        [&request, &provider, &stream, &simple, &response, &fetch];
    for value in containers {
        for output in [format!("{value:?}"), format!("{value:#?}")] {
            assert!(!output.contains(secret));
            assert!(output.contains("已脱敏"));
        }
    }
    assert_eq!(
        request.env().unwrap().get(secret).map(String::as_str),
        Some(secret)
    );
    assert_eq!(provider.values().get(secret), Some(&text(secret)));
}

#[test]
fn options_and_tools_are_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Tool>();
    assert_send_sync::<StreamOptions>();
    assert_send_sync::<SimpleStreamOptions>();
    assert_send_sync::<ProviderResponse>();
}

#[track_caller]
fn tool_fixture() -> Tool {
    Tool::new("tool".into(), "说明".into(), JsonSchema::Bool(true))
}
fn stream_fixture() -> StreamOptions {
    StreamOptions::new(RequestOptions::new())
}
fn object<const N: usize>(values: [(&str, JsonValue); N]) -> JsonObject {
    values
        .into_iter()
        .map(|(name, value)| (name.to_owned(), value))
        .collect()
}
fn text(value: &str) -> JsonValue {
    JsonValue::String(value.into())
}
fn number(value: i64) -> JsonValue {
    JsonValue::Number(value.into())
}
