use crate::{proto, Client, ClientError};

pub struct Initial;
pub struct WithKey;
pub trait ResolveIncidentRequestState {}
impl ResolveIncidentRequestState for Initial {}
impl ResolveIncidentRequestState for WithKey {}

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

    pub fn with_operation_reference(mut self, operation_reference: u64) -> Self {
        self.operation_reference = Some(operation_reference);
        self
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
    pub fn with_incident_key(mut self, incident_key: i64) -> ResolveIncidentRequest<WithKey> {
        self.incident_key = incident_key;
        self.transition()
    }
}

impl ResolveIncidentRequest<WithKey> {
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
}

#[derive(Debug, Clone)]
pub struct ResolveIncidentResponse {}

impl From<proto::ResolveIncidentResponse> for ResolveIncidentResponse {
    fn from(_value: proto::ResolveIncidentResponse) -> ResolveIncidentResponse {
        ResolveIncidentResponse {}
    }
}
