use crate::{proto, Client, ClientError};

/// Initial state for the UpdateJobRetriesRequest builder pattern
pub struct Initial;

/// State indicating the job key has been set
pub struct WithKey;

/// State indicating the retries have been set
pub struct WithRetries;

/// Marker trait for UpdateJobRetriesRequest states
pub trait UpdateJobRetriesRequestState {}
impl UpdateJobRetriesRequestState for Initial {}
impl UpdateJobRetriesRequestState for WithKey {}
impl UpdateJobRetriesRequestState for WithRetries {}

/// Request to update the number of retries for a job
#[derive(Debug, Clone)]
pub struct UpdateJobRetriesRequest<T: UpdateJobRetriesRequestState> {
    client: Client,
    /// The unique key identifying the job
    job_key: i64,
    /// The new number of retries for the job
    retries: i32,
    /// Optional reference key for tracking the operation
    operation_reference: Option<u64>,
    _state: std::marker::PhantomData<T>,
}

impl<T: UpdateJobRetriesRequestState> UpdateJobRetriesRequest<T> {
    /// Creates a new UpdateJobRetriesRequest in its initial state
    pub(crate) fn new(client: Client) -> UpdateJobRetriesRequest<Initial> {
        UpdateJobRetriesRequest {
            client,
            job_key: 0,
            retries: 0,
            operation_reference: None,
            _state: std::marker::PhantomData,
        }
    }

    /// Sets a reference key for tracking this operation
    pub fn with_operation_reference(mut self, operation_reference: u64) -> Self {
        self.operation_reference = Some(operation_reference);
        self
    }

    /// Internal helper to transition between builder states
    fn transition<NewState: UpdateJobRetriesRequestState>(
        self,
    ) -> UpdateJobRetriesRequest<NewState> {
        UpdateJobRetriesRequest {
            client: self.client,
            job_key: self.job_key,
            retries: self.retries,
            operation_reference: self.operation_reference,
            _state: std::marker::PhantomData,
        }
    }
}

impl UpdateJobRetriesRequest<Initial> {
    /// Sets the job key to identify which job to update
    pub fn with_job_key(mut self, job_key: i64) -> UpdateJobRetriesRequest<WithKey> {
        self.job_key = job_key;
        self.transition()
    }
}

impl UpdateJobRetriesRequest<WithKey> {
    /// Sets the new number of retries for the job
    pub fn with_retries(mut self, retries: i32) -> UpdateJobRetriesRequest<WithRetries> {
        self.retries = retries;
        self.transition()
    }
}

impl UpdateJobRetriesRequest<WithRetries> {
    /// Sends the job retries update request to the Zeebe workflow engine
    pub async fn send(mut self) -> Result<UpdateJobRetriesResponse, ClientError> {
        let res = self
            .client
            .gateway_client
            .update_job_retries(proto::UpdateJobRetriesRequest {
                job_key: self.job_key,
                retries: self.retries,
                operation_reference: self.operation_reference,
            })
            .await?;

        Ok(res.into_inner().into())
    }
}

/// Response from updating job retries
#[derive(Debug, Clone)]
pub struct UpdateJobRetriesResponse {}

impl From<proto::UpdateJobRetriesResponse> for UpdateJobRetriesResponse {
    fn from(_value: proto::UpdateJobRetriesResponse) -> UpdateJobRetriesResponse {
        UpdateJobRetriesResponse {}
    }
}
