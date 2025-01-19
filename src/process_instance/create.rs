use crate::process_instance::ProcessInstanceError;
use crate::proto;
use crate::Client;
use crate::ClientError;
use serde::de::DeserializeOwned;
use serde::Serialize;

/// Version constant indicating latest deployed version should be used
const LATEST_VERSION: i32 = -1;

/// Initial state for the CreateProcessInstanceRequest builder pattern
#[derive(Debug, Clone)]
pub struct Initial;

/// State indicating process definition has been set
#[derive(Debug, Clone)]
pub struct WithProcess;

/// State indicating variables have been set
#[derive(Debug, Clone)]
pub struct WithVariables;

/// State indicating result handling has been configured
#[derive(Debug, Clone)]
pub struct WithResult;

/// Marker trait for CreateProcessInstanceRequest states
pub trait CreateProcessInstanceState {}
impl CreateProcessInstanceState for Initial {}
impl CreateProcessInstanceState for WithProcess {}
impl CreateProcessInstanceState for WithVariables {}
impl CreateProcessInstanceState for WithResult {}

/// Request to create and start a new process instance
///
/// Creates and starts an instance of the specified process. The process definition
/// can be specified either using its unique key or using the BPMN process ID and version.
/// Pass -1 as the version to use the latest deployed version.
#[derive(Debug, Clone)]
pub struct CreateProcessInstanceRequest<T: CreateProcessInstanceState> {
    client: Client,
    /// The unique key identifying the process definition
    process_definition_key: Option<i64>,
    /// The BPMN process ID of the process definition
    bpmn_process_id: Option<String>,
    /// The version of the process definition (-1 for latest)
    version: Option<i32>,
    /// Variables to instantiate the process with
    input: Option<serde_json::Value>,
    /// List of start instructions
    start_instructions: Vec<String>,
    /// The tenant ID of the process
    tenant_id: String,
    /// Reference key for tracking this operation
    operation_reference: Option<u64>,
    /// Variables to fetch from completed instance
    fetch_variables: Option<Vec<String>>,
    /// Request timeout in milliseconds
    request_timeout: i64,
    _state: std::marker::PhantomData<T>,
}

impl<T: CreateProcessInstanceState> CreateProcessInstanceRequest<T> {
    /// Creates a new CreateProcessInstanceRequest in its initial state
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

    /// Internal helper to transition between builder states
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

    /// Sets the version of the process definition to use
    ///
    /// Use -1 to select the latest deployed version
    pub fn with_version(mut self, version: i32) -> Self {
        self.version = Some(version);
        self
    }

    /// Sets the tenant ID for the process instance
    pub fn with_tenant_id(mut self, tenant_id: String) -> Self {
        self.tenant_id = tenant_id;
        self
    }

    /// Sets a reference key for tracking this operation
    pub fn with_operation_reference(mut self, operation_reference: u64) -> Self {
        self.operation_reference = Some(operation_reference);
        self
    }
}

impl CreateProcessInstanceRequest<Initial> {
    /// Sets the BPMN process ID to identify which process to instantiate
    ///
    /// # Arguments
    /// * `bpmn_process_id` - The BPMN process ID of the process to create
    pub fn with_bpmn_process_id(
        mut self,
        bpmn_process_id: String,
    ) -> CreateProcessInstanceRequest<WithProcess> {
        self.bpmn_process_id = Some(bpmn_process_id);
        self.transition()
    }

    /// Sets the process definition key to identify which process to instantiate
    ///
    /// # Arguments
    /// * `process_definition_key` - The unique key identifying the process definition
    pub fn with_process_definition_key(
        mut self,
        process_definition_key: i64,
    ) -> CreateProcessInstanceRequest<WithProcess> {
        self.process_definition_key = Some(process_definition_key);
        self.transition()
    }
}

impl CreateProcessInstanceRequest<WithProcess> {
    /// Sets the variables to instantiate the process with
    ///
    /// # Arguments
    /// * `variables` - Variables that will be used as instance payload
    ///
    /// # Errors
    /// Returns ProcessInstanceError if variables cannot be serialized to JSON
    pub fn with_variables<T: Serialize>(
        mut self,
        variables: T,
    ) -> Result<CreateProcessInstanceRequest<WithVariables>, ProcessInstanceError> {
        self.input = Some(serde_json::to_value(variables)?);
        Ok(self.transition())
    }

    /// Creates the process instance without any input variables
    pub fn without_input(self) -> CreateProcessInstanceRequest<WithVariables> {
        self.transition()
    }
}

impl CreateProcessInstanceRequest<WithVariables> {
    /// Sends the process instance creation request to the Zeebe workflow engine
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
    pub fn with_result(
        mut self,
        fetch_variables: Option<Vec<String>>,
    ) -> CreateProcessInstanceRequest<WithResult> {
        self.fetch_variables = fetch_variables;
        self.transition()
    }
}

impl CreateProcessInstanceRequest<WithResult> {
    /// Internal helper to send request and get raw protobuf response
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

    /// Sends the request and returns serialized result variables as JSON
    pub async fn send_with_serialized_result(
        self,
    ) -> Result<CreateProcessInstanceWithResultSerialized, ClientError> {
        let res = self.send().await?;
        Ok(res.into())
    }

    /// Sends the request and deserializes result variables into the specified type
    pub async fn send_with_result<T: DeserializeOwned>(
        self,
    ) -> Result<CreateProcessInstanceWithResult<T>, ClientError> {
        let res = self.send().await?;
        Ok(res.try_into()?)
    }
}

/// Response from creating a process instance
#[derive(Debug, Clone)]
pub struct CreateProcessInstanceResponse {
    /// The unique key identifying the process definition
    process_definition_key: i64,
    /// The BPMN process ID of the created instance
    bpmn_process_id: String,
    /// The version of the process that was used
    version: i32,
    /// The unique key identifying the created process instance
    process_instance_key: i64,
    /// The tenant ID of the process instance
    tenant_id: String,
}

impl CreateProcessInstanceResponse {
    /// Returns the unique key identifying the process definition
    pub fn process_definition_key(&self) -> i64 {
        self.process_definition_key
    }

    /// Returns the BPMN process ID of the created instance
    pub fn bpmn_process_id(&self) -> &str {
        &self.bpmn_process_id
    }

    /// Returns the version of the process that was used
    pub fn version(&self) -> i32 {
        self.version
    }

    /// Returns the unique key identifying the created process instance
    pub fn process_instance_key(&self) -> i64 {
        self.process_instance_key
    }

    /// Returns the tenant ID of the process instance
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
    /// The base response information
    response: CreateProcessInstanceResponse,
    /// Result variables as JSON string
    variables: String,
}

impl CreateProcessInstanceWithResultSerialized {
    /// Returns the base process instance creation response
    pub fn response(&self) -> &CreateProcessInstanceResponse {
        &self.response
    }

    /// Returns the result variables as a JSON string
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
#[derive(Debug, Clone)]
pub struct CreateProcessInstanceWithResult<T: DeserializeOwned> {
    /// The base response information
    response: CreateProcessInstanceResponse,
    /// Result variables deserialized into the specified type
    data: T,
}

impl<T: DeserializeOwned> CreateProcessInstanceWithResult<T> {
    /// Returns the base process instance creation response
    pub fn response(&self) -> &CreateProcessInstanceResponse {
        &self.response
    }

    /// Returns the deserialized result variables
    pub fn data(&self) -> &T {
        &self.data
    }
}

impl<T: DeserializeOwned> TryFrom<proto::CreateProcessInstanceWithResultResponse>
    for CreateProcessInstanceWithResult<T>
{
    type Error = ProcessInstanceError;
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
            data: serde_json::from_str(&value.variables)?,
        })
    }
}
