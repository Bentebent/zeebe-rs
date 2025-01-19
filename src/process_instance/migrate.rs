use crate::proto;
use crate::Client;
use crate::ClientError;

/// Initial state for the MigrateProcessInstanceRequest builder pattern
#[derive(Debug, Clone)]
pub struct Initial;

/// State indicating process instance has been set
#[derive(Debug, Clone)]
pub struct WithProcessInstance;

/// State indicating migration variables have been configured
#[derive(Debug, Clone)]
pub struct WithVariables;

/// Marker trait for MigrateProcessInstanceRequest states
pub trait MigrateProcessInstanceState {}
impl MigrateProcessInstanceState for Initial {}
impl MigrateProcessInstanceState for WithProcessInstance {}
impl MigrateProcessInstanceState for WithVariables {}

/// Instructions for mapping elements between process definitions
#[derive(Debug, Clone)]
pub struct MappingInstruction {
    /// ID of the source element in the current process
    source_element_id: String,
    /// ID of the target element in the new process
    target_element_id: String,
}

/// Plan defining how to migrate a process instance
#[derive(Debug, Clone)]
pub struct MigrationPlan {
    /// Key of the process definition to migrate to
    target_process_definition_key: i64,
    /// Instructions for mapping elements between processes
    mapping_instructions: Vec<MappingInstruction>,
}

/// Request to migrate a process instance to a different process definition
#[derive(Debug, Clone)]
pub struct MigrateProcessInstanceRequest<T: MigrateProcessInstanceState> {
    client: Client,
    /// Key of the process instance to migrate
    process_instance_key: i64,
    /// Optional migration plan defining the target process and mappings
    migration_plan: Option<MigrationPlan>,
    /// Optional reference key for tracking this operation
    operation_reference: Option<u64>,
    _state: std::marker::PhantomData<T>,
}

impl<T: MigrateProcessInstanceState> MigrateProcessInstanceRequest<T> {
    /// Creates a new MigrateProcessInstanceRequest in its initial state
    pub(crate) fn new(client: Client) -> MigrateProcessInstanceRequest<Initial> {
        MigrateProcessInstanceRequest {
            client,
            process_instance_key: 0,
            migration_plan: None,
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
    fn transition<NewState: MigrateProcessInstanceState>(
        self,
    ) -> MigrateProcessInstanceRequest<NewState> {
        MigrateProcessInstanceRequest {
            client: self.client,
            process_instance_key: self.process_instance_key,
            migration_plan: self.migration_plan,
            operation_reference: self.operation_reference,
            _state: std::marker::PhantomData,
        }
    }
}

impl MigrateProcessInstanceRequest<Initial> {
    /// Sets the process instance key identifying which instance to migrate
    pub fn with_process_instance_key(
        mut self,
        process_instance_key: i64,
    ) -> MigrateProcessInstanceRequest<WithProcessInstance> {
        self.process_instance_key = process_instance_key;
        self.transition()
    }
}

impl MigrateProcessInstanceRequest<WithProcessInstance> {
    /// Sets the migration plan with target process and element mappings
    pub fn with_migration_plan(
        mut self,
        target_process_definition_key: i64,
        mapping_instructions: Vec<MappingInstruction>,
    ) -> MigrateProcessInstanceRequest<WithVariables> {
        self.migration_plan = Some(MigrationPlan {
            target_process_definition_key,
            mapping_instructions,
        });

        self.transition()
    }

    /// Continues without specifying a migration plan
    pub fn without_migration_plan(self) -> MigrateProcessInstanceRequest<WithVariables> {
        self.transition()
    }
}

impl MigrateProcessInstanceRequest<WithVariables> {
    /// Sends the process instance migration request to the Zeebe workflow engine
    pub async fn send(mut self) -> Result<MigrateProcessInstanceResponse, ClientError> {
        let res = self
            .client
            .gateway_client
            .migrate_process_instance(proto::MigrateProcessInstanceRequest {
                process_instance_key: self.process_instance_key,
                migration_plan: self.migration_plan.map(|p| {
                    proto::migrate_process_instance_request::MigrationPlan {
                        target_process_definition_key: p.target_process_definition_key,
                        mapping_instructions: p
                            .mapping_instructions
                            .into_iter()
                            .map(
                                |i| proto::migrate_process_instance_request::MappingInstruction {
                                    source_element_id: i.source_element_id,
                                    target_element_id: i.target_element_id,
                                },
                            )
                            .collect(),
                    }
                }),
                operation_reference: self.operation_reference,
            })
            .await?;

        Ok(res.into_inner().into())
    }
}

/// Response from migrating a process instance
#[derive(Debug, Clone)]
pub struct MigrateProcessInstanceResponse {}

impl From<proto::MigrateProcessInstanceResponse> for MigrateProcessInstanceResponse {
    fn from(_value: proto::MigrateProcessInstanceResponse) -> MigrateProcessInstanceResponse {
        MigrateProcessInstanceResponse {}
    }
}
