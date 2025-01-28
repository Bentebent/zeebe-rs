use oauth2::{
    basic::BasicClient, reqwest, AuthUrl, ClientId, ClientSecret, EndpointNotSet, EndpointSet,
    TokenResponse, TokenUrl,
};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};
use thiserror::Error;
use tokio::{
    sync::Notify,
    time::{error::Elapsed, timeout},
};
use tonic::{metadata::MetadataValue, service::Interceptor, Status};

const OAUTH_REFRESH_INTERVAL_SEC: u64 = 10;
const OAUTH_REFRESH_MARGIN_SEC: u64 = 15;

#[derive(Debug, Clone)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub auth_url: String,
    pub audience: String,
}

impl OAuthConfig {
    pub(crate) fn new(
        client_id: String,
        client_secret: String,
        auth_url: String,
        audience: String,
    ) -> Self {
        OAuthConfig {
            client_id,
            client_secret,
            auth_url,
            audience,
        }
    }
}

/// Represents the different types of errors that can occur during OAuth operations.
///
/// The `OAuthError` enum encapsulates various error scenarios that may arise during the OAuth process,
/// such as acquiring token locks, making requests, handling timeouts, or token unavailability.
///
/// # Variants
///
/// - `LockUnavailable`
///   Indicates a failure to acquire a lock for the token. This typically occurs when a lock
///   mechanism is being used to ensure safe concurrent access to OAuth tokens.
///   - Fields:
///     - `String`: A message providing additional context about the lock failure.
///
/// - `Request`
///   Represents a failure that occurred during an OAuth-related request.
///   - Fields:
///     - `String`: A message describing the nature of the request failure.
///
/// - `Timeout`
///   Indicates that a timeout occurred during an OAuth operation. This variant wraps the `Elapsed`
///   type from the `tokio` crate, which provides details about the timeout event.
///   - Source: `Elapsed`
///
/// - `TokenUnavailable`
///   Indicates that a required token was not available. This error typically occurs when attempting
///   to retrieve or use an OAuth token that has not been issued or is otherwise inaccessible.
#[derive(Error, Debug)]
pub enum OAuthError {
    #[error("failed to acquire token lock")]
    LockUnavailable(String),

    #[error("failed request")]
    Request(String),

    #[error("timeout")]
    Timeout(#[from] Elapsed),

    #[error("token unavailable")]
    TokenUnavailable,
}

impl<T> From<std::sync::PoisonError<T>> for OAuthError {
    fn from(err: std::sync::PoisonError<T>) -> Self {
        OAuthError::LockUnavailable(err.to_string())
    }
}

#[derive(Clone, Debug)]
struct CachedToken {
    secret: String,
    expire_at: SystemTime,
}

#[derive(Clone, Debug)]
pub(crate) struct OAuthProvider {
    client: BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>,
    reqwest_client: reqwest::Client,
    audience: String,
    request_timeout: Duration,
    cached_token: Arc<Mutex<Option<CachedToken>>>,
    token_refreshed: Arc<Notify>,
}

impl OAuthProvider {
    fn new(config: OAuthConfig, request_timeout: Duration) -> Self {
        let audience = config.audience.clone();
        let client = BasicClient::new(ClientId::new(config.client_id))
            .set_client_secret(ClientSecret::new(config.client_secret))
            .set_auth_uri(AuthUrl::new(config.auth_url.clone()).unwrap())
            .set_token_uri(TokenUrl::new(config.auth_url.clone()).unwrap());

        let reqwest_client = reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("Client should build");

        OAuthProvider {
            client,
            reqwest_client,
            audience,
            request_timeout,
            cached_token: Arc::new(Mutex::new(None)),
            token_refreshed: Arc::new(Notify::new()),
        }
    }

    async fn token_refreshed(&self) {
        self.token_refreshed.notified().await;
    }

    fn read_token(&self) -> Result<String, OAuthError> {
        let cached_token = self.cached_token.lock()?;

        if let Some(cached_token) = &*cached_token {
            return Ok(cached_token.secret.clone());
        }

        Err(OAuthError::TokenUnavailable)
    }

    fn cached_token_is_expired(&self) -> Result<bool, OAuthError> {
        let lock = self.cached_token.lock()?;

        let expired = if let Some(token) = lock.as_ref() {
            token.expire_at <= SystemTime::now()
        } else {
            true
        };

        Ok(expired)
    }

    fn run(self: Arc<Self>, refresh_interval: Duration) {
        tokio::task::spawn(async move {
            let mut interval = tokio::time::interval(refresh_interval);
            loop {
                interval.tick().await;
                let _ = self.refresh_token().await;
            }
        });
    }

    async fn refresh_token(&self) -> Result<(), OAuthError> {
        if !self.cached_token_is_expired()? {
            return Ok(());
        }

        let token_request = self
            .client
            .exchange_client_credentials()
            .add_extra_param("audience", self.audience.clone());

        let result = match timeout(
            self.request_timeout,
            token_request.request_async(&self.reqwest_client),
        )
        .await
        {
            Ok(Ok(response)) => response,
            Ok(Err(err)) => return Err(OAuthError::Request(err.to_string())),
            Err(err) => return Err(OAuthError::Timeout(err)),
        };

        let expiry = std::time::SystemTime::now() + result.expires_in().unwrap_or_default()
            - Duration::from_secs(OAUTH_REFRESH_MARGIN_SEC);
        let new_token = CachedToken {
            secret: result.access_token().secret().to_owned(),
            expire_at: expiry,
        };

        let _ = self.cached_token.lock()?.replace(new_token);
        self.token_refreshed.notify_one();

        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct OAuthInterceptor {
    oauth_provider: Option<Arc<OAuthProvider>>,
}

impl OAuthInterceptor {
    pub(crate) fn new(oauth_config: OAuthConfig, auth_timeout: Duration) -> Self {
        let provider = Arc::new(OAuthProvider::new(oauth_config, auth_timeout));

        provider
            .clone()
            .run(Duration::from_secs(OAUTH_REFRESH_INTERVAL_SEC));

        OAuthInterceptor {
            oauth_provider: Some(provider),
        }
    }

    pub(crate) async fn auth_initialized(&self) {
        if let Some(provider) = &self.oauth_provider {
            provider.token_refreshed().await;
        }
    }
}

impl Interceptor for OAuthInterceptor {
    fn call(
        &mut self,
        mut request: tonic::Request<()>,
    ) -> std::result::Result<tonic::Request<()>, Status> {
        if let Some(oauth_client) = &mut self.oauth_provider {
            let token = match oauth_client.read_token() {
                Ok(token) => token,
                Err(err) => {
                    return Err(tonic::Status::unauthenticated(format!(
                        "{}: {}",
                        "failed to get token", err
                    )));
                }
            };

            request.metadata_mut().insert(
                "authorization",
                MetadataValue::try_from(&format!("Bearer {}", token)).map_err(|_| {
                    tonic::Status::unauthenticated(format!(
                        "{}: {}",
                        "token is not a valid header value", token
                    ))
                })?,
            );
        }

        Ok(request)
    }
}
