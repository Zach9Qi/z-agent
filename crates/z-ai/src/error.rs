//! 唯一错误类型；库产生的错误只描述约束和状态，不包含凭据原文。

/// 本 crate 的统一结果别名。
pub type Result<T, E = Error> = core::result::Result<T, E>;

/// 公开边界的参数或账号状态错误。
///
/// 库生成的参数说明不包含输入原文；调用方自行构造错误时也应避免放入秘密。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// 外部参数不符合公开边界的约束。
    #[error("参数错误: {0}")]
    InvalidInput(String),
    /// 显式选择的本地账号不存在。
    #[error("账号不存在")]
    NotFound,
    /// 本地账号身份已被占用，与 provider 是否相同无关。
    #[error("账号已存在")]
    AlreadyExists,
    /// 显式选择的 provider 与账号归属不同。
    #[error("账号所属 provider 不匹配")]
    ProviderMismatch,
    /// 预期版本与目标账号当前版本不同。
    #[error("账号版本冲突")]
    RevisionConflict,
    /// 存储的版本序号已耗尽，操作未改变状态。
    #[error("账号版本序号已耗尽")]
    RevisionExhausted,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn displays_chinese_state_errors_without_context() {
        for (error, expected) in [
            (
                Error::InvalidInput("字段不能为空".into()),
                "参数错误: 字段不能为空",
            ),
            (Error::NotFound, "账号不存在"),
            (Error::AlreadyExists, "账号已存在"),
            (Error::ProviderMismatch, "账号所属 provider 不匹配"),
            (Error::RevisionConflict, "账号版本冲突"),
            (Error::RevisionExhausted, "账号版本序号已耗尽"),
        ] {
            assert_eq!(error.to_string(), expected);
        }
    }
}
