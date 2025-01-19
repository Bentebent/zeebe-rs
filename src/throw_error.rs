use crate::{proto, Client, ClientError};
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ThrowErrorError {
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
}

pub struct Initial;
pub struct WithKey;
pub struct WithCode;
pub trait ThrowErrorRequestState {}
impl ThrowErrorRequestState for Initial {}
impl ThrowErrorRequestState for WithKey {}
impl ThrowErrorRequestState for WithCode {}

#[derive(Debug, Clone)]
pub struct ThrowErrorRequest<T: ThrowErrorRequestState> {
    client: Client,
    job_key: i64,
    error_code: String,
    error_message: String,
    variables: serde_json::Value,
    _state: std::marker::PhantomData<T>,
}

impl<T: ThrowErrorRequestState> ThrowErrorRequest<T> {
    pub(crate) fn new(client: Client) -> ThrowErrorRequest<Initial> {
        ThrowErrorRequest {
            client,
            job_key: 0,
            error_code: String::new(),
            error_message: String::new(),
            variables: serde_json::Value::default(),
            _state: std::marker::PhantomData,
        }
    }

    fn transition<NewState: ThrowErrorRequestState>(self) -> ThrowErrorRequest<NewState> {
        ThrowErrorRequest {
            client: self.client,
            job_key: self.job_key,
            error_code: self.error_code,
            error_message: self.error_message,
            variables: self.variables,
            _state: std::marker::PhantomData,
        }
    }
}

impl ThrowErrorRequest<Initial> {
    pub fn with_job_key(mut self, job_key: i64) -> ThrowErrorRequest<WithKey> {
        self.job_key = job_key;
        self.transition()
    }
}

impl ThrowErrorRequest<WithKey> {
    pub fn with_error_code(mut self, error_code: String) -> ThrowErrorRequest<WithCode> {
        self.error_code = error_code;
        self.transition()
    }
}

impl ThrowErrorRequest<WithCode> {
    pub fn with_error_message(mut self, error_message: String) -> Self {
        self.error_message = error_message;
        self
    }
    pub fn with_variables<T: Serialize>(mut self, data: T) -> Result<Self, ThrowErrorError> {
        self.variables = serde_json::to_value(data)?;
        Ok(self)
    }

    pub async fn send(mut self) -> Result<ThrowErrorResponse, ClientError> {
        let res = self
            .client
            .gateway_client
            .throw_error(proto::ThrowErrorRequest {
                job_key: self.job_key,
                error_code: self.error_code,
                error_message: self.error_message,
                variables: self.variables.to_string(),
            })
            .await?;

        Ok(res.into_inner().into())
    }
}

#[derive(Debug, Clone)]
pub struct ThrowErrorResponse {}

impl From<proto::ThrowErrorResponse> for ThrowErrorResponse {
    fn from(_value: proto::ThrowErrorResponse) -> ThrowErrorResponse {
        ThrowErrorResponse {}
    }
}
