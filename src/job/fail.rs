use crate::{proto, Client, ClientError};
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FailJobError {
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
}

pub struct Initial;
pub struct WithKey;

pub trait FailJobRequestState {}
impl FailJobRequestState for Initial {}
impl FailJobRequestState for WithKey {}

#[derive(Debug, Clone)]
pub struct FailJobRequest<T: FailJobRequestState> {
    client: Client,
    job_key: i64,
    retries: i32,
    error_message: String,
    retry_back_off: i64,
    variables: serde_json::Value,
    _state: std::marker::PhantomData<T>,
}

impl<T: FailJobRequestState> FailJobRequest<T> {
    pub fn new(client: Client) -> FailJobRequest<Initial> {
        FailJobRequest {
            client,
            job_key: 0,
            retries: 0,
            error_message: String::new(),
            retry_back_off: 0,
            variables: serde_json::Value::default(),
            _state: std::marker::PhantomData,
        }
    }

    fn transition<NewState: FailJobRequestState>(self) -> FailJobRequest<NewState> {
        FailJobRequest {
            client: self.client,
            job_key: self.job_key,
            retries: self.retries,
            error_message: self.error_message,
            retry_back_off: self.retry_back_off,
            variables: self.variables,
            _state: std::marker::PhantomData,
        }
    }
}

impl FailJobRequest<Initial> {
    pub fn with_job_key(mut self, job_key: i64) -> FailJobRequest<WithKey> {
        self.job_key = job_key;
        self.transition()
    }
}

impl FailJobRequest<WithKey> {
    pub fn with_retries(mut self, retries: i32) -> Self {
        self.retries = retries;
        self
    }

    pub fn with_error_message(mut self, error_message: String) -> Self {
        self.error_message = error_message;
        self
    }

    pub fn with_retry_back_off(mut self, retry_back_off_sec: i64) -> Self {
        self.retry_back_off = retry_back_off_sec * 1000;
        self
    }

    pub fn with_variables<T: Serialize>(mut self, data: T) -> Result<Self, FailJobError> {
        self.variables = serde_json::to_value(data)?;
        Ok(self)
    }

    pub async fn send(mut self) -> Result<FailJobResponse, ClientError> {
        let res = self
            .client
            .gateway_client
            .fail_job(proto::FailJobRequest {
                job_key: self.job_key,
                retries: self.retries,
                error_message: self.error_message,
                retry_back_off: self.retry_back_off,
                variables: self.variables.to_string(),
            })
            .await?;

        Ok(res.into_inner().into())
    }
}

#[derive(Debug, Clone)]
pub struct FailJobResponse {}

impl From<proto::FailJobResponse> for FailJobResponse {
    fn from(_value: proto::FailJobResponse) -> FailJobResponse {
        FailJobResponse {}
    }
}
