//! 完全离线的公开账号 API 测试，验证身份隔离、完整快照、CAS 与删除重建边界。
#![allow(clippy::unwrap_used, clippy::expect_used)]

use z_ai::{
    Error,
    account::{Account, AccountInfo, AccountSnapshot, AccountStore, Revision},
    auth::{
        AccountAuth, ApiKeyCredential, AuthResult, Credential, CredentialKind, ModelAuth,
        OAuthCredential, SecretString,
    },
    identity::{AccountId, ProviderId, UnixMillis},
    options::{ProviderEnv, ProviderHeaders},
    value::{JsonObject, JsonValue},
};

#[test]
fn two_keys_and_two_oauth_configs_for_one_provider_coexist_without_remote_deduplication() {
    let provider = provider("shared-provider");
    let mut store = AccountStore::new();
    let fixtures = [
        ("key-a", key_auth("key-a"), CredentialKind::ApiKey),
        ("key-b", key_auth("key-b"), CredentialKind::ApiKey),
        ("oauth-a", oauth_auth("oauth-a"), CredentialKind::OAuth),
        ("oauth-b", oauth_auth("oauth-b"), CredentialKind::OAuth),
    ];
    for (name, auth, _) in &fixtures {
        store
            .insert(Account::new(
                id(name),
                provider.clone(),
                "可重复名称".to_owned(),
                auth.clone(),
            ))
            .unwrap();
    }
    let summaries: Vec<_> = store.list().collect();
    assert_eq!(summaries.len(), 4);
    for (name, auth, kind) in &fixtures {
        let snapshot = store.snapshot(&provider, &id(name)).unwrap();
        assert_eq!(snapshot.auth(), auth);
        assert_eq!(snapshot.label(), "可重复名称");
        assert_eq!(snapshot.provider(), &provider);
        assert_eq!(snapshot.account().id(), &id(name));
        let info = summaries
            .iter()
            .find(|info| info.id() == &id(name))
            .unwrap();
        assert_eq!(info.kind(), *kind);
        assert_eq!(info.provider(), &provider);
        assert_eq!(info.label(), "可重复名称");
        assert_eq!(info.revision(), snapshot.revision());
    }

    // 两个 OAuth 配置故意拥有相同远端用户，仍按本地 ID 隔离完整配置和写入
    for (name, _, _) in &fixtures {
        let before = snapshots(&store, &provider);
        let selected = store.snapshot(&provider, &id(name)).unwrap();
        let replacement = oauth_auth(&format!("updated-{name}"));
        store
            .replace_auth(
                &provider,
                &id(name),
                selected.revision(),
                replacement.clone(),
            )
            .unwrap();
        assert_eq!(
            store.snapshot(&provider, &id(name)).unwrap().auth(),
            &replacement
        );
        for other in before.iter().filter(|snapshot| snapshot.id() != &id(name)) {
            assert_eq!(store.snapshot(&provider, other.id()).unwrap(), *other);
        }
    }
}

#[test]
fn duplicate_ids_are_rejected_across_providers_without_consuming_a_revision() {
    let provider = provider("provider-a");
    let other_provider = self::provider("provider-b");
    let id = id("global-id");
    let mut store = AccountStore::new();
    let revision = store
        .insert(Account::new(
            id.clone(),
            provider.clone(),
            "原名".to_owned(),
            key_auth("old"),
        ))
        .unwrap();
    let before = store.snapshot(&provider, &id).unwrap();
    for target in [&provider, &other_provider] {
        assert!(matches!(
            store.insert(Account::new(
                id.clone(),
                target.clone(),
                "替换".to_owned(),
                oauth_auth("new")
            )),
            Err(Error::AlreadyExists)
        ));
        assert_eq!(store.snapshot(&provider, &id).unwrap(), before);
        assert_eq!(store.list().len(), 1);
    }
    let next = store
        .insert(Account::new(
            self::id("independent"),
            other_provider,
            String::new(),
            key_auth("next"),
        ))
        .unwrap();
    assert_eq!(next.as_u64(), revision.as_u64() + 1);
}

#[test]
fn missing_or_wrong_provider_selection_never_falls_back_or_mutates_state() {
    let provider = provider("provider-a");
    let wrong_provider = self::provider("provider-b");
    let id = id("selected");
    let missing = self::id("not-present");
    let mut store = AccountStore::new();
    let revision = store
        .insert(Account::new(
            id.clone(),
            provider.clone(),
            "名称".to_owned(),
            key_auth("original"),
        ))
        .unwrap();
    let before = store.snapshot(&provider, &id).unwrap();
    for (target_provider, target_id, mismatch) in
        [(&wrong_provider, &id, true), (&provider, &missing, false)]
    {
        let results = [
            store
                .snapshot(target_provider, target_id)
                .map(|snapshot| snapshot.revision()),
            store.replace_auth(target_provider, target_id, revision, oauth_auth("rejected")),
            store.rename(
                target_provider,
                target_id,
                revision,
                "拒绝的新名".to_owned(),
            ),
            store.delete(target_provider, target_id, revision),
        ];
        for result in results {
            let error = result.unwrap_err();
            if mismatch {
                assert!(matches!(error, Error::ProviderMismatch));
            } else {
                assert!(matches!(error, Error::NotFound));
            }
            assert_eq!(store.snapshot(&provider, &id).unwrap(), before);
            assert_eq!(store.list().len(), 1);
        }
    }
    let next = store
        .rename(&provider, &id, revision, "有效改名".to_owned())
        .unwrap();
    assert_eq!(next.as_u64(), revision.as_u64() + 1);
}

#[test]
fn replacing_auth_preserves_local_metadata_and_the_entire_owned_old_snapshot() {
    let provider = provider("provider");
    let id = id("stable-local-id");
    let old_auth = oauth_auth("old");
    let mut store = AccountStore::new();
    store
        .insert(Account::new(
            id.clone(),
            provider.clone(),
            "本地名称".to_owned(),
            old_auth.clone(),
        ))
        .unwrap();
    let old = store.snapshot(&provider, &id).unwrap();
    let saved = old.clone();
    let new_auth = oauth_auth("new");
    let next = store
        .replace_auth(&provider, &id, old.revision(), new_auth.clone())
        .unwrap();
    let current = store.snapshot(&provider, &id).unwrap();
    assert_eq!(current.id(), old.id());
    assert_eq!(current.provider(), old.provider());
    assert_eq!(current.label(), old.label());
    assert_eq!(current.auth(), &new_auth);
    assert_eq!(current.revision(), next);
    assert_ne!(current.revision(), old.revision());
    assert_eq!(old, saved);
    assert_eq!(old.auth(), &old_auth);
    assert_eq!(
        old.auth().headers().unwrap().get("delete-header"),
        Some(&None)
    );
    assert_eq!(old.auth().base_url(), Some("https://example.invalid/old"));
    assert_eq!(
        old.auth().env().unwrap().get("CONFIG"),
        Some(&"old".to_owned())
    );
    assert_eq!(
        old.auth().extensions().get("account-config"),
        Some(&JsonValue::String("old".to_owned()))
    );
    let Credential::OAuth(oauth) = old.auth().credential() else {
        unreachable!()
    };
    assert_eq!(oauth.access().expose_secret(), "access-old");
    assert_eq!(oauth.refresh().expose_secret(), "refresh-old");
    assert_eq!(oauth.expires().as_i64(), -1);
    assert_eq!(
        oauth.extensions().get("remote-user"),
        Some(&JsonValue::String("same-remote-user".to_owned()))
    );

    // 完整替换而不是合并：新值未提供的旧 headers、端点、环境与扩展必须全部消失
    let minimal = AccountAuth::new(Credential::ApiKey(ApiKeyCredential::new(None)));
    store
        .replace_auth(&provider, &id, next, minimal.clone())
        .unwrap();
    assert_eq!(store.snapshot(&provider, &id).unwrap().auth(), &minimal);
    assert_eq!(old, saved);
}

#[test]
fn another_accounts_update_does_not_invalidate_the_selected_revision() {
    let provider = provider("provider");
    let mut store = AccountStore::new();
    for name in ["a", "b"] {
        store
            .insert(Account::new(
                id(name),
                provider.clone(),
                name.to_owned(),
                key_auth(name),
            ))
            .unwrap();
    }
    let a = store.snapshot(&provider, &id("a")).unwrap();
    let b = store.snapshot(&provider, &id("b")).unwrap();
    let b_revision = store
        .replace_auth(&provider, &id("b"), b.revision(), oauth_auth("b-new"))
        .unwrap();
    assert_eq!(store.snapshot(&provider, &id("a")).unwrap(), a);
    let a_revision = store
        .replace_auth(&provider, &id("a"), a.revision(), oauth_auth("a-new"))
        .unwrap();
    assert_eq!(a_revision.as_u64(), b_revision.as_u64() + 1);
    assert_eq!(
        store.snapshot(&provider, &id("b")).unwrap().revision(),
        b_revision
    );
}

#[test]
fn two_results_from_the_same_revision_cannot_both_commit_or_change_the_counter() {
    let provider = provider("provider");
    let id = id("selected");
    let mut store = AccountStore::new();
    let revision = store
        .insert(Account::new(
            id.clone(),
            provider.clone(),
            String::new(),
            oauth_auth("old"),
        ))
        .unwrap();
    let first = oauth_auth("first");
    let current = store
        .replace_auth(&provider, &id, revision, first.clone())
        .unwrap();
    let before = store.snapshot(&provider, &id).unwrap();
    for auth in [first, oauth_auth("second")] {
        assert!(matches!(
            store.replace_auth(&provider, &id, revision, auth),
            Err(Error::RevisionConflict)
        ));
        assert_eq!(store.snapshot(&provider, &id).unwrap(), before);
    }
    assert!(matches!(
        store.rename(&provider, &id, revision, "旧改名".to_owned()),
        Err(Error::RevisionConflict)
    ));
    assert!(matches!(
        store.delete(&provider, &id, revision),
        Err(Error::RevisionConflict)
    ));
    assert_eq!(store.snapshot(&provider, &id).unwrap(), before);
    let next = store
        .rename(&provider, &id, current, "有效改名".to_owned())
        .unwrap();
    assert_eq!(next.as_u64(), current.as_u64() + 1);
}

#[test]
fn reauthorization_and_rename_each_reject_in_flight_old_auth_results() {
    let provider = provider("provider");
    let id = id("selected");
    let mut store = AccountStore::new();
    let revision = store
        .insert(Account::new(
            id.clone(),
            provider.clone(),
            "原名".to_owned(),
            oauth_auth("old"),
        ))
        .unwrap();
    let authorized = store
        .replace_auth(&provider, &id, revision, key_auth("new-authorization"))
        .unwrap();
    assert!(matches!(
        store.replace_auth(&provider, &id, revision, oauth_auth("old-refresh")),
        Err(Error::RevisionConflict)
    ));
    let renamed = store
        .rename(&provider, &id, authorized, "新名".to_owned())
        .unwrap();
    let before = store.snapshot(&provider, &id).unwrap();
    assert!(matches!(
        store.replace_auth(&provider, &id, authorized, oauth_auth("before-rename")),
        Err(Error::RevisionConflict)
    ));
    assert_eq!(store.snapshot(&provider, &id).unwrap(), before);
    assert_eq!(before.label(), "新名");
    assert_eq!(before.auth(), &key_auth("new-authorization"));
    let same_name = store
        .rename(&provider, &id, renamed, "新名".to_owned())
        .unwrap();
    assert_eq!(same_name.as_u64(), renamed.as_u64() + 1);
    assert!(matches!(
        store.delete(&provider, &id, renamed),
        Err(Error::RevisionConflict)
    ));
}

#[test]
fn deletion_does_not_revoke_snapshots_and_recreation_rejects_old_versions() {
    let provider = provider("provider");
    let id = id("reused-id");
    let mut store = AccountStore::new();
    store
        .insert(Account::new(
            id.clone(),
            provider.clone(),
            "旧名".to_owned(),
            oauth_auth("old"),
        ))
        .unwrap();
    let old = store.snapshot(&provider, &id).unwrap();
    let deleted = store.delete(&provider, &id, old.revision()).unwrap();
    assert_eq!(deleted.as_u64(), old.revision().as_u64() + 1);
    assert_eq!(store.list().len(), 0);
    assert!(matches!(
        store.snapshot(&provider, &id),
        Err(Error::NotFound)
    ));
    assert!(matches!(
        store.delete(&provider, &id, old.revision()),
        Err(Error::NotFound)
    ));
    assert!(matches!(
        store.rename(&provider, &id, old.revision(), "旧改名".to_owned()),
        Err(Error::NotFound)
    ));
    assert!(matches!(
        store.replace_auth(&provider, &id, old.revision(), oauth_auth("late")),
        Err(Error::NotFound)
    ));
    assert_eq!(store.list().len(), 0);
    assert_eq!(old.auth(), &oauth_auth("old"));
    assert_eq!(old.label(), "旧名");

    let recreated = store
        .insert(Account::new(
            id.clone(),
            provider.clone(),
            "重建".to_owned(),
            key_auth("rebuilt"),
        ))
        .unwrap();
    assert_eq!(recreated.as_u64(), deleted.as_u64() + 1);
    let before = store.snapshot(&provider, &id).unwrap();
    assert!(matches!(
        store.replace_auth(&provider, &id, old.revision(), oauth_auth("stale")),
        Err(Error::RevisionConflict)
    ));
    assert!(matches!(
        store.rename(&provider, &id, old.revision(), "过期名称".to_owned()),
        Err(Error::RevisionConflict)
    ));
    assert!(matches!(
        store.delete(&provider, &id, old.revision()),
        Err(Error::RevisionConflict)
    ));
    assert_eq!(store.snapshot(&provider, &id).unwrap(), before);
    assert_eq!(
        store
            .rename(&provider, &id, recreated, "新名".to_owned())
            .unwrap()
            .as_u64(),
        recreated.as_u64() + 1
    );
}

#[test]
fn deleting_and_recreating_under_another_provider_never_retargets_old_operations() {
    let old_provider = provider("old-provider");
    let new_provider = provider("new-provider");
    let id = id("reused");
    let mut store = AccountStore::new();
    let old = store
        .insert(Account::new(
            id.clone(),
            old_provider.clone(),
            String::new(),
            key_auth("old"),
        ))
        .unwrap();
    store.delete(&old_provider, &id, old).unwrap();
    let new = store
        .insert(Account::new(
            id.clone(),
            new_provider.clone(),
            String::new(),
            key_auth("new"),
        ))
        .unwrap();
    assert!(matches!(
        store.snapshot(&old_provider, &id),
        Err(Error::ProviderMismatch)
    ));
    assert!(matches!(
        store.replace_auth(&old_provider, &id, old, oauth_auth("late")),
        Err(Error::ProviderMismatch)
    ));
    assert!(matches!(
        store.replace_auth(&new_provider, &id, old, oauth_auth("late")),
        Err(Error::RevisionConflict)
    ));
    assert_eq!(store.snapshot(&new_provider, &id).unwrap().revision(), new);
    assert_eq!(
        store.snapshot(&new_provider, &id).unwrap().auth(),
        &key_auth("new")
    );
}

#[test]
fn missing_keys_empty_refresh_and_auth_dtos_remain_data_without_implicit_resolution() {
    let provider = provider("provider");
    let mut store = AccountStore::new();
    let env = ProviderEnv::from([("ONLY_CONFIG".to_owned(), "project".to_owned())]);
    let no_key = ApiKeyCredential::new(None).with_env(env.clone());
    assert_eq!(no_key.key(), None);
    assert_eq!(no_key.env(), Some(&env));
    let auth = AccountAuth::new(Credential::ApiKey(no_key));
    store
        .insert(Account::new(
            id("without-key"),
            provider.clone(),
            String::new(),
            auth.clone(),
        ))
        .unwrap();
    assert_eq!(
        store
            .snapshot(&provider, &id("without-key"))
            .unwrap()
            .auth(),
        &auth
    );
    let oauth = OAuthCredential::new(secret("permanent-access"), secret(""), UnixMillis::new(-5));
    assert_eq!(oauth.refresh().expose_secret(), "");
    assert_eq!(oauth.expires().as_i64(), -5);
    store
        .insert(Account::new(
            id("empty-refresh"),
            provider.clone(),
            String::new(),
            AccountAuth::new(Credential::OAuth(oauth.clone())),
        ))
        .unwrap();
    assert_eq!(
        store
            .snapshot(&provider, &id("empty-refresh"))
            .unwrap()
            .auth()
            .credential(),
        &Credential::OAuth(oauth)
    );
    assert_ne!(
        ApiKeyCredential::new(None),
        ApiKeyCredential::new(Some(secret("")))
    );

    let empty = ModelAuth::new();
    assert_eq!(empty, ModelAuth::default());
    assert_eq!(empty.api_key(), None);
    assert_eq!(empty.headers(), None);
    assert_eq!(empty.base_url(), None);
    let headers = ProviderHeaders::from([
        ("remove".to_owned(), None),
        ("set".to_owned(), Some("value".to_owned())),
    ]);
    let model_auth = ModelAuth::new()
        .with_api_key(secret("key"))
        .with_headers(headers.clone())
        .with_base_url("https://example.invalid".to_owned());
    assert_eq!(model_auth.api_key().unwrap().expose_secret(), "key");
    assert_eq!(model_auth.headers(), Some(&headers));
    assert_eq!(model_auth.base_url(), Some("https://example.invalid"));
    let result = AuthResult::new(model_auth.clone())
        .with_env(env.clone())
        .with_source("host-provided".to_owned());
    assert_eq!(result.auth(), &model_auth);
    assert_eq!(result.env(), Some(&env));
    assert_eq!(result.source(), Some("host-provided"));
    let absent = AuthResult::new(ModelAuth::new());
    assert_eq!(absent.env(), None);
    assert_eq!(absent.source(), None);
    assert_ne!(
        absent,
        AuthResult::new(ModelAuth::new()).with_env(ProviderEnv::new())
    );
}

#[test]
fn account_and_auth_public_types_remain_send_and_sync() {
    assert_send_sync::<SecretString>();
    assert_send_sync::<ApiKeyCredential>();
    assert_send_sync::<OAuthCredential>();
    assert_send_sync::<Credential>();
    assert_send_sync::<AccountAuth>();
    assert_send_sync::<ModelAuth>();
    assert_send_sync::<AuthResult>();
    assert_send_sync::<Account>();
    assert_send_sync::<AccountInfo>();
    assert_send_sync::<AccountSnapshot>();
    assert_send_sync::<AccountStore>();
    assert_send_sync::<Revision>();
}

fn assert_send_sync<T: Send + Sync>() {}

#[track_caller]
fn provider(value: &str) -> ProviderId {
    ProviderId::new(value.to_owned()).unwrap()
}

#[track_caller]
fn id(value: &str) -> AccountId {
    AccountId::new(value.to_owned()).unwrap()
}

fn secret(value: &str) -> SecretString {
    SecretString::new(value.to_owned())
}

fn key_auth(value: &str) -> AccountAuth {
    AccountAuth::new(Credential::ApiKey(ApiKeyCredential::new(Some(secret(
        value,
    )))))
}

fn oauth_auth(config: &str) -> AccountAuth {
    let oauth = OAuthCredential::new(
        secret(&format!("access-{config}")),
        secret(&format!("refresh-{config}")),
        UnixMillis::new(-1),
    )
    .with_extensions(JsonObject::from([
        (
            "remote-user".to_owned(),
            JsonValue::String("same-remote-user".to_owned()),
        ),
        (
            "configuration".to_owned(),
            JsonValue::String(config.to_owned()),
        ),
    ]));
    AccountAuth::new(Credential::OAuth(oauth))
        .with_headers(ProviderHeaders::from([
            ("authorization".to_owned(), Some(format!("Bearer {config}"))),
            ("delete-header".to_owned(), None),
        ]))
        .with_base_url(format!("https://example.invalid/{config}"))
        .with_env(ProviderEnv::from([(
            "CONFIG".to_owned(),
            config.to_owned(),
        )]))
        .with_extensions(JsonObject::from([(
            "account-config".to_owned(),
            JsonValue::String(config.to_owned()),
        )]))
}

#[track_caller]
fn snapshots(store: &AccountStore, provider: &ProviderId) -> Vec<AccountSnapshot> {
    store
        .list()
        .map(|info| store.snapshot(provider, info.id()).unwrap())
        .collect()
}
