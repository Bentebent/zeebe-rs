use crate::proto;
use crate::Client;
use crate::ClientError;
use serde::Serialize;

use super::ProcessInstanceError;

#[derive(Debug, Clone)]
pub struct Initial;

#[derive(Debug, Clone)]
pub struct WithProcessInstance;
pub trait ModifyProcessInstanceState {}
impl ModifyProcessInstanceState for Initial {}
impl ModifyProcessInstanceState for WithProcessInstance {}

#[derive(Debug, Clone)]
pub struct TerminateInstruction {
    element_instance_key: i64,
}

#[derive(Debug, Clone)]
pub struct VariableInstruction {
    variables: serde_json::Value,
    scope_id: String,
}

#[derive(Debug, Clone)]
pub struct ActivateInstruction {
    element_id: String,
    ancestor_element_instance_key: i64,
    variable_instructions: Vec<VariableInstruction>,
}

#[derive(Debug, Clone)]
pub struct ActivateInstructionBuilder {
    source_request: ModifyProcessInstanceRequest<WithProcessInstance>,
    element_id: String,
    ancestor_element_instance_key: i64,
    variable_instructions: Vec<VariableInstruction>,
}

impl ActivateInstructionBuilder {
    fn new(
        source_request: ModifyProcessInstanceRequest<WithProcessInstance>,
        element_id: String,
        ancestor_element_instance_key: i64,
    ) -> Self {
        ActivateInstructionBuilder {
            source_request,
            element_id,
            ancestor_element_instance_key,
            variable_instructions: vec![],
        }
    }

    pub fn with_variable_instruction<T: Serialize>(
        mut self,
        scope_id: String,
        data: T,
    ) -> Result<Self, ProcessInstanceError> {
        self.variable_instructions.push(VariableInstruction {
            scope_id,
            variables: serde_json::to_value(data)?,
        });
        Ok(self)
    }

    pub fn build(mut self) -> ModifyProcessInstanceRequest<WithProcessInstance> {
        self.source_request
            .activate_instructions
            .push(ActivateInstruction {
                element_id: self.element_id,
                ancestor_element_instance_key: self.ancestor_element_instance_key,
                variable_instructions: self.variable_instructions,
            });
        self.source_request
    }
}

#[derive(Debug, Clone)]
pub struct ModifyProcessInstanceRequest<T: ModifyProcessInstanceState> {
    client: Client,
    process_instance_key: i64,
    activate_instructions: Vec<ActivateInstruction>,
    terminate_instructions: Vec<TerminateInstruction>,
    operation_reference: Option<u64>,
    _state: std::marker::PhantomData<T>,
}

impl<T: ModifyProcessInstanceState> ModifyProcessInstanceRequest<T> {
    pub fn new(client: Client) -> ModifyProcessInstanceRequest<Initial> {
        ModifyProcessInstanceRequest {
            client,
            process_instance_key: 0,
            activate_instructions: vec![],
            terminate_instructions: vec![],
            operation_reference: None,
            _state: std::marker::PhantomData,
        }
    }
    pub fn with_operation_reference(mut self, operation_reference: u64) -> Self {
        self.operation_reference = Some(operation_reference);
        self
    }

    fn transition<NewState: ModifyProcessInstanceState>(
        self,
    ) -> ModifyProcessInstanceRequest<NewState> {
        ModifyProcessInstanceRequest {
            client: self.client,
            process_instance_key: self.process_instance_key,
            activate_instructions: self.activate_instructions,
            terminate_instructions: self.terminate_instructions,
            operation_reference: self.operation_reference,
            _state: std::marker::PhantomData,
        }
    }
}

impl ModifyProcessInstanceRequest<Initial> {
    pub fn with_process_instance_key(
        mut self,
        process_instance_key: i64,
    ) -> ModifyProcessInstanceRequest<WithProcessInstance> {
        self.process_instance_key = process_instance_key;
        self.transition()
    }
}

impl ModifyProcessInstanceRequest<WithProcessInstance> {
    pub fn with_activate_instruction(
        self,
        element_id: String,
        ancestor_element_instance_key: i64,
    ) -> ActivateInstructionBuilder {
        ActivateInstructionBuilder::new(self, element_id, ancestor_element_instance_key)
    }

    pub fn with_terminate_instruction(mut self, element_instance_key: i64) -> Self {
        self.terminate_instructions.push(TerminateInstruction {
            element_instance_key,
        });
        self
    }
    pub fn with_terminate_instructions(mut self, element_instance_keys: Vec<i64>) -> Self {
        self.terminate_instructions
            .extend(
                element_instance_keys
                    .into_iter()
                    .map(|element_instance_key| TerminateInstruction {
                        element_instance_key,
                    }),
            );
        self
    }
    pub async fn send(mut self) -> Result<ModifyProcessInstanceResponse, ClientError> {
        let res = self
            .client
            .gateway_client
            .modify_process_instance(proto::ModifyProcessInstanceRequest {
                process_instance_key: self.process_instance_key,
                activate_instructions: self
                    .activate_instructions
                    .into_iter()
                    .map(
                        |a| proto::modify_process_instance_request::ActivateInstruction {
                            element_id: a.element_id,
                            ancestor_element_instance_key: a.ancestor_element_instance_key,
                            variable_instructions: a
                                .variable_instructions
                                .into_iter()
                                .map(|i| {
                                    proto::modify_process_instance_request::VariableInstruction {
                                        variables: i.variables.to_string(),
                                        scope_id: i.scope_id,
                                    }
                                })
                                .collect(),
                        },
                    )
                    .collect(),
                terminate_instructions: self
                    .terminate_instructions
                    .into_iter()
                    .map(
                        |t| proto::modify_process_instance_request::TerminateInstruction {
                            element_instance_key: t.element_instance_key,
                        },
                    )
                    .collect(),
                operation_reference: self.operation_reference,
            })
            .await?;

        Ok(res.into_inner().into())
    }
}

#[derive(Debug, Clone)]
pub struct ModifyProcessInstanceResponse {}

impl From<proto::ModifyProcessInstanceResponse> for ModifyProcessInstanceResponse {
    fn from(_value: proto::ModifyProcessInstanceResponse) -> ModifyProcessInstanceResponse {
        ModifyProcessInstanceResponse {}
    }
}
