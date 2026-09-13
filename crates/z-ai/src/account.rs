//! 显式账号选择、拥有值的快照与纯内存条件写入。
//! 版本仅在同一存储实例的连续生命周期内有效；本地 CAS 不保证远端令牌有效，也不协调网络刷新。

use std::{collections::BTreeMap, fmt};

use crate::{
    auth::{AccountAuth, CredentialKind},
    error::{Error, Result},
    identity::{AccountId, ProviderId},
};

/// 存储分配的版本标识，只能观察或从已存在的摘要、快照、写入结果取得。
///
/// 版本不是时间戳，也不适用于跨存储实例的恢复、复制或比较。
/// CAS 比较目标账号当前版本，而不是整个存储最近分配的版本。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Revision(u64);

impl Revision {
    /// 返回用于观察与诊断的数值；不提供从数值构造版本的公开入口。
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

/// 独立的本地命名凭据配置，不等同于远端用户。
///
/// 本地 ID 与 provider 归属在记录存在期间不可改变，名称可重复或为空。
/// ID、provider 和名称是由调用方管理的非敏感标签，不得填入秘密。
/// 不按远端用户、组织、显示名、密钥或令牌合并账号。
#[derive(Clone, PartialEq)]
pub struct Account {
    id: AccountId,
    provider: ProviderId,
    label: String,
    auth: AccountAuth,
}

impl Account {
    /// 用已校验的 ID、provider、本地名称和完整认证配置构造账号。
    #[must_use]
    pub fn new(id: AccountId, provider: ProviderId, label: String, auth: AccountAuth) -> Self {
        Self {
            id,
            provider,
            label,
            auth,
        }
    }

    /// 借用稳定本地 ID，不是远端用户 ID。
    #[must_use]
    pub fn id(&self) -> &AccountId {
        &self.id
    }

    /// 借用固定的 provider 归属。
    #[must_use]
    pub fn provider(&self) -> &ProviderId {
        &self.provider
    }

    /// 借用本地显示名，凭据更新不会替换它。
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// 显式借用完整认证配置，不解析为可发送的 HTTP 认证。
    #[must_use]
    pub fn auth(&self) -> &AccountAuth {
        &self.auth
    }
}

impl fmt::Debug for Account {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Account")
            .field("id", &self.id)
            .field("provider", &self.provider)
            .field("label", &self.label)
            .field("auth", &"[REDACTED]")
            .finish()
    }
}

/// 列表使用的非凭据白名单摘要。
///
/// 仅包含 ID、provider、本地显示名、认证种类和版本；不含到期时间或任何认证配置。
/// 调用方须把 ID、provider 和名称作为非敏感标签管理。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountInfo {
    id: AccountId,
    provider: ProviderId,
    label: String,
    kind: CredentialKind,
    revision: Revision,
}

impl AccountInfo {
    /// 借用本地账号 ID。
    #[must_use]
    pub fn id(&self) -> &AccountId {
        &self.id
    }

    /// 借用 provider 归属。
    #[must_use]
    pub fn provider(&self) -> &ProviderId {
        &self.provider
    }

    /// 借用本地显示名。
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// 返回认证种类，不代表远端凭据有效。
    #[must_use]
    pub fn kind(&self) -> CredentialKind {
        self.kind
    }

    /// 返回生成摘要时该账号的版本。
    #[must_use]
    pub fn revision(&self) -> Revision {
        self.revision
    }
}

/// 拥有完整账号、配置和版本的稳定值快照。
///
/// 后续更新、改名或删除不会改变或撤销外部快照。
/// 快照不代表认证已经解析、令牌尚未过期或远端授权仍然有效；宿主自行管理其秘密与生命周期。
#[derive(Clone, PartialEq)]
pub struct AccountSnapshot {
    account: Account,
    revision: Revision,
}

impl AccountSnapshot {
    /// 借用快照内的完整账号，不提供修改存储的旁路。
    #[must_use]
    pub fn account(&self) -> &Account {
        &self.account
    }

    /// 借用本地账号 ID。
    #[must_use]
    pub fn id(&self) -> &AccountId {
        self.account.id()
    }

    /// 借用 provider 归属。
    #[must_use]
    pub fn provider(&self) -> &ProviderId {
        self.account.provider()
    }

    /// 借用快照时的本地显示名。
    #[must_use]
    pub fn label(&self) -> &str {
        self.account.label()
    }

    /// 显式借用快照时的完整认证配置。
    #[must_use]
    pub fn auth(&self) -> &AccountAuth {
        self.account.auth()
    }

    /// 返回用于后续条件写入的目标账号版本。
    #[must_use]
    pub fn revision(&self) -> Revision {
        self.revision
    }

    /// 生成只含非凭据白名单字段的拥有值摘要。
    #[must_use]
    pub fn info(&self) -> AccountInfo {
        AccountInfo {
            id: self.account.id.clone(),
            provider: self.account.provider.clone(),
            label: self.account.label.clone(),
            kind: self.account.auth.credential().kind(),
            revision: self.revision,
        }
    }
}

impl fmt::Debug for AccountSnapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AccountSnapshot")
            .field("account", &self.account)
            .field("revision", &self.revision)
            .finish()
    }
}

/// 仅在本实例内保证条件写入一致性的纯内存账号存储。
///
/// 所有写入要求独占借用，账号 ID 在整个存储内唯一。没有默认账号、自动回退、
/// 隐式更新或可变账号引用，也不实现 `Clone`、自动序列化或版本重置。
/// 全局单调计数器只分配新版本；对其他账号的写入不改变目标账号的版本。
///
/// 本地 CAS 不阻止远端重复刷新、不保证冲突后旧令牌可用，也不能回滚远端令牌轮换。
/// 宿主必须协调网络刷新与该账号的所有写入，或自行恢复、重新授权；本库不自动重试。
/// 删除不取消在途请求、不撤销远端授权，也不清除调用方已持有的快照或秘密。
///
/// # Examples
///
/// ```
/// use z_ai::{
///     account::{Account, AccountStore},
///     auth::{AccountAuth, ApiKeyCredential, Credential},
///     identity::{AccountId, ProviderId},
/// };
/// let provider = ProviderId::new("example-provider".to_owned())?;
/// let id = AccountId::new("local-config".to_owned())?;
/// let auth = AccountAuth::new(Credential::ApiKey(ApiKeyCredential::new(None)));
/// let mut store = AccountStore::new();
/// store.insert(Account::new(id.clone(), provider.clone(), "配置".to_owned(), auth))?;
/// let snapshot = store.snapshot(&provider, &id)?;
/// store.rename(&provider, &id, snapshot.revision(), "新名称".to_owned())?;
/// assert_eq!(snapshot.label(), "配置");
/// # Ok::<(), z_ai::Error>(())
/// ```
///
/// 存储不能复制为带有相同版本的另一个可写实例：
///
/// ```compile_fail
/// use z_ai::account::AccountStore;
/// let store = AccountStore::new();
/// let duplicate = store.clone();
/// ```
#[derive(Default)]
pub struct AccountStore {
    accounts: BTreeMap<AccountId, AccountSnapshot>,
    sequence: u64,
}

impl AccountStore {
    /// 构造独立的空存储；版本不与其他实例共享。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 新增账号并返回其初始版本，绝不覆盖已有账号。
    ///
    /// # Errors
    ///
    /// - ID 已存在时返回 [`Error::AlreadyExists`]，即使 provider 不同也拒绝
    /// - 无法分配新版本时返回 [`Error::RevisionExhausted`]
    ///
    /// 失败时内容与版本计数器均不改变。
    pub fn insert(&mut self, account: Account) -> Result<Revision> {
        if self.accounts.contains_key(account.id()) {
            return Err(Error::AlreadyExists);
        }
        let revision = self.next_revision()?;
        self.accounts
            .insert(account.id.clone(), AccountSnapshot { account, revision });
        self.sequence = revision.0;
        Ok(revision)
    }

    /// 显式选择 provider 与本地账号，返回拥有完整值的快照。
    ///
    /// 即使 OAuth 已过期或 API Key 缺失也能读取，不执行环境回退或默认选择。
    ///
    /// # Errors
    ///
    /// - ID 不存在时返回 [`Error::NotFound`]
    /// - ID 存在但 provider 不符时返回 [`Error::ProviderMismatch`]
    pub fn snapshot(&self, provider: &ProviderId, id: &AccountId) -> Result<AccountSnapshot> {
        self.selected(provider, id).cloned()
    }

    /// 按本地 ID 顺序生成拥有值的白名单摘要，不返回任何凭据或配置。
    pub fn list(&self) -> impl ExactSizeIterator<Item = AccountInfo> + '_ {
        self.accounts.values().map(AccountSnapshot::info)
    }

    /// 以目标账号版本为条件，一次替换完整凭据及账号认证配置。
    ///
    /// 保留本地 ID、provider 与显示名；不合并部分 OAuth 响应，也不向不存在的账号插入。
    /// 成功仅代表本地写入成功，不证明新旧令牌的远端有效性。
    ///
    /// # Errors
    ///
    /// - ID 不存在时返回 [`Error::NotFound`]
    /// - provider 不符时返回 [`Error::ProviderMismatch`]
    /// - 目标账号版本不符时返回 [`Error::RevisionConflict`]
    /// - 无法分配新版本时返回 [`Error::RevisionExhausted`]
    ///
    /// 失败时内容与版本计数器均不改变。
    pub fn replace_auth(
        &mut self,
        provider: &ProviderId,
        id: &AccountId,
        expected: Revision,
        auth: AccountAuth,
    ) -> Result<Revision> {
        self.check_revision(provider, id, expected)?;
        let revision = self.next_revision()?;
        let snapshot = self.accounts.get_mut(id).ok_or(Error::NotFound)?;
        snapshot.account.auth = auth;
        snapshot.revision = revision;
        self.sequence = revision.0;
        Ok(revision)
    }

    /// 以目标账号版本为条件修改本地名称，认证配置保持不变。
    ///
    /// 即使名称与原值相同也分配新版本，因此可能使在途刷新结果发生本地冲突。
    ///
    /// # Errors
    ///
    /// - ID 不存在时返回 [`Error::NotFound`]
    /// - provider 不符时返回 [`Error::ProviderMismatch`]
    /// - 目标账号版本不符时返回 [`Error::RevisionConflict`]
    /// - 无法分配新版本时返回 [`Error::RevisionExhausted`]
    ///
    /// 失败时内容与版本计数器均不改变。
    pub fn rename(
        &mut self,
        provider: &ProviderId,
        id: &AccountId,
        expected: Revision,
        label: String,
    ) -> Result<Revision> {
        self.check_revision(provider, id, expected)?;
        let revision = self.next_revision()?;
        let snapshot = self.accounts.get_mut(id).ok_or(Error::NotFound)?;
        snapshot.account.label = label;
        snapshot.revision = revision;
        self.sequence = revision.0;
        Ok(revision)
    }

    /// 以目标账号版本为条件删除记录并消耗一个新版本，返回该删除版本。
    ///
    /// 删除后旧结果不能复活记录；同 ID 重建会获得新版本以防止 ABA 覆盖。
    /// 不撤销外部快照、远端授权或在途请求；重复删除不是幂等成功。
    ///
    /// # Errors
    ///
    /// - ID 不存在时返回 [`Error::NotFound`]
    /// - provider 不符时返回 [`Error::ProviderMismatch`]
    /// - 目标账号版本不符时返回 [`Error::RevisionConflict`]
    /// - 无法分配新版本时返回 [`Error::RevisionExhausted`]
    ///
    /// 失败时内容与版本计数器均不改变。
    pub fn delete(
        &mut self,
        provider: &ProviderId,
        id: &AccountId,
        expected: Revision,
    ) -> Result<Revision> {
        self.check_revision(provider, id, expected)?;
        let revision = self.next_revision()?;
        self.accounts.remove(id);
        self.sequence = revision.0;
        Ok(revision)
    }

    /// 仅验证选择，不回退到其他账号。
    fn selected(&self, provider: &ProviderId, id: &AccountId) -> Result<&AccountSnapshot> {
        let snapshot = self.accounts.get(id).ok_or(Error::NotFound)?;
        if snapshot.provider() != provider {
            return Err(Error::ProviderMismatch);
        }
        Ok(snapshot)
    }

    /// CAS 只比较目标账号的版本，不使用全局最新序号。
    fn check_revision(
        &self,
        provider: &ProviderId,
        id: &AccountId,
        expected: Revision,
    ) -> Result<()> {
        if self.selected(provider, id)?.revision != expected {
            return Err(Error::RevisionConflict);
        }
        Ok(())
    }

    /// 只计算下一个版本，实际成功写入后才提交计数器。
    fn next_revision(&self) -> Result<Revision> {
        self.sequence
            .checked_add(1)
            .map(Revision)
            .ok_or(Error::RevisionExhausted)
    }
}

impl fmt::Debug for AccountStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AccountStore")
            .field("accounts", &"[REDACTED]")
            .field("sequence", &self.sequence)
            .finish()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use crate::auth::{ApiKeyCredential, Credential};

    use super::*;

    #[test]
    fn exhausted_revision_rejects_every_write_without_changing_any_state() {
        let provider = ProviderId::new("provider".to_owned()).unwrap();
        let id = AccountId::new("existing".to_owned()).unwrap();
        let auth = AccountAuth::new(Credential::ApiKey(ApiKeyCredential::new(None)));
        let account = Account::new(
            id.clone(),
            provider.clone(),
            "原名".to_owned(),
            auth.clone(),
        );
        let mut store = AccountStore::new();
        store.sequence = u64::MAX - 1;
        let revision = store.insert(account.clone()).unwrap();
        assert_eq!(revision.as_u64(), u64::MAX);
        let before = store.accounts.clone();
        let other = Account::new(
            AccountId::new("other".to_owned()).unwrap(),
            provider.clone(),
            "其他".to_owned(),
            auth.clone(),
        );

        assert!(matches!(store.insert(other), Err(Error::RevisionExhausted)));
        assert_eq!(store.accounts, before);
        assert_eq!(store.sequence, u64::MAX);
        assert!(matches!(
            store.replace_auth(&provider, &id, revision, auth),
            Err(Error::RevisionExhausted)
        ));
        assert_eq!(store.accounts, before);
        assert_eq!(store.sequence, u64::MAX);
        assert!(matches!(
            store.rename(&provider, &id, revision, "新名".to_owned()),
            Err(Error::RevisionExhausted)
        ));
        assert_eq!(store.accounts, before);
        assert_eq!(store.sequence, u64::MAX);
        assert!(matches!(
            store.delete(&provider, &id, revision),
            Err(Error::RevisionExhausted)
        ));
        assert_eq!(store.accounts, before);
        assert_eq!(store.sequence, u64::MAX);
        assert!(matches!(store.insert(account), Err(Error::AlreadyExists)));
        assert_eq!(store.accounts, before);
        assert_eq!(store.sequence, u64::MAX);
    }

    #[test]
    fn selection_and_revision_failures_precede_exhaustion_without_mutation() {
        let provider = ProviderId::new("provider".to_owned()).unwrap();
        let other_provider = ProviderId::new("other".to_owned()).unwrap();
        let id = AccountId::new("existing".to_owned()).unwrap();
        let missing = AccountId::new("missing".to_owned()).unwrap();
        let auth = AccountAuth::new(Credential::ApiKey(ApiKeyCredential::new(None)));
        let mut store = AccountStore::new();
        let revision = store
            .insert(Account::new(
                id.clone(),
                provider.clone(),
                String::new(),
                auth.clone(),
            ))
            .unwrap();
        store.sequence = u64::MAX;
        let before = store.accounts.clone();
        assert!(matches!(
            store.replace_auth(&other_provider, &id, revision, auth),
            Err(Error::ProviderMismatch)
        ));
        assert!(matches!(
            store.rename(&provider, &missing, revision, String::new()),
            Err(Error::NotFound)
        ));
        assert!(matches!(
            store.delete(&provider, &id, Revision(0)),
            Err(Error::RevisionConflict)
        ));
        assert_eq!(store.accounts, before);
        assert_eq!(store.sequence, u64::MAX);
    }
}
