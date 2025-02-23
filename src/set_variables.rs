use crate::{Client, ClientError, proto};
use serde::Serialize;

pub struct Initial;
pub struct WithInstanceKey;
pub struct WithVariables;

pub trait SetVariablesRequestState {}
impl SetVariablesRequestState for Initial {}
impl SetVariablesRequestState for WithInstanceKey {}
impl SetVariablesRequestState for WithVariables {}

/// Request to update variables for a particular scope
///
/// # Variable Scoping
/// Variables can be set either locally or hierarchically:
/// - Local: Variables only visible in specified scope
/// - Hierarchical: Variables propagate up to parent scopes
///
/// # Examples
/// Two scopes with variables:
/// - Scope 1: `{ "foo": 2 }`
/// - Scope 2: `{ "bar": 1 }`
///
/// Setting `{ "foo": 5 }` in scope 2:
/// - Local=true: Scope 2 becomes `{ "bar": 1, "foo": 5 }`, Scope 1 unchanged
/// - Local=false: Scope 1 becomes `{ "foo": 5 }`, Scope 2 unchanged
///
/// ```ignore
///
/// #[derive(Serialize)]
/// struct Foo {
///     bar: String
/// }
///
/// client
///     .set_variables()
///     .with_element_instance_key(123456)
///     .with_variable(Foo {bar: String::from("foobar")})
///     .send()
///     .await?;
/// ```
#[derive(Debug, Clone)]
pub struct SetVariablesRequest<T: SetVariablesRequestState> {
    client: Client,
    element_instance_key: i64,
    variables: serde_json::Value,
    local: bool,
    operation_reference: Option<u64>,
    _state: std::marker::PhantomData<T>,
}

impl<T: SetVariablesRequestState> SetVariablesRequest<T> {
    pub(crate) fn new(client: Client) -> SetVariablesRequest<Initial> {
        SetVariablesRequest {
            client,
            element_instance_key: 0,
            variables: serde_json::Value::default(),
            local: false,
            operation_reference: None,
            _state: std::marker::PhantomData,
        }
    }

    fn transition<NewState: SetVariablesRequestState>(self) -> SetVariablesRequest<NewState> {
        SetVariablesRequest {
            client: self.client,
            element_instance_key: self.element_instance_key,
            variables: self.variables,
            local: self.local,
            operation_reference: self.operation_reference,
            _state: std::marker::PhantomData,
        }
    }
}

impl SetVariablesRequest<Initial> {
    /// Sets the element instance key identifying the scope
    ///
    /// # Arguments
    /// * `element_instance_key` - Key of element instance to update variables for
    ///
    /// # Returns
    /// A `SetVariablesRequest` in the `WithInstanceKey` state
    pub fn with_element_instance_key(
        mut self,
        element_instance_key: i64,
    ) -> SetVariablesRequest<WithInstanceKey> {
        self.element_instance_key = element_instance_key;
        self.transition()
    }
}

impl SetVariablesRequest<WithInstanceKey> {
    /// Sets the variables to update in the scope
    ///
    /// # Arguments
    /// * `data` - Variables as serializable type that will be converted to JSON
    ///
    /// # Errors
    /// Returns `ClientError` if serialization fails
    ///
    /// # Returns
    /// A `SetVariablesRequest` in the `WithVariables` state
    pub fn with_variable<T: Serialize>(
        mut self,
        data: T,
    ) -> Result<SetVariablesRequest<WithVariables>, ClientError> {
        self.variables = serde_json::to_value(data)
            .map_err(|e| ClientError::SerializationFailed { source: e })?;
        Ok(self.transition())
    }
}

impl SetVariablesRequest<WithVariables> {
    /// Sends the set variables request to the gateway
    ///
    /// # Returns
    /// Response containing the unique key for this operation
    ///
    /// # Errors
    /// - `NOT_FOUND`: No element exists with given key
    /// - `ClientError`: If the request fails
    pub async fn send(mut self) -> Result<SetVariablesResponse, ClientError> {
        let res = self
            .client
            .gateway_client
            .set_variables(proto::SetVariablesRequest {
                element_instance_key: self.element_instance_key,
                variables: self.variables.to_string(),
                local: self.local,
                operation_reference: self.operation_reference,
            })
            .await?;

        Ok(res.into_inner().into())
    }

    /// Sets a reference ID to correlate this operation with other events
    ///
    /// # Arguments
    /// * `operation_reference` - Unique identifier for correlation
    ///
    /// # Returns
    /// The updated `SetVariablesRequest` with the operation reference set
    pub fn with_operation_reference(mut self, operation_reference: u64) -> Self {
        self.operation_reference = Some(operation_reference);
        self
    }

    /// Controls variable scope visibility
    ///
    /// # Arguments
    /// * `is_local_scope` - If true, variables only visible in target scope
    ///                      If false, variables propagate up to parent scopes
    ///
    /// # Returns
    /// The updated `SetVariablesRequest` with the local scope flag set
    pub fn set_local_scope(mut self, is_local_scope: bool) -> Self {
        self.local = is_local_scope;
        self
    }
}

/// Response from setting variables containing the operation key
///
/// The key uniquely identifies this set variables operation and can be used
/// to correlate this operation with other events
#[derive(Debug, Clone)]
pub struct SetVariablesResponse {
    key: i64,
}

impl From<proto::SetVariablesResponse> for SetVariablesResponse {
    fn from(value: proto::SetVariablesResponse) -> SetVariablesResponse {
        SetVariablesResponse { key: value.key }
    }
}

impl SetVariablesResponse {
    /// Returns the unique key identifying this set variables operation
    ///
    /// # Returns
    /// The unique key for this operation
    pub fn key(&self) -> i64 {
        self.key
    }
}
