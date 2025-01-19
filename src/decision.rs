use crate::{proto, Client, ClientError};
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EvaluateDecisionError {
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
}

pub struct Initial;
pub struct WithKey;
pub struct WithId;
pub trait EvaluateDecisionRequestState {}
impl EvaluateDecisionRequestState for Initial {}
impl EvaluateDecisionRequestState for WithKey {}
impl EvaluateDecisionRequestState for WithId {}

#[derive(Debug, Clone)]
pub struct EvaluateDecisionRequest<T: EvaluateDecisionRequestState> {
    client: Client,
    decision_key: i64,
    decision_id: String,
    variables: serde_json::Value,
    tenant_id: String,
    _state: std::marker::PhantomData<T>,
}

impl<T: EvaluateDecisionRequestState> EvaluateDecisionRequest<T> {
    pub fn new(client: Client) -> EvaluateDecisionRequest<Initial> {
        EvaluateDecisionRequest {
            client,
            decision_key: 0,
            decision_id: String::new(),
            variables: serde_json::Value::default(),
            tenant_id: String::new(),
            _state: std::marker::PhantomData,
        }
    }

    pub fn with_tenant_id(mut self, tenant_id: String) -> Self {
        self.tenant_id = tenant_id;
        self
    }

    fn transition<NewState: EvaluateDecisionRequestState>(
        self,
    ) -> EvaluateDecisionRequest<NewState> {
        EvaluateDecisionRequest {
            client: self.client,
            decision_key: self.decision_key,
            decision_id: self.decision_id,
            variables: self.variables,
            tenant_id: self.tenant_id,
            _state: std::marker::PhantomData,
        }
    }
}

impl EvaluateDecisionRequest<Initial> {
    pub fn with_decision_key(mut self, decision_kery: i64) -> EvaluateDecisionRequest<WithKey> {
        self.decision_key = decision_kery;
        self.transition()
    }
}

impl EvaluateDecisionRequest<WithKey> {
    pub fn with_decision_id(mut self, decision_id: String) -> EvaluateDecisionRequest<WithId> {
        self.decision_id = decision_id;
        self.transition()
    }
}

impl EvaluateDecisionRequest<WithId> {
    pub fn with_variables<T: Serialize>(mut self, data: T) -> Result<Self, EvaluateDecisionError> {
        self.variables = serde_json::to_value(data)?;
        Ok(self)
    }

    pub async fn send<T: DeserializeOwned>(
        mut self,
    ) -> Result<EvaluateDecisionResponse<T>, ClientError> {
        let res = self
            .client
            .gateway_client
            .evaluate_decision(proto::EvaluateDecisionRequest {
                decision_key: self.decision_key,
                decision_id: self.decision_id,
                variables: self.variables.to_string(),
                tenant_id: self.tenant_id,
            })
            .await?;

        Ok(res.into_inner().try_into()?)
    }
}

#[derive(Debug, Clone)]
pub struct EvaluatedDecisionInput {
    input_id: String,
    input_name: String,
    input_value: String,
}

impl From<proto::EvaluatedDecisionInput> for EvaluatedDecisionInput {
    fn from(value: proto::EvaluatedDecisionInput) -> EvaluatedDecisionInput {
        EvaluatedDecisionInput {
            input_id: value.input_id,
            input_name: value.input_name,
            input_value: value.input_value,
        }
    }
}

impl EvaluatedDecisionInput {
    pub fn input_id(&self) -> &str {
        &self.input_id
    }

    pub fn input_name(&self) -> &str {
        &self.input_name
    }

    pub fn input_value(&self) -> &str {
        &self.input_value
    }
}

#[derive(Debug, Clone)]
pub struct EvaluatedDecisionOutput {
    output_id: String,
    output_name: String,
    output_value: String,
}

impl From<proto::EvaluatedDecisionOutput> for EvaluatedDecisionOutput {
    fn from(value: proto::EvaluatedDecisionOutput) -> EvaluatedDecisionOutput {
        EvaluatedDecisionOutput {
            output_id: value.output_id,
            output_name: value.output_name,
            output_value: value.output_value,
        }
    }
}

impl EvaluatedDecisionOutput {
    pub fn output_id(&self) -> &str {
        &self.output_id
    }

    pub fn output_name(&self) -> &str {
        &self.output_name
    }

    pub fn output_value(&self) -> &str {
        &self.output_value
    }
}

#[derive(Debug, Clone)]
pub struct MatchedDecisionRule {
    rule_id: String,
    rule_index: i32,
    evaluated_outputs: Vec<EvaluatedDecisionOutput>,
}

impl From<proto::MatchedDecisionRule> for MatchedDecisionRule {
    fn from(value: proto::MatchedDecisionRule) -> MatchedDecisionRule {
        MatchedDecisionRule {
            rule_id: value.rule_id,
            rule_index: value.rule_index,
            evaluated_outputs: value
                .evaluated_outputs
                .into_iter()
                .map(|e| e.into())
                .collect(),
        }
    }
}

impl MatchedDecisionRule {
    pub fn rule_id(&self) -> &str {
        &self.rule_id
    }

    pub fn rule_index(&self) -> i32 {
        self.rule_index
    }

    pub fn evaluated_outputs(&self) -> &[EvaluatedDecisionOutput] {
        &self.evaluated_outputs
    }
}

#[derive(Debug, Clone)]
pub struct EvaluatedDecision {
    decision_key: i64,
    decision_id: String,
    decision_name: String,
    decision_version: i32,
    decision_type: String,
    decision_output: String,
    matched_rules: Vec<MatchedDecisionRule>,
    evaluated_inputs: Vec<EvaluatedDecisionInput>,
    tenant_id: String,
}

impl From<proto::EvaluatedDecision> for EvaluatedDecision {
    fn from(value: proto::EvaluatedDecision) -> EvaluatedDecision {
        EvaluatedDecision {
            decision_key: value.decision_key,
            decision_id: value.decision_id,
            decision_name: value.decision_name,
            decision_version: value.decision_version,
            decision_type: value.decision_type,
            decision_output: value.decision_output,
            matched_rules: value.matched_rules.into_iter().map(|m| m.into()).collect(),
            evaluated_inputs: value
                .evaluated_inputs
                .into_iter()
                .map(|e| e.into())
                .collect(),
            tenant_id: value.tenant_id,
        }
    }
}

impl EvaluatedDecision {
    pub fn decision_key(&self) -> i64 {
        self.decision_key
    }

    pub fn decision_id(&self) -> &str {
        &self.decision_id
    }

    pub fn decision_name(&self) -> &str {
        &self.decision_name
    }

    pub fn decision_version(&self) -> i32 {
        self.decision_version
    }

    pub fn decision_type(&self) -> &str {
        &self.decision_type
    }

    pub fn decision_output(&self) -> &str {
        &self.decision_output
    }

    pub fn matched_rules(&self) -> &[MatchedDecisionRule] {
        &self.matched_rules
    }

    pub fn evaluated_inputs(&self) -> &[EvaluatedDecisionInput] {
        &self.evaluated_inputs
    }

    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }
}

#[derive(Debug, Clone)]
pub struct EvaluateDecisionResponse<T: DeserializeOwned> {
    decision_key: i64,
    decision_id: String,
    decision_name: String,
    decision_version: i32,
    decision_requirements_id: String,
    decision_requirements_key: i64,
    decision_output: T,
    evaluated_decisions: Vec<EvaluatedDecision>,
    failed_decision_id: String,
    failure_message: String,
    tenant_id: String,
    decision_instance_key: i64,
}

impl<T: DeserializeOwned> TryFrom<proto::EvaluateDecisionResponse> for EvaluateDecisionResponse<T> {
    type Error = EvaluateDecisionError;
    fn try_from(
        value: proto::EvaluateDecisionResponse,
    ) -> Result<EvaluateDecisionResponse<T>, Self::Error> {
        Ok(EvaluateDecisionResponse {
            decision_key: value.decision_key,
            decision_id: value.decision_id,
            decision_name: value.decision_name,
            decision_version: value.decision_version,
            decision_requirements_id: value.decision_requirements_id,
            decision_requirements_key: value.decision_requirements_key,
            decision_output: serde_json::from_str(&value.decision_output)?,
            evaluated_decisions: value
                .evaluated_decisions
                .into_iter()
                .map(|e| e.into())
                .collect(),
            failed_decision_id: value.failed_decision_id,
            failure_message: value.failure_message,
            tenant_id: value.tenant_id,
            decision_instance_key: value.decision_instance_key,
        })
    }
}

impl<T: DeserializeOwned> EvaluateDecisionResponse<T> {
    pub fn decision_key(&self) -> i64 {
        self.decision_key
    }

    pub fn decision_id(&self) -> &str {
        &self.decision_id
    }

    pub fn decision_name(&self) -> &str {
        &self.decision_name
    }

    pub fn decision_version(&self) -> i32 {
        self.decision_version
    }

    pub fn decision_requirements_id(&self) -> &str {
        &self.decision_requirements_id
    }

    pub fn decision_requirements_key(&self) -> i64 {
        self.decision_requirements_key
    }

    pub fn decision_output(&self) -> &T {
        &self.decision_output
    }

    pub fn evaluated_decisions(&self) -> &[EvaluatedDecision] {
        &self.evaluated_decisions
    }

    pub fn failed_decision_id(&self) -> &str {
        &self.failed_decision_id
    }

    pub fn failure_message(&self) -> &str {
        &self.failure_message
    }

    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    pub fn decision_instance_key(&self) -> i64 {
        self.decision_instance_key
    }
}
