use crate::proto;
use crate::Client;
use crate::ClientError;

/// Initial state for the CancelProcessInstanceRequest builder pattern
#[derive(Debug, Clone)]
pub struct Initial;

/// State indicating the process instance key has been set
#[derive(Debug, Clone)]
pub struct WithProcessInstance;

/// Marker trait for CancelProcessInstanceRequest states
pub trait CancelProcessInstanceState {}
impl CancelProcessInstanceState for Initial {}
impl CancelProcessInstanceState for WithProcessInstance {}

/// Request to cancel a running process instance
/// # Examples
///
/// ```
/// client
///     .cancel_process_instance()
///     .with_process_instance_key(123456)
///     .send()
///     .await?;
/// ```
///
/// Errors:
/// - NOT_FOUND: no process instance exists with the given key
#[derive(Debug, Clone)]
pub struct CancelProcessInstanceRequest<T: CancelProcessInstanceState> {
    client: Client,
    process_instance_key: i64,
    operation_reference: Option<u64>,
    _state: std::marker::PhantomData<T>,
}

impl<T: CancelProcessInstanceState> CancelProcessInstanceRequest<T> {
    pub(crate) fn new(client: Client) -> CancelProcessInstanceRequest<Initial> {
        CancelProcessInstanceRequest {
            client,
            process_instance_key: 0,
            operation_reference: None,
            _state: std::marker::PhantomData,
        }
    }

    fn transition<NewState: CancelProcessInstanceState>(
        self,
    ) -> CancelProcessInstanceRequest<NewState> {
        CancelProcessInstanceRequest {
            client: self.client,
            process_instance_key: self.process_instance_key,
            operation_reference: self.operation_reference,
            _state: std::marker::PhantomData,
        }
    }
}

impl CancelProcessInstanceRequest<Initial> {
    /// Sets the process instance key identifying which instance to cancel
    ///
    /// # Arguments
    ///
    /// * `process_instance_key` - The unique key identifying the process instance to cancel
    ///
    /// # Returns
    ///
    /// A new CancelProcessInstanceRequest in the WithProcessInstance state
    pub fn with_process_instance_key(
        mut self,
        process_instance_key: i64,
    ) -> CancelProcessInstanceRequest<WithProcessInstance> {
        self.process_instance_key = process_instance_key;
        self.transition()
    }
}

impl CancelProcessInstanceRequest<WithProcessInstance> {
    /// Sends the process instance cancellation request to the Zeebe workflow engine
    ///
    /// # Returns
    ///
    /// A Result containing the CancelProcessInstanceResponse if successful, or a ClientError if not
    pub async fn send(mut self) -> Result<CancelProcessInstanceResponse, ClientError> {
        let res = self
            .client
            .gateway_client
            .cancel_process_instance(proto::CancelProcessInstanceRequest {
                process_instance_key: self.process_instance_key,
                operation_reference: self.operation_reference,
            })
            .await?;

        Ok(res.into_inner().into())
    }

    /// Sets a reference key for tracking this operation
    ///
    /// # Arguments
    ///
    /// * `operation_reference` - The reference key for tracking the operation
    ///
    /// # Returns
    ///
    /// The updated CancelProcessInstanceRequest with the operation reference set
    pub fn with_operation_reference(mut self, operation_reference: u64) -> Self {
        self.operation_reference = Some(operation_reference);
        self
    }
}

/// Empty response from canceling a process instance
#[derive(Debug, Clone)]
pub struct CancelProcessInstanceResponse {}

impl From<proto::CancelProcessInstanceResponse> for CancelProcessInstanceResponse {
    fn from(_value: proto::CancelProcessInstanceResponse) -> CancelProcessInstanceResponse {
        CancelProcessInstanceResponse {}
    }
}
