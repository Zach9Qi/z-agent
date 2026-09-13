//! 工具名称、完整 schema 与约束采样数据；不执行工具或校验 schema 支持情况。

use std::collections::BTreeMap;

use crate::value::JsonSchema;

/// 工具描述；参数文档可以包含布尔 schema、引用、定义及扩展关键字。
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Tool {
    name: String,
    description: String,
    parameters: JsonSchema,
    constrained_sampling: Option<ConstrainedSampling>,
}

impl Tool {
    /// 保存工具定义，不限制提供方专有命名或执行参数 schema。
    #[must_use]
    pub fn new(name: String, description: String, parameters: JsonSchema) -> Self {
        Self {
            name,
            description,
            parameters,
            constrained_sampling: None,
        }
    }

    /// 指定约束采样；未调用时保留未提供状态。
    #[must_use]
    pub fn with_constrained_sampling(mut self, sampling: ConstrainedSampling) -> Self {
        self.constrained_sampling = Some(sampling);
        self
    }

    /// 工具名称，原样保留。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    /// 工具说明，原样保留。
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }
    /// 完整参数 schema 文档。
    #[must_use]
    pub fn parameters(&self) -> &JsonSchema {
        &self.parameters
    }
    /// 约束采样配置；`None` 与显式禁用不同。
    #[must_use]
    pub fn constrained_sampling(&self) -> Option<&ConstrainedSampling> {
        self.constrained_sampling.as_ref()
    }
}

/// 工具的约束采样选择；这里只保存数据，不承诺提供方支持。
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ConstrainedSampling {
    /// 显式禁用，对应上游 `false`。
    Disabled,
    /// 请求 JSON schema 约束采样。
    JsonSchema(Strictness),
    /// 保存不同提供方的语法文档；允许空变体映射。
    Grammar(GrammarVariants),
}

/// JSON schema 约束采样的要求程度。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Strictness {
    /// 优先使用，提供方不支持时可由宿主降级。
    Prefer,
    /// 必须使用，具体执行由宿主负责。
    Require,
}

/// 支持保存的语法格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GrammarFormat {
    /// `OpenAI` 的 Lark 语法表示。
    OpenAiLark,
    /// `OpenAI` 的正则表达式表示。
    OpenAiRegex,
}

/// 每个格式对应的原始语法文本；不检查语法，也不补全缺失的变体。
pub type GrammarVariants = BTreeMap<GrammarFormat, String>;
