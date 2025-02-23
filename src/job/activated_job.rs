use crate::{ClientError, proto};
use serde::de::DeserializeOwned;

/// Represents an activated job
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
    /// The key, a unique identifier for the job.
    ///
    /// # Returns
    ///
    /// * `i64` - The unique key of the job.
    pub fn key(&self) -> i64 {
        self.key
    }

    /// The type of the job (should match what was requested).
    ///
    /// # Returns
    ///
    /// * `&str` - The type of the job.
    pub fn job_type(&self) -> &str {
        &self.job_type
    }

    /// The job's process instance key.
    ///
    /// # Returns
    ///
    /// * `i64` - The process instance key of the job.
    pub fn process_instance_key(&self) -> i64 {
        self.process_instance_key
    }

    /// The BPMN process ID of the job process definition.
    ///
    /// # Returns
    ///
    /// * `&str` - The BPMN process ID.
    pub fn bpmn_process_id(&self) -> &str {
        &self.bpmn_process_id
    }

    /// The version of the job process definition.
    ///
    /// # Returns
    ///
    /// * `i32` - The version of the process definition.
    pub fn process_definition_version(&self) -> i32 {
        self.process_definition_version
    }

    /// The key of the job process definition.
    ///
    /// # Returns
    ///
    /// * `i64` - The key of the process definition.
    pub fn process_definition_key(&self) -> i64 {
        self.process_definition_key
    }

    /// The associated task element ID.
    ///
    /// # Returns
    ///
    /// * `&str` - The element ID.
    pub fn element_id(&self) -> &str {
        &self.element_id
    }

    /// The unique key identifying the associated task, unique within the scope
    /// of the process instance.
    ///
    /// # Returns
    ///
    /// * `i64` - The element instance key.
    pub fn element_instance_key(&self) -> i64 {
        self.element_instance_key
    }

    /// A set of custom headers defined during modelling; returned as a serialized
    /// JSON document.
    ///
    /// # Returns
    ///
    /// * `&str` - The custom headers as a JSON document.
    pub fn custom_headers(&self) -> &str {
        &self.custom_headers
    }

    /// The name of the worker which activated this job.
    ///
    /// # Returns
    ///
    /// * `&str` - The worker name.
    pub fn worker(&self) -> &str {
        &self.worker
    }

    /// The amount of retries left to this job (should always be positive).
    ///
    /// # Returns
    ///
    /// * `i32` - The number of retries left.
    pub fn retries(&self) -> i32 {
        self.retries
    }

    /// When the job can be activated again, sent as a UNIX epoch timestamp.
    ///
    /// # Returns
    ///
    /// * `i64` - The deadline as a UNIX epoch timestamp.
    pub fn deadline(&self) -> i64 {
        self.deadline
    }

    /// JSON document, computed at activation time, consisting of all visible
    /// variables to the task scope.
    ///
    /// # Returns
    ///
    /// * `&str` - The variables as a JSON document.
    pub fn variables(&self) -> &str {
        &self.variables
    }

    /// Deserializes the variables JSON document into a specified type.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type to deserialize into.
    ///
    /// # Returns
    ///
    /// * `Result<T, ClientError>` - The deserialized data or an error if deserialization fails.
    pub fn data<T: DeserializeOwned>(&self) -> Result<T, ClientError> {
        serde_json::from_str(&self.variables).map_err(|e| ClientError::DeserializationFailed {
            value: self.variables.clone(),
            source: e,
        })
    }

    /// The id of the tenant that owns the job.
    ///
    /// # Returns
    ///
    /// * `&str` - The tenant ID.
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
