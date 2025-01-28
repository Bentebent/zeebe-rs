use crate::proto;
use crate::{Client, ClientError};
use std::path::PathBuf;
use thiserror::Error;

/// Represents errors that can occure while deploying a resource
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

#[derive(Default, Clone)]
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
/// # Examples
/// ```ignore
/// let result = client
///     .deploy_resource()
///     .with_resource_file(PathBuf::from("./examples/resources/hello_world.bpmn"))
///     .read_resource_files()?
///     .send()
///     .await?;
/// ```
///
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
    pub(crate) fn new(client: Client) -> DeployResourceRequest<Initial> {
        DeployResourceRequest {
            client,
            resource_file_paths: None,
            resource_definitions: vec![],
            tenant_id: None,
            _state: std::marker::PhantomData,
        }
    }

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
    /// Adds a single resource file to the deployment request.
    ///
    /// This method allows you to specify a single resource file,
    /// such as a BPMN, DMN, or Form file, to be included in the deployment.
    ///
    /// # Arguments
    ///
    /// * `file_path` - A `PathBuf` representing the path to the resource file.
    ///
    /// # Returns
    ///
    /// A `DeployResourceRequest<WithFile>` instance with the specified resource file added.
    pub fn with_resource_file(mut self, file_path: PathBuf) -> DeployResourceRequest<WithFile> {
        self.resource_file_paths = Some(vec![file_path]);
        self.transition()
    }

    /// Adds multiple resource files to the deployment request.
    ///
    /// This method allows you to specify multiple resource files,
    /// such as BPMN, DMN, or Form files, to be included in the deployment.
    ///
    /// # Arguments
    ///
    /// * `file_paths` - A `Vec<PathBuf>` containing the paths to the resource files.
    ///
    /// # Returns
    ///
    /// A `DeployResourceRequest<WithFile>` instance with the specified resource files added.
    pub fn with_resource_files(
        mut self,
        file_paths: Vec<PathBuf>,
    ) -> DeployResourceRequest<WithFile> {
        self.resource_file_paths = Some(file_paths);
        self.transition()
    }

    /// Adds a single resource definition to the deployment request.
    ///
    /// This method allows you to specify a single resource definition,
    /// including its name and raw content bytes, to be included in the deployment.
    ///
    /// # Arguments
    ///
    /// * `name` - A `String` representing the resource name (e.g., `process.bpmn`).
    /// * `definition` - A `Vec<u8>` containing the raw content bytes of the resource.
    ///
    /// # Returns
    ///
    /// A `DeployResourceRequest<WithDefinition>` instance with the specified resource definition added.
    pub fn with_definition(
        mut self,
        name: String,
        definition: Vec<u8>,
    ) -> DeployResourceRequest<WithDefinition> {
        self.resource_definitions.push((name, definition));
        self.transition()
    }

    /// Adds multiple resource definitions to the deployment request.
    ///
    /// This method allows you to specify multiple resource definitions,
    /// each including its name and raw content bytes, to be included in the deployment.
    ///
    /// # Arguments
    ///
    /// * `definitions` - A `Vec<(String, Vec<u8>)>` containing pairs of resource names and their raw content bytes.
    ///
    /// # Returns
    ///
    /// A `DeployResourceRequest<WithDefinition>` instance with the specified resource definitions added.
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
    ) -> Result<DeployResourceRequest<WithDefinition>, ClientError> {
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
    /// Sets the tenant ID that will own the deployed resources.
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - A `String` representing the ID of the tenant that will own these resources.
    ///
    /// # Notes
    ///
    /// - This field is required when multi-tenancy is enabled.
    /// - This field must not be set when multi-tenancy is disabled.
    pub fn with_tenant(mut self, tenant_id: String) -> DeployResourceRequest<WithDefinition> {
        self.tenant_id = Some(tenant_id);
        self
    }

    /// Sends the deploy resource request to the gateway.
    ///
    /// # Returns
    ///
    /// A `Result` containing:
    /// - `Ok(DeployResourceResponse)` on success, which includes:
    ///   - Deployment key
    ///   - List of deployed resources with metadata
    ///   - Tenant ID
    /// - `Err(ClientError)` on failure, with possible errors:
    ///   - `PERMISSION_DENIED`: Deployment to an unauthorized tenant.
    ///   - `INVALID_ARGUMENT`:
    ///     - No resources provided.
    ///     - Invalid resource content.
    ///     - Missing or invalid tenant ID when multi-tenancy is enabled.
    ///     - Tenant ID provided when multi-tenancy is disabled.
    ///
    /// # Errors
    ///
    /// This function will return an error if the request fails due to permission issues or invalid arguments.
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
    bpmn_process_id: String,
    version: i32,
    process_definition_key: i64,
    resource_name: String,
    tenant_id: String,
}

/// Metadata information for a deployed BPMN process definition.
///
/// This struct encapsulates the identifying information and metadata for a process
/// definition deployed to a Zeebe workflow engine. Each process definition is
/// uniquely identified by a combination of its BPMN process ID and version number.
impl ProcessMetadata {
    /// Returns the BPMN process ID.
    ///
    /// # Returns
    ///
    /// A string slice representing the BPMN process ID.
    pub fn bpmn_process_id(&self) -> &str {
        &self.bpmn_process_id
    }

    /// Returns the version of this process definition.
    ///
    /// # Returns
    ///
    /// An integer representing the version of the process definition.
    pub fn version(&self) -> i32 {
        self.version
    }

    /// Returns the unique key assigned to this process definition by Zeebe.
    ///
    /// # Returns
    ///
    /// A 64-bit integer representing the unique key of the process definition.
    pub fn process_definition_key(&self) -> i64 {
        self.process_definition_key
    }

    /// Returns the name of the resource this process was deployed from.
    ///
    /// # Returns
    ///
    /// A string slice representing the name of the resource file.
    pub fn resource_name(&self) -> &str {
        &self.resource_name
    }

    /// Returns the ID of the tenant that owns this process definition.
    ///
    /// The tenant ID is used in multi-tenant setups to segregate process
    /// definitions by tenant. If multi-tenancy is disabled, this will be an
    /// empty string.
    ///
    /// # Returns
    ///
    /// A string slice representing the tenant ID.
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

/// Metadata information for a deployed DMN decision definition.
#[derive(Debug, Clone)]
pub struct DecisionMetadata {
    dmn_decision_id: String,
    dmn_decision_name: String,
    version: i32,
    decision_key: i64,
    dmn_decision_requirement_id: String,
    decision_requirements_key: i64,
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

/// Metadata information for a deployed DMN decision requirement definition.
#[derive(Debug, Clone)]
pub struct DecisionRequirementsMetadata {
    dmn_decision_requirements_id: String,
    dmn_decision_requirements_name: String,
    version: i32,
    decision_requirements_key: i64,
    resource_name: String,
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
    form_id: String,
    version: i32,
    form_key: i64,
    resource_name: String,
    tenant_id: String,
}

impl FormMetadata {
    /// Returns the unique identifier for the form.
    ///
    /// # Returns
    /// A string slice representing the form's unique identifier.
    pub fn form_id(&self) -> &str {
        &self.form_id
    }

    /// Returns the version of the form.
    ///
    /// # Returns
    /// An integer representing the form's version.
    pub fn version(&self) -> i32 {
        self.version
    }

    /// Returns the unique key assigned by Zeebe.
    ///
    /// # Returns
    /// A 64-bit integer representing the unique key.
    pub fn form_key(&self) -> i64 {
        self.form_key
    }

    /// Returns the name of the resource file from which this form was deployed.
    ///
    /// # Returns
    /// A string slice representing the resource file name.
    pub fn resource_name(&self) -> &str {
        &self.resource_name
    }

    /// Returns the ID of the tenant that owns this form.
    ///
    /// # Returns
    /// A string slice representing the tenant's ID.
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
    Process(ProcessMetadata),
    Decision(DecisionMetadata),
    DecisionRequirements(DecisionRequirementsMetadata),
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
    metadata: Option<Metadata>,
}

impl Deployment {
    /// Retrieves the metadata associated with this deployed resource, if available.
    ///
    /// # Returns
    ///
    /// An `Option` containing a reference to the `Metadata` if it exists, or `None` if the metadata is not available.
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
    key: i64,
    deployments: Vec<Deployment>,
    tenant_id: String,
}

impl DeployResourceResponse {
    /// Returns the unique key associated with this deployment operation.
    ///
    /// # Returns
    ///
    /// An `i64` representing the unique key.
    pub fn key(&self) -> i64 {
        self.key
    }

    /// Returns a slice of `Deployment` representing the successfully deployed resources.
    ///
    /// # Returns
    ///
    /// A slice of `Deployment` structs.
    pub fn deployments(&self) -> &[Deployment] {
        &self.deployments
    }

    /// Returns the ID of the tenant that owns these resources.
    ///
    /// # Returns
    ///
    /// A string slice representing the tenant ID.
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

pub trait DeleteResourceRequestState {}
impl DeleteResourceRequestState for Initial {}
impl DeleteResourceRequestState for WithKey {}

/// Request to delete a deployed resource in Zeebe
///
/// # Examples
///
/// ```ignore
/// let response = client
///     .delete_resource()
///     .with_resource_key(12345)
///     .send()
///     .await?;
/// ```
///
/// # Errors
///
/// Sending a delete request may result in the following errors:
/// - `NOT_FOUND`: No resource exists with the given key.
///
/// # Notes
///
/// The delete resource operation is fire-and-forget, meaning there is no detailed response.
#[derive(Debug, Clone)]
pub struct DeleteResourceRequest<T> {
    client: Client,
    resource_key: i64,
    operation_reference: Option<u64>,
    _state: std::marker::PhantomData<T>,
}

impl<T: DeleteResourceRequestState> DeleteResourceRequest<T> {
    pub(crate) fn new(client: Client) -> DeleteResourceRequest<Initial> {
        DeleteResourceRequest {
            client,
            resource_key: 0,
            operation_reference: None,
            _state: std::marker::PhantomData,
        }
    }

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

    /// Sets a reference ID to correlate this operation with other events
    ///
    /// # Arguments
    /// * `operation_reference` - Unique identifier for correlation
    pub fn with_operation_reference(mut self, operation_reference: u64) -> Self {
        self.operation_reference = Some(operation_reference);
        self
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
