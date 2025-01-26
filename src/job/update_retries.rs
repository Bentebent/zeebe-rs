use crate::{proto, Client, ClientError};

pub struct Initial;
pub struct WithKey;
pub struct WithRetries;

pub trait UpdateJobRetriesRequestState {}
impl UpdateJobRetriesRequestState for Initial {}
impl UpdateJobRetriesRequestState for WithKey {}
impl UpdateJobRetriesRequestState for WithRetries {}

/// Request to update the number of retries for a job
///
/// This struct represents a request to update the number of retries for a
/// job in Zeebe.
/// It uses a builder pattern with different states to ensure that the job key
/// and retries are set before sending the request.
///
/// # Examples
/// ```ignore
/// client
///     .update_job_retries()
///     .with_job_key(123456)
///     .with_retries(1)
///     .send()
///     .await?;
/// ```
#[derive(Debug, Clone)]
pub struct UpdateJobRetriesRequest<T: UpdateJobRetriesRequestState> {
    client: Client,
    job_key: i64,
    retries: i32,
    operation_reference: Option<u64>,
    _state: std::marker::PhantomData<T>,
}

impl<T: UpdateJobRetriesRequestState> UpdateJobRetriesRequest<T> {
    pub(crate) fn new(client: Client) -> UpdateJobRetriesRequest<Initial> {
        UpdateJobRetriesRequest {
            client,
            job_key: 0,
            retries: 0,
            operation_reference: None,
            _state: std::marker::PhantomData,
        }
    }

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
    ///
    /// # Arguments
    ///
    /// * `job_key` - The unique key identifying the job
    ///
    /// # Returns
    ///
    /// The UpdateJobRetriesRequest in the WithKey state
    pub fn with_job_key(mut self, job_key: i64) -> UpdateJobRetriesRequest<WithKey> {
        self.job_key = job_key;
        self.transition()
    }
}

impl UpdateJobRetriesRequest<WithKey> {
    /// Sets the new number of retries for the job
    ///
    /// # Arguments
    ///
    /// * `retries` - The new number of retries for the job
    ///
    /// # Returns
    ///
    /// The UpdateJobRetriesRequest in the WithRetries state
    pub fn with_retries(mut self, retries: i32) -> UpdateJobRetriesRequest<WithRetries> {
        self.retries = retries;
        self.transition()
    }
}

impl UpdateJobRetriesRequest<WithRetries> {
    /// Sends the job retries update request to Zeebe.
    ///
    /// # Returns
    ///
    /// A Result containing the UpdateJobRetriesResponse if successful, or a ClientError if there was an error
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

    /// Sets a reference key for tracking this operation
    ///
    /// # Arguments
    ///
    /// * `operation_reference` - The reference key to track the operation
    ///
    /// # Returns
    ///
    /// The updated UpdateJobRetriesRequest with the operation reference set
    pub fn with_operation_reference(mut self, operation_reference: u64) -> Self {
        self.operation_reference = Some(operation_reference);
        self
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
