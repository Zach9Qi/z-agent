//! 开放字符串标识与 Unix 毫秒；不生成身份、不归一化输入、不读取时钟。

use std::{fmt, str::FromStr};

#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize};

use crate::{Error, Result};

macro_rules! string_id {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        ///
        /// 拒绝空或纯空白值，保留大小写和首尾空白，不绑定内置目录。
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[cfg_attr(feature = "serde", derive(Serialize))]
        #[cfg_attr(feature = "serde", serde(transparent))]
        pub struct $name(String);

        impl $name {
            /// 校验并保存调用方提供的标识。
            ///
            /// # Errors
            ///
            /// 空或纯空白标识返回 [`Error::InvalidInput`]，错误不包含输入原文。
            pub fn new(value: String) -> Result<Self> {
                if value.trim().is_empty() {
                    return Err(Error::InvalidInput(
                        concat!(stringify!($name), " 不能为空或纯空白").into(),
                    ));
                }
                Ok(Self(value))
            }

            /// 借用未经归一化的原始标识。
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// 取回拥有所有权的原始标识。
            #[must_use]
            pub fn into_string(self) -> String {
                self.0
            }
        }

        impl FromStr for $name {
            type Err = Error;

            fn from_str(value: &str) -> Result<Self> {
                Self::new(value.to_owned())
            }
        }

        impl TryFrom<String> for $name {
            type Error = Error;

            fn try_from(value: String) -> Result<Self> {
                Self::new(value)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        #[cfg(feature = "serde")]
        #[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(
                deserializer: D,
            ) -> core::result::Result<Self, D::Error> {
                Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
            }
        }
    };
}

string_id!(ProviderId, "服务提供方的开放标识，与本地账号身份无关。");
string_id!(ApiId, "聊天协议的开放标识，不限定已知 API。");
string_id!(ModelId, "模型的开放标识；模型身份还需结合 provider。");
string_id!(AccountId, "调用方提供的本地账号标识，不代表远端用户。");
string_id!(ToolCallId, "工具调用的关联标识，用于连接调用与结果。");

/// 距 Unix 纪元的有符号 `i64` 毫秒数，允许纪元之前的时间。
///
/// 本类型不读取时钟，不做秒与毫秒的隐式换算。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct UnixMillis(i64);

impl UnixMillis {
    /// 保存明确以毫秒为单位的时间戳。
    #[must_use]
    pub const fn new(milliseconds: i64) -> Self {
        Self(milliseconds)
    }

    /// 返回原始有符号毫秒数。
    #[must_use]
    pub const fn as_i64(self) -> i64 {
        self.0
    }
}

impl From<i64> for UnixMillis {
    fn from(milliseconds: i64) -> Self {
        Self::new(milliseconds)
    }
}

impl From<UnixMillis> for i64 {
    fn from(timestamp: UnixMillis) -> Self {
        timestamp.as_i64()
    }
}

impl fmt::Display for UnixMillis {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}
