//! 服务端报告的用量与费用；不重算总量，也不重复累计子集。

use crate::error::{Error, Result};

/// 四类 token 对应的金额，单位为美元；所有值必须有限且非负。
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "TokenAmountsData"))]
pub struct TokenAmounts {
    input: f64,
    output: f64,
    cache_read: f64,
    cache_write: f64,
}

impl TokenAmounts {
    /// 保存四类美元金额，不推导任何总额。
    ///
    /// # Errors
    /// 任一金额为负数或非有限值时返回 [`Error::InvalidInput`]。
    pub fn new(input: f64, output: f64, cache_read: f64, cache_write: f64) -> Result<Self> {
        for amount in [input, output, cache_read, cache_write] {
            validate_amount(amount)?;
        }
        Ok(Self {
            input,
            output,
            cache_read,
            cache_write,
        })
    }

    /// 输入 token 对应的金额。
    #[must_use]
    pub fn input(&self) -> f64 {
        self.input
    }
    /// 输出 token 对应的金额。
    #[must_use]
    pub fn output(&self) -> f64 {
        self.output
    }
    /// 缓存读取对应的金额。
    #[must_use]
    pub fn cache_read(&self) -> f64 {
        self.cache_read
    }
    /// 缓存写入对应的金额。
    #[must_use]
    pub fn cache_write(&self) -> f64 {
        self.cache_write
    }
}

/// 服务端报告的费用，保留独立报告的总额，不要求等于各项之和。
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "UsageCostData"))]
pub struct UsageCost {
    amounts: TokenAmounts,
    total: f64,
}

impl UsageCost {
    /// 保存各类费用与服务端总额，单位均为美元。
    ///
    /// # Errors
    /// 总额为负数或非有限值时返回 [`Error::InvalidInput`]。
    pub fn new(amounts: TokenAmounts, total: f64) -> Result<Self> {
        validate_amount(total)?;
        Ok(Self { amounts, total })
    }

    /// 输入 token 对应的金额。
    #[must_use]
    pub fn input(&self) -> f64 {
        self.amounts.input()
    }
    /// 输出 token 对应的金额。
    #[must_use]
    pub fn output(&self) -> f64 {
        self.amounts.output()
    }
    /// 缓存读取对应的金额。
    #[must_use]
    pub fn cache_read(&self) -> f64 {
        self.amounts.cache_read()
    }
    /// 缓存写入对应的金额。
    #[must_use]
    pub fn cache_write(&self) -> f64 {
        self.amounts.cache_write()
    }
    /// 服务端报告的总额，不执行重新计价。
    #[must_use]
    pub fn total(&self) -> f64 {
        self.total
    }
}

/// 四类已报告的 token 数；Rust 契约限定为非负整数。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UsageCounts {
    input: u64,
    output: u64,
    cache_read: u64,
    cache_write: u64,
}

impl UsageCounts {
    /// 保存输入、输出、缓存读取及缓存写入计数。
    #[must_use]
    pub fn new(input: u64, output: u64, cache_read: u64, cache_write: u64) -> Self {
        Self {
            input,
            output,
            cache_read,
            cache_write,
        }
    }

    /// 非缓存输入计数。
    #[must_use]
    pub fn input(&self) -> u64 {
        self.input
    }
    /// 输出计数，已包含推理计数。
    #[must_use]
    pub fn output(&self) -> u64 {
        self.output
    }
    /// 缓存读取计数。
    #[must_use]
    pub fn cache_read(&self) -> u64 {
        self.cache_read
    }
    /// 缓存写入计数，已包含一小时写入计数。
    #[must_use]
    pub fn cache_write(&self) -> u64 {
        self.cache_write
    }
}

/// 一次调用报告的用量；工具用量不自动并入主模型用量。
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "UsageData"))]
pub struct Usage {
    counts: UsageCounts,
    total_tokens: u64,
    cost: UsageCost,
    reasoning: Option<u64>,
    cache_write_1h: Option<u64>,
}

impl Usage {
    /// 保存报告值；总计无需等于四类计数之和，细分计数初始为未报告。
    #[must_use]
    pub fn new(counts: UsageCounts, total_tokens: u64, cost: UsageCost) -> Self {
        Self {
            counts,
            total_tokens,
            cost,
            reasoning: None,
            cache_write_1h: None,
        }
    }

    /// 补充已包含在输出计数内的推理计数；零表示已报告零而非未报告。
    ///
    /// # Errors
    /// 推理计数超过输出计数时返回 [`Error::InvalidInput`]。
    pub fn with_reasoning(mut self, reasoning: u64) -> Result<Self> {
        if self.counts.output < reasoning {
            return Err(Error::InvalidInput("reasoning 不得超过 output".into()));
        }
        self.reasoning = Some(reasoning);
        Ok(self)
    }

    /// 补充已包含在缓存写入计数内的一小时保留计数。
    ///
    /// # Errors
    /// 一小时计数超过缓存写入计数时返回 [`Error::InvalidInput`]。
    pub fn with_cache_write_1h(mut self, cache_write_1h: u64) -> Result<Self> {
        if self.counts.cache_write < cache_write_1h {
            return Err(Error::InvalidInput(
                "cache_write_1h 不得超过 cache_write".into(),
            ));
        }
        self.cache_write_1h = Some(cache_write_1h);
        Ok(self)
    }

    /// 非缓存输入计数。
    #[must_use]
    pub fn input(&self) -> u64 {
        self.counts.input()
    }
    /// 输出计数，已包含推理计数。
    #[must_use]
    pub fn output(&self) -> u64 {
        self.counts.output()
    }
    /// 缓存读取计数。
    #[must_use]
    pub fn cache_read(&self) -> u64 {
        self.counts.cache_read()
    }
    /// 缓存写入计数，已包含一小时计数。
    #[must_use]
    pub fn cache_write(&self) -> u64 {
        self.counts.cache_write()
    }
    /// 服务端总计，不重算也不累计细分子集。
    #[must_use]
    pub fn total_tokens(&self) -> u64 {
        self.total_tokens
    }
    /// 服务端报告的费用。
    #[must_use]
    pub fn cost(&self) -> &UsageCost {
        &self.cost
    }
    /// 推理子集；未报告与报告零不同。
    #[must_use]
    pub fn reasoning(&self) -> Option<u64> {
        self.reasoning
    }
    /// 一小时缓存写入子集；未报告与报告零不同。
    #[must_use]
    pub fn cache_write_1h(&self) -> Option<u64> {
        self.cache_write_1h
    }
}

fn validate_amount(amount: f64) -> Result<()> {
    if !amount.is_finite() || amount < 0.0 {
        return Err(Error::InvalidInput("费用必须是有限非负数".into()));
    }
    Ok(())
}

#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
struct TokenAmountsData {
    input: f64,
    output: f64,
    cache_read: f64,
    cache_write: f64,
}

#[cfg(feature = "serde")]
impl TryFrom<TokenAmountsData> for TokenAmounts {
    type Error = Error;
    fn try_from(data: TokenAmountsData) -> Result<Self> {
        Self::new(data.input, data.output, data.cache_read, data.cache_write)
    }
}

#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
struct UsageCostData {
    amounts: TokenAmounts,
    total: f64,
}

#[cfg(feature = "serde")]
impl TryFrom<UsageCostData> for UsageCost {
    type Error = Error;
    fn try_from(data: UsageCostData) -> Result<Self> {
        Self::new(data.amounts, data.total)
    }
}

#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
struct UsageData {
    counts: UsageCounts,
    total_tokens: u64,
    cost: UsageCost,
    reasoning: Option<u64>,
    cache_write_1h: Option<u64>,
}

#[cfg(feature = "serde")]
impl TryFrom<UsageData> for Usage {
    type Error = Error;
    fn try_from(data: UsageData) -> Result<Self> {
        let mut usage = Self::new(data.counts, data.total_tokens, data.cost);
        if let Some(reasoning) = data.reasoning {
            usage = usage.with_reasoning(reasoning)?;
        }
        if let Some(cache_write_1h) = data.cache_write_1h {
            usage = usage.with_cache_write_1h(cache_write_1h)?;
        }
        Ok(usage)
    }
}
