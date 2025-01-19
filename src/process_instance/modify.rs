use crate::{proto, Client, ClientError};
use serde::Serialize;

/// Initial state for the ModifyProcessInstanceRequest builder pattern
#[derive(Debug, Clone)]
pub struct Initial;

/// State indicating process instance key has been set
#[derive(Debug, Clone)]
pub struct WithProcessInstance;

/// Marker trait for ModifyProcessInstanceRequest states
pub trait ModifyProcessInstanceState {}
impl ModifyProcessInstanceState for Initial {}
impl ModifyProcessInstanceState for WithProcessInstance {}

/// Instruction to terminate a specific element instance
#[derive(Debug, Clone)]
pub struct TerminateInstruction {
    /// The unique key of the element instance to terminate
    element_instance_key: i64,
}

/// Instruction to set variables in a specific scope
#[derive(Debug, Clone)]
pub struct VariableInstruction {
    /// Variables to set as JSON document
    variables: serde_json::Value,
    /// ID of the element scope for the variables
    scope_id: String,
}

/// Instruction to activate a specific element
#[derive(Debug, Clone)]
pub struct ActivateInstruction {
    /// ID of the element to activate
    element_id: String,
    /// Key of the ancestor scope to create the element instance in
    ancestor_element_instance_key: i64,
    /// Variable instructions for the new element instance
    variable_instructions: Vec<VariableInstruction>,
}

/// Builder for constructing element activation instructions
#[derive(Debug, Clone)]
pub struct ActivateInstructionBuilder {
    /// Source request being built
    source_request: ModifyProcessInstanceRequest<WithProcessInstance>,
    /// ID of element to activate
    element_id: String,
    /// Key of ancestor scope
    ancestor_element_instance_key: i64,
    /// Variable instructions
    variable_instructions: Vec<VariableInstruction>,
}

impl ActivateInstructionBuilder {
    /// Creates a new activation instruction builder
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

    /// Adds a variable instruction to the activation
    ///
    /// # Arguments
    /// * `scope_id` - ID of the element scope for the variables
    /// * `data` - Variables to set in the scope
    pub fn with_variable_instruction<T: Serialize>(
        mut self,
        scope_id: String,
        data: T,
    ) -> Result<Self, ClientError> {
        self.variable_instructions.push(VariableInstruction {
            scope_id,
            variables: serde_json::to_value(data)?,
        });
        Ok(self)
    }

    /// Builds the activation instruction and adds it to the request
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

/// Request to modify a process instance by activating/terminating elements
#[derive(Debug, Clone)]
pub struct ModifyProcessInstanceRequest<T: ModifyProcessInstanceState> {
    client: Client,
    /// Key of the process instance to modify
    process_instance_key: i64,
    /// Instructions for activating elements
    activate_instructions: Vec<ActivateInstruction>,
    /// Instructions for terminating elements  
    terminate_instructions: Vec<TerminateInstruction>,
    /// Optional reference key for tracking
    operation_reference: Option<u64>,
    _state: std::marker::PhantomData<T>,
}

impl<T: ModifyProcessInstanceState> ModifyProcessInstanceRequest<T> {
    /// Creates a new ModifyProcessInstanceRequest in its initial state
    pub(crate) fn new(client: Client) -> ModifyProcessInstanceRequest<Initial> {
        ModifyProcessInstanceRequest {
            client,
            process_instance_key: 0,
            activate_instructions: vec![],
            terminate_instructions: vec![],
            operation_reference: None,
            _state: std::marker::PhantomData,
        }
    }

    /// Sets a reference key for tracking this operation
    pub fn with_operation_reference(mut self, operation_reference: u64) -> Self {
        self.operation_reference = Some(operation_reference);
        self
    }

    /// Internal helper to transition between builder states
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
    /// Sets the process instance key identifying which instance to modify
    pub fn with_process_instance_key(
        mut self,
        process_instance_key: i64,
    ) -> ModifyProcessInstanceRequest<WithProcessInstance> {
        self.process_instance_key = process_instance_key;
        self.transition()
    }
}

impl ModifyProcessInstanceRequest<WithProcessInstance> {
    /// Starts building an instruction to activate a specific element
    ///
    /// # Arguments
    /// * `element_id` - ID of the element to activate
    /// * `ancestor_element_instance_key` - Key of the ancestor scope
    pub fn with_activate_instruction(
        self,
        element_id: String,
        ancestor_element_instance_key: i64,
    ) -> ActivateInstructionBuilder {
        ActivateInstructionBuilder::new(self, element_id, ancestor_element_instance_key)
    }

    /// Adds an instruction to terminate a specific element instance
    ///
    /// # Arguments
    /// * `element_instance_key` - Key of the element instance to terminate
    pub fn with_terminate_instruction(mut self, element_instance_key: i64) -> Self {
        self.terminate_instructions.push(TerminateInstruction {
            element_instance_key,
        });
        self
    }

    /// Adds instructions to terminate multiple element instances
    ///
    /// # Arguments
    /// * `element_instance_keys` - Keys of element instances to terminate
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

    /// Sends the process instance modification request to the Zeebe workflow engine
    ///
    /// # Errors
    /// - NOT_FOUND: No process instance exists with the given key
    /// - INVALID_ARGUMENT: Invalid instructions or variables provided
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

/// Response from modifying a process instance
#[derive(Debug, Clone)]
pub struct ModifyProcessInstanceResponse {}

impl From<proto::ModifyProcessInstanceResponse> for ModifyProcessInstanceResponse {
    fn from(_value: proto::ModifyProcessInstanceResponse) -> ModifyProcessInstanceResponse {
        ModifyProcessInstanceResponse {}
    }
}
