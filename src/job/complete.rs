use crate::{proto, Client, ClientError};
use serde::Serialize;

/// Corrections that can be applied when completing a job
#[derive(Debug, Clone)]
pub struct JobResultCorrections {
    assignee: Option<String>,
    due_date: Option<String>,
    follow_up_date: Option<String>,
    candidate_users: Option<Vec<String>>,
    candidate_groups: Option<Vec<String>>,
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
    fn new(source_request: CompleteJobRequest<WithKey>) -> JobResultBuilder {
        JobResultBuilder {
            source_request,
            job_result: None,
        }
    }

    /// Sets whether the job completion was denied
    ///
    /// # Arguments
    ///
    /// * `denied` - A boolean indicating if the job completion was denied
    ///
    /// # Returns
    ///
    /// The updated `JobResultBuilder` instance
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
    ///
    /// # Returns
    ///
    /// A mutable reference to `JobResultCorrections`
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
    ///
    /// * `assignee` - The new assignee to be set for the job
    ///
    /// # Returns
    ///
    /// The updated `JobResultBuilder` instance
    pub fn with_assignee(mut self, assignee: String) -> Self {
        self.ensure_corrections().assignee = Some(assignee);
        self
    }

    /// Sets a new due date for the job
    ///
    /// # Arguments
    ///
    /// * `due_date` - The new due date to be set for the job
    ///
    /// # Returns
    ///
    /// The updated `JobResultBuilder` instance
    pub fn with_due_date(mut self, due_date: String) -> Self {
        self.ensure_corrections().due_date = Some(due_date);
        self
    }

    /// Sets a new follow-up date for the job
    ///
    /// # Arguments
    ///
    /// * `follow_up_date` - The new follow-up date to be set for the job
    ///
    /// # Returns
    ///
    /// The updated `JobResultBuilder` instance
    pub fn with_follow_up_date(mut self, follow_up_date: String) -> Self {
        self.ensure_corrections().follow_up_date = Some(follow_up_date);
        self
    }

    /// Sets new candidate users for the job
    ///
    /// # Arguments
    ///
    /// * `candidate_users` - List of user IDs that are candidates for this job
    ///
    /// # Returns
    ///
    /// The updated `JobResultBuilder` instance
    pub fn with_candidate_users(mut self, candidate_users: Vec<String>) -> Self {
        self.ensure_corrections().candidate_users = Some(candidate_users);
        self
    }

    /// Sets new candidate groups for the job
    ///
    /// # Arguments
    ///
    /// * `candidate_groups` - List of group IDs that are candidates for this job
    ///
    /// # Returns
    ///
    /// The updated `JobResultBuilder` instance
    pub fn with_candidate_groups(mut self, candidate_groups: Vec<String>) -> Self {
        self.ensure_corrections().candidate_groups = Some(candidate_groups);
        self
    }

    /// Sets a new priority for the job
    ///
    /// # Arguments
    ///
    /// * `priority` - The new priority value to be set for the job
    ///
    /// # Returns
    ///
    /// The updated `JobResultBuilder` instance
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.ensure_corrections().priority = Some(priority);
        self
    }

    /// Builds the final CompleteJobRequest with the configured job result
    ///
    /// Consumes the builder and returns the configured request ready for sending
    ///
    /// # Returns
    ///
    /// The configured `CompleteJobRequest<WithKey>` instance
    pub fn build(mut self) -> CompleteJobRequest<WithKey> {
        self.source_request.result = self.job_result;
        self.source_request
    }
}

#[derive(Debug, Clone)]
pub struct Initial;

#[derive(Debug, Clone)]
pub struct WithKey;

pub trait CompleteJobRequestState {}
impl CompleteJobRequestState for Initial {}
impl CompleteJobRequestState for WithKey {}

/// Request to complete a job in Zeebe for a process instance
///
/// # Examples
///
/// ```ignore
/// client
///     .complete_job()
///     .with_job_key(123456)
///     .send()
///     .await?;
/// ```
#[derive(Debug, Clone)]
pub struct CompleteJobRequest<T: CompleteJobRequestState> {
    client: Client,
    job_key: i64,
    variables: serde_json::Value,
    result: Option<JobResult>,
    _state: std::marker::PhantomData<T>,
}

impl<T: CompleteJobRequestState> CompleteJobRequest<T> {
    pub(crate) fn new(client: Client) -> CompleteJobRequest<Initial> {
        CompleteJobRequest {
            client,
            job_key: 0,
            variables: serde_json::Value::default(),
            result: None,
            _state: std::marker::PhantomData,
        }
    }

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
    ///
    /// # Arguments
    ///
    /// * `job_key` - The unique key identifying the job
    ///
    /// # Returns
    ///
    /// A new instance of `CompleteJobRequest<WithKey>`
    pub fn with_job_key(mut self, job_key: i64) -> CompleteJobRequest<WithKey> {
        self.job_key = job_key;
        self.transition()
    }
}

impl CompleteJobRequest<WithKey> {
    /// Sets variables to be included with the job completion
    ///
    /// # Arguments
    ///
    /// * `data` - The variables to be set when completing the job
    ///
    /// # Returns
    ///
    /// A result containing the updated `CompleteJobRequest<WithKey>` instance or a `ClientError`
    pub fn with_variables<T: Serialize>(mut self, data: T) -> Result<Self, ClientError> {
        self.variables = serde_json::to_value(data)
            .map_err(|e| ClientError::SerializationFailed { source: e })?;
        Ok(self)
    }

    /// Starts building a job result with corrections
    ///
    /// # Returns
    ///
    /// A new instance of `JobResultBuilder`
    pub fn with_job_result(self) -> JobResultBuilder {
        JobResultBuilder::new(self)
    }

    /// Sends the job completion request to the Zeebe workflow engine
    ///
    /// # Returns
    ///
    /// A result containing the `CompleteJobResponse` or a `ClientError`
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
