use crate::proto;
use crate::Client;
use crate::ClientError;
use serde::de::DeserializeOwned;
use serde::Serialize;

const LATEST_VERSION: i32 = -1;

#[derive(Debug, Clone)]
pub struct Initial;

#[derive(Debug, Clone)]
pub struct WithProcess;

#[derive(Debug, Clone)]
pub struct WithVariables;

#[derive(Debug, Clone)]
pub struct WithResult;

pub trait CreateProcessInstanceState {}
impl CreateProcessInstanceState for Initial {}
impl CreateProcessInstanceState for WithProcess {}
impl CreateProcessInstanceState for WithVariables {}
impl CreateProcessInstanceState for WithResult {}

/// Request to create a process instance in Zeebe
///
/// This builder-like struct allows you to configure and send a request to create a new process instance
/// in the Zeebe workflow engine. The request goes through several states to ensure all required parameters
/// are set before sending.
///
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
#[derive(Debug, Clone)]
pub struct CreateProcessInstanceRequest<T: CreateProcessInstanceState> {
    client: Client,
    process_definition_key: Option<i64>,
    bpmn_process_id: Option<String>,
    version: Option<i32>,
    input: Option<serde_json::Value>,
    start_instructions: Vec<String>,
    tenant_id: String,
    operation_reference: Option<u64>,
    fetch_variables: Option<Vec<String>>,
    request_timeout: i64,
    _state: std::marker::PhantomData<T>,
}

impl<T: CreateProcessInstanceState> CreateProcessInstanceRequest<T> {
    pub(crate) fn new(client: Client) -> CreateProcessInstanceRequest<Initial> {
        CreateProcessInstanceRequest {
            client,
            process_definition_key: None,
            bpmn_process_id: None,
            version: None,
            input: None,
            start_instructions: vec![],
            tenant_id: String::default(),
            operation_reference: None,
            fetch_variables: None,
            request_timeout: 0,
            _state: std::marker::PhantomData,
        }
    }

    fn transition<NewState: CreateProcessInstanceState>(
        self,
    ) -> CreateProcessInstanceRequest<NewState> {
        CreateProcessInstanceRequest {
            client: self.client,
            process_definition_key: self.process_definition_key,
            bpmn_process_id: self.bpmn_process_id,
            version: self.version,
            input: self.input,
            start_instructions: self.start_instructions,
            tenant_id: self.tenant_id,
            operation_reference: self.operation_reference,
            fetch_variables: self.fetch_variables,
            request_timeout: self.request_timeout,
            _state: std::marker::PhantomData,
        }
    }
}

impl CreateProcessInstanceRequest<Initial> {
    /// Sets the BPMN process ID to identify in which process to instantiate.
    ///
    /// # Arguments
    ///
    /// * `bpmn_process_id` - The BPMN process ID of the instance to create.
    ///
    /// # Returns
    ///
    /// A `CreateProcessInstanceRequest<WithProcess>` to continue the request building.
    pub fn with_bpmn_process_id(
        mut self,
        bpmn_process_id: String,
    ) -> CreateProcessInstanceRequest<WithProcess> {
        self.bpmn_process_id = Some(bpmn_process_id);
        self.transition()
    }

    /// Sets the process definition key to identify in which process to instantiate.
    ///
    /// # Arguments
    ///
    /// * `process_definition_key` - The unique key identifying the process definition.
    ///
    /// # Returns
    ///
    /// A `CreateProcessInstanceRequest<WithProcess>` to continue the request building.
    pub fn with_process_definition_key(
        mut self,
        process_definition_key: i64,
    ) -> CreateProcessInstanceRequest<WithProcess> {
        self.process_definition_key = Some(process_definition_key);
        self.transition()
    }
}

impl CreateProcessInstanceRequest<WithProcess> {
    /// Sets the variables to instantiate the process with.
    ///
    /// # Arguments
    ///
    /// * `variables` - Variables that will be used as instance payload.
    ///
    /// # Errors
    ///
    /// Returns `ClientError` if variables cannot be serialized to JSON.
    ///
    /// # Returns
    ///
    /// A `Result` containing either a `CreateProcessInstanceRequest<WithVariables>` if successful, or a `ClientError` if serialization fails.
    pub fn with_variables<T: Serialize>(
        mut self,
        variables: T,
    ) -> Result<CreateProcessInstanceRequest<WithVariables>, ClientError> {
        self.input = Some(
            serde_json::to_value(variables)
                .map_err(|e| ClientError::SerializationFailed { source: e })?,
        );
        Ok(self.transition())
    }

    /// Creates the process instance without any input variables
    ///
    /// # Returns
    ///
    /// A `CreateProcessInstanceRequest<WithVariables>` to continue the request building.
    pub fn without_input(self) -> CreateProcessInstanceRequest<WithVariables> {
        self.transition()
    }
}

impl CreateProcessInstanceRequest<WithVariables> {
    /// Sends the process instance creation request to the Zeebe workflow engine.
    ///
    /// # Errors
    ///
    /// Returns `ClientError` if the request fails.
    pub async fn send(mut self) -> Result<CreateProcessInstanceResponse, ClientError> {
        let res = self
            .client
            .gateway_client
            .create_process_instance(proto::CreateProcessInstanceRequest {
                process_definition_key: self.process_definition_key.unwrap_or(0),
                bpmn_process_id: self.bpmn_process_id.unwrap_or_default(),
                version: self.version.unwrap_or(LATEST_VERSION),
                variables: self
                    .input
                    .map_or(String::new(), |variables| variables.to_string()),
                start_instructions: self
                    .start_instructions
                    .into_iter()
                    .map(|s| proto::ProcessInstanceCreationStartInstruction { element_id: s })
                    .collect(),
                tenant_id: self.tenant_id,
                operation_reference: self.operation_reference,
            })
            .await?;

        Ok(res.into_inner().into())
    }

    /// Configures which variables to fetch when the process completes
    ///
    /// # Arguments
    /// * `fetch_variables` - Optional list of variable names to fetch, or None to fetch all
    ///
    /// # Returns
    ///
    /// A `CreateProcessInstanceRequest<WithResult>` to continue the request building.
    pub fn with_result(
        mut self,
        fetch_variables: Option<Vec<String>>,
    ) -> CreateProcessInstanceRequest<WithResult> {
        self.fetch_variables = fetch_variables;
        self.transition()
    }

    /// Sets the version of the process definition to use.
    ///
    /// Use `-1` to select the latest deployed version.
    ///
    /// # Arguments
    ///
    /// * `version` - The version of the process definition.
    ///
    /// # Returns
    ///
    /// A `CreateProcessInstanceRequest<WithVariables>` to continue the request building.
    pub fn with_version(mut self, version: i32) -> Self {
        self.version = Some(version);
        self
    }

    /// Sets the tenant ID for the process instance.
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - The tenant ID.
    ///
    /// # Returns
    ///
    /// A `CreateProcessInstanceRequest<WithVariables>` to continue the request building.
    pub fn with_tenant_id(mut self, tenant_id: String) -> Self {
        self.tenant_id = tenant_id;
        self
    }

    /// Sets a reference key for tracking this operation.
    ///
    /// # Arguments
    ///
    /// * `operation_reference` - The reference key.
    ///
    /// # Returns
    ///
    /// A `CreateProcessInstanceRequest<WithVariables>` to continue the request building.
    pub fn with_operation_reference(mut self, operation_reference: u64) -> Self {
        self.operation_reference = Some(operation_reference);
        self
    }
}

impl CreateProcessInstanceRequest<WithResult> {
    async fn send(mut self) -> Result<proto::CreateProcessInstanceWithResultResponse, ClientError> {
        let res = self
            .client
            .gateway_client
            .create_process_instance_with_result(proto::CreateProcessInstanceWithResultRequest {
                request: Some(proto::CreateProcessInstanceRequest {
                    process_definition_key: self.process_definition_key.unwrap_or(0),
                    bpmn_process_id: self.bpmn_process_id.unwrap_or_default(),
                    version: self.version.unwrap_or(LATEST_VERSION),
                    variables: self
                        .input
                        .map_or(String::new(), |variables| variables.to_string()),
                    start_instructions: self
                        .start_instructions
                        .into_iter()
                        .map(|s| proto::ProcessInstanceCreationStartInstruction { element_id: s })
                        .collect(),
                    tenant_id: self.tenant_id,
                    operation_reference: self.operation_reference,
                }),
                request_timeout: self.request_timeout,
                fetch_variables: self.fetch_variables.unwrap_or_default(),
            })
            .await?;

        Ok(res.into_inner())
    }

    /// Sends the request and returns serialized result variables as JSON.
    ///
    /// # Errors
    ///
    /// Returns `ClientError` if the request fails.
    pub async fn send_with_serialized_result(
        self,
    ) -> Result<CreateProcessInstanceWithResultSerialized, ClientError> {
        let res = self.send().await?;
        Ok(res.into())
    }

    /// Sends the request and deserializes result variables into the specified type.
    ///
    /// # Errors
    ///
    /// Returns `ClientError` if the request fails or deserialization fails.
    pub async fn send_with_result<T: DeserializeOwned>(
        self,
    ) -> Result<CreateProcessInstanceWithResult<T>, ClientError> {
        let res = self.send().await?;
        res.try_into()
    }
}

/// Response from creating a process instance
#[derive(Debug, Clone)]
pub struct CreateProcessInstanceResponse {
    process_definition_key: i64,
    bpmn_process_id: String,
    version: i32,
    process_instance_key: i64,
    tenant_id: String,
}

impl CreateProcessInstanceResponse {
    /// Returns the unique key identifying the process definition.
    ///
    /// # Returns
    ///
    /// The unique key as an `i64`.
    pub fn process_definition_key(&self) -> i64 {
        self.process_definition_key
    }

    /// Returns the BPMN process ID of the created instance.
    ///
    /// # Returns
    ///
    /// A string slice representing the BPMN process ID.
    pub fn bpmn_process_id(&self) -> &str {
        &self.bpmn_process_id
    }

    /// Returns the version of the process that was used.
    ///
    /// # Returns
    ///
    /// The version as an `i32`.
    pub fn version(&self) -> i32 {
        self.version
    }

    /// Returns the unique key identifying the created process instance.
    ///
    /// # Returns
    ///
    /// The unique key as an `i64`.
    pub fn process_instance_key(&self) -> i64 {
        self.process_instance_key
    }

    /// Returns the tenant ID of the process instance.
    ///
    /// # Returns
    ///
    /// A string slice representing the tenant ID.
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }
}

impl From<proto::CreateProcessInstanceResponse> for CreateProcessInstanceResponse {
    fn from(value: proto::CreateProcessInstanceResponse) -> CreateProcessInstanceResponse {
        CreateProcessInstanceResponse {
            process_definition_key: value.process_definition_key,
            bpmn_process_id: value.bpmn_process_id,
            version: value.version,
            process_instance_key: value.process_instance_key,
            tenant_id: value.tenant_id,
        }
    }
}

/// Response from creating a process instance with serialized result variables
#[derive(Debug, Clone)]
pub struct CreateProcessInstanceWithResultSerialized {
    response: CreateProcessInstanceResponse,
    variables: String,
}

/// A response type for process instance creation with serialized variables
impl CreateProcessInstanceWithResultSerialized {
    /// Returns a reference to the underlying process instance creation response
    /// containing basic information about the created process instance such as
    /// process definition key, version, and instance key.
    ///
    /// # Returns
    /// * `&CreateProcessInstanceResponse` - A reference to the base process instance response
    pub fn response(&self) -> &CreateProcessInstanceResponse {
        &self.response
    }

    /// Returns the process instance result variables as a JSON-formatted string.
    /// These variables represent the final state of the process instance after completion.
    ///
    /// # Returns
    /// * `&str` - A reference to the JSON string containing the result variables
    pub fn variables(&self) -> &str {
        &self.variables
    }
}

impl From<proto::CreateProcessInstanceWithResultResponse>
    for CreateProcessInstanceWithResultSerialized
{
    fn from(
        value: proto::CreateProcessInstanceWithResultResponse,
    ) -> CreateProcessInstanceWithResultSerialized {
        CreateProcessInstanceWithResultSerialized {
            response: CreateProcessInstanceResponse {
                process_definition_key: value.process_definition_key,
                bpmn_process_id: value.bpmn_process_id,
                version: value.version,
                process_instance_key: value.process_instance_key,
                tenant_id: value.tenant_id,
            },
            variables: value.variables,
        }
    }
}

/// Response from creating a process instance with deserialized result variables
///
/// # Type Parameters
/// - `T`: The type of the deserialized result variables, which must implement `DeserializeOwned`.
#[derive(Debug, Clone)]
pub struct CreateProcessInstanceWithResult<T: DeserializeOwned> {
    response: CreateProcessInstanceResponse,
    data: T,
}

impl<T: DeserializeOwned> CreateProcessInstanceWithResult<T> {
    /// Returns a reference to the base process instance creation response.
    ///
    /// # Returns
    /// A reference to a `CreateProcessInstanceResponse` containing the details of the process instance creation.
    pub fn response(&self) -> &CreateProcessInstanceResponse {
        &self.response
    }

    /// Returns a reference to the deserialized result variables.
    ///
    /// # Returns
    /// A reference to the deserialized result variables of type `T`.
    pub fn data(&self) -> &T {
        &self.data
    }
}

impl<T: DeserializeOwned> TryFrom<proto::CreateProcessInstanceWithResultResponse>
    for CreateProcessInstanceWithResult<T>
{
    type Error = ClientError;
    fn try_from(
        value: proto::CreateProcessInstanceWithResultResponse,
    ) -> Result<CreateProcessInstanceWithResult<T>, Self::Error> {
        Ok(CreateProcessInstanceWithResult {
            response: CreateProcessInstanceResponse {
                process_definition_key: value.process_definition_key,
                bpmn_process_id: value.bpmn_process_id,
                version: value.version,
                process_instance_key: value.process_instance_key,
                tenant_id: value.tenant_id,
            },
            data: serde_json::from_str(&value.variables).map_err(|e| {
                ClientError::DeserializationFailed {
                    value: value.variables.clone(),
                    source: e,
                }
            })?,
        })
    }
}
