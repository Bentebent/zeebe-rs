use crate::proto;
use crate::Client;
use crate::ClientError;

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

#[derive(Debug, Clone)]
pub struct MappingInstruction {
    source_element_id: String,
    target_element_id: String,
}

#[derive(Debug, Clone)]
pub struct MigrationPlan {
    target_process_definition_key: i64,
    mapping_instructions: Vec<MappingInstruction>,
}

#[derive(Debug, Clone)]
pub struct MigrateProcessInstanceRequest<T: MigrateProcessInstanceState> {
    client: Client,
    process_instance_key: i64,
    migration_plan: Option<MigrationPlan>,
    operation_reference: Option<u64>,
    _state: std::marker::PhantomData<T>,
}

impl<T: MigrateProcessInstanceState> MigrateProcessInstanceRequest<T> {
    pub fn new(client: Client) -> MigrateProcessInstanceRequest<Initial> {
        MigrateProcessInstanceRequest {
            client,
            process_instance_key: 0,
            migration_plan: None,
            operation_reference: None,
            _state: std::marker::PhantomData,
        }
    }

    pub fn with_operation_reference(mut self, operation_reference: u64) -> Self {
        self.operation_reference = Some(operation_reference);
        self
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
    pub fn with_process_instance_key(
        mut self,
        process_instance_key: i64,
    ) -> MigrateProcessInstanceRequest<WithProcessInstance> {
        self.process_instance_key = process_instance_key;
        self.transition()
    }
}

impl MigrateProcessInstanceRequest<WithProcessInstance> {
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

    pub fn without_migration_plan(self) -> MigrateProcessInstanceRequest<WithVariables> {
        self.transition()
    }
}

impl MigrateProcessInstanceRequest<WithVariables> {
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

#[derive(Debug, Clone)]
pub struct MigrateProcessInstanceResponse {}

impl From<proto::MigrateProcessInstanceResponse> for MigrateProcessInstanceResponse {
    fn from(_value: proto::MigrateProcessInstanceResponse) -> MigrateProcessInstanceResponse {
        MigrateProcessInstanceResponse {}
    }
}
