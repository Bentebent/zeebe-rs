use crate::proto;
use crate::Client;
use crate::ClientError;

#[derive(Debug, Clone)]
pub struct Initial;

#[derive(Debug, Clone)]
pub struct WithProcessInstance;

pub trait CancelProcessInstanceState {}
impl CancelProcessInstanceState for Initial {}
impl CancelProcessInstanceState for WithProcessInstance {}

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

    pub fn with_operation_reference(mut self, operation_reference: u64) -> Self {
        self.operation_reference = Some(operation_reference);
        self
    }
}

impl CancelProcessInstanceRequest<Initial> {
    pub fn with_process_instance_key(
        mut self,
        process_instance_key: i64,
    ) -> CancelProcessInstanceRequest<WithProcessInstance> {
        self.process_instance_key = process_instance_key;
        self.transition()
    }
}

impl CancelProcessInstanceRequest<WithProcessInstance> {
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
}

#[derive(Debug, Clone)]
pub struct CancelProcessInstanceResponse {}

impl From<proto::CancelProcessInstanceResponse> for CancelProcessInstanceResponse {
    fn from(_value: proto::CancelProcessInstanceResponse) -> CancelProcessInstanceResponse {
        CancelProcessInstanceResponse {}
    }
}
