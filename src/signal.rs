use crate::{Client, ClientError, proto};
use serde::Serialize;

pub struct Initial;
pub struct WithName;

pub trait BroadcastSignalRequestState {}
impl BroadcastSignalRequestState for Initial {}
impl BroadcastSignalRequestState for WithName {}

/// Request to broadcast a signal across the cluster
///
/// A signal can trigger multiple catching signal events in different process instances.
/// Signal events are matched by name and tenant ID if multi-tenancy is enabled.
///
/// # Examples
/// ```ignore
/// client
///     .broadcast_signal()
///     .with_signal_name(String::from("Hello_Signal"))
///     .send()
///     .await?;
/// ```
#[derive(Debug, Clone)]
pub struct BroadcastSignalRequest<T: BroadcastSignalRequestState> {
    client: Client,
    signal_name: String,
    variables: serde_json::Value,
    tenant_id: String,
    _state: std::marker::PhantomData<T>,
}

impl<T: BroadcastSignalRequestState> BroadcastSignalRequest<T> {
    pub(crate) fn new(client: Client) -> BroadcastSignalRequest<Initial> {
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
    /// Sets the name of the signal to broadcast
    ///
    /// # Arguments
    /// * `signal_name` - Name that will be matched with signal catch events
    ///
    /// # Returns
    /// A `BroadcastSignalRequest` in the `WithName` state
    pub fn with_signal_name(mut self, signal_name: String) -> BroadcastSignalRequest<WithName> {
        self.signal_name = signal_name;
        self.transition()
    }
}

impl BroadcastSignalRequest<WithName> {
    /// Sets variables that will be available to all triggered signal events
    ///
    /// # Arguments
    /// * `data` - Variables as serializable type that will be converted to JSON
    ///
    /// # Returns
    /// A `Result` containing the updated `BroadcastSignalRequest` or a `ClientError`
    ///
    /// # Notes
    /// Must be a JSON object, e.g. `{ "a": 1, "b": 2 }`. Arrays like `[1, 2]` are not valid.
    pub fn with_variables<T: Serialize>(mut self, data: T) -> Result<Self, ClientError> {
        self.variables = serde_json::to_value(data)
            .map_err(|e| ClientError::SerializationFailed { source: e })?;
        Ok(self)
    }

    /// Sets the tenant ID that owns this signal
    ///
    /// # Arguments
    /// * `tenant_id` - ID of tenant that owns the signal
    ///
    /// # Returns
    /// The updated `BroadcastSignalRequest`
    pub fn with_tenant_id(mut self, tenant_id: String) -> Self {
        self.tenant_id = tenant_id;
        self
    }

    /// Sends the broadcast signal request to the gateway
    ///
    /// # Returns
    /// A `Result` containing the `BroadcastSignalResponse` or a `ClientError`
    ///
    /// # Errors
    /// - `INVALID_ARGUMENT`: Missing signal name or invalid variables format
    /// - `PERMISSION_DENIED`: Not authorized for tenant
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

/// Response from broadcasting a signal
///
/// Contains:
/// - Key uniquely identifying this signal broadcast
/// - Tenant ID that owns the signal
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
    /// Returns the unique identifier for this signal broadcast operation
    ///
    /// # Returns
    /// The unique identifier for this signal broadcast operation
    pub fn key(&self) -> i64 {
        self.key
    }

    /// Returns the ID of tenant that owns the signal
    ///
    /// # Returns
    /// The ID of tenant that owns the signal, empty if multi-tenancy is disabled
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }
}
