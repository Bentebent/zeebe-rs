use std::time::Duration;

use crate::{proto, Client, ClientError};

pub struct Initial;
pub struct WithKey;
pub struct WithTimeout;

pub trait UpdateJobTimeoutRequestState {}
impl UpdateJobTimeoutRequestState for Initial {}
impl UpdateJobTimeoutRequestState for WithKey {}
impl UpdateJobTimeoutRequestState for WithTimeout {}

/// Request to update the deadline of a job
///
/// Updates the deadline using the timeout (in ms) provided. This can be used
/// for extending or shortening the job deadline.
///
/// # Examples
/// ```ignore
/// client
///     .update_job_timeout()
///     .with_job_key(123456)
///     .with_timeout(Duration::from_secs(10))
///     .send()
///     .await?;
/// ```
#[derive(Debug, Clone)]
pub struct UpdateJobTimeoutRequest<T: UpdateJobTimeoutRequestState> {
    client: Client,
    job_key: i64,
    timeout: i64,
    operation_reference: Option<u64>,
    _state: std::marker::PhantomData<T>,
}

impl<T: UpdateJobTimeoutRequestState> UpdateJobTimeoutRequest<T> {
    pub(crate) fn new(client: Client) -> UpdateJobTimeoutRequest<Initial> {
        UpdateJobTimeoutRequest {
            client,
            job_key: 0,
            timeout: 0,
            operation_reference: None,
            _state: std::marker::PhantomData,
        }
    }

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
    ///
    /// # Arguments
    /// - `job_key`: The unique key identifying the job
    ///
    /// # Returns
    /// An `UpdateJobTimeoutRequest` in the `WithKey` state
    pub fn with_job_key(mut self, job_key: i64) -> UpdateJobTimeoutRequest<WithKey> {
        self.job_key = job_key;
        self.transition()
    }
}

impl UpdateJobTimeoutRequest<WithKey> {
    /// Sets the new timeout duration
    ///
    /// # Arguments
    /// - `timeout`: The new timeout duration
    ///
    /// # Returns
    /// An `UpdateJobTimeoutRequest` in the `WithTimeout` state
    pub fn with_timeout(mut self, timeout: Duration) -> UpdateJobTimeoutRequest<WithTimeout> {
        self.timeout = timeout.as_millis() as i64;
        self.transition()
    }
}

impl UpdateJobTimeoutRequest<WithTimeout> {
    /// Sends the job timeout update request to the Zeebe workflow engine
    ///
    /// # Errors
    /// - `ClientError::NotFound`: No job exists with the given key
    /// - `ClientError::InvalidState`: No deadline exists for the given job key
    ///
    /// # Returns
    /// A `Result` containing the `UpdateJobTimeoutResponse` on success, or a `ClientError` on failure
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

    /// Sets a reference key for tracking this operation
    ///
    /// # Arguments
    /// - `operation_reference`: A unique reference key for tracking the operation
    ///
    /// # Returns
    /// The `UpdateJobTimeoutRequest` with the operation reference set
    pub fn with_operation_reference(mut self, operation_reference: u64) -> Self {
        self.operation_reference = Some(operation_reference);
        self
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
