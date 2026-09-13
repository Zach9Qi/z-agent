//! 秘密哨兵覆盖内外层常规与美化调试、白名单摘要以及账号操作的错误输出。
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fmt;

use z_ai::{
    Error,
    account::{Account, AccountStore},
    auth::{
        AccountAuth, ApiKeyCredential, AuthResult, Credential, CredentialKind, ModelAuth,
        OAuthCredential, SecretString,
    },
    identity::{AccountId, ApiId, ModelId, ProviderId, UnixMillis},
    model::{Model, ModelCapabilities, ModelCompat, ModelCost, ModelCostRates, ModelIdentity},
    options::{
        DeferredFetchOptions, ProviderEnv, ProviderHeaders, ProviderOptions, ProviderResponse,
        RequestOptions, SimpleStreamOptions, StreamOptions,
    },
    value::{JsonObject, JsonValue},
};

const SECRETS: [&str; 12] = [
    "SENTINEL_KEY_7c9",
    "SENTINEL_ACCESS_7c9",
    "SENTINEL_REFRESH_7c9",
    "SENTINEL_HEADER_NAME_7c9",
    "SENTINEL_HEADER_VALUE_7c9",
    "SENTINEL_ENV_NAME_7c9",
    "SENTINEL_ENV_VALUE_7c9",
    "SENTINEL_URL_7c9",
    "SENTINEL_EXTENSION_NAME_7c9",
    "SENTINEL_EXTENSION_VALUE_7c9",
    "SENTINEL_SOURCE_7c9",
    "SENTINEL_DELETED_HEADER_7c9",
];

#[test]
fn every_credential_and_account_layer_redacts_regular_and_pretty_debug() {
    let secret = secret(SECRETS[0]);
    let api_key = ApiKeyCredential::new(Some(secret.clone())).with_env(env());
    let oauth = OAuthCredential::new(
        self::secret(SECRETS[1]),
        self::secret(SECRETS[2]),
        UnixMillis::new(123),
    )
    .with_extensions(extensions());
    let key_credential = Credential::ApiKey(api_key.clone());
    let oauth_credential = Credential::OAuth(oauth.clone());
    let key_auth = account_auth(key_credential.clone());
    let oauth_auth = account_auth(oauth_credential.clone());
    let model_auth = ModelAuth::new()
        .with_api_key(secret.clone())
        .with_headers(headers())
        .with_base_url(url());
    let result = AuthResult::new(model_auth.clone())
        .with_env(env())
        .with_source(SECRETS[10].to_owned());
    let provider = ProviderId::new("public-provider".to_owned()).unwrap();
    let key_id = AccountId::new("public-key-config".to_owned()).unwrap();
    let oauth_id = AccountId::new("public-oauth-config".to_owned()).unwrap();
    let key_account = Account::new(
        key_id.clone(),
        provider.clone(),
        "公开的密钥配置名称".to_owned(),
        key_auth.clone(),
    );
    let oauth_account = Account::new(
        oauth_id.clone(),
        provider.clone(),
        "公开的授权配置名称".to_owned(),
        oauth_auth.clone(),
    );
    let mut store = AccountStore::new();
    store.insert(key_account.clone()).unwrap();
    store.insert(oauth_account.clone()).unwrap();
    let key_snapshot = store.snapshot(&provider, &key_id).unwrap();
    let oauth_snapshot = store.snapshot(&provider, &oauth_id).unwrap();

    let layers: [&dyn fmt::Debug; 14] = [
        &secret,
        &api_key,
        &oauth,
        &key_credential,
        &oauth_credential,
        &key_auth,
        &oauth_auth,
        &model_auth,
        &result,
        &key_account,
        &oauth_account,
        &key_snapshot,
        &oauth_snapshot,
        &store,
    ];
    for layer in layers {
        for output in [format!("{layer:?}"), format!("{layer:#?}")] {
            assert_no_secrets(&output);
            assert!(!output.is_empty());
        }
    }
    assert_eq!(format!("{secret:?}"), "SecretString([REDACTED])");
    assert_eq!(format!("{secret:#?}"), "SecretString([REDACTED])");

    // 显式读取仍须得到原值；自动脱敏不能通过删除或破坏存储中的秘密来实现
    assert_eq!(api_key.key().unwrap().expose_secret(), SECRETS[0]);
    assert_eq!(oauth.access().expose_secret(), SECRETS[1]);
    assert_eq!(oauth.refresh().expose_secret(), SECRETS[2]);
    assert_eq!(api_key.env(), Some(&env()));
    assert_eq!(oauth.extensions(), &extensions());
    assert_eq!(key_snapshot.auth().headers(), Some(&headers()));
    assert_eq!(oauth_snapshot.auth().env(), Some(&env()));
    assert_eq!(oauth_snapshot.auth().extensions(), &extensions());
    assert_eq!(oauth_snapshot.auth().base_url(), Some(url().as_str()));
    assert_eq!(result.source(), Some(SECRETS[10]));
}

#[test]
fn list_is_an_owned_five_field_whitelist_with_no_auth_configuration() {
    let provider = ProviderId::new("public-provider".to_owned()).unwrap();
    let id = AccountId::new("public-account".to_owned()).unwrap();
    let auth = account_auth(Credential::OAuth(
        OAuthCredential::new(secret(SECRETS[1]), secret(SECRETS[2]), UnixMillis::new(123))
            .with_extensions(extensions()),
    ));
    let mut store = AccountStore::new();
    let revision = store
        .insert(Account::new(
            id.clone(),
            provider.clone(),
            "公开名称".to_owned(),
            auth,
        ))
        .unwrap();
    let listed: Vec<_> = store.list().collect();
    let info = listed.first().unwrap();
    assert_eq!(info.id(), &id);
    assert_eq!(info.provider(), &provider);
    assert_eq!(info.label(), "公开名称");
    assert_eq!(info.kind(), CredentialKind::OAuth);
    assert_eq!(info.revision(), revision);
    for output in [
        format!("{info:?}"),
        format!("{info:#?}"),
        format!("{listed:#?}"),
    ] {
        assert_no_secrets(&output);
        for forbidden in [
            "credential:",
            "auth:",
            "headers:",
            "base_url:",
            "env:",
            "extensions:",
            "expires:",
        ] {
            assert!(!output.contains(forbidden), "摘要不应包含 {forbidden}");
        }
        for public in [
            "public-account",
            "public-provider",
            "公开名称",
            "OAuth",
            "revision",
        ] {
            assert!(output.contains(public), "摘要应保留 {public}");
        }
    }
    store
        .rename(&provider, &id, revision, "后来名称".to_owned())
        .unwrap();
    assert_eq!(info.label(), "公开名称");
    assert_eq!(info.revision(), revision);
}

#[test]
fn rejected_operations_never_include_secret_arguments_in_error_debug_or_display() {
    let provider = ProviderId::new("public-provider".to_owned()).unwrap();
    let wrong_provider = ProviderId::new("wrong-provider".to_owned()).unwrap();
    let id = AccountId::new("public-account".to_owned()).unwrap();
    let missing = AccountId::new(SECRETS[0].to_owned()).unwrap();
    let auth = account_auth(Credential::ApiKey(ApiKeyCredential::new(Some(secret(
        SECRETS[0],
    )))));
    let mut store = AccountStore::new();
    let revision = store
        .insert(Account::new(
            id.clone(),
            provider.clone(),
            "公开名称".to_owned(),
            auth.clone(),
        ))
        .unwrap();
    let updated = store
        .rename(&provider, &id, revision, "新名称".to_owned())
        .unwrap();
    let errors = [
        store
            .insert(Account::new(
                id.clone(),
                provider.clone(),
                SECRETS[0].to_owned(),
                auth.clone(),
            ))
            .unwrap_err(),
        store.snapshot(&provider, &missing).unwrap_err(),
        store.snapshot(&wrong_provider, &id).unwrap_err(),
        store
            .replace_auth(&provider, &id, revision, auth.clone())
            .unwrap_err(),
        store
            .replace_auth(&wrong_provider, &id, updated, auth.clone())
            .unwrap_err(),
        store
            .replace_auth(&provider, &missing, updated, auth)
            .unwrap_err(),
        store
            .rename(&provider, &id, revision, SECRETS[0].to_owned())
            .unwrap_err(),
        store.delete(&provider, &id, revision).unwrap_err(),
        // 实际耗尽路径在账号领域私有单测中构造，此处检查该错误类型的输出契约
        Error::RevisionExhausted,
        AccountId::new(" \t\n".to_owned()).unwrap_err(),
    ];
    for error in errors {
        for output in [
            format!("{error:?}"),
            format!("{error:#?}"),
            error.to_string(),
        ] {
            assert_no_secrets(&output);
            assert!(!output.is_empty());
        }
    }
    assert_eq!(store.snapshot(&provider, &id).unwrap().revision(), updated);
}

#[test]
fn model_and_request_configuration_containers_redact_unknown_keys_and_values() {
    let api = ApiId::new("public-api".to_owned()).unwrap();
    let provider = ProviderId::new("public-provider".to_owned()).unwrap();
    let model_id = ModelId::new("public-model".to_owned()).unwrap();
    let compat = ModelCompat::new(api.clone(), extensions());
    let model = Model::new(
        ModelIdentity::new(model_id, "公开模型".to_owned(), api.clone(), provider),
        url(),
        ModelCapabilities::new(false, Vec::new(), 0, 0),
        ModelCost::new(ModelCostRates::new(0.0, 0.0, 0.0, 0.0).unwrap()),
    )
    .with_headers(env())
    .with_sampling_params(extensions())
    .with_compat(compat.clone());
    let provider_options = ProviderOptions::new(api, extensions());
    let request = RequestOptions::new()
        .with_headers(headers())
        .with_env(env());
    let stream = StreamOptions::new(request.clone())
        .with_sampling_params(extensions())
        .with_metadata(extensions())
        .with_provider_options(provider_options.clone());
    let simple = SimpleStreamOptions::new(stream.clone());
    let deferred = DeferredFetchOptions::new(request.clone());
    let response = ProviderResponse::new(200, env());
    let layers: [&dyn fmt::Debug; 8] = [
        &compat,
        &model,
        &provider_options,
        &request,
        &stream,
        &simple,
        &deferred,
        &response,
    ];
    for layer in layers {
        for output in [format!("{layer:?}"), format!("{layer:#?}")] {
            assert_no_secrets(&output);
            assert!(!output.is_empty());
        }
    }
    assert_eq!(model.base_url(), url());
    assert_eq!(model.headers(), Some(&env()));
    assert_eq!(model.sampling_params(), Some(&extensions()));
    assert_eq!(request.headers(), Some(&headers()));
    assert_eq!(request.env(), Some(&env()));
}

fn secret(value: &str) -> SecretString {
    SecretString::new(value.to_owned())
}

fn headers() -> ProviderHeaders {
    ProviderHeaders::from([
        (SECRETS[3].to_owned(), Some(SECRETS[4].to_owned())),
        (SECRETS[11].to_owned(), None),
    ])
}

fn env() -> ProviderEnv {
    ProviderEnv::from([(SECRETS[5].to_owned(), SECRETS[6].to_owned())])
}

fn extensions() -> JsonObject {
    JsonObject::from([(
        SECRETS[8].to_owned(),
        JsonValue::Object(JsonObject::from([(
            "nested".to_owned(),
            JsonValue::String(SECRETS[9].to_owned()),
        )])),
    )])
}

fn url() -> String {
    format!("https://example.invalid/?secret={}", SECRETS[7])
}

fn account_auth(credential: Credential) -> AccountAuth {
    AccountAuth::new(credential)
        .with_headers(headers())
        .with_base_url(url())
        .with_env(env())
        .with_extensions(extensions())
}

#[track_caller]
fn assert_no_secrets(output: &str) {
    for secret in SECRETS {
        assert!(!output.contains(secret), "常规输出不得泄露秘密哨兵");
    }
}
