use crate::{proto, Client, ClientError};
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CompleteJobError {
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct JobResultCorrections {
    assignee: Option<String>,
    due_date: Option<String>,
    follow_up_date: Option<String>,
    candidate_users: Option<Vec<String>>,
    candidate_groups: Option<Vec<String>>,
    priority: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct JobResult {
    denied: Option<bool>,
    corrections: Option<JobResultCorrections>,
}

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
    pub fn with_assignee(mut self, assignee: String) -> Self {
        if let Some(job_result) = &mut self.job_result {
            if let Some(corrections) = &mut job_result.corrections {
                corrections.assignee = Some(assignee);
            } else {
                job_result.corrections = Some(JobResultCorrections {
                    assignee: Some(assignee),
                    due_date: None,
                    follow_up_date: None,
                    candidate_users: None,
                    candidate_groups: None,
                    priority: None,
                });
            }
        } else {
            self.job_result = Some(JobResult {
                denied: None,
                corrections: Some(JobResultCorrections {
                    assignee: Some(assignee),
                    due_date: None,
                    follow_up_date: None,
                    candidate_users: None,
                    candidate_groups: None,
                    priority: None,
                }),
            });
        }
        self
    }

    pub fn with_due_date(mut self, due_date: String) -> Self {
        if let Some(job_result) = &mut self.job_result {
            if let Some(corrections) = &mut job_result.corrections {
                corrections.due_date = Some(due_date);
            } else {
                job_result.corrections = Some(JobResultCorrections {
                    assignee: None,
                    due_date: Some(due_date),
                    follow_up_date: None,
                    candidate_users: None,
                    candidate_groups: None,
                    priority: None,
                });
            }
        } else {
            self.job_result = Some(JobResult {
                denied: None,
                corrections: Some(JobResultCorrections {
                    assignee: None,
                    due_date: Some(due_date),
                    follow_up_date: None,
                    candidate_users: None,
                    candidate_groups: None,
                    priority: None,
                }),
            });
        }
        self
    }

    pub fn with_follow_up_date(mut self, follow_up_date: String) -> Self {
        if let Some(job_result) = &mut self.job_result {
            if let Some(corrections) = &mut job_result.corrections {
                corrections.follow_up_date = Some(follow_up_date);
            } else {
                job_result.corrections = Some(JobResultCorrections {
                    assignee: None,
                    due_date: None,
                    follow_up_date: Some(follow_up_date),
                    candidate_users: None,
                    candidate_groups: None,
                    priority: None,
                });
            }
        } else {
            self.job_result = Some(JobResult {
                denied: None,
                corrections: Some(JobResultCorrections {
                    assignee: None,
                    due_date: None,
                    follow_up_date: Some(follow_up_date),
                    candidate_users: None,
                    candidate_groups: None,
                    priority: None,
                }),
            });
        }
        self
    }

    pub fn with_candidate_users(mut self, candidate_users: Vec<String>) -> Self {
        if let Some(job_result) = &mut self.job_result {
            if let Some(corrections) = &mut job_result.corrections {
                corrections.candidate_users = Some(candidate_users);
            } else {
                job_result.corrections = Some(JobResultCorrections {
                    assignee: None,
                    due_date: None,
                    follow_up_date: None,
                    candidate_users: Some(candidate_users),
                    candidate_groups: None,
                    priority: None,
                });
            }
        } else {
            self.job_result = Some(JobResult {
                denied: None,
                corrections: Some(JobResultCorrections {
                    assignee: None,
                    due_date: None,
                    follow_up_date: None,
                    candidate_users: Some(candidate_users),
                    candidate_groups: None,
                    priority: None,
                }),
            });
        }
        self
    }

    pub fn with_candidate_groups(mut self, candidate_groups: Vec<String>) -> Self {
        if let Some(job_result) = &mut self.job_result {
            if let Some(corrections) = &mut job_result.corrections {
                corrections.candidate_groups = Some(candidate_groups);
            } else {
                job_result.corrections = Some(JobResultCorrections {
                    assignee: None,
                    due_date: None,
                    follow_up_date: None,
                    candidate_users: None,
                    candidate_groups: Some(candidate_groups),
                    priority: None,
                });
            }
        } else {
            self.job_result = Some(JobResult {
                denied: None,
                corrections: Some(JobResultCorrections {
                    assignee: None,
                    due_date: None,
                    follow_up_date: None,
                    candidate_users: None,
                    candidate_groups: Some(candidate_groups),
                    priority: None,
                }),
            });
        }
        self
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        if let Some(job_result) = &mut self.job_result {
            if let Some(corrections) = &mut job_result.corrections {
                corrections.priority = Some(priority);
            } else {
                job_result.corrections = Some(JobResultCorrections {
                    assignee: None,
                    due_date: None,
                    follow_up_date: None,
                    candidate_users: None,
                    candidate_groups: None,
                    priority: Some(priority),
                });
            }
        } else {
            self.job_result = Some(JobResult {
                denied: None,
                corrections: Some(JobResultCorrections {
                    assignee: None,
                    due_date: None,
                    follow_up_date: None,
                    candidate_users: None,
                    candidate_groups: None,
                    priority: Some(priority),
                }),
            });
        }
        self
    }

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

#[derive(Debug, Clone)]
pub struct CompleteJobRequest<T: CompleteJobRequestState> {
    client: Client,
    job_key: i64,
    variables: serde_json::Value,
    result: Option<JobResult>,
    _state: std::marker::PhantomData<T>,
}

impl<T: CompleteJobRequestState> CompleteJobRequest<T> {
    pub fn new(client: Client) -> CompleteJobRequest<Initial> {
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
    pub fn with_job_key(mut self, job_key: i64) -> CompleteJobRequest<WithKey> {
        self.job_key = job_key;
        self.transition()
    }
}

impl CompleteJobRequest<WithKey> {
    pub fn with_variables<T: Serialize>(mut self, data: T) -> Result<Self, CompleteJobError> {
        self.variables = serde_json::to_value(data)?;
        Ok(self)
    }

    pub fn with_job_result(self) -> JobResultBuilder {
        JobResultBuilder::new(self)
    }

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

#[derive(Debug, Clone)]
pub struct CompleteJobResponse {}

impl From<proto::CompleteJobResponse> for CompleteJobResponse {
    fn from(_value: proto::CompleteJobResponse) -> CompleteJobResponse {
        CompleteJobResponse {}
    }
}
