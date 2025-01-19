use crate::{proto, Client, ClientError};
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SignalError {
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
}

pub struct Initial;
pub struct WithName;
pub trait BroadcastSignalRequestState {}
impl BroadcastSignalRequestState for Initial {}
impl BroadcastSignalRequestState for WithName {}

#[derive(Debug, Clone)]
pub struct BroadcastSignalRequest<T: BroadcastSignalRequestState> {
    client: Client,
    signal_name: String,
    variables: serde_json::Value,
    tenant_id: String,
    _state: std::marker::PhantomData<T>,
}

impl<T: BroadcastSignalRequestState> BroadcastSignalRequest<T> {
    pub fn new(client: Client) -> BroadcastSignalRequest<Initial> {
        BroadcastSignalRequest {
            client,
            signal_name: String::new(),
            variables: serde_json::Value::default(),
            tenant_id: String::new(),
            _state: std::marker::PhantomData,
        }
    }

    fn transition<NewState: BroadcastSignalRequestState>(self) -> BroadcastSignalRequest<NewState> {
        BroadcastSignalRequest {
            client: self.client,
            signal_name: self.signal_name,
            variables: self.variables,
            tenant_id: self.tenant_id,
            _state: std::marker::PhantomData,
        }
    }
}

impl BroadcastSignalRequest<Initial> {
    pub fn with_signal_name(mut self, signal_name: String) -> BroadcastSignalRequest<WithName> {
        self.signal_name = signal_name;
        self.transition()
    }
}

impl BroadcastSignalRequest<WithName> {
    pub fn with_variables<T: Serialize>(mut self, data: T) -> Result<Self, SignalError> {
        self.variables = serde_json::to_value(data)?;
        Ok(self)
    }

    pub fn with_tenant_id(mut self, tenant_id: String) -> Self {
        self.tenant_id = tenant_id;
        self
    }

    pub async fn send(mut self) -> Result<BroadcastSignalResponse, ClientError> {
        let res = self
            .client
            .gateway_client
            .broadcast_signal(proto::BroadcastSignalRequest {
                signal_name: self.signal_name,
                variables: self.variables.to_string(),
                tenant_id: self.tenant_id,
            })
            .await?;

        Ok(res.into_inner().into())
    }
}

#[derive(Debug, Clone)]
pub struct BroadcastSignalResponse {
    key: i64,
    tenant_id: String,
}

impl From<proto::BroadcastSignalResponse> for BroadcastSignalResponse {
    fn from(value: proto::BroadcastSignalResponse) -> BroadcastSignalResponse {
        BroadcastSignalResponse {
            key: value.key,
            tenant_id: value.tenant_id,
        }
    }
}

impl BroadcastSignalResponse {
    pub fn key(&self) -> i64 {
        self.key
    }

    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }
}
