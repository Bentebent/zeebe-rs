use crate::{proto, Client, ClientError};
use serde::{de::DeserializeOwned, Serialize};

pub struct Initial;
pub struct WithKey;
pub struct WithId;
pub trait EvaluateDecisionRequestState {}
impl EvaluateDecisionRequestState for Initial {}
impl EvaluateDecisionRequestState for WithKey {}
impl EvaluateDecisionRequestState for WithId {}

/// Request to evaluate a DMN decision
///
/// The decision to evaluate can be specified either by using its unique key
/// (as returned by DeployResource), or using the decision ID. When using the
/// decision ID, the latest deployed version of the decision is used.
///
/// # Examples
/// ```ignore
/// client
///     .evaluate_decision()
///     .with_decision_key(123456)
///     .with_decision_id(String::from("decision_id"))
///     .send()
///     .await?;
/// ```
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
    pub(crate) fn new(client: Client) -> EvaluateDecisionRequest<Initial> {
        EvaluateDecisionRequest {
            client,
            decision_key: 0,
            decision_id: String::new(),
            variables: serde_json::Value::default(),
            tenant_id: String::new(),
            _state: std::marker::PhantomData,
        }
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
    /// Sets the unique key of the decision to evaluate
    ///
    /// # Arguments
    /// * `decision_key` - The unique key identifying the decision (as returned by DeployResource)
    ///
    /// # Returns
    /// The updated `EvaluateDecisionRequest` in the `WithKey` state.
    pub fn with_decision_key(mut self, decision_key: i64) -> EvaluateDecisionRequest<WithKey> {
        self.decision_key = decision_key;
        self.transition()
    }
}

impl EvaluateDecisionRequest<WithKey> {
    /// Sets the ID of the decision to evaluate
    ///
    /// # Arguments
    /// * `decision_id` - The ID of the decision to evaluate
    ///
    /// # Returns
    /// The updated `EvaluateDecisionRequest` in the `WithId` state.
    pub fn with_decision_id(mut self, decision_id: String) -> EvaluateDecisionRequest<WithId> {
        self.decision_id = decision_id;
        self.transition()
    }
}

impl EvaluateDecisionRequest<WithId> {
    /// Sets the variables used for decision evaluation
    ///
    /// The variables must be a JSON object, as variables will be mapped in a key-value fashion.
    /// For example: `{ "a": 1, "b": 2 }` will create two variables named "a" and "b".
    ///
    /// # Arguments
    /// * `data` - The variables to be used for decision evaluation
    ///
    /// # Returns
    /// The updated `EvaluateDecisionRequest` with the variables set.
    ///
    /// # Errors
    /// Returns a `ClientError` if the variables cannot be serialized to JSON.
    pub fn with_variables<T: Serialize>(mut self, data: T) -> Result<Self, ClientError> {
        self.variables = serde_json::to_value(data)
            .map_err(|e| ClientError::SerializationFailed { source: e })?;
        Ok(self)
    }

    /// Sends the decision evaluation request
    ///
    /// # Type Parameters
    /// * `T` - The type of the decision output
    ///
    /// # Returns
    /// A `Result` containing the `EvaluateDecisionResponse` or a `ClientError`.
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

        res.into_inner().try_into()
    }

    /// Sets the tenant ID for the decision evaluation
    ///
    /// # Arguments
    /// * `tenant_id` - The ID of the tenant that owns the decision
    ///
    /// # Returns
    /// The updated `EvaluateDecisionRequest` with the tenant ID set.
    pub fn with_tenant_id(mut self, tenant_id: String) -> Self {
        self.tenant_id = tenant_id;
        self
    }
}

/// Represents an evaluated input in a decision
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
    /// Returns the unique identifier of the evaluated input.
    ///
    /// # Returns
    /// A string slice that holds the unique identifier of the evaluated input.
    pub fn input_id(&self) -> &str {
        &self.input_id
    }

    /// Returns the name/label of the evaluated input.
    ///
    /// # Returns
    /// A string slice that holds the name/label of the evaluated input.
    pub fn input_name(&self) -> &str {
        &self.input_name
    }

    /// Returns the value of the input that was used during decision evaluation.
    ///
    /// # Returns
    /// A string slice that holds the value of the input used during decision evaluation.
    pub fn input_value(&self) -> &str {
        &self.input_value
    }
}

/// Represents an evaluated output in a decision
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
    /// Returns the unique identifier of the evaluated output
    ///
    /// # Returns
    /// A string slice that holds the unique identifier of the evaluated output.
    pub fn output_id(&self) -> &str {
        &self.output_id
    }

    /// Returns the name/label of the evaluated output
    ///
    /// # Returns
    /// A string slice that holds the name/label of the evaluated output.
    pub fn output_name(&self) -> &str {
        &self.output_name
    }

    /// Returns the value of the output that was used during decision evaluation
    ///
    /// # Returns
    /// A string slice that holds the value of the output used during decision evaluation.
    pub fn output_value(&self) -> &str {
        &self.output_value
    }
}

/// Represents a matched rule in a decision
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
    /// Returns the unique identifier of the matched rule.
    ///
    /// # Returns
    /// A string slice that holds the unique identifier of the matched rule.
    pub fn rule_id(&self) -> &str {
        &self.rule_id
    }

    /// Returns the index position of the matched rule within the decision table.
    ///
    /// # Returns
    /// An integer representing the index position of the matched rule.
    pub fn rule_index(&self) -> i32 {
        self.rule_index
    }

    /// Returns a slice containing all evaluated outputs for this matched rule.
    ///
    /// # Returns
    /// A slice of `EvaluatedDecisionOutput` containing all evaluated outputs for this matched rule.
    pub fn evaluated_outputs(&self) -> &[EvaluatedDecisionOutput] {
        &self.evaluated_outputs
    }
}

/// Represents an evaluated decision
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
    /// Returns the unique key identifying the evaluated decision
    ///
    /// # Returns
    ///
    /// An `i64` representing the unique key of the evaluated decision
    pub fn decision_key(&self) -> i64 {
        self.decision_key
    }

    /// Returns the ID of the decision which was evaluated
    ///
    /// # Returns
    ///
    /// A string slice (`&str`) representing the ID of the evaluated decision
    pub fn decision_id(&self) -> &str {
        &self.decision_id
    }

    /// Returns the name of the decision which was evaluated
    ///
    /// # Returns
    ///
    /// A string slice (`&str`) representing the name of the evaluated decision
    pub fn decision_name(&self) -> &str {
        &self.decision_name
    }

    /// Returns the version of the decision which was evaluated
    ///
    /// # Returns
    ///
    /// An `i32` representing the version of the evaluated decision
    pub fn decision_version(&self) -> i32 {
        self.decision_version
    }

    /// Returns the type of the decision which was evaluated
    ///
    /// # Returns
    ///
    /// A string slice (`&str`) representing the type of the evaluated decision
    pub fn decision_type(&self) -> &str {
        &self.decision_type
    }

    /// Returns the JSON output of the evaluated decision
    ///
    /// The output is a JSON-formatted string representing the decision result
    ///
    /// # Returns
    pub fn decision_output(&self) -> &str {
        &self.decision_output
    }

    /// A string slice (`&str`) representing the JSON output of the evaluated decision
    ///
    /// Returns a slice containing all rules that matched during decision evaluation
    ///
    /// # Returns
    ///
    /// A slice (`&[MatchedDecisionRule]`) containing all matched rules
    pub fn matched_rules(&self) -> &[MatchedDecisionRule] {
        &self.matched_rules
    }

    /// Returns a slice containing all inputs that were evaluated as part of the decision
    ///
    /// # Returns
    ///
    /// A slice (`&[EvaluatedDecisionInput]`) containing all evaluated inputs
    pub fn evaluated_inputs(&self) -> &[EvaluatedDecisionInput] {
        &self.evaluated_inputs
    }

    /// Returns the tenant identifier of the evaluated decision
    ///
    /// # Returns
    ///
    /// A string slice (`&str`) representing the tenant identifier of the evaluated decision
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }
}

/// The response from evaluating a decision
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
    type Error = ClientError;
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
            decision_output: serde_json::from_str(&value.decision_output).map_err(|e| {
                ClientError::DeserializationFailed {
                    value: value.decision_output.clone(),
                    source: e,
                }
            })?,
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

/// Represents the response of evaluating a decision, parameterized by the type `T`.
///
/// This struct provides various methods to access details about the evaluated decision,
/// including its key, ID, name, version, and output, as well as information about any
/// failures that occurred during the evaluation.
impl<T: DeserializeOwned> EvaluateDecisionResponse<T> {
    /// Returns the unique key identifying the evaluated decision.
    ///
    /// # Returns
    ///
    /// An `i64` representing the unique key of the decision.
    pub fn decision_key(&self) -> i64 {
        self.decision_key
    }

    /// Returns the ID of the decision which was evaluated.
    ///
    /// # Returns
    ///
    /// A string slice representing the ID of the evaluated decision.
    pub fn decision_id(&self) -> &str {
        &self.decision_id
    }

    /// Returns the name of the decision which was evaluated.
    ///
    /// # Returns
    ///
    /// A string slice representing the name of the evaluated decision.
    pub fn decision_name(&self) -> &str {
        &self.decision_name
    }

    /// Returns the version of the decision which was evaluated.
    ///
    /// # Returns
    ///
    /// An `i32` representing the version of the evaluated decision.
    pub fn decision_version(&self) -> i32 {
        self.decision_version
    }

    /// Returns the ID of the decision requirements graph that the decision is part of.
    ///
    /// # Returns
    ///
    /// A string slice representing the ID of the decision requirements graph.
    pub fn decision_requirements_id(&self) -> &str {
        &self.decision_requirements_id
    }

    /// Returns the unique key identifying the decision requirements graph.
    ///
    /// # Returns
    ///
    /// An `i64` representing the unique key of the decision requirements graph.
    pub fn decision_requirements_key(&self) -> i64 {
        self.decision_requirements_key
    }

    /// Returns the output result of the decision evaluation.
    ///
    /// # Returns
    ///
    /// A reference to the output of the decision evaluation of type `T`.
    pub fn decision_output(&self) -> &T {
        &self.decision_output
    }

    /// Returns a list of all decisions that were evaluated within the requested decision evaluation.
    ///
    /// # Returns
    ///
    /// A slice of `EvaluatedDecision` representing all evaluated decisions.
    pub fn evaluated_decisions(&self) -> &[EvaluatedDecision] {
        &self.evaluated_decisions
    }

    /// Returns the ID of the decision which failed during evaluation, if any.
    ///
    /// # Returns
    ///
    /// A string slice representing the ID of the failed decision, if applicable.
    pub fn failed_decision_id(&self) -> &str {
        &self.failed_decision_id
    }

    /// Returns a message describing why the decision evaluation failed, if applicable.
    ///
    /// # Returns
    ///
    /// A string slice representing the failure message, if applicable.
    pub fn failure_message(&self) -> &str {
        &self.failure_message
    }

    /// Returns the tenant identifier of the evaluated decision.
    ///
    /// # Returns
    ///
    /// A string slice representing the tenant ID of the evaluated decision.
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    /// Returns the unique key identifying this decision evaluation.
    ///
    /// # Returns
    ///
    /// An `i64` representing the unique key of this decision evaluation.
    pub fn decision_instance_key(&self) -> i64 {
        self.decision_instance_key
    }
}
