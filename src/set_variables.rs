use crate::{proto, Client, ClientError};
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SetVariablesError {
    #[error("failed to deserialize json")]
    DeserializeFailed(#[from] serde_json::Error),
}

pub struct Initial;
pub struct WithInstanceKey;
pub struct WithVariables;

pub trait SetVariablesRequestState {}
impl SetVariablesRequestState for Initial {}
impl SetVariablesRequestState for WithInstanceKey {}
impl SetVariablesRequestState for WithVariables {}

#[derive(Debug, Clone)]
pub struct SetVariablesRequest<T: SetVariablesRequestState> {
    client: Client,
    element_instance_key: i64,
    variables: serde_json::Value,
    local: bool,
    operation_reference: Option<u64>,
    _state: std::marker::PhantomData<T>,
}

impl<T: SetVariablesRequestState> SetVariablesRequest<T> {
    pub(crate) fn new(client: Client) -> SetVariablesRequest<Initial> {
        SetVariablesRequest {
            client,
            element_instance_key: 0,
            variables: serde_json::Value::default(),
            local: false,
            operation_reference: None,
            _state: std::marker::PhantomData,
        }
    }

    fn transition<NewState: SetVariablesRequestState>(self) -> SetVariablesRequest<NewState> {
        SetVariablesRequest {
            client: self.client,
            element_instance_key: self.element_instance_key,
            variables: self.variables,
            local: self.local,
            operation_reference: self.operation_reference,
            _state: std::marker::PhantomData,
        }
    }

    pub fn with_operation_reference(mut self, operation_reference: u64) -> Self {
        self.operation_reference = Some(operation_reference);
        self
    }

    pub fn set_local_scope(mut self, is_local_scope: bool) -> Self {
        self.local = is_local_scope;
        self
    }
}

impl SetVariablesRequest<Initial> {
    pub fn with_element_instance_key(
        mut self,
        element_instance_key: i64,
    ) -> SetVariablesRequest<WithInstanceKey> {
        self.element_instance_key = element_instance_key;
        self.transition()
    }
}

impl SetVariablesRequest<WithInstanceKey> {
    pub fn with_variable<T: Serialize>(
        mut self,
        data: T,
    ) -> Result<SetVariablesRequest<WithVariables>, SetVariablesError> {
        self.variables = serde_json::to_value(data)?;
        Ok(self.transition())
    }
}

impl SetVariablesRequest<WithVariables> {
    pub async fn send(mut self) -> Result<SetVariablesResponse, ClientError> {
        let res = self
            .client
            .gateway_client
            .set_variables(proto::SetVariablesRequest {
                element_instance_key: self.element_instance_key,
                variables: self.variables.to_string(),
                local: self.local,
                operation_reference: self.operation_reference,
            })
            .await?;

        Ok(res.into_inner().into())
    }
}

#[derive(Debug, Clone)]
pub struct SetVariablesResponse {
    key: i64,
}

impl From<proto::SetVariablesResponse> for SetVariablesResponse {
    fn from(value: proto::SetVariablesResponse) -> SetVariablesResponse {
        SetVariablesResponse { key: value.key }
    }
}

impl SetVariablesResponse {
    pub fn key(&self) -> i64 {
        self.key
    }
}
