use crate::{proto, Client, ClientError};
use serde::Serialize;
use thiserror::Error;

/// Errors that can occur during job completion
#[derive(Error, Debug)]
pub enum CompleteJobError {
    /// Error that occurred during JSON serialization/deserialization
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
}

/// Corrections that can be applied when completing a job
#[derive(Debug, Clone)]
pub struct JobResultCorrections {
    /// New assignee for the job
    assignee: Option<String>,
    /// New due date for the job
    due_date: Option<String>,
    /// New follow-up date for the job
    follow_up_date: Option<String>,
    /// New candidate users for the job
    candidate_users: Option<Vec<String>>,
    /// New candidate groups for the job
    candidate_groups: Option<Vec<String>>,
    /// New priority for the job
    priority: Option<i32>,
}

/// Result of a completed job including possible corrections
#[derive(Debug, Clone)]
pub struct JobResult {
    /// Whether the job completion was denied
    denied: Option<bool>,
    /// Corrections to apply to the job
    corrections: Option<JobResultCorrections>,
}

/// Builder for constructing job results
#[derive(Debug, Clone)]
pub struct JobResultBuilder {
    source_request: CompleteJobRequest<WithKey>,
    job_result: Option<JobResult>,
}

impl JobResultBuilder {
    /// Creates a new JobResultBuilder from a CompleteJobRequest
    fn new(source_request: CompleteJobRequest<WithKey>) -> JobResultBuilder {
        JobResultBuilder {
            source_request,
            job_result: None,
        }
    }

    /// Sets whether the job completion was denied
    pub fn with_denied(mut self, denied: bool) -> Self {
        if let Some(job_result) = self.job_result.as_mut() {
            job_result.denied = Some(denied);
        } else {
            self.job_result = Some(JobResult {
                denied: Some(denied),
                corrections: None,
            });
        }

        self
    }

    /// Internal helper to ensure job result corrections are initialized
    ///
    /// Creates default corrections if they don't exist and returns a mutable reference
    /// to allow modifying individual correction fields.
    fn ensure_corrections(&mut self) -> &mut JobResultCorrections {
        if self.job_result.is_none() {
            self.job_result = Some(JobResult {
                denied: None,
                corrections: Some(JobResultCorrections {
                    assignee: None,
                    due_date: None,
                    follow_up_date: None,
                    candidate_users: None,
                    candidate_groups: None,
                    priority: None,
                }),
            });
        }

        let job_result = self.job_result.as_mut().unwrap();
        if job_result.corrections.is_none() {
            job_result.corrections = Some(JobResultCorrections {
                assignee: None,
                due_date: None,
                follow_up_date: None,
                candidate_users: None,
                candidate_groups: None,
                priority: None,
            });
        }

        job_result.corrections.as_mut().unwrap()
    }

    /// Sets a new assignee for the job
    ///
    /// # Arguments
    /// * `assignee` - The new assignee to be set for the job
    pub fn with_assignee(mut self, assignee: String) -> Self {
        self.ensure_corrections().assignee = Some(assignee);
        self
    }

    /// Sets a new due date for the job
    ///
    /// # Arguments
    /// * `due_date` - The new due date to be set for the job
    pub fn with_due_date(mut self, due_date: String) -> Self {
        self.ensure_corrections().due_date = Some(due_date);
        self
    }

    /// Sets a new follow-up date for the job
    ///
    /// # Arguments
    /// * `follow_up_date` - The new follow-up date to be set for the job
    pub fn with_follow_up_date(mut self, follow_up_date: String) -> Self {
        self.ensure_corrections().follow_up_date = Some(follow_up_date);
        self
    }

    /// Sets new candidate users for the job
    ///
    /// # Arguments
    /// * `candidate_users` - List of user IDs that are candidates for this job
    pub fn with_candidate_users(mut self, candidate_users: Vec<String>) -> Self {
        self.ensure_corrections().candidate_users = Some(candidate_users);
        self
    }

    /// Sets new candidate groups for the job
    ///
    /// # Arguments
    /// * `candidate_groups` - List of group IDs that are candidates for this job
    pub fn with_candidate_groups(mut self, candidate_groups: Vec<String>) -> Self {
        self.ensure_corrections().candidate_groups = Some(candidate_groups);
        self
    }

    /// Sets a new priority for the job
    ///
    /// # Arguments
    /// * `priority` - The new priority value to be set for the job
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.ensure_corrections().priority = Some(priority);
        self
    }

    /// Builds the final CompleteJobRequest with the configured job result
    ///
    /// Consumes the builder and returns the configured request ready for sending
    pub fn build(mut self) -> CompleteJobRequest<WithKey> {
        self.source_request.result = self.job_result;
        self.source_request
    }
}

/// Initial state for the CompleteJobRequest builder pattern
#[derive(Debug, Clone)]
pub struct Initial;

/// State indicating the job key has been set
#[derive(Debug, Clone)]
pub struct WithKey;

/// Marker trait for CompleteJobRequest states
pub trait CompleteJobRequestState {}
impl CompleteJobRequestState for Initial {}
impl CompleteJobRequestState for WithKey {}

/// Request to complete a job
#[derive(Debug, Clone)]
pub struct CompleteJobRequest<T: CompleteJobRequestState> {
    client: Client,
    /// The unique key identifying the job
    job_key: i64,
    /// Variables to be set when completing the job
    variables: serde_json::Value,
    /// Optional result including corrections
    result: Option<JobResult>,
    _state: std::marker::PhantomData<T>,
}

impl<T: CompleteJobRequestState> CompleteJobRequest<T> {
    /// Creates a new CompleteJobRequest in its initial state
    pub(crate) fn new(client: Client) -> CompleteJobRequest<Initial> {
        CompleteJobRequest {
            client,
            job_key: 0,
            variables: serde_json::Value::default(),
            result: None,
            _state: std::marker::PhantomData,
        }
    }

    /// Internal helper to transition between builder states
    fn transition<NewState: CompleteJobRequestState>(self) -> CompleteJobRequest<NewState> {
        CompleteJobRequest {
            client: self.client,
            job_key: self.job_key,
            variables: self.variables,
            result: self.result,
            _state: std::marker::PhantomData,
        }
    }
}

impl CompleteJobRequest<Initial> {
    /// Sets the job key to identify which job to complete
    pub fn with_job_key(mut self, job_key: i64) -> CompleteJobRequest<WithKey> {
        self.job_key = job_key;
        self.transition()
    }
}

impl CompleteJobRequest<WithKey> {
    /// Sets variables to be included with the job completion
    pub fn with_variables<T: Serialize>(mut self, data: T) -> Result<Self, CompleteJobError> {
        self.variables = serde_json::to_value(data)?;
        Ok(self)
    }

    /// Starts building a job result with corrections
    pub fn with_job_result(self) -> JobResultBuilder {
        JobResultBuilder::new(self)
    }

    /// Sends the job completion request to the Zeebe workflow engine
    pub async fn send(mut self) -> Result<CompleteJobResponse, ClientError> {
        let res = self
            .client
            .gateway_client
            .complete_job(proto::CompleteJobRequest {
                job_key: self.job_key,
                variables: self.variables.to_string(),
                result: self.result.map(|x| proto::JobResult {
                    denied: x.denied,
                    corrections: x.corrections.map(|c| proto::JobResultCorrections {
                        assignee: c.assignee,
                        due_date: c.due_date,
                        follow_up_date: c.follow_up_date,
                        candidate_users: c.candidate_users.map(|u| proto::StringList { values: u }),
                        candidate_groups: c
                            .candidate_groups
                            .map(|g| proto::StringList { values: g }),
                        priority: c.priority,
                    }),
                }),
            })
            .await?;

        Ok(res.into_inner().into())
    }
}

/// Response from completing a job
#[derive(Debug, Clone)]
pub struct CompleteJobResponse {}

impl From<proto::CompleteJobResponse> for CompleteJobResponse {
    fn from(_value: proto::CompleteJobResponse) -> CompleteJobResponse {
        CompleteJobResponse {}
    }
}
