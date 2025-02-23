use crate::Client;
use crate::ClientError;
use crate::proto;

#[derive(Debug, Clone)]
pub struct Initial;

#[derive(Debug, Clone)]
pub struct WithProcessInstance;

#[derive(Debug, Clone)]
pub struct WithVariables;

pub trait MigrateProcessInstanceState {}
impl MigrateProcessInstanceState for Initial {}
impl MigrateProcessInstanceState for WithProcessInstance {}
impl MigrateProcessInstanceState for WithVariables {}

/// Instructions for mapping elements between process definitions
#[derive(Debug, Clone)]
pub struct MappingInstruction {
    source_element_id: String,
    target_element_id: String,
}

/// Plan defining how to migrate a process instance
#[derive(Debug, Clone)]
pub struct MigrationPlan {
    target_process_definition_key: i64,
    mapping_instructions: Vec<MappingInstruction>,
}

/// Request to migrate a process instance to a different process definition
/// # Examples
/// ```ignore
/// client
///     .migrate_process_instance()
///     .with_process_instance_key(12356)
///     .without_migration_plan()
///     .send()
///     .await?;
/// ```
#[derive(Debug, Clone)]
pub struct MigrateProcessInstanceRequest<T: MigrateProcessInstanceState> {
    client: Client,
    process_instance_key: i64,
    migration_plan: Option<MigrationPlan>,
    operation_reference: Option<u64>,
    _state: std::marker::PhantomData<T>,
}

impl<T: MigrateProcessInstanceState> MigrateProcessInstanceRequest<T> {
    pub(crate) fn new(client: Client) -> MigrateProcessInstanceRequest<Initial> {
        MigrateProcessInstanceRequest {
            client,
            process_instance_key: 0,
            migration_plan: None,
            operation_reference: None,
            _state: std::marker::PhantomData,
        }
    }

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
    ///
    /// # Arguments
    ///
    /// * `process_instance_key` - The key of the process instance to migrate
    ///
    /// # Returns
    ///
    /// The updated MigrateProcessInstanceRequest in the WithProcessInstance state
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
    ///
    /// # Arguments
    ///
    /// * `target_process_definition_key` - The key of the target process definition
    /// * `mapping_instructions` - The instructions for mapping elements between processes
    ///
    /// # Returns
    ///
    /// The updated MigrateProcessInstanceRequest in the WithVariables state
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
    ///
    /// # Returns
    ///
    /// The updated MigrateProcessInstanceRequest in the WithVariables state
    pub fn without_migration_plan(self) -> MigrateProcessInstanceRequest<WithVariables> {
        self.transition()
    }
}

impl MigrateProcessInstanceRequest<WithVariables> {
    /// Sends the process instance migration request to the Zeebe workflow engine
    ///
    /// # Returns
    ///
    /// A Result containing the MigrateProcessInstanceResponse or a ClientError
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

    /// Sets a reference key for tracking this operation
    ///
    /// # Arguments
    ///
    /// * `operation_reference` - The reference key for tracking the operation
    ///
    /// # Returns
    ///
    /// The updated MigrateProcessInstanceRequest with the operation reference set
    pub fn with_operation_reference(mut self, operation_reference: u64) -> Self {
        self.operation_reference = Some(operation_reference);
        self
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
