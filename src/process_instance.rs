use crate::proto;
use crate::Client;
use crate::ClientError;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::json;
use thiserror::Error;

const LATEST_VERSION: i32 = -1;

#[derive(Error, Debug)]
pub enum ProcessInstanceError {
    #[error("failed to deserialize json")]
    DeserializeFailed(#[from] serde_json::Error),
}

pub struct Empty;
pub struct WithProcess;
pub struct WithVariables;
pub struct WithResult;
pub trait CreateProcessInstanceState {}

impl CreateProcessInstanceState for Empty {}
impl CreateProcessInstanceState for WithProcess {}
impl CreateProcessInstanceState for WithVariables {}
impl CreateProcessInstanceState for WithResult {}

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
    pub fn new(client: Client) -> CreateProcessInstanceRequest<Empty> {
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

    pub fn with_version(mut self, version: i32) -> Self {
        self.version = Some(version);
        self
    }

    pub fn with_tenant_id(mut self, tenant_id: String) -> Self {
        self.tenant_id = tenant_id;
        self
    }

    pub fn with_operation_reference(mut self, operation_reference: u64) -> Self {
        self.operation_reference = Some(operation_reference);
        self
    }
}

impl CreateProcessInstanceRequest<Empty> {
    pub fn with_bpmn_process_id(
        mut self,
        bpmn_process_id: String,
    ) -> CreateProcessInstanceRequest<WithProcess> {
        self.bpmn_process_id = Some(bpmn_process_id);
        self.transition()
    }
    pub fn with_process_definition_key(
        mut self,
        process_definition_key: i64,
    ) -> CreateProcessInstanceRequest<WithProcess> {
        self.process_definition_key = Some(process_definition_key);
        self.transition()
    }
}

impl CreateProcessInstanceRequest<WithProcess> {
    pub fn with_json<T: Into<serde_json::Value>>(
        mut self,
        variables: T,
    ) -> CreateProcessInstanceRequest<WithVariables> {
        self.input = Some(variables.into());
        self.transition()
    }
    pub fn with_input<T: Serialize>(
        mut self,
        variables: T,
    ) -> CreateProcessInstanceRequest<WithVariables> {
        self.input = Some(json!(variables));
        self.transition()
    }

    pub fn without_input(self) -> CreateProcessInstanceRequest<WithVariables> {
        self.transition()
    }
}

impl CreateProcessInstanceRequest<WithVariables> {
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

    pub fn with_result(
        mut self,
        fetch_variables: Option<Vec<String>>,
    ) -> CreateProcessInstanceRequest<WithResult> {
        self.fetch_variables = fetch_variables;
        self.transition()
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

    pub async fn send_with_serialized_result(
        self,
    ) -> Result<CreateProcessInstanceWithResultSerialized, ClientError> {
        let res = self.send().await?;
        Ok(res.into())
    }

    pub async fn send_with_result<T: DeserializeOwned>(
        self,
    ) -> Result<CreateProcessInstanceWithResult<T>, ClientError> {
        let res = self.send().await?;
        Ok(res.try_into()?)
    }
}

#[derive(Debug, Clone)]
pub struct CreateProcessInstanceResponse {
    process_definition_key: i64,
    bpmn_process_id: String,
    version: i32,
    process_instance_key: i64,
    tenant_id: String,
}

impl CreateProcessInstanceResponse {
    pub fn process_definition_key(&self) -> i64 {
        self.process_definition_key
    }

    pub fn bpmn_process_id(&self) -> &str {
        &self.bpmn_process_id
    }

    pub fn version(&self) -> i32 {
        self.version
    }

    pub fn process_instance_key(&self) -> i64 {
        self.process_instance_key
    }

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

#[derive(Debug, Clone)]
pub struct CreateProcessInstanceWithResultSerialized {
    response: CreateProcessInstanceResponse,
    variables: String,
}

impl CreateProcessInstanceWithResultSerialized {
    pub fn response(&self) -> &CreateProcessInstanceResponse {
        &self.response
    }

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

#[derive(Debug, Clone)]
pub struct CreateProcessInstanceWithResult<T>
where
    T: DeserializeOwned,
{
    response: CreateProcessInstanceResponse,
    data: T,
}

impl<T: DeserializeOwned> CreateProcessInstanceWithResult<T> {
    pub fn response(&self) -> &CreateProcessInstanceResponse {
        &self.response
    }

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
