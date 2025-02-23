use crate::{Client, ClientError, proto};
use serde::Serialize;

pub struct Initial;
pub struct WithKey;
pub struct WithCode;

pub trait ThrowErrorRequestState {}
impl ThrowErrorRequestState for Initial {}
impl ThrowErrorRequestState for WithKey {}
impl ThrowErrorRequestState for WithCode {}

/// Request to throw a business error for a job
///
/// The error will be caught by an error catch event in the process.
/// If no matching catch event exists, an incident will be raised instead.
///
/// # Examples
/// ```ignore
/// client
///     .throw_error()
///     .with_job_key(123456)
///     .with_error_code(String::from("error_code"))
///     .send()
///     .await?;
/// ```
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
    /// Sets the job key for which to throw the error
    ///
    /// # Arguments
    /// * `job_key` - Unique job identifier from job activation
    ///
    /// # Returns
    /// A `ThrowErrorRequest` in the `WithKey` state
    pub fn with_job_key(mut self, job_key: i64) -> ThrowErrorRequest<WithKey> {
        self.job_key = job_key;
        self.transition()
    }
}

impl ThrowErrorRequest<WithKey> {
    /// Sets the error code that will match an error catch event
    ///
    /// # Arguments
    /// * `error_code` - Code that will be matched with an error catch event in the process
    ///
    /// # Returns
    /// A `ThrowErrorRequest` in the `WithCode` state
    pub fn with_error_code(mut self, error_code: String) -> ThrowErrorRequest<WithCode> {
        self.error_code = error_code;
        self.transition()
    }
}

impl ThrowErrorRequest<WithCode> {
    /// Sets an optional message describing the error
    ///
    /// # Arguments
    /// * `error_message` - Additional context about the error
    ///
    /// # Returns
    /// The updated `ThrowErrorRequest` with the error message set
    pub fn with_error_message(mut self, error_message: String) -> Self {
        self.error_message = error_message;
        self
    }

    /// Sets variables that will be available in the error catch event scope
    ///
    /// # Arguments
    /// * `data` - Variables as serializable type that will be converted to JSON
    ///     JSON document that will instantiate the variables at the local scope of the
    ///     error catch event that catches the thrown error; it must be a JSON object, as variables will be mapped in a
    ///     key-value fashion. e.g. { "a": 1, "b": 2 } will create two variables, named "a" and
    ///     "b" respectively, with their associated values. [{ "a": 1, "b": 2 }] would not be a
    ///     valid argument, as the root of the JSON document is an array and not an object.
    ///
    /// # Errors
    /// Returns `ClientError` if serialization fails
    ///
    /// # Returns
    /// A `Result` containing the updated `ThrowErrorRequest` with the variables set, or a `ClientError`
    pub fn with_variables<T: Serialize>(mut self, data: T) -> Result<Self, ClientError> {
        self.variables = serde_json::to_value(data)?;
        Ok(self)
    }

    /// Sends the throw error request to the gateway
    ///
    /// # Errors
    /// - `NOT_FOUND`: No job exists with given key
    /// - `FAILED_PRECONDITION`: Job is not in activated state
    ///
    /// # Returns
    /// A `Result` containing a `ThrowErrorResponse` or a `ClientError`
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

/// Empty response since throw error operation is fire-and-forget
#[derive(Debug, Clone)]
pub struct ThrowErrorResponse {}

impl From<proto::ThrowErrorResponse> for ThrowErrorResponse {
    fn from(_value: proto::ThrowErrorResponse) -> ThrowErrorResponse {
        ThrowErrorResponse {}
    }
}
