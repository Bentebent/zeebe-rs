use crate::{
    decision::EvaluateDecisionRequest,
    incident::ResolveIncidentRequest,
    job::{
        complete::CompleteJobRequest, fail::FailJobRequest,
        update_retries::UpdateJobRetriesRequest, update_timeout::UpdateJobTimeoutRequest,
    },
    message::PublishMessageRequest,
    oauth::{OAuthConfig, OAuthInterceptor},
    process_instance::{
        cancel::CancelProcessInstanceRequest, create::CreateProcessInstanceRequest,
        migrate::MigrateProcessInstanceRequest, modify::ModifyProcessInstanceRequest,
    },
    proto::gateway_client::GatewayClient,
    resource::{DeleteResourceRequest, DeployResourceError, DeployResourceRequest},
    set_variables::SetVariablesRequest,
    signal::BroadcastSignalRequest,
    throw_error::ThrowErrorRequest,
    topology::TopologyRequest,
};
use std::{path::Path, time::Duration};
use thiserror::Error;
use tonic::{
    codegen::InterceptedService,
    transport::{Certificate, Channel, ClientTlsConfig},
};

#[derive(Error, Debug)]
pub enum ClientError {
    #[error(transparent)]
    RequestFailed(#[from] tonic::Status),

    #[error(transparent)]
    JsonError(#[from] serde_json::Error),

    #[error(transparent)]
    ResourceError(#[from] DeployResourceError),
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
pub struct Initial;
pub struct WithAddress;

pub trait ClientBuilderState {}
impl ClientBuilderState for Initial {}
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

impl ClientBuilder<Initial> {
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
    pub fn builder() -> ClientBuilder<Initial> {
        ClientBuilder::default()
    }

    pub async fn auth_initialized(&self) {
        self.auth_interceptor.auth_initialized().await;
    }

    pub fn topology(&self) -> TopologyRequest {
        TopologyRequest::new(self.clone())
    }

    pub fn deploy_resource(&self) -> DeployResourceRequest<crate::resource::Initial> {
        DeployResourceRequest::<crate::resource::Initial>::new(self.clone())
    }

    pub fn delete_resource(&self) -> DeleteResourceRequest<crate::resource::Initial> {
        DeleteResourceRequest::<crate::resource::Initial>::new(self.clone())
    }

    pub fn create_process_instance(
        &self,
    ) -> CreateProcessInstanceRequest<crate::process_instance::create::Initial> {
        CreateProcessInstanceRequest::<crate::process_instance::create::Initial>::new(self.clone())
    }

    pub fn cancel_process_instance(
        &self,
    ) -> CancelProcessInstanceRequest<crate::process_instance::cancel::Initial> {
        CancelProcessInstanceRequest::<crate::process_instance::cancel::Initial>::new(self.clone())
    }

    pub fn migrate_process_instance(
        &self,
    ) -> MigrateProcessInstanceRequest<crate::process_instance::migrate::Initial> {
        MigrateProcessInstanceRequest::<crate::process_instance::migrate::Initial>::new(
            self.clone(),
        )
    }

    pub fn modify_process_instance(
        &self,
    ) -> ModifyProcessInstanceRequest<crate::process_instance::modify::Initial> {
        ModifyProcessInstanceRequest::<crate::process_instance::modify::Initial>::new(self.clone())
    }

    pub fn set_variables(&self) -> SetVariablesRequest<crate::set_variables::Initial> {
        SetVariablesRequest::<crate::set_variables::Initial>::new(self.clone())
    }

    pub fn publish_message(&self) -> PublishMessageRequest<crate::message::Initial> {
        PublishMessageRequest::<crate::message::Initial>::new(self.clone())
    }

    pub fn broadcast_signal(&self) -> BroadcastSignalRequest<crate::signal::Initial> {
        BroadcastSignalRequest::<crate::signal::Initial>::new(self.clone())
    }

    pub fn resolve_incident(&self) -> ResolveIncidentRequest<crate::incident::Initial> {
        ResolveIncidentRequest::<crate::incident::Initial>::new(self.clone())
    }

    pub fn throw_error(&self) -> ThrowErrorRequest<crate::throw_error::Initial> {
        ThrowErrorRequest::<crate::throw_error::Initial>::new(self.clone())
    }

    pub fn evaluate_decision(&self) -> EvaluateDecisionRequest<crate::decision::Initial> {
        EvaluateDecisionRequest::<crate::decision::Initial>::new(self.clone())
    }

    pub fn complete_job(&self) -> CompleteJobRequest<crate::job::complete::Initial> {
        CompleteJobRequest::<crate::job::complete::Initial>::new(self.clone())
    }

    pub fn fail_job(&self) -> FailJobRequest<crate::job::fail::Initial> {
        FailJobRequest::<crate::job::fail::Initial>::new(self.clone())
    }

    pub fn update_job_timeout(
        &self,
    ) -> UpdateJobTimeoutRequest<crate::job::update_timeout::Initial> {
        UpdateJobTimeoutRequest::<crate::job::update_timeout::Initial>::new(self.clone())
    }

    pub fn update_job_retries(
        &self,
    ) -> UpdateJobRetriesRequest<crate::job::update_retries::Initial> {
        UpdateJobRetriesRequest::<crate::job::update_retries::Initial>::new(self.clone())
    }
}
