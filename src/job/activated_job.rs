use crate::proto;
use serde::de::DeserializeOwned;

#[derive(Debug, Clone)]
pub struct ActivatedJob {
    key: i64,
    job_type: String,
    process_instance_key: i64,
    bpmn_process_id: String,
    process_definition_version: i32,
    process_definition_key: i64,
    element_id: String,
    element_instance_key: i64,
    custom_headers: String,
    worker: String,
    retries: i32,
    deadline: i64,
    variables: String,
    tenant_id: String,
}

impl ActivatedJob {
    pub fn key(&self) -> i64 {
        self.key
    }

    pub fn job_type(&self) -> &str {
        &self.job_type
    }

    pub fn process_instance_key(&self) -> i64 {
        self.process_instance_key
    }

    pub fn bpmn_process_id(&self) -> &str {
        &self.bpmn_process_id
    }

    pub fn process_definition_version(&self) -> i32 {
        self.process_definition_version
    }

    pub fn process_definition_key(&self) -> i64 {
        self.process_definition_key
    }

    pub fn element_id(&self) -> &str {
        &self.element_id
    }

    pub fn element_instance_key(&self) -> i64 {
        self.element_instance_key
    }

    pub fn custom_headers(&self) -> &str {
        &self.custom_headers
    }

    pub fn worker(&self) -> &str {
        &self.worker
    }

    pub fn retries(&self) -> i32 {
        self.retries
    }

    pub fn deadline(&self) -> i64 {
        self.deadline
    }

    pub fn variables(&self) -> &str {
        &self.variables
    }

    pub fn data<T: DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_str(&self.variables)
    }

    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }
}

impl From<proto::ActivatedJob> for ActivatedJob {
    fn from(value: proto::ActivatedJob) -> Self {
        ActivatedJob {
            key: value.key,
            job_type: value.r#type,
            process_instance_key: value.process_instance_key,
            bpmn_process_id: value.bpmn_process_id,
            process_definition_version: value.process_definition_version,
            process_definition_key: value.process_definition_key,
            element_id: value.element_id,
            element_instance_key: value.element_instance_key,
            custom_headers: value.custom_headers,
            worker: value.worker,
            retries: value.retries,
            deadline: value.deadline,
            variables: value.variables,
            tenant_id: value.tenant_id,
        }
    }
}
