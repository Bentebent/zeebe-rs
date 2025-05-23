use crate::{
    decision::EvaluateDecisionRequest,
    incident::ResolveIncidentRequest,
    job::{
        complete::CompleteJobRequest, fail::FailJobRequest,
        update_retries::UpdateJobRetriesRequest, update_timeout::UpdateJobTimeoutRequest,
    },
    message::PublishMessageRequest,
    oauth::{AuthType, OAuthConfig, OAuthInterceptor},
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
    worker::WorkerBuilder,
};
use std::{path::Path, time::Duration};
use thiserror::Error;
use tonic::{
    codegen::InterceptedService,
    transport::{Certificate, Channel, ClientTlsConfig},
};

/// Represents the different types of errors that can occur in the client.
///
/// The `ClientError` enum encapsulates various error types, providing a unified way to handle errors
/// that arise during client operations, including networking issues, JSON processing errors, and
/// resource-related errors. Each variant includes contextual information to help diagnose and resolve
/// issues effectively.
///
/// # Variants
///
/// - `RequestFailed`
///   Represents an error originating from a gRPC request. This variant wraps the `tonic::Status`
///   error type, which provides details about the request failure.
///   - Source: `tonic::Status`
///
/// - `JsonError`
///   Represents a general JSON parsing or serialization error. This variant wraps the `serde_json::Error`
///   type, which is returned when JSON data cannot be processed correctly.
///   - Source: `serde_json::Error`
///
/// - `ResourceError`
///   Represents an error related to resource deployment. This variant wraps the `DeployResourceError`
///   type, which provides context about specific resource-related issues.
///   - Source: `DeployResourceError`
///
/// - `DeserializationFailed`
///   Indicates that deserialization of a JSON string failed. This variant provides additional
///   context by including the original value that caused the error, as well as the underlying
///   `serde_json::Error`.
///   - Fields:
///     - `value`: The JSON string that failed to deserialize.
///     - `source`: The underlying `serde_json::Error`.
///
/// - `SerializationFailed`
///   Indicates that serialization of data into JSON failed. This variant includes the underlying
///   `serde_json::Error` to provide details about the failure.
///   - Fields:
///     - `source`: The underlying `serde_json::Error`.
#[derive(Error, Debug)]
pub enum ClientError {
    #[error(transparent)]
    RequestFailed(#[from] tonic::Status),

    #[error(transparent)]
    JsonError(#[from] serde_json::Error),

    #[error(transparent)]
    ResourceError(#[from] DeployResourceError),

    #[error("deserialize failed on {value:?}")]
    DeserializationFailed {
        value: String,
        source: serde_json::Error,
    },

    #[error("serialize failed")]
    SerializationFailed { source: serde_json::Error },
}

/// Represents errors that can occur while building a `Client`.
///
/// The `ClientBuilderError` enum provides variants for different types of errors
/// that can occur during the client building process, such as loading certificates,
/// transport errors, HTTP errors, and URI parsing errors.
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

#[derive(Default, Clone)]
pub struct Initial;

#[derive(Default, Clone)]
pub struct WithAddress;

pub trait ClientBuilderState {}
impl ClientBuilderState for Initial {}
impl ClientBuilderState for WithAddress {}

/// A builder for configuring and creating a `Client`.
///
/// The `ClientBuilder` allows you to configure various aspects of the client,
/// such as the endpoint, TLS settings, timeouts, and OAuth configuration.
#[derive(Debug, Clone)]
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
}

impl ClientBuilder<Initial> {
    fn set_endpoint(&mut self, zeebe_address: &str, port: u16) {
        self.endpoint = Some(format!("{}:{}", zeebe_address, port));
    }

    /// Sets the endpoint for the Zeebe client.
    ///
    /// # Arguments
    ///
    /// * `zeebe_address` - A string slice that holds the address of the Zeebe broker.
    /// * `port` - A 16-bit unsigned integer that holds the port number of the Zeebe broker.
    ///
    /// # Returns
    ///
    /// A `ClientBuilder<WithAddress>` instance with the Zeebe endpoint set.
    pub fn with_address(mut self, zeebe_address: &str, port: u16) -> ClientBuilder<WithAddress> {
        self.set_endpoint(zeebe_address, port);
        self.transition()
    }
}

impl ClientBuilder<WithAddress> {
    /// Configures OAuth authentication for the client.
    ///
    /// # Arguments
    ///
    /// * `client_id` - The client ID for OAuth authentication.
    /// * `client_secret` - The client secret for OAuth authentication.
    /// * `auth_url` - The URL for the OAuth authentication server.
    /// * `audience` - The audience for the OAuth token.
    /// * `auth_timeout` - The timeout duration for the OAuth authentication process.
    /// * `auth_type` - If OAuth credentials should use request body or basic auth, defaults to request body
    ///
    /// # Returns
    ///
    /// A `ClientBuilder<WithAddress>` instance with OAuth configuration set.
    pub fn with_oauth(
        mut self,
        client_id: String,
        client_secret: String,
        auth_url: String,
        audience: String,
        auth_timeout: Duration,
        auth_type: Option<AuthType>,
    ) -> Self {
        self.oauth_config = Some(OAuthConfig::new(
            client_id,
            client_secret,
            auth_url,
            audience,
            auth_type,
        ));
        self.auth_timeout = Some(auth_timeout);

        self
    }

    /// Configures TLS for the client root certificates required for Camunda Cloud
    ///
    /// # Returns
    ///
    /// A `ClientBuilder<WithAddress>` instance with tls configuration set.
    pub fn with_cloud_tls(mut self) -> Self {
        self.tls = Some(ClientTlsConfig::new().with_enabled_roots());
        self
    }

    /// Configures TLS for the client using a PEM file.
    ///
    /// # Arguments
    ///
    /// * `pem` - The path to the PEM file containing the TLS certificate.
    ///
    /// # Returns
    ///
    /// A `Result` containing either a `ClientBuilder<WithAddress>` instance with TLS configuration set,
    /// or a `ClientBuilderError` if reading the PEM file fails.
    pub fn with_tls(mut self, pem: &Path) -> Result<Self, ClientBuilderError> {
        let cert = std::fs::read_to_string(pem)?;
        self.tls = Some(ClientTlsConfig::new().ca_certificate(Certificate::from_pem(&cert)));

        Ok(self)
    }

    /// Builds the gRPC channel for the client.
    ///
    /// # Returns
    ///
    /// A `Result` containing either a `Channel` instance or a `ClientBuilderError` if the channel
    /// could not be created.
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

        if let Some(timeout) = self.timeout {
            channel = channel.timeout(timeout);
        }

        if let Some(keep_alive) = self.keep_alive {
            channel = channel.keep_alive_timeout(keep_alive);
        }

        Ok(channel.connect().await?)
    }

    /// Builds the client with the configured settings.
    ///
    /// # Returns
    ///
    /// A `Result` containing either a `Client` instance or a `ClientBuilderError` if the client
    /// could not be built.
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

    /// Sets the timeout duration for the client.
    ///
    /// # Arguments
    ///
    /// * `timeout` - The timeout duration.
    ///
    /// # Returns
    ///
    /// A `ClientBuilder<WithAddress>` instance with the timeout configuration set.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets the keep-alive duration for the client.
    ///
    /// # Arguments
    ///
    /// * `keep_alive` - The keep-alive duration.
    ///
    /// # Returns
    ///
    /// A `ClientBuilder<WithAddress>` instance with the keep-alive configuration set.
    pub fn with_keep_alive(mut self, keep_alive: Duration) -> Self {
        self.keep_alive = Some(keep_alive);
        self
    }
}

/// A client for interacting with the Zeebe cluster.
///
/// The `Client` struct provides methods to create various requests and operations
/// on the Zeebe cluster, such as deploying resources, managing process instances,
/// handling jobs, and more.
///
/// # Examples
///
/// ```ignore
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = zeebe_rs::Client::builder()
///         .with_address("http://localhost", 26500)
///         .build()
///         .await?;
///
///    let topology = client.topology().send().await;
///
///    Ok(())
/// }
/// ```
/// # Notes
///
/// Each method returns a request builder that can be further configured and then sent
/// to the Zeebe cluster. The requests are asynchronous and return futures that need to
/// be awaited.
#[derive(Clone, Debug)]
pub struct Client {
    pub(crate) gateway_client: GatewayClient<InterceptedService<Channel, OAuthInterceptor>>,
    pub(crate) auth_interceptor: OAuthInterceptor,
}

impl Client {
    /// Creates a new `ClientBuilder` instance for configuring and building a `Client`.
    ///
    /// The `ClientBuilder` allows you to set various configurations such as the endpoint,
    /// TLS settings, timeouts, and OAuth configuration before building the `Client`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = zeebe_rs::Client::builder()
    ///         .with_address("http://localhost", 26500)
    ///         .build()
    ///         .await?;
    ///
    ///     let topology = client.topology().send().await;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn builder() -> ClientBuilder<Initial> {
        ClientBuilder::default()
    }

    /// Waits for the first OAuth token to be fetched before returning.
    /// Returns instantly if OAuth is not enabled.
    /// # Examples
    ///
    /// ```ignore
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = zeebe_rs::Client::builder()
    ///         // Configure client with OAuth...
    ///        .build()
    ///        .await?;
    ///
    ///     // Await first OAuth token before proceeding
    ///     let _ = client.auth_initialized().await;
    ///     
    ///     // Fetch topology after acquiring OAuth token
    ///     let topology = client.topology().send().await;
    ///
    ///    Ok(())
    ///}
    /// ```
    pub async fn auth_initialized(&self) {
        self.auth_interceptor.auth_initialized().await;
    }

    /// Creates a `TopologyRequest` to build a request for fetching the toplogy
    /// of the Zeebe cluster.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let topology = client.topology().send().await;
    /// ```
    pub fn topology(&self) -> TopologyRequest {
        TopologyRequest::new(self.clone())
    }

    /// Creates a `DeployResourceRequest` to build a request for deploying a
    /// resource to Zeebe.
    ///
    /// # Examples
    /// ```ignore
    ///  let result = client
    ///     .deploy_resource()
    ///     .with_resource_file(PathBuf::from("./examples/resources/hello_world.bpmn"))
    ///     .read_resource_files()?
    ///     .send()
    ///     .await?;
    /// ```
    pub fn deploy_resource(&self) -> DeployResourceRequest<crate::resource::Initial> {
        DeployResourceRequest::<crate::resource::Initial>::new(self.clone())
    }

    /// Creates a `DeleteResourceRequest` to build a request for deleting a
    /// deployed resource in Zeebe.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let response = client
    ///     .delete_resource()
    ///     .with_resource_key(12345)
    ///     .send()
    ///     .await?;
    /// ```
    pub fn delete_resource(&self) -> DeleteResourceRequest<crate::resource::Initial> {
        DeleteResourceRequest::<crate::resource::Initial>::new(self.clone())
    }

    /// Creates a `CreateProcessInstanceRequest` to build a request for creating
    /// a process instance in Zeebe.
    /// # Examples
    ///
    /// ```ignore
    /// // Create a process instance with a BPMN process ID and no input variables
    /// client
    ///     .create_process_instance()
    ///     .with_bpmn_process_id(String::from("order-process"))
    ///     .without_input()
    ///     .send()
    ///     .await?;
    ///
    /// // Create a process instance with a process definition key and input variables
    /// client
    ///     .create_process_instance()
    ///     .with_process_definition_key(12345)
    ///     .with_variables(json!({"orderId": 123}))
    ///     .unwrap()
    ///     .send()
    ///     .await?;
    /// ```
    pub fn create_process_instance(
        &self,
    ) -> CreateProcessInstanceRequest<crate::process_instance::create::Initial> {
        CreateProcessInstanceRequest::<crate::process_instance::create::Initial>::new(self.clone())
    }

    /// Creates a `CancelProcessInstanceRequest` to cancel an active
    /// process instance in Zeebe.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// client
    ///     .cancel_process_instance()
    ///     .with_process_instance_key(123456)
    ///     .send()
    ///     .await?;
    /// ```
    ///
    pub fn cancel_process_instance(
        &self,
    ) -> CancelProcessInstanceRequest<crate::process_instance::cancel::Initial> {
        CancelProcessInstanceRequest::<crate::process_instance::cancel::Initial>::new(self.clone())
    }

    /// Creates a `MigrateProcessInstanceRequest` to migrate a running process
    /// instance in Zeebe.
    ///
    /// # Examples
    /// ```ignore
    /// client
    ///     .migrate_process_instance()
    ///     .with_process_instance_key(12356)
    ///     .without_migration_plan()
    ///     .send()
    ///     .await?;
    /// ```
    pub fn migrate_process_instance(
        &self,
    ) -> MigrateProcessInstanceRequest<crate::process_instance::migrate::Initial> {
        MigrateProcessInstanceRequest::<crate::process_instance::migrate::Initial>::new(
            self.clone(),
        )
    }

    /// Creates a `ModifyProcessInstanceRequest` to modify a running process
    /// instance in Zeebe.
    ///
    /// # Examples
    /// ```ignore
    /// client
    ///     .modify_process_instance()
    ///     .with_process_instance_key(12345)
    ///         .with_activate_instruction("element_id".to_string(), 67890)
    ///         .with_variable_instruction("scope_id".to_string(), serde_json::json!({"key": "value"}))?
    ///         .build()
    ///     .with_terminate_instruction(54321)
    ///     .with_operation_reference(98765)
    ///     .send()
    ///     .await?;
    /// ```
    pub fn modify_process_instance(
        &self,
    ) -> ModifyProcessInstanceRequest<crate::process_instance::modify::Initial> {
        ModifyProcessInstanceRequest::<crate::process_instance::modify::Initial>::new(self.clone())
    }

    /// Creates a `SetVariablesRequest` to update variables for a particular scope.
    ///
    /// # Examples
    /// ```ignore
    ///
    /// #[derive(Serialize)]
    /// struct Foo {
    ///     bar: String
    /// }
    ///
    /// client
    ///     .set_variables()
    ///     .with_element_instance_key(123456)
    ///     .with_variable(Foo {bar: String::from("foobar")})
    ///     .send()
    ///     .await?;
    /// ```
    pub fn set_variables(&self) -> SetVariablesRequest<crate::set_variables::Initial> {
        SetVariablesRequest::<crate::set_variables::Initial>::new(self.clone())
    }

    /// Creates a `PublishMessageRequest` to publish a message to a running
    /// process instance in Zeebe.
    ///
    /// # Examples
    /// ```ignore
    /// // Publish a message without a correlation key
    /// client
    ///     .publish_message()
    ///     .with_name(String::from("hello_world"))
    ///     .without_correlation_key()
    ///     .with_variables(HelloWorld {
    ///            hello: String::from("foo"),
    ///      })?
    ///      .send()
    ///      .await?;
    ///
    /// // Publish a message with a correlation key
    /// client
    ///     .publish_message()
    ///     .with_name(String::from("hello_message"))
    ///     .with_correlation_key(String::from("foo"))
    ///     .send()
    ///     .await?;
    /// ```
    pub fn publish_message(&self) -> PublishMessageRequest<crate::message::Initial> {
        PublishMessageRequest::<crate::message::Initial>::new(self.clone())
    }

    /// Creates a `BroadcastSignalRequest` to publish a signal to Zeebe.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// client
    ///     .broadcast_signal()
    ///     .with_signal_name(String::from("Hello_Signal"))
    ///     .send()
    ///     .await?;
    /// ```
    pub fn broadcast_signal(&self) -> BroadcastSignalRequest<crate::signal::Initial> {
        BroadcastSignalRequest::<crate::signal::Initial>::new(self.clone())
    }

    /// Creates a `ResolveIncidentRequest` to resolve an active incident in
    /// Zeebe.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// client
    ///     .resolve_incident()
    ///     .with_incident_key(123456)
    ///     .send()
    ///     .await?;
    /// ```
    pub fn resolve_incident(&self) -> ResolveIncidentRequest<crate::incident::Initial> {
        ResolveIncidentRequest::<crate::incident::Initial>::new(self.clone())
    }

    /// Creates a `ThrowErrorRequest` to throw a business error.
    ///
    /// # Examples
    /// ```ignore
    /// client
    ///     .throw_error()
    ///     .with_job_key(123456)
    ///     .with_error_code(String::from("error_code"))
    ///     .send()
    ///     .await?;
    /// ```
    pub fn throw_error(&self) -> ThrowErrorRequest<crate::throw_error::Initial> {
        ThrowErrorRequest::<crate::throw_error::Initial>::new(self.clone())
    }

    /// Creates a `EvaluateDecisionRequest` to request the evaluation of a DMN decision.
    ///
    /// # Examples
    /// ```ignore
    /// client
    ///     .evaluate_decision()
    ///     .with_decision_key(123456)
    ///     .with_decision_id(String::from("decision_id"))
    ///     .send()
    ///     .await?;
    /// ```
    pub fn evaluate_decision(&self) -> EvaluateDecisionRequest<crate::decision::Initial> {
        EvaluateDecisionRequest::<crate::decision::Initial>::new(self.clone())
    }

    /// Creates a `CompleteJobRequest` to complete a job for a process instance
    /// in Zeebe.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// client
    ///     .complete_job()
    ///     .with_job_key(123456)
    ///     .send()
    ///     .await?;
    /// ```
    pub fn complete_job(&self) -> CompleteJobRequest<crate::job::complete::Initial> {
        CompleteJobRequest::<crate::job::complete::Initial>::new(self.clone())
    }

    /// Creates a `FailJobRequest` to fail a job for a process instance in Zeebe.
    ///
    /// # Examples
    /// ```ignore
    /// client
    ///     .fail_job()
    ///     .with_job_key(123456)
    ///     .send()
    ///     .await?;
    /// ```
    pub fn fail_job(&self) -> FailJobRequest<crate::job::fail::Initial> {
        FailJobRequest::<crate::job::fail::Initial>::new(self.clone())
    }

    /// Creates a `UpdateJobTimeoutRequest` to update the timeout for a running
    /// job in Zeebe.
    ///
    /// # Examples
    /// ```ignore
    /// client
    ///     .update_job_timeout()
    ///     .with_job_key(123456)
    ///     .with_timeout(Duration::from_secs(10))
    ///     .send()
    ///     .await?;
    /// ```
    pub fn update_job_timeout(
        &self,
    ) -> UpdateJobTimeoutRequest<crate::job::update_timeout::Initial> {
        UpdateJobTimeoutRequest::<crate::job::update_timeout::Initial>::new(self.clone())
    }

    /// Creates a `UpdateJobRetriesRequest` that updates the number of retries
    /// for a job in Zeebe.
    ///
    ///
    /// # Examples
    /// ```ignore
    /// client
    ///     .update_job_retries()
    ///     .with_job_key(123456)
    ///     .with_retries(1)
    ///     .send()
    ///     .await?;
    /// ```
    pub fn update_job_retries(
        &self,
    ) -> UpdateJobRetriesRequest<crate::job::update_retries::Initial> {
        UpdateJobRetriesRequest::<crate::job::update_retries::Initial>::new(self.clone())
    }

    /// Creates a `WorkerBuilder` to build a worker for processing Zeebe jobs.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// client
    ///     .worker()
    ///     .with_job_type(String::from("example-service"))
    ///     .with_job_timeout(Duration::from_secs(5 * 60))
    ///     .with_request_timeout(Duration::from_secs(10))
    ///     .with_max_jobs_to_activate(4)
    ///     .with_concurrency_limit(2)
    ///     .with_handler(|client, job| async move {
    ///        let _ = client.complete_job().with_job_key(job.key()).send().await;
    ///    })
    ///    .build()
    ///    .run()
    ///    .await;
    /// ```
    pub fn worker(&self) -> WorkerBuilder<crate::worker::Initial> {
        WorkerBuilder::new(self.clone())
    }
}
