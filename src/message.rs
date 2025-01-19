use crate::{proto, Client, ClientError};
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MessageError {
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
}

pub struct Initial;
pub struct WithName;
pub struct WithKey;
pub trait PublishMessageRequestState {}
impl PublishMessageRequestState for Initial {}
impl PublishMessageRequestState for WithName {}
impl PublishMessageRequestState for WithKey {}

#[derive(Debug, Clone)]
pub struct PublishMessageRequest<T: PublishMessageRequestState> {
    client: Client,
    name: String,
    correlation_key: String,
    time_to_live: i64,
    message_id: String,
    variables: serde_json::Value,
    tenant_id: String,
    _state: std::marker::PhantomData<T>,
}

impl<T: PublishMessageRequestState> PublishMessageRequest<T> {
    pub fn new(client: Client) -> PublishMessageRequest<Initial> {
        PublishMessageRequest {
            client,
            name: String::new(),
            correlation_key: String::new(),
            time_to_live: 0,
            message_id: String::new(),
            variables: serde_json::Value::default(),
            tenant_id: String::new(),
            _state: std::marker::PhantomData,
        }
    }

    fn transition<NewState: PublishMessageRequestState>(self) -> PublishMessageRequest<NewState> {
        PublishMessageRequest {
            client: self.client,
            name: self.name,
            correlation_key: self.correlation_key,
            time_to_live: self.time_to_live,
            message_id: self.message_id,
            variables: self.variables,
            tenant_id: self.tenant_id,
            _state: std::marker::PhantomData,
        }
    }
}

impl PublishMessageRequest<Initial> {
    pub fn with_name(mut self, name: String) -> PublishMessageRequest<WithName> {
        self.name = name;
        self.transition()
    }
}

impl PublishMessageRequest<WithName> {
    pub fn with_correlation_key(
        mut self,
        correlation_key: String,
    ) -> PublishMessageRequest<WithKey> {
        self.correlation_key = correlation_key;
        self.transition()
    }
}

impl PublishMessageRequest<WithKey> {
    pub fn with_time_to_live(mut self, ttl_sec: i64) -> Self {
        self.time_to_live = ttl_sec * 1000;
        self
    }

    pub fn with_message_id(mut self, message_id: String) -> Self {
        self.message_id = message_id;
        self
    }

    pub fn with_variables<T: Serialize>(mut self, data: T) -> Result<Self, MessageError> {
        self.variables = serde_json::to_value(data)?;
        Ok(self)
    }

    pub fn with_tenant_id(mut self, tenant_id: String) -> Self {
        self.tenant_id = tenant_id;
        self
    }

    pub async fn send(mut self) -> Result<PublishMessageResponse, ClientError> {
        let res = self
            .client
            .gateway_client
            .publish_message(proto::PublishMessageRequest {
                name: self.name,
                correlation_key: self.correlation_key,
                time_to_live: self.time_to_live,
                message_id: self.message_id,
                variables: self.variables.to_string(),
                tenant_id: self.tenant_id,
            })
            .await?;

        Ok(res.into_inner().into())
    }
}

#[derive(Debug, Clone)]
pub struct PublishMessageResponse {
    key: i64,
    tenant_id: String,
}

impl From<proto::PublishMessageResponse> for PublishMessageResponse {
    fn from(value: proto::PublishMessageResponse) -> PublishMessageResponse {
        PublishMessageResponse {
            key: value.key,
            tenant_id: value.tenant_id,
        }
    }
}

impl PublishMessageResponse {
    pub fn key(&self) -> i64 {
        self.key
    }

    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }
}
