//! 基础值的外部 API：标识、精度、空值、schema 及可选 serde。
#![allow(clippy::unwrap_used, clippy::expect_used)]

use z_ai::{
    AccountId, ApiId, Error, JsonNumber, JsonObject, JsonSchema, JsonValue, ModelId, ProviderId,
    ToolCallId, UnixMillis,
};

#[test]
fn rejects_empty_ids_without_normalizing_valid_inputs() {
    for invalid in ["", " ", "\t\r\n", "\u{2003}"] {
        assert!(matches!(
            ProviderId::new(invalid.into()),
            Err(Error::InvalidInput(_))
        ));
        assert!(ApiId::new(invalid.into()).is_err());
        assert!(ModelId::new(invalid.into()).is_err());
        assert!(AccountId::new(invalid.into()).is_err());
        assert!(ToolCallId::new(invalid.into()).is_err());
    }
    let provider: ProviderId = " custom/Provider ".parse().unwrap();
    assert_eq!(provider.as_str(), " custom/Provider ");
    assert_eq!(provider.to_string(), " custom/Provider ");
    assert_eq!(provider.clone().into_string(), " custom/Provider ");
    assert_eq!(
        ProviderId::try_from(provider.to_string()).unwrap(),
        provider
    );
}

#[test]
fn retains_signed_milliseconds_without_clock_or_unit_conversion() {
    for milliseconds in [i64::MIN, -1, 0, 1, i64::MAX] {
        assert_eq!(UnixMillis::new(milliseconds).as_i64(), milliseconds);
        assert_eq!(i64::from(UnixMillis::from(milliseconds)), milliseconds);
        assert_eq!(
            UnixMillis::new(milliseconds).to_string(),
            milliseconds.to_string()
        );
    }
}

#[test]
fn preserves_integer_categories_and_rejects_non_finite_floats() {
    assert_eq!(JsonNumber::from(i64::MIN).as_i64(), Some(i64::MIN));
    assert_eq!(JsonNumber::from(u64::MAX).as_u64(), Some(u64::MAX));
    assert_eq!(JsonNumber::from(u64::MAX).as_f64(), None);
    assert_eq!(JsonNumber::from(1_i64).as_u64(), None);
    assert_eq!(JsonNumber::new(1.5).unwrap().as_f64(), Some(1.5));
    assert_eq!(JsonNumber::new(1.0).unwrap().as_i64(), None);
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(matches!(
            JsonNumber::try_from(value),
            Err(Error::InvalidInput(_))
        ));
    }
    for value in [f64::MIN, f64::MAX, f64::MIN_POSITIVE, -0.0] {
        assert_eq!(
            JsonNumber::new(value).unwrap().as_f64().unwrap().to_bits(),
            value.to_bits()
        );
    }
}

#[test]
fn distinguishes_missing_null_false_and_zero_in_objects() {
    let mut object = JsonObject::from([
        ("null".into(), JsonValue::Null),
        ("false".into(), JsonValue::Bool(false)),
        ("zero".into(), JsonValue::from(0_i64)),
    ]);
    assert_eq!(object.get("missing"), None);
    assert_eq!(object.get("null"), Some(&JsonValue::Null));
    assert_eq!(object.get("false"), Some(&JsonValue::Bool(false)));
    assert_eq!(object.get("zero"), Some(&JsonValue::Number(0_i64.into())));
    assert_eq!(
        object.insert("null".into(), JsonValue::from("replacement")),
        Some(JsonValue::Null)
    );
    assert_eq!(object.remove("null"), Some(JsonValue::from("replacement")));
    let pairs: Vec<_> = object.iter().map(|(key, _)| key.as_str()).collect();
    assert_eq!(pairs, ["false", "zero"]);
    assert_eq!(object.len(), 2);
    assert!(!object.is_empty());
    assert!(!object.contains_key("null"));
    let rebuilt: JsonObject = object.clone().into_iter().collect();
    assert_eq!(rebuilt, object);
    for (_, value) in &mut object {
        *value = JsonValue::Null;
    }
    assert_eq!(object.get("false"), Some(&JsonValue::Null));
    assert_eq!(JsonObject::from(object.clone().into_map()), object);
    assert_eq!(JsonObject::new(), JsonObject::default());
}

#[test]
fn stores_boolean_and_object_schemas_without_executing_them() {
    assert_eq!(JsonSchema::from(false), JsonSchema::Bool(false));
    let schema = JsonObject::from([
        ("$ref".into(), JsonValue::from("#/$defs/anything")),
        ("$defs".into(), JsonValue::Object(JsonObject::new())),
        ("custom".into(), JsonValue::Array(vec![JsonValue::Null])),
    ]);
    assert_eq!(
        JsonSchema::try_from(JsonValue::Object(schema.clone())).unwrap(),
        JsonSchema::Object(schema)
    );
    for invalid in [
        JsonValue::Null,
        JsonValue::Array(Vec::new()),
        JsonValue::from(1_i64),
        JsonValue::from("object"),
    ] {
        assert!(JsonSchema::try_from(invalid).is_err());
    }
}

#[test]
fn public_foundations_remain_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ProviderId>();
    assert_send_sync::<ApiId>();
    assert_send_sync::<ModelId>();
    assert_send_sync::<AccountId>();
    assert_send_sync::<ToolCallId>();
    assert_send_sync::<UnixMillis>();
    assert_send_sync::<JsonValue>();
    assert_send_sync::<JsonSchema>();
    assert_send_sync::<Error>();
}

#[cfg(feature = "serde")]
mod serialization {
    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct OptionalData {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        value: Option<JsonValue>,
    }

    #[test]
    fn round_trips_missing_null_and_all_numeric_categories() {
        let values = [
            None,
            Some(JsonValue::Null),
            Some(JsonValue::Bool(false)),
            Some(JsonValue::Number(0_i64.into())),
            Some(JsonValue::Number(u64::MAX.into())),
            Some(JsonValue::Number(i64::MIN.into())),
            Some(JsonValue::Number(JsonNumber::new(1.0).unwrap())),
            Some(JsonValue::Object(JsonObject::from([(
                "nested".into(),
                JsonValue::Array(vec![JsonValue::Null]),
            )]))),
        ];
        for value in values {
            let data = OptionalData { value };
            let encoded = serde_json::to_string(&data).unwrap();
            let decoded: OptionalData = serde_json::from_str(&encoded).unwrap();
            assert_eq!(decoded, data);
        }
        let missing: OptionalData = serde_json::from_str("{}").unwrap();
        let explicit: OptionalData = serde_json::from_str(r#"{"value":{"type":"Null"}}"#).unwrap();
        assert_eq!(missing.value, None);
        assert_eq!(explicit.value, Some(JsonValue::Null));
    }

    #[test]
    fn round_trips_finite_float_boundaries_without_losing_bits() {
        for value in [
            f64::MIN,
            f64::MAX,
            f64::MIN_POSITIVE,
            -0.0,
            1.234_567_890_123_456_7,
        ] {
            let number = JsonNumber::new(value).unwrap();
            let text = serde_json::to_string(&number).unwrap();
            let decoded: JsonNumber = serde_json::from_str(&text).unwrap();
            assert_eq!(decoded.as_f64().unwrap().to_bits(), value.to_bits());
        }
        let mut bits = 7_u64;
        for _ in 0..1024 {
            bits = bits.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let value = f64::from_bits(bits);
            if !value.is_finite() {
                continue;
            }
            let text = serde_json::to_string(&JsonNumber::new(value).unwrap()).unwrap();
            let decoded: JsonNumber = serde_json::from_str(&text).unwrap();
            assert_eq!(
                decoded.as_f64().unwrap().to_bits(),
                bits,
                "浮点往返失败: {text}"
            );
        }
    }

    #[test]
    fn deserialization_cannot_bypass_id_and_number_validation() {
        for invalid in [r#"""#, r#"" ""#, r#""\t\n""#] {
            assert!(serde_json::from_str::<ProviderId>(invalid).is_err());
            assert!(serde_json::from_str::<ApiId>(invalid).is_err());
            assert!(serde_json::from_str::<ModelId>(invalid).is_err());
            assert!(serde_json::from_str::<AccountId>(invalid).is_err());
            assert!(serde_json::from_str::<ToolCallId>(invalid).is_err());
        }
        for invalid in [
            r#"{"type":"Float","value":1e400}"#,
            r#"{"type":"Float","value":null}"#,
            r#"{"type":"Unsigned","value":-1}"#,
            r#"{"type":"Signed","value":18446744073709551615}"#,
        ] {
            assert!(serde_json::from_str::<JsonNumber>(invalid).is_err());
        }
        let provider: ProviderId = serde_json::from_str(r#"" X ""#).unwrap();
        assert_eq!(provider.as_str(), " X ");
        for milliseconds in [i64::MIN, i64::MAX] {
            let timestamp: UnixMillis = serde_json::from_str(&milliseconds.to_string()).unwrap();
            assert_eq!(timestamp.as_i64(), milliseconds);
        }
    }
}
