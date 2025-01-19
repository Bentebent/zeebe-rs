use crate::{proto, Client, ClientError};

pub struct Initial;
pub struct WithKey;
pub struct WithTimeout;
pub trait UpdateJobTimeoutRequestState {}
impl UpdateJobTimeoutRequestState for Initial {}
impl UpdateJobTimeoutRequestState for WithKey {}
impl UpdateJobTimeoutRequestState for WithTimeout {}

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

    pub fn with_operation_reference(mut self, operation_reference: u64) -> Self {
        self.operation_reference = Some(operation_reference);
        self
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
    pub fn with_job_key(mut self, job_key: i64) -> UpdateJobTimeoutRequest<WithKey> {
        self.job_key = job_key;
        self.transition()
    }
}

impl UpdateJobTimeoutRequest<WithKey> {
    pub fn with_timeout(mut self, timeout_sec: i64) -> UpdateJobTimeoutRequest<WithTimeout> {
        self.timeout = timeout_sec * 1000;
        self.transition()
    }
}

impl UpdateJobTimeoutRequest<WithTimeout> {
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

#[derive(Debug, Clone)]
pub struct UpdateJobTimeoutResponse {}

impl From<proto::UpdateJobTimeoutResponse> for UpdateJobTimeoutResponse {
    fn from(_value: proto::UpdateJobTimeoutResponse) -> UpdateJobTimeoutResponse {
        UpdateJobTimeoutResponse {}
    }
}
