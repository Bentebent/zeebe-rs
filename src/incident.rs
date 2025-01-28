use crate::{proto, Client, ClientError};

pub struct Initial;
pub struct WithKey;

pub trait ResolveIncidentRequestState {}
impl ResolveIncidentRequestState for Initial {}
impl ResolveIncidentRequestState for WithKey {}

/// Request builder for resolving incidents in Zeebe.
///
/// This simply marks the incident as resolved. Most likely a call to
/// UpdateJobRetries or SetVariables will be necessary to actually resolve the
/// underlying problem before calling this.
///
/// # Examples
/// ```ignore
/// client
///     .resolve_incident()
///     .with_incident_key(123456)
///     .send()
///     .await?;
/// ```
#[derive(Debug, Clone)]
pub struct ResolveIncidentRequest<T: ResolveIncidentRequestState> {
    client: Client,
    incident_key: i64,
    operation_reference: Option<u64>,
    _state: std::marker::PhantomData<T>,
}

impl<T: ResolveIncidentRequestState> ResolveIncidentRequest<T> {
    pub(crate) fn new(client: Client) -> ResolveIncidentRequest<Initial> {
        ResolveIncidentRequest {
            client,
            incident_key: 0,
            operation_reference: None,
            _state: std::marker::PhantomData,
        }
    }

    fn transition<NewState: ResolveIncidentRequestState>(self) -> ResolveIncidentRequest<NewState> {
        ResolveIncidentRequest {
            client: self.client,
            incident_key: self.incident_key,
            operation_reference: self.operation_reference,
            _state: std::marker::PhantomData,
        }
    }
}

impl ResolveIncidentRequest<Initial> {
    /// Sets the unique identifier of the incident to resolve
    ///
    /// This is a required field and must be set before sending the request.
    ///
    /// # Arguments
    /// * `incident_key` - The unique key identifying the incident to be resolved
    ///
    /// # Returns
    /// The updated `ResolveIncidentRequest` with the incident key set
    pub fn with_incident_key(mut self, incident_key: i64) -> ResolveIncidentRequest<WithKey> {
        self.incident_key = incident_key;
        self.transition()
    }
}

impl ResolveIncidentRequest<WithKey> {
    /// Sends the incident resolution request to the broker
    ///
    /// # Returns
    /// * `Ok(ResolveIncidentResponse)` - The incident was successfully marked as resolved
    /// * `Err(ClientError)` - If the request fails
    ///
    /// # Errors
    /// Will return error if:
    /// * No incident exists with the given key
    /// * The connection to the broker fails
    pub async fn send(mut self) -> Result<ResolveIncidentResponse, ClientError> {
        let res = self
            .client
            .gateway_client
            .resolve_incident(proto::ResolveIncidentRequest {
                incident_key: self.incident_key,
                operation_reference: self.operation_reference,
            })
            .await?;

        Ok(res.into_inner().into())
    }

    /// Sets a reference key that will be included in all records resulting from this operation
    ///
    /// This is an optional identifier that can be used to track the operation across the system.
    ///
    /// # Arguments
    /// * `operation_reference` - The reference key to set
    ///
    /// # Returns
    /// The updated `ResolveIncidentRequest` with the operation reference set
    pub fn with_operation_reference(mut self, operation_reference: u64) -> Self {
        self.operation_reference = Some(operation_reference);
        self
    }
}

/// Empty response received after successfully resolving an incident
#[derive(Debug, Clone)]
pub struct ResolveIncidentResponse {}

impl From<proto::ResolveIncidentResponse> for ResolveIncidentResponse {
    fn from(_value: proto::ResolveIncidentResponse) -> ResolveIncidentResponse {
        ResolveIncidentResponse {}
    }
}
