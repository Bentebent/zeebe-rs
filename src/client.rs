use std::{path::Path, time::Duration};

use tonic::{
    codegen::InterceptedService,
    transport::{Certificate, Channel, ClientTlsConfig},
};

use thiserror::Error;

use crate::{
    deploy_resource::DeployResource,
    oauth::{OAuthConfig, OAuthInterceptor},
    proto::gateway_client::GatewayClient,
    topology::TopologyRequest,
};

#[derive(Error, Debug)]
pub enum ClientError {
    #[error(transparent)]
    RequestFailed(#[from] tonic::Status),
}

#[derive(Error, Debug)]
pub enum ClientBuilderError {
    #[error("failed to load certificate")]
    Certificate(#[from] std::io::Error),

    #[error(transparent)]
    Transport(#[from] tonic::transport::Error),

    #[error(transparent)]
    Http(#[from] tonic::codegen::http::Error),

    #[error("unable to parse URI")]
    InvalidUri(#[from] tonic::codegen::http::uri::InvalidUri),
}

#[derive(Default)]
pub struct Empty;
pub struct WithAddress;

pub trait ClientBuilderState {}
impl ClientBuilderState for Empty {}
impl ClientBuilderState for WithAddress {}

#[derive(Debug)]
pub struct ClientBuilder<S: ClientBuilderState> {
    endpoint: Option<String>,
    tls: Option<ClientTlsConfig>,
    timeout: Option<Duration>,
    keep_alive: Option<Duration>,
    auth_timeout: Option<Duration>,
    oauth_config: Option<OAuthConfig>,
    _state: std::marker::PhantomData<S>,
}

impl<S: ClientBuilderState + Default> Default for ClientBuilder<S> {
    fn default() -> Self {
        Self {
            endpoint: Default::default(),
            tls: Default::default(),
            timeout: Default::default(),
            auth_timeout: Default::default(),
            keep_alive: Default::default(),
            oauth_config: Default::default(),
            _state: std::marker::PhantomData,
        }
    }
}

impl<S: ClientBuilderState> ClientBuilder<S> {
    fn transition<NewState: ClientBuilderState>(self) -> ClientBuilder<NewState> {
        ClientBuilder {
            endpoint: self.endpoint,
            tls: self.tls,
            timeout: self.timeout,
            auth_timeout: self.auth_timeout,
            keep_alive: self.keep_alive,
            oauth_config: self.oauth_config,
            _state: std::marker::PhantomData,
        }
    }

    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout = Some(Duration::from_secs(seconds));
        self
    }

    pub fn with_keep_alive(mut self, seconds: u64) -> Self {
        self.keep_alive = Some(Duration::from_secs(seconds));
        self
    }
}

impl ClientBuilder<Empty> {
    fn set_endpoint(&mut self, zeebe_address: &str, port: u16) {
        self.endpoint = Some(format!("{}:{}", zeebe_address, port));
    }

    pub fn with_address(mut self, zeebe_address: &str, port: u16) -> ClientBuilder<WithAddress> {
        self.set_endpoint(zeebe_address, port);
        self.transition()
    }
}

impl ClientBuilder<WithAddress> {
    pub fn with_oauth(
        mut self,
        client_id: String,
        client_secret: String,
        auth_url: String,
        audience: String,
        auth_timeout: Duration,
    ) -> ClientBuilder<WithAddress> {
        self.oauth_config = Some(OAuthConfig::new(
            client_id,
            client_secret,
            auth_url,
            audience,
        ));
        self.auth_timeout = Some(auth_timeout);
        self
    }

    pub fn with_tls(
        mut self,
        pem: &Path,
    ) -> Result<ClientBuilder<WithAddress>, ClientBuilderError> {
        let cert = std::fs::read_to_string(pem)?;
        self.tls = Some(ClientTlsConfig::new().ca_certificate(Certificate::from_pem(&cert)));

        Ok(self)
    }

    async fn build_channel(&self) -> Result<Channel, ClientBuilderError> {
        let endpoint = self
            .endpoint
            .as_ref()
            .expect("Only transition to buildable if endpoint is set")
            .to_owned();
        let mut channel = Channel::from_shared(endpoint)?;

        if let Some(ref tls) = self.tls {
            channel = channel.tls_config(tls.clone())?;
        }

        Ok(channel.connect().await?)
    }

    pub async fn build(self) -> Result<Client, ClientBuilderError> {
        let channel = self.build_channel().await?;

        let auth_interceptor = if let Some(cfg) = self.oauth_config {
            OAuthInterceptor::new(
                cfg,
                self.auth_timeout
                    .expect("Only build oauth provider if auth timeout is set"),
            )
        } else {
            OAuthInterceptor::default()
        };
        let gateway_client = GatewayClient::with_interceptor(channel, auth_interceptor.clone());

        Ok(Client {
            gateway_client,
            auth_interceptor,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Client {
    pub(crate) gateway_client: GatewayClient<InterceptedService<Channel, OAuthInterceptor>>,
    pub(crate) auth_interceptor: OAuthInterceptor,
}

impl Client {
    pub fn builder() -> ClientBuilder<Empty> {
        ClientBuilder::default()
    }

    pub async fn auth_initialized(&self) {
        self.auth_interceptor.auth_initialized().await;
    }

    pub fn request_topology(&self) -> TopologyRequest {
        TopologyRequest::new(self.clone())
    }

    pub fn deploy_resource(&self) -> DeployResource<crate::deploy_resource::Empty> {
        DeployResource::<crate::deploy_resource::Empty>::new(self.clone())
    }
}
