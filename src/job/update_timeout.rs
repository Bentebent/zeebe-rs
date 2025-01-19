use crate::{proto, Client, ClientError};

/// Initial state for the UpdateJobTimeoutRequest builder pattern
pub struct Initial;

/// State indicating the job key has been set
pub struct WithKey;

/// State indicating the timeout has been set
pub struct WithTimeout;

/// Marker trait for UpdateJobTimeoutRequest states
pub trait UpdateJobTimeoutRequestState {}
impl UpdateJobTimeoutRequestState for Initial {}
impl UpdateJobTimeoutRequestState for WithKey {}
impl UpdateJobTimeoutRequestState for WithTimeout {}

/// Request to update the deadline of a job
///
/// Updates the deadline using the timeout (in ms) provided. This can be used
/// for extending or shortening the job deadline.
#[derive(Debug, Clone)]
pub struct UpdateJobTimeoutRequest<T: UpdateJobTimeoutRequestState> {
    client: Client,
    /// The unique key identifying the job
    job_key: i64,
    /// The duration of the new timeout in ms
    timeout: i64,
    /// Optional reference key for tracking this operation
    operation_reference: Option<u64>,
    _state: std::marker::PhantomData<T>,
}

impl<T: UpdateJobTimeoutRequestState> UpdateJobTimeoutRequest<T> {
    /// Creates a new UpdateJobTimeoutRequest in its initial state
    pub(crate) fn new(client: Client) -> UpdateJobTimeoutRequest<Initial> {
        UpdateJobTimeoutRequest {
            client,
            job_key: 0,
            timeout: 0,
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
    fn transition<NewState: UpdateJobTimeoutRequestState>(
        self,
    ) -> UpdateJobTimeoutRequest<NewState> {
        UpdateJobTimeoutRequest {
            client: self.client,
            job_key: self.job_key,
            timeout: self.timeout,
            operation_reference: self.operation_reference,
            _state: std::marker::PhantomData,
        }
    }
}

impl UpdateJobTimeoutRequest<Initial> {
    /// Sets the job key to identify which job to update
    pub fn with_job_key(mut self, job_key: i64) -> UpdateJobTimeoutRequest<WithKey> {
        self.job_key = job_key;
        self.transition()
    }
}

impl UpdateJobTimeoutRequest<WithKey> {
    /// Sets the new timeout duration in seconds
    ///
    /// # Arguments
    /// * `timeout_sec` - The new timeout duration in seconds
    pub fn with_timeout(mut self, timeout_sec: i64) -> UpdateJobTimeoutRequest<WithTimeout> {
        self.timeout = timeout_sec * 1000;
        self.transition()
    }
}

impl UpdateJobTimeoutRequest<WithTimeout> {
    /// Sends the job timeout update request to the Zeebe workflow engine
    ///
    /// # Errors
    /// - NOT_FOUND: no job exists with the given key
    /// - INVALID_STATE: no deadline exists for the given job key
    pub async fn send(mut self) -> Result<UpdateJobTimeoutResponse, ClientError> {
        let res = self
            .client
            .gateway_client
            .update_job_timeout(proto::UpdateJobTimeoutRequest {
                job_key: self.job_key,
                timeout: self.timeout,
                operation_reference: self.operation_reference,
            })
            .await?;

        Ok(res.into_inner().into())
    }
}

/// Response from updating a job timeout
#[derive(Debug, Clone)]
pub struct UpdateJobTimeoutResponse {}

impl From<proto::UpdateJobTimeoutResponse> for UpdateJobTimeoutResponse {
    fn from(_value: proto::UpdateJobTimeoutResponse) -> UpdateJobTimeoutResponse {
        UpdateJobTimeoutResponse {}
    }
}
