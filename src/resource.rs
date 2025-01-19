use crate::proto;
use crate::{Client, ClientError};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DeployResourceError {
    #[error("failed to load file {file_path:?} {source:?}")]
    ResourceLoad {
        file_path: PathBuf,
        source: std::io::Error,
    },

    #[error("missing name {file_path:?}")]
    MissingName { file_path: PathBuf },
}

#[derive(Default)]
pub struct Initial;

pub struct WithFile;
pub struct WithDefinition;
pub struct WithKey;

pub trait DeployResourceState {}
impl DeployResourceState for Initial {}
impl DeployResourceState for WithFile {}
impl DeployResourceState for WithDefinition {}

#[derive(Debug)]
pub struct DeployResourceRequest<T: DeployResourceState> {
    client: Client,
    resource_file_paths: Option<Vec<PathBuf>>,
    resource_definitions: Vec<(String, Vec<u8>)>,
    tenant_id: Option<String>,
    _state: std::marker::PhantomData<T>,
}

impl<T: DeployResourceState> DeployResourceRequest<T> {
    pub fn new(client: Client) -> DeployResourceRequest<Initial> {
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
    pub fn with_resource_file(mut self, file_path: PathBuf) -> DeployResourceRequest<WithFile> {
        self.resource_file_paths = Some(vec![file_path]);
        self.transition()
    }

    pub fn with_resource_files(
        mut self,
        file_paths: Vec<PathBuf>,
    ) -> DeployResourceRequest<WithFile> {
        self.resource_file_paths = Some(file_paths);
        self.transition()
    }

    pub fn with_definition(
        mut self,
        name: String,
        definition: Vec<u8>,
    ) -> DeployResourceRequest<WithDefinition> {
        self.resource_definitions.push((name, definition));
        self.transition()
    }

    pub fn with_definitions(
        mut self,
        mut definitions: Vec<(String, Vec<u8>)>,
    ) -> DeployResourceRequest<WithDefinition> {
        self.resource_definitions.append(&mut definitions);
        self.transition()
    }
}

impl DeployResourceRequest<WithFile> {
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
    pub fn with_tenant(mut self, tenant_id: String) -> DeployResourceRequest<WithDefinition> {
        self.tenant_id = Some(tenant_id);
        self
    }

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

impl ProcessMetadata {
    pub fn bpmn_process_id(&self) -> &str {
        &self.bpmn_process_id
    }

    pub fn version(&self) -> i32 {
        self.version
    }

    pub fn process_definition_key(&self) -> i64 {
        self.process_definition_key
    }

    pub fn resource_name(&self) -> &str {
        &self.resource_name
    }

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
    dmn_decision_id: String,
    dmn_decision_name: String,
    version: i32,
    decision_key: i64,
    dmn_decision_requirement_id: String,
    decision_requirements_key: i64,
    tenant_id: String,
}

impl DecisionMetadata {
    pub fn dmn_decision_id(&self) -> &str {
        &self.dmn_decision_id
    }

    pub fn dmn_decision_name(&self) -> &str {
        &self.dmn_decision_name
    }

    pub fn version(&self) -> i32 {
        self.version
    }

    pub fn decision_key(&self) -> i64 {
        self.decision_key
    }

    pub fn dmn_decision_requirement_id(&self) -> &str {
        &self.dmn_decision_requirement_id
    }

    pub fn decision_requirements_key(&self) -> i64 {
        self.decision_requirements_key
    }

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
    dmn_decision_requirements_id: String,
    dmn_decision_requirements_name: String,
    version: i32,
    decision_requirements_key: i64,
    resource_name: String,
    tenant_id: String,
}

impl DecisionRequirementsMetadata {
    pub fn dmn_decision_requirements_id(&self) -> &str {
        &self.dmn_decision_requirements_id
    }

    pub fn dmn_decision_requirements_name(&self) -> &str {
        &self.dmn_decision_requirements_name
    }

    pub fn version(&self) -> i32 {
        self.version
    }

    pub fn decision_requirements_key(&self) -> i64 {
        self.decision_requirements_key
    }

    pub fn resource_name(&self) -> &str {
        &self.resource_name
    }

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

#[derive(Debug, Clone)]
pub struct FormMetadata {
    form_id: String,
    version: i32,
    form_key: i64,
    resource_name: String,
    tenant_id: String,
}

impl FormMetadata {
    pub fn form_id(&self) -> &str {
        &self.form_id
    }

    pub fn version(&self) -> i32 {
        self.version
    }

    pub fn form_key(&self) -> i64 {
        self.form_key
    }

    pub fn resource_name(&self) -> &str {
        &self.resource_name
    }

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

#[derive(Debug, Clone)]
pub struct Deployment {
    metadata: Option<Metadata>,
}

impl Deployment {
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

#[derive(Debug, Clone)]
pub struct DeployResourceResponse {
    key: i64,
    deployments: Vec<Deployment>,
    tenant_id: String,
}

impl DeployResourceResponse {
    pub fn key(&self) -> i64 {
        self.key
    }

    pub fn deployments(&self) -> &Vec<Deployment> {
        &self.deployments
    }

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

#[derive(Debug, Clone)]
pub struct DeleteResourceRequest<T> {
    client: Client,
    resource_key: i64,
    operation_reference: Option<u64>,
    _state: std::marker::PhantomData<T>,
}

pub trait DeleteResourceRequestState {}
impl DeleteResourceRequestState for Initial {}
impl DeleteResourceRequestState for WithKey {}

impl<T: DeleteResourceRequestState> DeleteResourceRequest<T> {
    pub(crate) fn new(client: Client) -> DeleteResourceRequest<Initial> {
        DeleteResourceRequest {
            client,
            resource_key: 0,
            operation_reference: None,
            _state: std::marker::PhantomData,
        }
    }

    pub fn with_operation_reference(mut self, operation_reference: u64) -> Self {
        self.operation_reference = Some(operation_reference);
        self
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
    pub fn with_resource_key(mut self, resource_key: i64) -> DeleteResourceRequest<WithKey> {
        self.resource_key = resource_key;
        self.transition()
    }
}

impl DeleteResourceRequest<WithKey> {
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

#[derive(Debug, Clone)]
pub struct DeleteResourceResponse {}

impl From<proto::DeleteResourceResponse> for DeleteResourceResponse {
    fn from(_value: proto::DeleteResourceResponse) -> DeleteResourceResponse {
        DeleteResourceResponse {}
    }
}
