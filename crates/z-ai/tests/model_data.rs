//! 从公共 API 验证模型、费率阶梯、思考映射和服务端用量的保存语义。
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use z_ai::{
    Error,
    identity::{ApiId, ModelId, ProviderId},
    model::{
        InputModality, Model, ModelCapabilities, ModelCompat, ModelCost, ModelCostRates,
        ModelCostTier, ModelIdentity, ModelThinkingLevel, ThinkingLevel, ThinkingMapping,
        TokenAmounts, Usage, UsageCost, UsageCounts,
    },
    value::{JsonObject, JsonValue},
};

#[test]
fn usage_preserves_reported_total_and_all_independent_subsets() {
    let counts = UsageCounts::new(11, 22, 33, 44);
    assert_eq!(counts.input(), 11);
    assert_eq!(counts.output(), 22);
    assert_eq!(counts.cache_read(), 33);
    assert_eq!(counts.cache_write(), 44);
    let cost = UsageCost::new(TokenAmounts::new(0.25, 0.5, 0.75, 1.0).unwrap(), 91.5).unwrap();
    let usage = Usage::new(counts, 999, cost)
        .with_reasoning(7)
        .unwrap()
        .with_cache_write_1h(13)
        .unwrap();
    assert_eq!(usage.input(), 11);
    assert_eq!(usage.output(), 22);
    assert_eq!(usage.cache_read(), 33);
    assert_eq!(usage.cache_write(), 44);
    assert_eq!(usage.reasoning(), Some(7));
    assert_eq!(usage.cache_write_1h(), Some(13));
    assert_eq!(usage.total_tokens(), 999, "总量不能从可见子项重算");
    assert_eq!(usage.cost().input().to_bits(), 0.25_f64.to_bits());
    assert_eq!(usage.cost().output().to_bits(), 0.5_f64.to_bits());
    assert_eq!(usage.cost().cache_read().to_bits(), 0.75_f64.to_bits());
    assert_eq!(usage.cost().cache_write().to_bits(), 1.0_f64.to_bits());
    assert_eq!(
        usage.cost().total().to_bits(),
        91.5_f64.to_bits(),
        "总费用不能重算"
    );
}

#[test]
fn usage_distinguishes_unreported_zero_and_rejects_out_of_subset_counts() {
    let usage = Usage::new(UsageCounts::new(0, 0, 0, 0), 0, zero_cost());
    assert_eq!(usage.reasoning(), None);
    assert_eq!(usage.cache_write_1h(), None);
    let zero = usage
        .clone()
        .with_reasoning(0)
        .unwrap()
        .with_cache_write_1h(0)
        .unwrap();
    assert_eq!(zero.reasoning(), Some(0));
    assert_eq!(zero.cache_write_1h(), Some(0));
    let error = usage.clone().with_reasoning(1).unwrap_err();
    assert!(matches!(error, Error::InvalidInput(_)));
    assert_eq!(error.to_string(), "参数错误: reasoning 不得超过 output");
    assert_eq!(
        usage.with_cache_write_1h(1).unwrap_err().to_string(),
        "参数错误: cache_write_1h 不得超过 cache_write"
    );
    let max = Usage::new(
        UsageCounts::new(u64::MAX, u64::MAX, u64::MAX, u64::MAX),
        1,
        zero_cost(),
    )
    .with_reasoning(u64::MAX)
    .unwrap()
    .with_cache_write_1h(u64::MAX)
    .unwrap();
    assert_eq!(max.total_tokens(), 1, "不应求和溢出或重复累计子集");
}

#[test]
fn costs_and_rates_reject_each_invalid_component_without_computing_prices() {
    for invalid in [-0.25, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(TokenAmounts::new(invalid, 0.0, 0.0, 0.0).is_err());
        assert!(TokenAmounts::new(0.0, invalid, 0.0, 0.0).is_err());
        assert!(TokenAmounts::new(0.0, 0.0, invalid, 0.0).is_err());
        assert!(TokenAmounts::new(0.0, 0.0, 0.0, invalid).is_err());
        assert!(UsageCost::new(TokenAmounts::new(0.0, 0.0, 0.0, 0.0).unwrap(), invalid).is_err());
        assert!(ModelCostRates::new(invalid, 0.0, 0.0, 0.0).is_err());
        assert!(ModelCostRates::new(0.0, invalid, 0.0, 0.0).is_err());
        assert!(ModelCostRates::new(0.0, 0.0, invalid, 0.0).is_err());
        assert!(ModelCostRates::new(0.0, 0.0, 0.0, invalid).is_err());
    }
    let amounts = TokenAmounts::new(0.0, f64::MAX, 1.0, 2.0).unwrap();
    assert_eq!(amounts.output().to_bits(), f64::MAX.to_bits());
    assert_eq!(
        TokenAmounts::new(-1.0, 0.0, 0.0, 0.0)
            .unwrap_err()
            .to_string(),
        "参数错误: 费用必须是有限非负数"
    );
}

#[test]
fn model_preserves_every_capability_and_optional_configuration_field() {
    let identity = ModelIdentity::new(
        "model/custom".parse().unwrap(),
        "自定义显示名".into(),
        "custom-api".parse().unwrap(),
        "custom-provider".parse().unwrap(),
    );
    assert_eq!(identity.name(), "自定义显示名");
    let capabilities = ModelCapabilities::new(
        true,
        vec![InputModality::Image, InputModality::Text],
        999_001,
        83_001,
    );
    assert!(capabilities.reasoning());
    assert_eq!(capabilities.context_window(), 999_001);
    assert_eq!(capabilities.max_tokens(), 83_001);
    let rates = ModelCostRates::new(0.25, 2.0, 0.5, 1.0).unwrap();
    let cost = ModelCost::new(rates).with_tiers(vec![
        ModelCostTier::new(200_000, ModelCostRates::new(3.0, 4.0, 5.0, 6.0).unwrap()),
        ModelCostTier::new(100_000, ModelCostRates::new(7.0, 8.0, 9.0, 10.0).unwrap()),
    ]);
    let compat = ModelCompat::new(
        "custom-api".parse().unwrap(),
        object([("unknown", JsonValue::Null)]),
    );
    let model = Model::new(
        identity,
        "https://example.invalid/private-endpoint".into(),
        capabilities,
        cost,
    )
    .with_thinking_level_map(BTreeMap::from([(
        ModelThinkingLevel::Off,
        ThinkingMapping::Unsupported,
    )]))
    .with_sampling_params(object([("top_k", number(17))]))
    .with_headers(BTreeMap::from([("x-model".into(), "model-header".into())]))
    .with_compat(compat);
    assert_eq!(model.id().as_str(), "model/custom");
    assert_eq!(model.name(), "自定义显示名");
    assert_eq!(model.api().as_str(), "custom-api");
    assert_eq!(model.provider().as_str(), "custom-provider");
    assert_eq!(model.base_url(), "https://example.invalid/private-endpoint");
    assert!(model.reasoning());
    assert_eq!(model.input(), &[InputModality::Image, InputModality::Text]);
    assert_eq!(model.context_window(), 999_001);
    assert_eq!(model.max_tokens(), 83_001);
    let rates = model.cost().rates();
    assert_eq!(rates.input().to_bits(), 0.25_f64.to_bits());
    assert_eq!(rates.output().to_bits(), 2.0_f64.to_bits());
    assert_eq!(rates.cache_read().to_bits(), 0.5_f64.to_bits());
    assert_eq!(rates.cache_write().to_bits(), 1.0_f64.to_bits());
    let tiers = model.cost().tiers().unwrap();
    assert_eq!(tiers[0].input_tokens_above(), 200_000);
    assert_eq!(
        tiers[1].input_tokens_above(),
        100_000,
        "不排序，也不执行选择阶梯算法"
    );
    assert_eq!(tiers[1].rates().output().to_bits(), 8.0_f64.to_bits());
    assert_eq!(
        model
            .thinking_level_map()
            .unwrap()
            .get(&ModelThinkingLevel::Off),
        Some(&ThinkingMapping::Unsupported)
    );
    assert_eq!(
        model.sampling_params().unwrap().get("top_k"),
        Some(&number(17))
    );
    assert_eq!(
        model.headers().unwrap().get("x-model").map(String::as_str),
        Some("model-header")
    );
    assert_eq!(model.compat().unwrap().api().as_str(), "custom-api");
    assert_eq!(
        model.compat().unwrap().values().get("unknown"),
        Some(&JsonValue::Null)
    );
}

#[test]
fn absent_model_configuration_is_not_replaced_by_empty_or_zero_defaults() {
    let model = model_fixture("p", "m", "api-one");
    assert!(!model.reasoning());
    assert!(model.input().is_empty());
    assert_eq!(model.context_window(), 0);
    assert_eq!(model.max_tokens(), 0);
    assert!(model.cost().tiers().is_none());
    assert!(model.thinking_level_map().is_none());
    assert!(model.sampling_params().is_none());
    assert!(model.headers().is_none());
    assert!(model.compat().is_none());
    let empty = model
        .with_headers(BTreeMap::new())
        .with_sampling_params(JsonObject::new())
        .with_thinking_level_map(BTreeMap::new());
    assert!(empty.headers().unwrap().is_empty());
    assert_eq!(empty.sampling_params().unwrap().iter().count(), 0);
    assert!(empty.thinking_level_map().unwrap().is_empty());
    let cost =
        ModelCost::new(ModelCostRates::new(0.0, 0.0, 0.0, 0.0).unwrap()).with_tiers(Vec::new());
    assert_eq!(cost.tiers().unwrap().len(), 0);
}

#[test]
fn model_identity_ignores_api_and_configuration_but_not_provider_or_id() {
    let original = model_fixture("p", "m", "api-one");
    let changed = model_fixture("p", "m", "api-two").with_headers(BTreeMap::new());
    assert!(original.same_identity(&changed));
    assert_ne!(original, changed);
    assert!(!original.same_identity(&model_fixture("other", "m", "api-one")));
    assert!(!original.same_identity(&model_fixture("p", "other", "api-one")));
    let labelled = Model::new(
        ModelIdentity::new(
            "m".parse().unwrap(),
            "改名".into(),
            "api-three".parse().unwrap(),
            "p".parse().unwrap(),
        ),
        "different-endpoint".into(),
        ModelCapabilities::new(true, vec![InputModality::Text], 9, 8),
        ModelCost::new(ModelCostRates::new(1.0, 2.0, 3.0, 4.0).unwrap()),
    );
    assert!(original.same_identity(&labelled));
}

#[test]
fn thinking_map_preserves_all_levels_missing_unsupported_and_native_empty_string() {
    let pairs = [
        (ThinkingLevel::Minimal, ModelThinkingLevel::Minimal),
        (ThinkingLevel::Low, ModelThinkingLevel::Low),
        (ThinkingLevel::Medium, ModelThinkingLevel::Medium),
        (ThinkingLevel::High, ModelThinkingLevel::High),
        (ThinkingLevel::XHigh, ModelThinkingLevel::XHigh),
        (ThinkingLevel::Max, ModelThinkingLevel::Max),
    ];
    let mut map = BTreeMap::new();
    assert_eq!(map.get(&ModelThinkingLevel::Off), None);
    map.insert(ModelThinkingLevel::Off, ThinkingMapping::Unsupported);
    for (request, model) in pairs {
        assert_eq!(ModelThinkingLevel::from(request), model);
        map.insert(
            model,
            ThinkingMapping::Mapped(format!("native-{request:?}")),
        );
    }
    assert_eq!(map.len(), 7);
    for (_, level) in pairs {
        assert!(
            matches!(map.get(&level), Some(ThinkingMapping::Mapped(value)) if value.starts_with("native-"))
        );
    }
    map.insert(
        ModelThinkingLevel::Minimal,
        ThinkingMapping::Mapped(String::new()),
    );
    let model = model_fixture("p", "m", "a").with_thinking_level_map(map);
    let map = model.thinking_level_map().unwrap();
    assert!(
        matches!(map.get(&ModelThinkingLevel::Minimal), Some(ThinkingMapping::Mapped(value)) if value.is_empty())
    );
    assert_eq!(
        map.get(&ModelThinkingLevel::Off),
        Some(&ThinkingMapping::Unsupported)
    );
}

fn compatibility_fixture() -> JsonObject {
    let routing = object([
        ("allow_fallbacks", JsonValue::Bool(false)),
        ("require_parameters", JsonValue::Bool(true)),
        (
            "sort",
            JsonValue::Object(object([
                ("by", text("price")),
                ("partition", JsonValue::Null),
            ])),
        ),
        (
            "max_price",
            JsonValue::Object(object([
                ("prompt", text("0.01")),
                ("completion", number(2)),
            ])),
        ),
        (
            "preferred_min_throughput",
            JsonValue::Object(object([("p50", number(70)), ("p99", number(5))])),
        ),
        ("preferred_max_latency", number(4)),
        (
            "only",
            JsonValue::Array(vec![text("vendor-a"), text("vendor-b")]),
        ),
    ]);
    let template = object([
        (
            "enabled",
            JsonValue::Object(object([
                ("$var", text("thinking.enabled")),
                ("omitWhenOff", JsonValue::Bool(false)),
            ])),
        ),
        (
            "effort",
            JsonValue::Object(object([
                ("$var", text("thinking.effort")),
                ("omitWhenOff", JsonValue::Bool(true)),
            ])),
        ),
        (
            "budget",
            JsonValue::Object(object([("$var", text("thinking.budget"))])),
        ),
        ("nullable", JsonValue::Null),
        ("flag", JsonValue::Bool(false)),
        ("zero", number(0)),
        ("plain", text("native")),
    ]);
    let fallbacks = JsonValue::Array(vec![JsonValue::Object(object([
        ("provider", text("anthropic")),
        ("model", text("fallback-v2")),
        (
            "cost",
            JsonValue::Object(object([
                ("input", number(1)),
                ("output", number(2)),
                ("cacheRead", number(3)),
                ("cacheWrite", number(4)),
            ])),
        ),
    ]))]);
    object([
        ("supportsStore", JsonValue::Bool(false)),
        ("thinkingFormat", text("chat-template")),
        ("chatTemplateKwargs", JsonValue::Object(template.clone())),
        ("chatTemplateArgs", JsonValue::Object(template)),
        ("openRouterRouting", JsonValue::Object(routing)),
        (
            "vercelGatewayRouting",
            JsonValue::Object(object([
                ("only", JsonValue::Array(vec![text("bedrock")])),
                (
                    "order",
                    JsonValue::Array(vec![text("anthropic"), text("openai")]),
                ),
            ])),
        ),
        ("allowedFallbackModels", fallbacks),
        ("extension", JsonValue::Null),
    ])
}

#[test]
fn compatibility_extensions_preserve_routing_fallback_and_dynamic_template_shapes() {
    let values = compatibility_fixture();
    let model = model_fixture("p", "m", "custom-api")
        .with_compat(ModelCompat::new("custom-api".parse().unwrap(), values));
    let values = model.compat().unwrap().values();
    assert_eq!(values.get("supportsStore"), Some(&JsonValue::Bool(false)));
    assert!(values.get("missing").is_none());
    assert_eq!(values.get("extension"), Some(&JsonValue::Null));
    let JsonValue::Object(routing) = values.get("openRouterRouting").unwrap() else {
        panic!("路由必须保留对象")
    };
    let JsonValue::Object(sort) = routing.get("sort").unwrap() else {
        panic!("排序必须保留对象")
    };
    assert_eq!(sort.get("partition"), Some(&JsonValue::Null));
    let JsonValue::Object(price) = routing.get("max_price").unwrap() else {
        panic!("价格必须保留对象")
    };
    assert_eq!(price.get("prompt"), Some(&text("0.01")));
    assert_eq!(price.get("completion"), Some(&number(2)));
    let JsonValue::Object(percentiles) = routing.get("preferred_min_throughput").unwrap() else {
        panic!("百分位必须保留对象")
    };
    assert_eq!(percentiles.get("p99"), Some(&number(5)));
    let JsonValue::Array(only) = routing.get("only").unwrap() else {
        panic!("顺序必须保留数组")
    };
    assert_eq!(only, &[text("vendor-a"), text("vendor-b")]);
    let JsonValue::Object(template) = values.get("chatTemplateKwargs").unwrap() else {
        panic!("模板必须保留对象")
    };
    let JsonValue::Object(enabled) = template.get("enabled").unwrap() else {
        panic!("变量必须保留对象")
    };
    assert_eq!(enabled.get("$var"), Some(&text("thinking.enabled")));
    assert_eq!(enabled.get("omitWhenOff"), Some(&JsonValue::Bool(false)));
    let JsonValue::Array(fallbacks) = values.get("allowedFallbackModels").unwrap() else {
        panic!("回退模型必须保留数组")
    };
    let JsonValue::Object(fallback) = &fallbacks[0] else {
        panic!("回退模型必须保留对象")
    };
    assert_eq!(fallback.get("model"), Some(&text("fallback-v2")));
    assert!(
        matches!(fallback.get("cost"), Some(JsonValue::Object(cost)) if cost.get("cacheWrite") == Some(&number(4)))
    );
}

#[test]
fn known_compatibility_families_retain_nondefault_flags_and_unknown_fields() {
    let families: [(&str, &[&str]); 4] = [
        (
            "openai-completions",
            &[
                "supportsStore",
                "supportsDeveloperRole",
                "supportsReasoningEffort",
                "supportsUsageInStreaming",
                "supportsFinishReason",
                "requiresToolResultName",
                "requiresAssistantAfterToolResult",
                "requiresThinkingAsText",
                "requiresReasoningContentOnAssistantMessages",
                "zaiToolStream",
                "supportsThinkingTokenBudget",
                "supportsOpenAIGrammarTools",
                "supportsStrictMode",
                "sendSessionAffinityHeaders",
                "supportsLongCacheRetention",
            ],
        ),
        (
            "openai-responses",
            &[
                "supportsDeveloperRole",
                "supportsLongCacheRetention",
                "supportsStrictMode",
                "supportsOpenAIGrammarTools",
                "supportsAdditionalTools",
                "supportsToolSearch",
                "supportsExplicitPromptCacheMode",
                "supportsMaxOutputTokens",
            ],
        ),
        (
            "anthropic-messages",
            &[
                "supportsEagerToolInputStreaming",
                "supportsLongCacheRetention",
                "sendSessionAffinityHeaders",
                "supportsCacheControlOnTools",
                "supportsTemperature",
                "forceAdaptiveThinking",
                "allowEmptySignature",
                "supportsStrictTools",
                "supportsMidConvoEffort",
                "supportsToolReferences",
            ],
        ),
        ("bedrock-converse-stream", &["supportsStrictMode"]),
    ];
    for (api, fields) in families {
        for flag in [false, true] {
            let mut values: JsonObject = fields
                .iter()
                .map(|field| ((*field).into(), JsonValue::Bool(flag)))
                .collect();
            values.insert("futureExtension".into(), JsonValue::Null);
            let model = model_fixture("p", "m", api)
                .with_compat(ModelCompat::new(api.parse().unwrap(), values));
            let compat = model.compat().unwrap();
            assert_eq!(compat.api().as_str(), api);
            for field in fields {
                assert_eq!(compat.values().get(field), Some(&JsonValue::Bool(flag)));
            }
            assert_eq!(
                compat.values().get("futureExtension"),
                Some(&JsonValue::Null)
            );
            assert!(compat.values().get("notProvided").is_none());
        }
    }
}

#[test]
fn model_and_inner_compat_debug_redact_entire_sensitive_containers() {
    let sentinel = "MODEL-SECRET-SENTINEL";
    let compat = ModelCompat::new(
        "public-api".parse().unwrap(),
        object([(sentinel, text(sentinel))]),
    );
    let model = Model::new(
        ModelIdentity::new(
            "m".parse().unwrap(),
            "label".into(),
            "a".parse().unwrap(),
            "p".parse().unwrap(),
        ),
        sentinel.into(),
        ModelCapabilities::new(false, Vec::new(), 0, 0),
        ModelCost::new(ModelCostRates::new(0.0, 0.0, 0.0, 0.0).unwrap()),
    )
    .with_headers(BTreeMap::from([(sentinel.into(), sentinel.into())]))
    .with_sampling_params(object([(sentinel, text(sentinel))]))
    .with_compat(compat.clone());
    for output in [
        format!("{compat:?}"),
        format!("{compat:#?}"),
        format!("{model:?}"),
        format!("{model:#?}"),
    ] {
        assert!(!output.contains(sentinel));
        assert!(output.contains("已脱敏"));
    }
    assert_eq!(model.base_url(), sentinel, "显式读取仍提供原始值");
}

#[test]
fn model_data_are_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Model>();
    assert_send_sync::<Usage>();
    assert_send_sync::<UsageCost>();
}

#[track_caller]
fn zero_cost() -> UsageCost {
    UsageCost::new(TokenAmounts::new(0.0, 0.0, 0.0, 0.0).unwrap(), 0.0).unwrap()
}

#[track_caller]
fn model_fixture(provider: &str, model: &str, api: &str) -> Model {
    Model::new(
        ModelIdentity::new(
            ModelId::new(model.into()).unwrap(),
            "name".into(),
            ApiId::new(api.into()).unwrap(),
            ProviderId::new(provider.into()).unwrap(),
        ),
        "https://example.invalid".into(),
        ModelCapabilities::new(false, Vec::new(), 0, 0),
        ModelCost::new(ModelCostRates::new(0.0, 0.0, 0.0, 0.0).unwrap()),
    )
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
