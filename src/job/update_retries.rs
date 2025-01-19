use crate::{proto, Client, ClientError};

pub struct Initial;
pub struct WithKey;
pub struct WithRetries;
pub trait UpdateJobRetriesRequestState {}
impl UpdateJobRetriesRequestState for Initial {}
impl UpdateJobRetriesRequestState for WithKey {}
impl UpdateJobRetriesRequestState for WithRetries {}

#[derive(Debug, Clone)]
pub struct UpdateJobRetriesRequest<T: UpdateJobRetriesRequestState> {
    client: Client,
    job_key: i64,
    retries: i32,
    operation_reference: Option<u64>,
    _state: std::marker::PhantomData<T>,
}

impl<T: UpdateJobRetriesRequestState> UpdateJobRetriesRequest<T> {
    pub fn new(client: Client) -> UpdateJobRetriesRequest<Initial> {
        UpdateJobRetriesRequest {
            client,
            job_key: 0,
            retries: 0,
            operation_reference: None,
            _state: std::marker::PhantomData,
        }
    }

    pub fn with_operation_reference(mut self, operation_reference: u64) -> Self {
        self.operation_reference = Some(operation_reference);
        self
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
    pub fn with_job_key(mut self, job_key: i64) -> UpdateJobRetriesRequest<WithKey> {
        self.job_key = job_key;
        self.transition()
    }
}

impl UpdateJobRetriesRequest<WithKey> {
    pub fn with_retries(mut self, retries: i32) -> UpdateJobRetriesRequest<WithRetries> {
        self.retries = retries;
        self.transition()
    }
}

impl UpdateJobRetriesRequest<WithRetries> {
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

#[derive(Debug, Clone)]
pub struct UpdateJobRetriesResponse {}

impl From<proto::UpdateJobRetriesResponse> for UpdateJobRetriesResponse {
    fn from(_value: proto::UpdateJobRetriesResponse) -> UpdateJobRetriesResponse {
        UpdateJobRetriesResponse {}
    }
}
