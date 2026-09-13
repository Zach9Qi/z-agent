//! `z-ai` 的纯核心类型；不执行 IO、认证流程或协议转换。

#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod account;
pub mod auth;
mod error;
pub mod event;
pub mod identity;
pub mod message;
pub mod model;
pub mod options;
pub mod tool;
pub mod value;

pub use error::{Error, Result};
pub use identity::{AccountId, ApiId, ModelId, ProviderId, ToolCallId, UnixMillis};
pub use value::{JsonNumber, JsonObject, JsonSchema, JsonValue};
