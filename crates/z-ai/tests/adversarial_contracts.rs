//! 对抗性黑盒测试：单次拒绝写入保持全存储及版本不变，完整认证替换不混用配置。
#![allow(clippy::unwrap_used, clippy::expect_used)]

use z_ai::{
    Error,
    account::{Account, AccountSnapshot, AccountStore},
    auth::{AccountAuth, ApiKeyCredential, Credential, OAuthCredential, SecretString},
    identity::UnixMillis,
    options::{ProviderEnv, ProviderHeaders},
    value::{JsonObject, JsonValue},
};

#[derive(Debug, Clone, Copy)]
enum WriteKind {
    Replace,
    Rename,
    Delete,
}

#[derive(Debug, Clone, Copy)]
enum Rejection {
    Missing,
    WrongProvider,
    Stale,
}

#[test]
fn each_rejected_write_preserves_all_accounts_and_the_next_revision() {
    for write in [WriteKind::Replace, WriteKind::Rename, WriteKind::Delete] {
        for rejection in [
            Rejection::Missing,
            Rejection::WrongProvider,
            Rejection::Stale,
        ] {
            for target in ["key-a", "oauth-a", "key-b", "oauth-b"] {
                let mut store = populated_store();
                let selected = snapshots(&store)
                    .into_iter()
                    .find(|item| item.id().as_str() == target)
                    .unwrap();
                let stale = selected.revision();
                let latest = store
                    .rename(selected.provider(), selected.id(), stale, "较新名称".into())
                    .unwrap();
                let before = snapshots(&store);
                let (provider, id, revision) = match rejection {
                    Rejection::Missing => (
                        selected.provider().clone(),
                        "missing".parse().unwrap(),
                        latest,
                    ),
                    Rejection::WrongProvider => {
                        let other = match selected.provider().as_str() {
                            "provider-a" => "provider-b",
                            _ => "provider-a",
                        };
                        (other.parse().unwrap(), selected.id().clone(), latest)
                    }
                    Rejection::Stale => (selected.provider().clone(), selected.id().clone(), stale),
                };
                let result = match write {
                    WriteKind::Replace => {
                        store.replace_auth(&provider, &id, revision, auth("rejected", true))
                    }
                    WriteKind::Rename => store.rename(&provider, &id, revision, "拒绝名称".into()),
                    WriteKind::Delete => store.delete(&provider, &id, revision),
                };
                let error = result.unwrap_err();
                assert!(
                    matches!(
                        (rejection, error),
                        (Rejection::Missing, Error::NotFound)
                            | (Rejection::WrongProvider, Error::ProviderMismatch)
                            | (Rejection::Stale, Error::RevisionConflict)
                    ),
                    "{target}/{write:?}/{rejection:?} 必须返回对应错误"
                );
                assert_eq!(
                    snapshots(&store),
                    before,
                    "{target}/{write:?}/{rejection:?} 单次失败不能改动任何记录或版本"
                );
                // latest 来自刚成功的写入，是全局最新序号，而非任意账号的旧版本。
                let next = store
                    .rename(
                        selected.provider(),
                        selected.id(),
                        latest,
                        "有效探针".into(),
                    )
                    .unwrap();
                assert_eq!(
                    next.as_u64(),
                    latest.as_u64() + 1,
                    "{target}/{write:?}/{rejection:?} 失败不能消耗全局版本"
                );
            }
        }
    }
}

#[test]
fn full_auth_replacement_clears_both_configuration_layers_and_preserves_old_snapshots() {
    let mut store = populated_store();
    let original = store
        .snapshot(&"provider-a".parse().unwrap(), &"key-a".parse().unwrap())
        .unwrap();
    let others: Vec<_> = snapshots(&store)
        .into_iter()
        .filter(|item| item.id() != original.id())
        .collect();
    let mut history = vec![(original.clone(), auth("key-a", false))];
    let replacements = [
        auth("new-oauth", true),
        auth("new-key", false),
        AccountAuth::new(Credential::ApiKey(ApiKeyCredential::new(None)))
            .with_headers(ProviderHeaders::new())
            .with_env(ProviderEnv::new())
            .with_base_url(String::new()),
        AccountAuth::new(Credential::OAuth(OAuthCredential::new(
            SecretString::new(String::new()),
            SecretString::new(String::new()),
            UnixMillis::new(i64::MIN),
        ))),
    ];
    for replacement in replacements {
        let revision = history.last().unwrap().0.revision();
        let next = store
            .replace_auth(
                original.provider(),
                original.id(),
                revision,
                replacement.clone(),
            )
            .unwrap();
        let current = store.snapshot(original.provider(), original.id()).unwrap();
        assert_eq!(
            current.auth(),
            &replacement,
            "凭据内层 env 与账号外层配置须整体替换，未提供的旧值不能残留"
        );
        assert_eq!(current.revision(), next, "新版本必须对应完整新配置");
        history.push((current, replacement));
        for (snapshot, expected_auth) in &history {
            assert_eq!(snapshot.auth(), expected_auth, "历代快照不能观察到后续配置");
            assert_eq!(snapshot.id(), original.id(), "认证更新不能改变本地身份");
            assert_eq!(
                snapshot.provider(),
                original.provider(),
                "不能改变 provider"
            );
            assert_eq!(snapshot.label(), "同名本地配置", "不能丢失本地名称");
        }
        for other in &others {
            assert_eq!(
                store.snapshot(other.provider(), other.id()).unwrap(),
                *other,
                "替换认证不能污染其他账号"
            );
        }
    }
    store
        .delete(
            original.provider(),
            original.id(),
            history.last().unwrap().0.revision(),
        )
        .unwrap();
    for (snapshot, expected_auth) in history {
        assert_eq!(
            snapshot.auth(),
            &expected_auth,
            "删除不能撤销任一代拥有值快照"
        );
    }
}

#[track_caller]
fn populated_store() -> AccountStore {
    let mut store = AccountStore::new();
    for (id, provider, oauth) in [
        ("key-a", "provider-a", false),
        ("oauth-a", "provider-a", true),
        ("key-b", "provider-b", false),
        ("oauth-b", "provider-b", true),
    ] {
        store
            .insert(Account::new(
                id.parse().unwrap(),
                provider.parse().unwrap(),
                "同名本地配置".into(),
                auth(id, oauth),
            ))
            .unwrap();
    }
    store
}

fn auth(tag: &str, oauth: bool) -> AccountAuth {
    let credential = if oauth {
        Credential::OAuth(
            OAuthCredential::new(
                SecretString::new(format!("access-{tag}")),
                SecretString::new(format!("refresh-{tag}")),
                UnixMillis::new(-1),
            )
            .with_extensions(JsonObject::from([
                ("remote-user".into(), JsonValue::from("same-user")),
                ("generation".into(), JsonValue::from(tag)),
            ])),
        )
    } else {
        Credential::ApiKey(
            ApiKeyCredential::new(Some(SecretString::new(format!("key-{tag}")))).with_env(
                ProviderEnv::from([("OVERLAP".into(), format!("credential-{tag}"))]),
            ),
        )
    };
    AccountAuth::new(credential)
        .with_headers(ProviderHeaders::from([
            ("set".into(), Some(tag.into())),
            ("delete".into(), None),
        ]))
        .with_base_url(format!("https://example.invalid/{tag}"))
        .with_env(ProviderEnv::from([(
            "OVERLAP".into(),
            format!("account-{tag}"),
        )]))
        .with_extensions(JsonObject::from([(
            "generation".into(),
            JsonValue::from(tag),
        )]))
}

#[track_caller]
fn snapshots(store: &AccountStore) -> Vec<AccountSnapshot> {
    store
        .list()
        .map(|info| store.snapshot(info.provider(), info.id()).unwrap())
        .collect()
}
