use crate::{proto, Client, ClientError};
use serde::Serialize;
use thiserror::Error;

/// Error types that can occur when publishing messages
#[derive(Error, Debug)]
pub enum MessageError {
    /// Error that occurs during JSON serialization/deserialization
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
}

/// Initial state for message request builder
pub struct Initial;
/// State after message name has been set
pub struct WithName;
/// State after correlation key has been set
pub struct WithKey;

/// Trait marking valid states for PublishMessageRequest
pub trait PublishMessageRequestState {}
impl PublishMessageRequestState for Initial {}
impl PublishMessageRequestState for WithName {}
impl PublishMessageRequestState for WithKey {}

/// Request builder for publishing messages to specific partitions.
///
/// Required fields must be set in this order:
/// 1. Message name
/// 2. Correlation key
///
/// Optional fields can be set after the required fields:
/// - Time to live
/// - Message ID
/// - Variables
/// - Tenant ID
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
    /// Creates a new PublishMessageRequest in its initial state
    pub(crate) fn new(client: Client) -> PublishMessageRequest<Initial> {
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

    /// Internal helper to transition between states while preserving fields
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
    /// Sets the name of the message
    ///
    /// This is a required field and must be set first.
    pub fn with_name(mut self, name: String) -> PublishMessageRequest<WithName> {
        self.name = name;
        self.transition()
    }
}

impl PublishMessageRequest<WithName> {
    /// Sets the correlation key of the message
    ///
    /// This is a required field and must be set second.
    /// The correlation key is used to determine which partition the message is published to.
    pub fn with_correlation_key(
        mut self,
        correlation_key: String,
    ) -> PublishMessageRequest<WithKey> {
        self.correlation_key = correlation_key;
        self.transition()
    }
}

impl PublishMessageRequest<WithKey> {
    /// Sets how long the message should be buffered on the broker
    ///
    /// # Arguments
    /// * `ttl_sec` - Time to live in seconds, will be converted to milliseconds internally
    pub fn with_time_to_live(mut self, ttl_sec: i64) -> Self {
        self.time_to_live = ttl_sec * 1000;
        self
    }

    /// Sets a unique identifier for the message
    ///
    /// The message ID ensures only one message with this ID will be published during its lifetime.
    /// If a message with the same ID was previously published (and is still alive), the publish
    /// will fail with ALREADY_EXISTS error.
    pub fn with_message_id(mut self, message_id: String) -> Self {
        self.message_id = message_id;
        self
    }

    /// Sets the message variables as a JSON document
    ///
    /// # Arguments
    /// * `data` - Any serializable type that will be converted to JSON
    ///
    /// # Errors
    /// Returns MessageError if the data cannot be serialized to JSON
    ///
    /// The root of the resulting JSON document must be an object, e.g. `{"a": "foo"}`.
    /// Arrays like `["foo"]` are not valid.
    pub fn with_variables<T: Serialize>(mut self, data: T) -> Result<Self, MessageError> {
        self.variables = serde_json::to_value(data)?;
        Ok(self)
    }

    /// Sets the tenant ID of the message
    pub fn with_tenant_id(mut self, tenant_id: String) -> Self {
        self.tenant_id = tenant_id;
        self
    }

    /// Sends the message publish request to the broker
    ///
    /// # Returns
    /// * `Ok(PublishMessageResponse)` - Contains the unique message key and tenant ID
    /// * `Err(ClientError)` - If the request fails
    ///
    /// # Errors
    /// Will return error if:
    /// * The connection to the broker fails
    /// * A message with the same ID was previously published and is still alive
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

/// Response received after successfully publishing a message
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
    /// Returns the unique ID of the message that was published
    pub fn key(&self) -> i64 {
        self.key
    }

    /// Returns the tenant ID of the published message
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }
}
