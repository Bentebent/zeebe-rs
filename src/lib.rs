#[allow(clippy::all)]
pub(crate) mod proto {
    tonic::include_proto!("gateway_protocol");
}

pub(crate) mod client;
pub(crate) mod decision;
pub(crate) mod incident;
pub(crate) mod job;
pub(crate) mod message;
pub(crate) mod oauth;
pub(crate) mod process_instance;
pub(crate) mod resource;
pub(crate) mod set_variables;
pub(crate) mod signal;
pub(crate) mod throw_error;
pub(crate) mod topology;
pub(crate) mod worker;

pub use client::{Client, ClientBuilder, ClientBuilderError, ClientError};
pub use decision::{
    EvaluateDecisionRequest, EvaluateDecisionResponse, EvaluatedDecision, EvaluatedDecisionInput,
    EvaluatedDecisionOutput,
};
pub use incident::{ResolveIncidentRequest, ResolveIncidentResponse};
pub use job::{
    activated_job::ActivatedJob,
    complete::{
        CompleteJobRequest, CompleteJobResponse, JobResult, JobResultBuilder, JobResultCorrections,
    },
    fail::{FailJobRequest, FailJobResponse},
    update_retries::{UpdateJobRetriesRequest, UpdateJobRetriesResponse},
    update_timeout::{UpdateJobTimeoutRequest, UpdateJobTimeoutResponse},
};
pub use oauth::OAuthError;
pub use process_instance::{
    cancel::{CancelProcessInstanceRequest, CancelProcessInstanceResponse},
    create::{
        CreateProcessInstanceRequest, CreateProcessInstanceResponse,
        CreateProcessInstanceWithResult, CreateProcessInstanceWithResultSerialized,
    },
    migrate::{
        MappingInstruction, MigrateProcessInstanceRequest, MigrateProcessInstanceResponse,
        MigrationPlan,
    },
    modify::{ModifyProcessInstanceRequest, ModifyProcessInstanceResponse},
};
pub use set_variables::{SetVariablesRequest, SetVariablesResponse};
pub use signal::{BroadcastSignalRequest, BroadcastSignalResponse};
pub use throw_error::{ThrowErrorRequest, ThrowErrorResponse};
pub use topology::{TopologyRequest, TopologyResponse};
