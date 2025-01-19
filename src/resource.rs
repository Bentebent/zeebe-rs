use crate::proto;
use crate::{Client, ClientError};
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during resource deployment
#[derive(Error, Debug)]
pub enum DeployResourceError {
    /// Failed to load resource file from filesystem
    #[error("failed to load file {file_path:?} {source:?}")]
    ResourceLoad {
        /// Path to file that failed to load
        file_path: PathBuf,
        source: std::io::Error,
    },

    /// Resource file name could not be determined
    #[error("missing name {file_path:?}")]
    MissingName {
        /// Path to file missing a name
        file_path: PathBuf,
    },
}

/// State machine types for deployment workflow
#[derive(Default)]
pub struct Initial;

pub struct WithFile;
pub struct WithDefinition;
pub struct WithKey;

pub trait DeployResourceState {}
impl DeployResourceState for Initial {}
impl DeployResourceState for WithFile {}
impl DeployResourceState for WithDefinition {}

/// Request to deploy one or more resources to Zeebe
///
/// Supports:
/// - BPMN process definitions
/// - DMN decision tables
/// - Custom forms
///
/// # State Machine Flow
/// 1. Create request with client
/// 2. Add resources via files or definitions
/// 3. Optionally set tenant ID
/// 4. Send request
///
/// # Notes
/// - Deployment is atomic - all resources succeed or none are deployed
/// - Resources are validated before deployment
///
/// # Errors
/// - PERMISSION_DENIED:
///   - Deployment to unauthorized tenant
/// - INVALID_ARGUMENT:
///   - No resources provided
///   - Invalid resource content (broken XML, invalid BPMN/DMN)
///   - Missing/invalid tenant ID with multi-tenancy enabled
///   - Tenant ID provided with multi-tenancy disabled
#[derive(Debug)]
pub struct DeployResourceRequest<T: DeployResourceState> {
    client: Client,
    resource_file_paths: Option<Vec<PathBuf>>,
    resource_definitions: Vec<(String, Vec<u8>)>,
    tenant_id: Option<String>,
    _state: std::marker::PhantomData<T>,
}

impl<T: DeployResourceState> DeployResourceRequest<T> {
    /// Creates a new deploy resource request in initial state
    pub(crate) fn new(client: Client) -> DeployResourceRequest<Initial> {
        DeployResourceRequest {
            client,
            resource_file_paths: None,
            resource_definitions: vec![],
            tenant_id: None,
            _state: std::marker::PhantomData,
        }
    }

    /// Internal helper to transition between states
    fn transition<NewState: DeployResourceState>(self) -> DeployResourceRequest<NewState> {
        DeployResourceRequest {
            client: self.client,
            resource_file_paths: self.resource_file_paths,
            resource_definitions: self.resource_definitions,
            tenant_id: None,
            _state: std::marker::PhantomData,
        }
    }
}

impl DeployResourceRequest<Initial> {
    /// Adds a single resource file to deploy
    ///
    /// # Arguments
    /// * `file_path` - Path to BPMN, DMN or Form file
    pub fn with_resource_file(mut self, file_path: PathBuf) -> DeployResourceRequest<WithFile> {
        self.resource_file_paths = Some(vec![file_path]);
        self.transition()
    }

    /// Adds multiple resource files to deploy
    ///
    /// # Arguments  
    /// * `file_paths` - Paths to BPMN, DMN or Form files
    pub fn with_resource_files(
        mut self,
        file_paths: Vec<PathBuf>,
    ) -> DeployResourceRequest<WithFile> {
        self.resource_file_paths = Some(file_paths);
        self.transition()
    }

    /// Adds a single resource definition to deploy
    ///
    /// # Arguments
    /// * `name` - Resource name (e.g. process.bpmn)
    /// * `definition` - Raw resource content bytes
    pub fn with_definition(
        mut self,
        name: String,
        definition: Vec<u8>,
    ) -> DeployResourceRequest<WithDefinition> {
        self.resource_definitions.push((name, definition));
        self.transition()
    }

    /// Adds multiple resource definitions to deploy
    ///
    /// # Arguments
    /// * `definitions` - List of (name, content) pairs for resources
    pub fn with_definitions(
        mut self,
        mut definitions: Vec<(String, Vec<u8>)>,
    ) -> DeployResourceRequest<WithDefinition> {
        self.resource_definitions.append(&mut definitions);
        self.transition()
    }
}

impl DeployResourceRequest<WithFile> {
    /// Reads resource files and transitions to definition state
    ///
    /// # Errors
    /// - ResourceLoad: Failed to read file
    /// - MissingName: File name missing
    pub fn read_resource_files(
        mut self,
    ) -> Result<DeployResourceRequest<WithDefinition>, DeployResourceError> {
        if let Some(resource_files) = self.resource_file_paths.take() {
            let contents: Result<Vec<(String, Vec<u8>)>, DeployResourceError> = resource_files
                .into_iter()
                .map(|file_path| {
                    let def = std::fs::read(file_path.clone()).map_err(|source| {
                        DeployResourceError::ResourceLoad {
                            file_path: file_path.clone(),
                            source,
                        }
                    })?;

                    let name = file_path
                        .file_name()
                        .ok_or(DeployResourceError::MissingName {
                            file_path: file_path.clone(),
                        })?;
                    Ok((name.to_string_lossy().into_owned(), def))
                })
                .collect();
            self.resource_definitions = contents?;
        }

        Ok(self.transition())
    }
}

impl DeployResourceRequest<WithDefinition> {
    /// Sets the tenant ID that will own the deployed resources
    ///
    /// # Arguments
    /// * `tenant_id` - ID of tenant that will own these resources
    ///
    /// # Notes
    /// - Required when multi-tenancy is enabled
    /// - Must not be set when multi-tenancy is disabled  
    pub fn with_tenant(mut self, tenant_id: String) -> DeployResourceRequest<WithDefinition> {
        self.tenant_id = Some(tenant_id);
        self
    }

    /// Sends the deploy resource request to the gateway
    ///
    /// # Returns
    /// Response containing:
    /// - Deployment key
    /// - List of deployed resources with metadata
    /// - Tenant ID
    ///
    /// # Errors
    /// - PERMISSION_DENIED: Deployment to unauthorized tenant
    /// - INVALID_ARGUMENT:
    ///   - No resources provided
    ///   - Invalid resource content
    ///   - Missing/invalid tenant ID with multi-tenancy enabled
    ///   - Tenant ID provided with multi-tenancy disabled
    pub async fn send(mut self) -> Result<DeployResourceResponse, ClientError> {
        let resources: Vec<_> = self
            .resource_definitions
            .into_iter()
            .map(|(name, content)| proto::Resource { name, content })
            .collect();

        let tenant_id = self.tenant_id.unwrap_or_default();

        let res = self
            .client
            .gateway_client
            .deploy_resource(proto::DeployResourceRequest {
                resources,
                tenant_id,
            })
            .await?;

        Ok(res.into_inner().into())
    }
}

#[derive(Debug, Clone)]
pub struct ProcessMetadata {
    /// Process ID that uniquely identifies this process definition
    bpmn_process_id: String,
    /// Version of this process definition
    version: i32,
    /// Unique key assigned to this process definition
    process_definition_key: i64,
    /// Name of resource file this was deployed from
    resource_name: String,
    /// ID of tenant that owns this process definition
    tenant_id: String,
}

impl ProcessMetadata {
    /// Returns the process ID that uniquely identifies this process definition
    ///
    /// The ID is defined in the BPMN XML via the 'id' attribute
    pub fn bpmn_process_id(&self) -> &str {
        &self.bpmn_process_id
    }

    /// Returns the version of this process definition
    ///
    /// Version is auto-incremented when deploying a process with same ID
    pub fn version(&self) -> i32 {
        self.version
    }

    /// Returns the unique key assigned to this process definition by Zeebe
    ///
    /// Key is globally unique across the cluster
    pub fn process_definition_key(&self) -> i64 {
        self.process_definition_key
    }

    /// Returns the name of the resource file this was deployed from
    ///
    /// Usually ends with .bpmn extension
    pub fn resource_name(&self) -> &str {
        &self.resource_name
    }

    /// Returns the ID of tenant that owns this process definition
    ///
    /// Empty if multi-tenancy is disabled
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }
}

impl From<proto::ProcessMetadata> for ProcessMetadata {
    fn from(value: proto::ProcessMetadata) -> ProcessMetadata {
        ProcessMetadata {
            bpmn_process_id: value.bpmn_process_id,
            version: value.version,
            process_definition_key: value.process_definition_key,
            resource_name: value.resource_name,
            tenant_id: value.tenant_id,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DecisionMetadata {
    /// ID that uniquely identifies this decision
    dmn_decision_id: String,
    /// Human-readable name of this decision
    dmn_decision_name: String,
    /// Version of this decision
    version: i32,
    /// Unique key assigned by Zeebe
    decision_key: i64,
    /// ID of decision requirements graph this belongs to
    dmn_decision_requirement_id: String,
    /// Key of decision requirements graph this belongs to
    decision_requirements_key: i64,
    /// ID of tenant that owns this decision
    tenant_id: String,
}

impl DecisionMetadata {
    /// Returns the unique identifier for this DMN decision
    ///
    /// The ID is defined in the DMN XML via the 'id' attribute
    pub fn dmn_decision_id(&self) -> &str {
        &self.dmn_decision_id
    }

    /// Returns the human-readable name for this DMN decision
    ///
    /// The name is defined in the DMN XML via the 'name' attribute
    pub fn dmn_decision_name(&self) -> &str {
        &self.dmn_decision_name
    }

    /// Returns the version of this decision definition
    ///
    /// Version is auto-incremented when deploying a decision with same ID
    pub fn version(&self) -> i32 {
        self.version
    }

    /// Returns the unique key assigned to this decision by Zeebe
    ///
    /// Key is globally unique across the cluster
    pub fn decision_key(&self) -> i64 {
        self.decision_key
    }

    /// Returns the ID of the decision requirements graph this belongs to
    ///
    /// Links to the parent DRG that contains this decision
    pub fn dmn_decision_requirement_id(&self) -> &str {
        &self.dmn_decision_requirement_id
    }

    /// Returns the key of the decision requirements graph this belongs to
    ///
    /// Links to the parent DRG via its unique key
    pub fn decision_requirements_key(&self) -> i64 {
        self.decision_requirements_key
    }

    /// Returns the ID of tenant that owns this decision
    ///
    /// Empty if multi-tenancy is disabled
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }
}

impl From<proto::DecisionMetadata> for DecisionMetadata {
    fn from(value: proto::DecisionMetadata) -> DecisionMetadata {
        DecisionMetadata {
            dmn_decision_id: value.dmn_decision_id,
            dmn_decision_name: value.dmn_decision_name,
            version: value.version,
            decision_key: value.decision_key,
            dmn_decision_requirement_id: value.dmn_decision_requirements_id,
            decision_requirements_key: value.decision_requirements_key,
            tenant_id: value.tenant_id,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DecisionRequirementsMetadata {
    /// ID that uniquely identifies this decision requirements graph
    dmn_decision_requirements_id: String,
    /// Human-readable name of this decision requirements graph
    dmn_decision_requirements_name: String,
    /// Version of this decision requirements graph
    version: i32,
    /// Unique key assigned by Zeebe
    decision_requirements_key: i64,
    /// Name of resource file this was deployed from
    resource_name: String,
    /// ID of tenant that owns this decision requirements graph
    tenant_id: String,
}

impl DecisionRequirementsMetadata {
    /// Returns the unique identifier for this decision requirements graph
    ///
    /// The ID is defined in the DMN XML via the 'id' attribute
    pub fn dmn_decision_requirements_id(&self) -> &str {
        &self.dmn_decision_requirements_id
    }

    /// Returns the human-readable name for this decision requirements graph
    ///
    /// The name is defined in the DMN XML via the 'name' attribute
    pub fn dmn_decision_requirements_name(&self) -> &str {
        &self.dmn_decision_requirements_name
    }

    /// Returns the version of this decision requirements graph
    ///
    /// Version is auto-incremented when deploying a DRG with same ID
    pub fn version(&self) -> i32 {
        self.version
    }

    /// Returns the unique key assigned to this DRG by Zeebe
    ///
    /// Key is globally unique across the cluster
    pub fn decision_requirements_key(&self) -> i64 {
        self.decision_requirements_key
    }

    /// Returns the name of the resource file this was deployed from
    ///
    /// Usually ends with .dmn extension
    pub fn resource_name(&self) -> &str {
        &self.resource_name
    }

    /// Returns the ID of tenant that owns this DRG
    ///
    /// Empty if multi-tenancy is disabled
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }
}

impl From<proto::DecisionRequirementsMetadata> for DecisionRequirementsMetadata {
    fn from(value: proto::DecisionRequirementsMetadata) -> DecisionRequirementsMetadata {
        DecisionRequirementsMetadata {
            dmn_decision_requirements_id: value.dmn_decision_requirements_id,
            dmn_decision_requirements_name: value.dmn_decision_requirements_name,
            version: value.version,
            decision_requirements_key: value.decision_requirements_key,
            resource_name: value.resource_name,
            tenant_id: value.tenant_id,
        }
    }
}

/// Metadata for a deployed form
#[derive(Debug, Clone)]
pub struct FormMetadata {
    /// ID that uniquely identifies this form
    form_id: String,
    /// Version of this form
    version: i32,
    /// Unique key assigned by Zeebe
    form_key: i64,
    /// Name of resource file this was deployed from
    resource_name: String,
    /// ID of tenant that owns this form
    tenant_id: String,
}

impl FormMetadata {
    /// Returns the unique form identifier
    pub fn form_id(&self) -> &str {
        &self.form_id
    }

    /// Returns this form's version
    pub fn version(&self) -> i32 {
        self.version
    }

    /// Returns the unique key assigned by Zeebe
    pub fn form_key(&self) -> i64 {
        self.form_key
    }

    /// Returns the name of the resource file this was deployed from
    pub fn resource_name(&self) -> &str {
        &self.resource_name
    }

    /// Returns the ID of the tenant that owns this form
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }
}

impl From<proto::FormMetadata> for FormMetadata {
    fn from(value: proto::FormMetadata) -> FormMetadata {
        FormMetadata {
            form_id: value.form_id,
            version: value.version,
            form_key: value.form_key,
            resource_name: value.resource_name,
            tenant_id: value.tenant_id,
        }
    }
}

/// Metadata for a deployed resource
#[derive(Debug, Clone)]
pub enum Metadata {
    /// Metadata for a BPMN process definition
    Process(ProcessMetadata),
    /// Metadata for a DMN decision
    Decision(DecisionMetadata),
    /// Metadata for a DMN decision requirements graph
    DecisionRequirements(DecisionRequirementsMetadata),
    /// Metadata for a form
    Form(FormMetadata),
}

impl From<proto::deployment::Metadata> for Metadata {
    fn from(value: proto::deployment::Metadata) -> Metadata {
        match value {
            proto::deployment::Metadata::Process(p) => Metadata::Process(p.into()),
            proto::deployment::Metadata::Decision(d) => Metadata::Decision(d.into()),
            proto::deployment::Metadata::DecisionRequirements(dr) => {
                Metadata::DecisionRequirements(dr.into())
            }
            proto::deployment::Metadata::Form(f) => Metadata::Form(f.into()),
        }
    }
}

/// A successfully deployed resource
#[derive(Debug, Clone)]
pub struct Deployment {
    /// Metadata for this deployed resource
    metadata: Option<Metadata>,
}

impl Deployment {
    /// Returns the metadata for this deployed resource if available
    pub fn metadata(&self) -> Option<&Metadata> {
        self.metadata.as_ref()
    }
}

impl From<proto::Deployment> for Deployment {
    fn from(value: proto::Deployment) -> Deployment {
        Deployment {
            metadata: value.metadata.map(|m| m.into()),
        }
    }
}

/// Response from deploying one or more resources
#[derive(Debug, Clone)]
pub struct DeployResourceResponse {
    /// Unique key for this deployment operation
    key: i64,
    /// List of successfully deployed resources
    deployments: Vec<Deployment>,
    /// ID of tenant that owns these resources
    tenant_id: String,
}

impl DeployResourceResponse {
    /// Returns the unique key for this deployment operation
    pub fn key(&self) -> i64 {
        self.key
    }

    /// Returns the list of successfully deployed resources
    pub fn deployments(&self) -> &Vec<Deployment> {
        &self.deployments
    }

    /// Returns the ID of tenant that owns these resources
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }
}

impl From<proto::DeployResourceResponse> for DeployResourceResponse {
    fn from(value: proto::DeployResourceResponse) -> DeployResourceResponse {
        DeployResourceResponse {
            key: value.key,
            deployments: value.deployments.into_iter().map(|d| d.into()).collect(),
            tenant_id: value.tenant_id,
        }
    }
}

/// Marker trait for delete resource state machine
pub trait DeleteResourceRequestState {}
impl DeleteResourceRequestState for Initial {}
impl DeleteResourceRequestState for WithKey {}

/// Request to delete a deployed resource (process, decision, or form)
///
/// # Effects
/// - Process deletion cancels running instances
/// - New instances use latest non-deleted version
/// - No instances possible when all versions deleted
/// - Decision deletion may cause incidents
///
/// # Errors
/// - NOT_FOUND: No resource exists with given key
#[derive(Debug, Clone)]
pub struct DeleteResourceRequest<T> {
    client: Client,
    resource_key: i64,
    operation_reference: Option<u64>,
    _state: std::marker::PhantomData<T>,
}

impl<T: DeleteResourceRequestState> DeleteResourceRequest<T> {
    /// Creates new delete resource request in initial state
    pub(crate) fn new(client: Client) -> DeleteResourceRequest<Initial> {
        DeleteResourceRequest {
            client,
            resource_key: 0,
            operation_reference: None,
            _state: std::marker::PhantomData,
        }
    }

    /// Sets a reference ID to correlate this operation with other events
    ///
    /// # Arguments
    /// * `operation_reference` - Unique identifier for correlation
    pub fn with_operation_reference(mut self, operation_reference: u64) -> Self {
        self.operation_reference = Some(operation_reference);
        self
    }

    /// Internal helper to transition between states
    fn transition<NewState: DeleteResourceRequestState>(self) -> DeleteResourceRequest<NewState> {
        DeleteResourceRequest {
            client: self.client,
            resource_key: self.resource_key,
            operation_reference: self.operation_reference,
            _state: std::marker::PhantomData,
        }
    }
}

impl DeleteResourceRequest<Initial> {
    /// Sets the key of the resource to delete
    ///
    /// # Arguments
    /// * `resource_key` - Key of process, decision, or form to delete
    pub fn with_resource_key(mut self, resource_key: i64) -> DeleteResourceRequest<WithKey> {
        self.resource_key = resource_key;
        self.transition()
    }
}

impl DeleteResourceRequest<WithKey> {
    /// Sends the delete resource request to the gateway
    ///
    /// # Errors
    /// - NOT_FOUND: No resource exists with given key
    pub async fn send(mut self) -> Result<DeleteResourceResponse, ClientError> {
        let res = self
            .client
            .gateway_client
            .delete_resource(proto::DeleteResourceRequest {
                resource_key: self.resource_key,
                operation_reference: self.operation_reference,
            })
            .await?;

        Ok(res.into_inner().into())
    }
}

/// Empty response since delete resource operation is fire-and-forget
#[derive(Debug, Clone)]
pub struct DeleteResourceResponse {}

impl From<proto::DeleteResourceResponse> for DeleteResourceResponse {
    fn from(_value: proto::DeleteResourceResponse) -> DeleteResourceResponse {
        DeleteResourceResponse {}
    }
}
