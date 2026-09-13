//! 自有 JSON 值及 schema 文档；仅保存数据，不解析或执行 schema。
//!
//! `serde` 使用带标签的自有格式，保留有符号整数、无符号整数、浮点类别。
//! [`JsonValue::Null`] 不是 serde 的空值，因此嵌套 `Option<JsonValue>` 不会折叠显式空值。
//! 浮点数通过 serde 的 `f64` 传递；后端需正确舍入以精确往返。
//! 使用 `serde_json` 的宿主应开启其 `float_roundtrip` 特性，本仓库测试已开启。
//! JSON 内容可能包含秘密，调试和序列化输出均不承诺脱敏。

use std::collections::{BTreeMap, btree_map};

#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize};

use crate::{Error, Result};

/// 完整的 JSON 数据值；未提供的字段应在外层使用 `Option` 表达。
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", content = "value"))]
pub enum JsonValue {
    /// 显式 JSON 空值，不表示字段缺失。
    Null,
    /// JSON 布尔值。
    Bool(bool),
    /// 保留数字类别和精度的 JSON 数字。
    Number(JsonNumber),
    /// JSON 字符串，不解释其内容。
    String(String),
    /// 顺序有意义的 JSON 数组。
    Array(Vec<Self>),
    /// 以字符串为键的 JSON 对象。
    Object(JsonObject),
}

impl From<bool> for JsonValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<String> for JsonValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for JsonValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<JsonNumber> for JsonValue {
    fn from(value: JsonNumber) -> Self {
        Self::Number(value)
    }
}

impl From<i64> for JsonValue {
    fn from(value: i64) -> Self {
        Self::Number(value.into())
    }
}

impl From<u64> for JsonValue {
    fn from(value: u64) -> Self {
        Self::Number(value.into())
    }
}

impl From<Vec<Self>> for JsonValue {
    fn from(value: Vec<Self>) -> Self {
        Self::Array(value)
    }
}

impl From<JsonObject> for JsonValue {
    fn from(value: JsonObject) -> Self {
        Self::Object(value)
    }
}

/// JSON 数字，精确保留 `i64`、`u64` 或有限 `f64`，不隐式跨类别转换。
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct JsonNumber(NumberRepr);

impl JsonNumber {
    /// 校验并保存浮点数；整数请使用 `From<i64>` 或 `From<u64>`。
    ///
    /// # Errors
    ///
    /// 非有限数（NaN、正负无穷）返回 [`Error::InvalidInput`]。
    pub fn new(value: f64) -> Result<Self> {
        if !value.is_finite() {
            return Err(Error::InvalidInput("JSON 浮点数必须有限".into()));
        }
        Ok(Self(NumberRepr::Float(value)))
    }

    /// 仅当原始数字为有符号整数时返回其值，不进行窄化或浮点转换。
    #[must_use]
    pub const fn as_i64(self) -> Option<i64> {
        match self.0 {
            NumberRepr::Signed(value) => Some(value),
            NumberRepr::Unsigned(_) | NumberRepr::Float(_) => None,
        }
    }

    /// 仅当原始数字为无符号整数时返回其值。
    #[must_use]
    pub const fn as_u64(self) -> Option<u64> {
        match self.0 {
            NumberRepr::Unsigned(value) => Some(value),
            NumberRepr::Signed(_) | NumberRepr::Float(_) => None,
        }
    }

    /// 仅当原始数字为有限浮点数时返回其值，整数不会有损转为浮点数。
    #[must_use]
    pub const fn as_f64(self) -> Option<f64> {
        match self.0 {
            NumberRepr::Float(value) => Some(value),
            NumberRepr::Signed(_) | NumberRepr::Unsigned(_) => None,
        }
    }
}

impl From<i64> for JsonNumber {
    fn from(value: i64) -> Self {
        Self(NumberRepr::Signed(value))
    }
}

impl From<u64> for JsonNumber {
    fn from(value: u64) -> Self {
        Self(NumberRepr::Unsigned(value))
    }
}

impl TryFrom<f64> for JsonNumber {
    type Error = Error;

    fn try_from(value: f64) -> Result<Self> {
        Self::new(value)
    }
}

#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
impl<'de> Deserialize<'de> for JsonNumber {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> core::result::Result<Self, D::Error> {
        match NumberRepr::deserialize(deserializer)? {
            NumberRepr::Signed(value) => Ok(Self::from(value)),
            NumberRepr::Unsigned(value) => Ok(Self::from(value)),
            NumberRepr::Float(value) => Self::new(value).map_err(serde::de::Error::custom),
        }
    }
}

/// JSON 对象；键按字符串排序，键顺序不是 JSON 数据语义的一部分。
///
/// 重复插入同一键会替换旧值；构造和收集也采用后值覆盖语义。
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct JsonObject(BTreeMap<String, JsonValue>);

impl JsonObject {
    /// 创建空对象，与 `Default` 等价。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 插入或替换一个键值对，返回被替换的旧值。
    pub fn insert(&mut self, key: String, value: JsonValue) -> Option<JsonValue> {
        self.0.insert(key, value)
    }

    /// 借用指定键的值；`None` 表示缺键，与 `Some(JsonValue::Null)` 不同。
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        self.0.get(key)
    }

    /// 判断对象是否包含指定键，包括值为显式空值的键。
    #[must_use]
    pub fn contains_key(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    /// 移除指定键，返回此前保存的值。
    pub fn remove(&mut self, key: &str) -> Option<JsonValue> {
        self.0.remove(key)
    }

    /// 返回对象中的键数量。
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// 判断是否没有任何键。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// 按键排序借用键值对。
    pub fn iter(&self) -> btree_map::Iter<'_, String, JsonValue> {
        self.0.iter()
    }

    /// 按键排序借用键及可变值；任何 JSON 值均可安全替换。
    pub fn iter_mut(&mut self) -> btree_map::IterMut<'_, String, JsonValue> {
        self.0.iter_mut()
    }

    /// 取回完整的标准库映射。
    #[must_use]
    pub fn into_map(self) -> BTreeMap<String, JsonValue> {
        self.0
    }
}

impl From<BTreeMap<String, JsonValue>> for JsonObject {
    fn from(values: BTreeMap<String, JsonValue>) -> Self {
        Self(values)
    }
}

impl<const N: usize> From<[(String, JsonValue); N]> for JsonObject {
    fn from(values: [(String, JsonValue); N]) -> Self {
        values.into_iter().collect()
    }
}

impl FromIterator<(String, JsonValue)> for JsonObject {
    fn from_iter<T: IntoIterator<Item = (String, JsonValue)>>(values: T) -> Self {
        Self(values.into_iter().collect())
    }
}

impl IntoIterator for JsonObject {
    type Item = (String, JsonValue);
    type IntoIter = btree_map::IntoIter<String, JsonValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a JsonObject {
    type Item = (&'a String, &'a JsonValue);
    type IntoIter = btree_map::Iter<'a, String, JsonValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a mut JsonObject {
    type Item = (&'a String, &'a mut JsonValue);
    type IntoIter = btree_map::IterMut<'a, String, JsonValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// JSON schema 文档，只保存布尔或完整对象；不执行验证、引用解析或转换。
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", content = "value"))]
pub enum JsonSchema {
    /// 布尔 schema；`true` 允许全部，`false` 拒绝全部，本库不执行其语义。
    Bool(bool),
    /// 对象 schema，可包含 `$ref`、`$defs` 及任意扩展关键字。
    Object(JsonObject),
}

impl From<bool> for JsonSchema {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<JsonObject> for JsonSchema {
    fn from(value: JsonObject) -> Self {
        Self::Object(value)
    }
}

impl TryFrom<JsonValue> for JsonSchema {
    type Error = Error;

    fn try_from(value: JsonValue) -> Result<Self> {
        match value {
            JsonValue::Bool(value) => Ok(Self::Bool(value)),
            JsonValue::Object(value) => Ok(Self::Object(value)),
            JsonValue::Null | JsonValue::Number(_) | JsonValue::String(_) | JsonValue::Array(_) => {
                Err(Error::InvalidInput(
                    "JSON schema 必须为布尔或对象文档".into(),
                ))
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", content = "value"))]
enum NumberRepr {
    Signed(i64),
    Unsigned(u64),
    Float(f64),
}

#[cfg(all(test, feature = "serde"))]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[cfg(feature = "serde")]
    #[test]
    fn rejects_non_finite_float_from_non_json_deserializer() {
        use serde::de::value::{
            Error as ValueError, F64Deserializer, MapDeserializer, StrDeserializer,
        };

        // 用非 JSON 反序列化器直接输入 NaN，证明校验不依赖 JSON 文本解析器。
        #[derive(Clone, Copy)]
        enum Field {
            Type,
            Value(f64),
        }

        impl serde::de::IntoDeserializer<'_, ValueError> for Field {
            type Deserializer = Self;

            fn into_deserializer(self) -> Self {
                self
            }
        }

        impl<'de> Deserializer<'de> for Field {
            type Error = ValueError;

            fn deserialize_any<V: serde::de::Visitor<'de>>(
                self,
                visitor: V,
            ) -> core::result::Result<V::Value, Self::Error> {
                match self {
                    Self::Type => StrDeserializer::new("Float").deserialize_any(visitor),
                    Self::Value(value) => F64Deserializer::new(value).deserialize_any(visitor),
                }
            }

            serde::forward_to_deserialize_any! {
                bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes byte_buf
                option unit unit_struct newtype_struct seq tuple tuple_struct map struct enum
                identifier ignored_any
            }
        }

        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let fields = [("type", Field::Type), ("value", Field::Value(value))];
            let deserializer = MapDeserializer::<_, ValueError>::new(fields.into_iter());
            assert!(JsonNumber::deserialize(deserializer).is_err());
        }
    }
}
