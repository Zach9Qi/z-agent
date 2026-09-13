//! 工具详情的存在性包装；即使宿主详情自身序列化为 null，也不折叠成字段缺失。

use serde::{Deserialize, Deserializer, Serialize, Serializer};

// serde 的字段级序列化钩子固定借用完整字段；移除该钩子时可删除此例外
#[allow(clippy::ref_option)]
pub(super) fn serialize<T: Serialize, S: Serializer>(
    details: &Option<T>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    details
        .as_ref()
        .map(Presence::Present)
        .serialize(serializer)
}

pub(super) fn deserialize<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    let presence = Option::<Presence<T>>::deserialize(deserializer)?;
    Ok(presence.map(|Presence::Present(value)| value))
}

#[derive(Serialize, Deserialize)]
enum Presence<T> {
    Present(T),
}
