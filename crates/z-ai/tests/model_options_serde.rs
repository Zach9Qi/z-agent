//! 可选 serde 的自有格式：逐字段观察、联合变体往返和构造不变量拒绝路径。
#![cfg(feature = "serde")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;
use z_ai::{
    model::{
        InputModality, Model, ModelCapabilities, ModelCompat, ModelCost, ModelCostRates,
        ModelCostTier, ModelIdentity, ModelThinkingLevel, ThinkingLevel, ThinkingMapping,
        TokenAmounts, Usage, UsageCost, UsageCounts,
    },
    options::{
        CacheRetention, DeferredConfig, DeferredFetchOptions, DeferredMode, DeferredWindow,
        ProviderOptions, ProviderResponse, RequestOptions, SimpleStreamOptions, StreamOptions,
        ThinkingBudgets, ToolChoice, Transport,
    },
    tool::{ConstrainedSampling, GrammarFormat, Strictness, Tool},
    value::{JsonObject, JsonSchema, JsonValue},
};

#[test]
fn usage_serde_checks_subsets_and_keeps_independent_totals_and_zero_breakdowns() {
    let usage = Usage::new(
        UsageCounts::new(11, 12, 13, 14),
        999,
        UsageCost::new(TokenAmounts::new(0.25, 0.5, 0.75, 1.0).unwrap(), 100.0).unwrap(),
    )
    .with_reasoning(0)
    .unwrap()
    .with_cache_write_1h(14)
    .unwrap();
    let restored: Usage = roundtrip(&usage);
    assert_eq!(restored.input(), 11);
    assert_eq!(restored.output(), 12);
    assert_eq!(restored.cache_read(), 13);
    assert_eq!(restored.cache_write(), 14);
    assert_eq!(restored.reasoning(), Some(0));
    assert_eq!(restored.cache_write_1h(), Some(14));
    assert_eq!(restored.total_tokens(), 999);
    assert_eq!(restored.cost().input().to_bits(), 0.25_f64.to_bits());
    assert_eq!(restored.cost().output().to_bits(), 0.5_f64.to_bits());
    assert_eq!(restored.cost().cache_read().to_bits(), 0.75_f64.to_bits());
    assert_eq!(restored.cost().cache_write().to_bits(), 1.0_f64.to_bits());
    assert_eq!(restored.cost().total().to_bits(), 100.0_f64.to_bits());
    let data = serde_json::to_value(&usage).unwrap();
    for (field, value) in [
        ("reasoning", json!(13)),
        ("cache_write_1h", json!(15)),
        ("total_tokens", json!(-1)),
    ] {
        let mut invalid = data.clone();
        invalid[field] = value;
        assert!(
            serde_json::from_value::<Usage>(invalid).is_err(),
            "反序列化必须拒绝 {field} 非法计数"
        );
    }
    for field in ["input", "output", "cache_read", "cache_write"] {
        let mut invalid = data.clone();
        invalid["counts"][field] = json!(-1);
        assert!(serde_json::from_value::<Usage>(invalid).is_err());
    }
    let mut missing = data;
    missing.as_object_mut().unwrap().remove("reasoning");
    missing.as_object_mut().unwrap().remove("cache_write_1h");
    let missing: Usage = serde_json::from_value(missing).unwrap();
    assert_eq!(missing.reasoning(), None);
    assert_eq!(missing.cache_write_1h(), None);
}

#[test]
fn amount_and_rate_deserialization_cannot_bypass_nonnegative_validation() {
    let amounts = TokenAmounts::new(1.0, 2.0, 3.0, 4.0).unwrap();
    for field in ["input", "output", "cache_read", "cache_write"] {
        let mut value = serde_json::to_value(&amounts).unwrap();
        value[field] = json!(-0.1);
        assert!(serde_json::from_value::<TokenAmounts>(value.clone()).is_err());
        let rate = json!({ "amounts": value });
        assert!(serde_json::from_value::<ModelCostRates>(rate).is_err());
    }
    let mut value = serde_json::to_value(UsageCost::new(amounts, 100.0).unwrap()).unwrap();
    value["total"] = json!(-1);
    assert!(serde_json::from_value::<UsageCost>(value).is_err());
    assert!(
        serde_json::from_str::<TokenAmounts>(
            r#"{"input":1e400,"output":0,"cache_read":0,"cache_write":0}"#
        )
        .is_err()
    );
}

#[test]
fn model_serde_preserves_all_fields_and_validates_nested_ids_rates_and_counts() {
    let model = model_fixture();
    let restored: Model = roundtrip(&model);
    assert_eq!(restored.id().as_str(), "model-x");
    assert_eq!(restored.provider().as_str(), "provider-x");
    assert_eq!(restored.api().as_str(), "api-x");
    assert_eq!(restored.name(), "name-x");
    assert_eq!(restored.base_url(), "https://example.invalid/private");
    assert!(restored.reasoning());
    assert_eq!(
        restored.input(),
        &[InputModality::Text, InputModality::Image]
    );
    assert_eq!(restored.context_window(), 999);
    assert_eq!(restored.max_tokens(), 111);
    assert_eq!(restored.cost().rates().input().to_bits(), 1.0_f64.to_bits());
    assert_eq!(
        restored.cost().tiers().unwrap()[0].input_tokens_above(),
        100
    );
    assert_eq!(
        restored.cost().tiers().unwrap()[0]
            .rates()
            .cache_write()
            .to_bits(),
        8.0_f64.to_bits()
    );
    assert_eq!(
        restored.headers().unwrap().get("x-key").map(String::as_str),
        Some("fixture-secret")
    );
    assert_eq!(
        restored.sampling_params().unwrap().get("n"),
        Some(&JsonValue::Null)
    );
    assert_eq!(restored.compat().unwrap().api().as_str(), "api-x");
    assert_eq!(
        restored.compat().unwrap().values().get("disabled"),
        Some(&JsonValue::Bool(false))
    );
    assert!(restored.compat().unwrap().values().get("missing").is_none());
    let data = serde_json::to_value(&model).unwrap();
    for field in ["id", "provider", "api"] {
        let mut invalid = data.clone();
        invalid["identity"][field] = json!("   ");
        assert!(serde_json::from_value::<Model>(invalid).is_err());
    }
    for field in ["context_window", "max_tokens"] {
        let mut invalid = data.clone();
        invalid["capabilities"][field] = json!(-1);
        assert!(serde_json::from_value::<Model>(invalid).is_err());
    }
    let mut invalid = data.clone();
    invalid["cost"]["tiers"][0]["input_tokens_above"] = json!(-1);
    assert!(serde_json::from_value::<Model>(invalid).is_err());
    let mut invalid = data;
    invalid["cost"]["rates"]["amounts"]["input"] = json!(-1);
    assert!(serde_json::from_value::<Model>(invalid).is_err());
}

#[test]
fn model_serde_distinguishes_missing_optional_containers_from_present_empty_ones() {
    let mut encoded = serde_json::to_value(model_fixture()).unwrap();
    for field in ["headers", "sampling_params", "compat", "thinking_level_map"] {
        encoded.as_object_mut().unwrap().remove(field);
    }
    encoded["cost"].as_object_mut().unwrap().remove("tiers");
    let missing: Model = serde_json::from_value(encoded).unwrap();
    assert!(missing.headers().is_none());
    assert!(missing.sampling_params().is_none());
    assert!(missing.compat().is_none());
    assert!(missing.thinking_level_map().is_none());
    assert!(missing.cost().tiers().is_none());
    let present: Model = roundtrip(
        &missing
            .with_headers(BTreeMap::new())
            .with_sampling_params(JsonObject::new())
            .with_thinking_level_map(BTreeMap::new())
            .with_compat(ModelCompat::new(
                "api-x".parse().unwrap(),
                JsonObject::new(),
            )),
    );
    assert!(present.headers().unwrap().is_empty());
    assert_eq!(present.sampling_params().unwrap().iter().count(), 0);
    assert!(present.thinking_level_map().unwrap().is_empty());
    assert_eq!(present.compat().unwrap().values().iter().count(), 0);
    let cost: ModelCost = roundtrip(
        &ModelCost::new(ModelCostRates::new(0.0, 0.0, 0.0, 0.0).unwrap()).with_tiers(Vec::new()),
    );
    assert!(cost.tiers().unwrap().is_empty());
}

#[test]
fn thinking_serde_keeps_all_keys_and_unsupported_distinct_from_missing() {
    let levels = [
        ModelThinkingLevel::Off,
        ModelThinkingLevel::Minimal,
        ModelThinkingLevel::Low,
        ModelThinkingLevel::Medium,
        ModelThinkingLevel::High,
        ModelThinkingLevel::XHigh,
        ModelThinkingLevel::Max,
    ];
    let map: BTreeMap<_, _> = levels
        .into_iter()
        .map(|level| (level, ThinkingMapping::Mapped(format!("native-{level:?}"))))
        .collect();
    let model: Model = roundtrip(&model_fixture().with_thinking_level_map(map));
    let map = model.thinking_level_map().unwrap();
    assert_eq!(map.len(), 7);
    for level in levels {
        assert!(
            matches!(map.get(&level), Some(ThinkingMapping::Mapped(native)) if native == &format!("native-{level:?}"))
        );
    }
    let model: Model = roundtrip(&model_fixture().with_thinking_level_map(BTreeMap::from([
        (ModelThinkingLevel::Off, ThinkingMapping::Unsupported),
        (
            ModelThinkingLevel::Low,
            ThinkingMapping::Mapped(String::new()),
        ),
    ])));
    let map = model.thinking_level_map().unwrap();
    assert_eq!(
        map.get(&ModelThinkingLevel::Off),
        Some(&ThinkingMapping::Unsupported)
    );
    assert_eq!(map.get(&ModelThinkingLevel::Minimal), None);
    assert!(
        matches!(map.get(&ModelThinkingLevel::Low), Some(ThinkingMapping::Mapped(native)) if native.is_empty())
    );
}

#[test]
fn tool_serde_roundtrips_complete_schema_and_all_sampling_variants() {
    let schema = JsonSchema::try_from(JsonValue::Object(object([
        ("$ref", JsonValue::String("#/$defs/x".into())),
        (
            "$defs",
            JsonValue::Object(object([("x", JsonValue::Bool(false))])),
        ),
        ("x-extra", JsonValue::Null),
    ])))
    .unwrap();
    let tool = Tool::new("lookup".into(), "工具说明".into(), schema);
    let restored: Tool = roundtrip(&tool);
    assert_eq!(restored.name(), "lookup");
    assert_eq!(restored.description(), "工具说明");
    assert!(restored.constrained_sampling().is_none());
    let JsonSchema::Object(schema) = restored.parameters() else {
        panic!("文档必须保留对象")
    };
    assert_eq!(schema.get("x-extra"), Some(&JsonValue::Null));
    assert!(
        matches!(schema.get("$defs"), Some(JsonValue::Object(defs)) if defs.get("x") == Some(&JsonValue::Bool(false)))
    );
    let variants = [
        ConstrainedSampling::Disabled,
        ConstrainedSampling::JsonSchema(Strictness::Prefer),
        ConstrainedSampling::JsonSchema(Strictness::Require),
        ConstrainedSampling::Grammar(BTreeMap::new()),
        ConstrainedSampling::Grammar(BTreeMap::from([
            (GrammarFormat::OpenAiLark, "lark".into()),
            (GrammarFormat::OpenAiRegex, "regex".into()),
        ])),
    ];
    for sampling in variants {
        let restored: Tool = roundtrip(&tool.clone().with_constrained_sampling(sampling.clone()));
        match (&sampling, restored.constrained_sampling().unwrap()) {
            (ConstrainedSampling::Disabled, ConstrainedSampling::Disabled) => (),
            (
                ConstrainedSampling::JsonSchema(expected),
                ConstrainedSampling::JsonSchema(actual),
            ) => assert_eq!(actual, expected),
            (ConstrainedSampling::Grammar(expected), ConstrainedSampling::Grammar(actual)) => {
                assert_eq!(actual.len(), expected.len());
                for (format, value) in expected {
                    assert_eq!(actual.get(format), Some(value));
                }
            }
            _ => panic!("约束采样变体必须原样保留"),
        }
    }
}

#[test]
fn request_and_stream_serde_keep_absence_header_deletion_and_nested_null_false_zero() {
    let request = RequestOptions::new()
        .with_headers(BTreeMap::from([
            ("delete".into(), None),
            ("set".into(), Some("value".into())),
        ]))
        .with_env(BTreeMap::from([("REGION".into(), "region-1".into())]))
        .with_timeout_ms(0)
        .with_max_retries(7)
        .with_max_retry_delay_ms(9);
    let stream = StreamOptions::new(request)
        .with_temperature(0.0)
        .unwrap()
        .with_max_tokens(0)
        .with_session_id(String::new())
        .with_websocket_connect_timeout_ms(8)
        .with_sampling_params(extension_values())
        .with_metadata(extension_values())
        .with_provider_options(ProviderOptions::new(
            "custom-api".parse().unwrap(),
            extension_values(),
        ));
    let restored: StreamOptions = roundtrip(&stream);
    let headers = restored.request().headers().unwrap();
    assert_eq!(headers.get("delete"), Some(&None));
    assert_eq!(headers.get("set"), Some(&Some("value".into())));
    assert_eq!(headers.get("missing"), None);
    assert_eq!(
        restored
            .request()
            .env()
            .unwrap()
            .get("REGION")
            .map(String::as_str),
        Some("region-1")
    );
    assert_eq!(restored.request().timeout_ms(), Some(0));
    assert_eq!(restored.request().max_retries(), Some(7));
    assert_eq!(restored.request().max_retry_delay_ms(), Some(9));
    assert_eq!(restored.temperature().unwrap().to_bits(), 0.0_f64.to_bits());
    assert_eq!(restored.max_tokens(), Some(0));
    assert_eq!(restored.session_id(), Some(""));
    assert_eq!(restored.websocket_connect_timeout_ms(), Some(8));
    assert_eq!(restored.transport(), None);
    assert_eq!(restored.cache_retention(), None);
    assert_eq!(
        restored.provider_options().unwrap().api().as_str(),
        "custom-api"
    );
    for values in [
        restored.sampling_params().unwrap(),
        restored.metadata().unwrap(),
        restored.provider_options().unwrap().values(),
    ] {
        assert!(values.get("missing").is_none());
        assert_eq!(values.get("null"), Some(&JsonValue::Null));
        assert_eq!(values.get("false"), Some(&JsonValue::Bool(false)));
        assert_eq!(values.get("zero"), Some(&JsonValue::Number(0_i64.into())));
        assert!(
            matches!(values.get("array"), Some(JsonValue::Array(array)) if array == &[JsonValue::Null, JsonValue::Bool(false)])
        );
    }
    let mut invalid = serde_json::to_value(&stream).unwrap();
    invalid["request"]["timeout_ms"] = json!(-1);
    assert!(serde_json::from_value::<StreamOptions>(invalid).is_err());
}

#[test]
fn simple_options_serde_roundtrips_every_enum_variant_and_budget_slot() {
    for transport in [
        Transport::Sse,
        Transport::WebSocket,
        Transport::WebSocketCached,
        Transport::Auto,
    ] {
        let restored: StreamOptions = roundtrip(&stream_fixture().with_transport(transport));
        assert_eq!(restored.transport(), Some(transport));
    }
    for retention in [
        CacheRetention::None,
        CacheRetention::Short,
        CacheRetention::Long,
    ] {
        let restored: StreamOptions = roundtrip(&stream_fixture().with_cache_retention(retention));
        assert_eq!(restored.cache_retention(), Some(retention));
    }
    for choice in [ToolChoice::Auto, ToolChoice::None] {
        let restored: SimpleStreamOptions =
            roundtrip(&SimpleStreamOptions::new(stream_fixture()).with_tool_choice(choice));
        assert_eq!(restored.tool_choice(), Some(choice));
    }
    for level in [
        ThinkingLevel::Minimal,
        ThinkingLevel::Low,
        ThinkingLevel::Medium,
        ThinkingLevel::High,
        ThinkingLevel::XHigh,
        ThinkingLevel::Max,
    ] {
        let restored: SimpleStreamOptions =
            roundtrip(&SimpleStreamOptions::new(stream_fixture()).with_reasoning(level));
        assert_eq!(restored.reasoning(), Some(level));
    }
    for window in [
        DeferredWindow::Minutes15,
        DeferredWindow::Hour1,
        DeferredWindow::Hours24,
    ] {
        let restored: SimpleStreamOptions =
            roundtrip(&SimpleStreamOptions::new(stream_fixture()).with_deferred(
                DeferredMode::Config(DeferredConfig::new().with_window(window)),
            ));
        assert!(
            matches!(restored.deferred(), Some(DeferredMode::Config(config)) if config.window() == Some(window))
        );
    }
    let empty: SimpleStreamOptions = roundtrip(&SimpleStreamOptions::new(stream_fixture()));
    assert!(empty.deferred().is_none());
    for mode in [
        DeferredMode::Disabled,
        DeferredMode::Enabled,
        DeferredMode::Config(DeferredConfig::new()),
    ] {
        let restored: SimpleStreamOptions =
            roundtrip(&SimpleStreamOptions::new(stream_fixture()).with_deferred(mode.clone()));
        assert_eq!(
            serde_json::to_value(restored.deferred()).unwrap(),
            serde_json::to_value(Some(mode)).unwrap()
        );
    }
    let restored: SimpleStreamOptions = roundtrip(
        &SimpleStreamOptions::new(stream_fixture()).with_thinking_budgets(
            ThinkingBudgets::new()
                .with_minimal(0)
                .with_low(1)
                .with_medium(2)
                .with_high(3),
        ),
    );
    let budgets = restored.thinking_budgets().unwrap();
    assert_eq!(budgets.minimal(), Some(0));
    assert_eq!(budgets.low(), Some(1));
    assert_eq!(budgets.medium(), Some(2));
    assert_eq!(budgets.high(), Some(3));
    for field in ["minimal", "low", "medium", "high"] {
        let mut invalid = serde_json::to_value(budgets).unwrap();
        invalid[field] = json!(-1);
        assert!(serde_json::from_value::<ThinkingBudgets>(invalid).is_err());
    }
}

#[test]
fn response_and_deferred_fetch_serde_keep_headers_status_and_wait() {
    let response: ProviderResponse = roundtrip(&ProviderResponse::new(
        202,
        BTreeMap::from([("secret".into(), "value".into())]),
    ));
    assert_eq!(response.status(), 202);
    assert_eq!(
        response.headers().get("secret").map(String::as_str),
        Some("value")
    );
    let fetch: DeferredFetchOptions =
        roundtrip(&DeferredFetchOptions::new(RequestOptions::new()).with_wait_ms(0));
    assert_eq!(fetch.wait_ms(), Some(0));
    assert!(fetch.request().headers().is_none());
    let mut invalid = serde_json::to_value(fetch).unwrap();
    invalid["wait_ms"] = json!(-1);
    assert!(serde_json::from_value::<DeferredFetchOptions>(invalid).is_err());
}

#[track_caller]
fn roundtrip<T: Serialize + DeserializeOwned>(value: &T) -> T {
    serde_json::from_str(&serde_json::to_string(value).unwrap()).unwrap()
}

#[track_caller]
fn model_fixture() -> Model {
    Model::new(
        ModelIdentity::new(
            "model-x".parse().unwrap(),
            "name-x".into(),
            "api-x".parse().unwrap(),
            "provider-x".parse().unwrap(),
        ),
        "https://example.invalid/private".into(),
        ModelCapabilities::new(
            true,
            vec![InputModality::Text, InputModality::Image],
            999,
            111,
        ),
        ModelCost::new(ModelCostRates::new(1.0, 2.0, 3.0, 4.0).unwrap()).with_tiers(vec![
            ModelCostTier::new(100, ModelCostRates::new(5.0, 6.0, 7.0, 8.0).unwrap()),
        ]),
    )
    .with_headers(BTreeMap::from([("x-key".into(), "fixture-secret".into())]))
    .with_sampling_params(object([("n", JsonValue::Null)]))
    .with_compat(ModelCompat::new(
        "api-x".parse().unwrap(),
        object([("disabled", JsonValue::Bool(false))]),
    ))
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
fn extension_values() -> JsonObject {
    object([
        ("null", JsonValue::Null),
        ("false", JsonValue::Bool(false)),
        ("zero", JsonValue::Number(0_i64.into())),
        (
            "array",
            JsonValue::Array(vec![JsonValue::Null, JsonValue::Bool(false)]),
        ),
    ])
}

// 自有格式允许任意 serde 数据源；后续测试不以 JSON 无法编码非有限数为校验替代。
#[derive(Clone)]
enum NonFiniteData {
    Request,
    Temperature(f64),
}

impl serde::de::IntoDeserializer<'_, serde::de::value::Error> for NonFiniteData {
    type Deserializer = Self;
    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

impl<'de> serde::Deserializer<'de> for NonFiniteData {
    type Error = serde::de::value::Error;
    fn deserialize_any<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        match self {
            Self::Request => {
                visitor.visit_map(serde::de::value::MapDeserializer::<_, Self::Error>::new(
                    std::iter::empty::<(&str, NonFiniteData)>(),
                ))
            }
            Self::Temperature(value) => visitor.visit_f64(value),
        }
    }
    fn deserialize_option<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_some(self)
    }
    serde::forward_to_deserialize_any! { bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes byte_buf unit unit_struct newtype_struct seq tuple tuple_struct map struct enum identifier ignored_any }
}

#[test]
fn public_amount_deserialization_checks_each_nonfinite_component_from_non_json_sources() {
    use serde::Deserialize;
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for bad_field in ["input", "output", "cache_read", "cache_write"] {
            let fields = ["input", "output", "cache_read", "cache_write"].map(|field| {
                let value = if field == bad_field { invalid } else { 0.0 };
                (field, NonFiniteData::Temperature(value))
            });
            let deserializer = serde::de::value::MapDeserializer::<_, serde::de::value::Error>::new(
                fields.into_iter(),
            );
            assert!(TokenAmounts::deserialize(deserializer).is_err());
        }
    }
    let valid = ["input", "output", "cache_read", "cache_write"]
        .map(|field| (field, NonFiniteData::Temperature(0.25)));
    let deserializer =
        serde::de::value::MapDeserializer::<_, serde::de::value::Error>::new(valid.into_iter());
    let amounts = TokenAmounts::deserialize(deserializer).unwrap();
    assert_eq!(amounts.input().to_bits(), 0.25_f64.to_bits());
}

#[test]
fn public_stream_deserialization_rejects_nonfinite_values_from_non_json_sources() {
    use serde::Deserialize;
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let fields = [
            ("request", NonFiniteData::Request),
            ("temperature", NonFiniteData::Temperature(invalid)),
        ];
        let deserializer = serde::de::value::MapDeserializer::<_, serde::de::value::Error>::new(
            fields.into_iter(),
        );
        assert!(StreamOptions::deserialize(deserializer).is_err());
    }
    let fields = [
        ("request", NonFiniteData::Request),
        ("temperature", NonFiniteData::Temperature(0.5)),
    ];
    let deserializer =
        serde::de::value::MapDeserializer::<_, serde::de::value::Error>::new(fields.into_iter());
    let stream = StreamOptions::deserialize(deserializer).unwrap();
    assert_eq!(stream.temperature().unwrap().to_bits(), 0.5_f64.to_bits());
}
