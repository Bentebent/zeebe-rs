use crate::{proto, Client, ClientError};
use serde::Serialize;
use thiserror::Error;

/// Errors that can occur when throwing a business error
#[derive(Error, Debug)]
pub enum ThrowErrorError {
    /// Failed to serialize variables to JSON format
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
}

// State machine types for enforcing valid request building
/// Initial state - no fields set
pub struct Initial;
/// After setting job key
pub struct WithKey;
/// After setting error code
pub struct WithCode;

/// Marker trait for state machine types
pub trait ThrowErrorRequestState {}
impl ThrowErrorRequestState for Initial {}
impl ThrowErrorRequestState for WithKey {}
impl ThrowErrorRequestState for WithCode {}

/// Request to throw a business error for a job
///
/// The error will be caught by an error catch event in the process.
/// If no matching catch event exists, an incident will be raised instead.
///
/// # State Machine
/// 1. Create request with client
/// 2. Set job key (required)
/// 3. Set error code (required)
/// 4. Optionally set message and variables
/// 5. Send request
///
/// # Errors
/// - NOT_FOUND: No job exists with given key
/// - FAILED_PRECONDITION: Job not in activated state
#[derive(Debug, Clone)]
pub struct ThrowErrorRequest<T: ThrowErrorRequestState> {
    client: Client,
    /// The unique job identifier from job activation
    job_key: i64,
    /// Error code that will be matched with an error catch event
    error_code: String,
    /// Optional message providing additional context
    error_message: String,
    /// Variables that will be available in the error catch event scope
    variables: serde_json::Value,
    _state: std::marker::PhantomData<T>,
}

impl<T: ThrowErrorRequestState> ThrowErrorRequest<T> {
    /// Creates new throw error request in initial state
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

    /// Internal helper to transition between states
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
    /// Returns JsonError if serialization fails
    /// - INVALID_ARGUMENT: Missing required fields
    pub fn with_variables<T: Serialize>(mut self, data: T) -> Result<Self, ThrowErrorError> {
        self.variables = serde_json::to_value(data)?;
        Ok(self)
    }

    /// Sends the throw error request to the gateway
    ///
    /// # Errors
    /// - NOT_FOUND: No job exists with given key
    /// - FAILED_PRECONDITION: Job is not in activated state
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
