use crate::{ActivatedJob, Client, client, proto};
use serde::Serialize;
use std::{
    future::Future,
    ops::{Deref, DerefMut},
    sync::Arc,
    time::Duration,
};
use thiserror::Error;
use tokio::{
    sync::{
        Semaphore,
        mpsc::{self, Receiver, Sender},
    },
    time::{Interval, interval, timeout},
};
use tracing::{debug, error, info};

/// An enum representing possible errors that can occur during job processing.
///
/// This enum provides different error variants that can be returned by a worker
/// when processing jobs, allowing for different types of failure handling.
///
/// # Type Parameters
///
/// * `T` - A serializable type that can be included with error data
#[derive(Debug, Error)]
pub enum WorkerError<T>
where
    T: Serialize + Send + 'static,
{
    #[error("fail job")]
    FailJob(String),

    #[error("fail job with data")]
    FailJobWithData { error_message: String, data: T },

    #[error("throw error")]
    ThrowError {
        error_code: String,
        error_message: Option<String>,
    },

    #[error("")]
    ThrowErrorWithData {
        error_code: String,
        error_message: Option<String>,
        data: T,
    },

    #[error(transparent)]
    ClientError(#[from] client::ClientError),
}

const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(15);

/// A wrapper struct that holds a shared state of type `T`.
///
/// This struct is designed to encapsulate a shared state that can be accessed
/// and modified by multiple worker instances
///
/// # Type Parameters
///
/// * `T` - The type of the shared state.
#[derive(Debug)]
pub struct SharedState<T>(pub T);

impl<T> Deref for SharedState<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for SharedState<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A trait for handling the output of job processing.
///
/// This trait defines how different output types should be handled after
/// job processing is complete. It provides built-in implementations for
/// common result types.
///
/// # Type Parameters
///
/// * `T` - The type of output produced by the job handler
///
/// # Examples
///
/// ```ignore
/// impl WorkerOutputHandler for MyStruct {
///     fn handle_result(self, client: Client, job: ActivatedJob) ->  impl Future<Output = ()> + Send + 'static {
///         if let Ok(req) = client
///                 .complete_job()
///                 .with_job_key(job.key())
///                 .with_variables(self)
///                 {
///                     let _ = req.send().await;
///                 }
///     }
/// ```
pub trait WorkerOutputHandler {
    fn handle_result(
        self,
        client: Client,
        job: ActivatedJob,
    ) -> impl Future<Output = ()> + Send + 'static;
}

impl WorkerOutputHandler for () {
    fn handle_result(
        self,
        _client: Client,
        _job: ActivatedJob,
    ) -> impl Future<Output = ()> + Send + 'static {
        std::future::ready(())
    }
}

impl<Output, T> WorkerOutputHandler for Result<Output, WorkerError<T>>
where
    Output: Serialize + Send + 'static,
    T: Serialize + Send + 'static,
{
    async fn handle_result(self, client: Client, job: ActivatedJob) {
        match self {
            Ok(value) => {
                if let Ok(req) = client
                    .complete_job()
                    .with_job_key(job.key())
                    .with_variables(value)
                {
                    let _ = req.send().await;
                    info!("Completed job {}", job.key());
                }
            }
            Err(error) => match error {
                WorkerError::FailJob(error_message) => {
                    let _ = client
                        .fail_job()
                        .with_job_key(job.key())
                        .with_retries(job.retries() - 1)
                        .with_error_message(error_message.clone())
                        .send()
                        .await;
                    error!("Failed job {} with error {}", job.key(), error_message);
                }
                WorkerError::FailJobWithData {
                    error_message,
                    data,
                } => {
                    if let Ok(req) = client
                        .fail_job()
                        .with_job_key(job.key())
                        .with_retries(job.retries() - 1)
                        .with_error_message(error_message.clone())
                        .with_variables(data)
                    {
                        let _ = req.send().await;
                        error!("Failed job {} with error {}", job.key(), error_message);
                    }
                }
                WorkerError::ThrowError {
                    error_code,
                    error_message,
                } => {
                    let mut builder = client
                        .throw_error()
                        .with_job_key(job.key())
                        .with_error_code(error_code.clone());

                    if let Some(error_message) = error_message.clone() {
                        builder = builder.with_error_message(error_message);
                    }

                    let _ = builder.send().await;
                    error!(
                        "Job {} threw error {} {}",
                        error_code,
                        error_message.unwrap_or(String::from("")),
                        job.key(),
                    );
                }
                WorkerError::ThrowErrorWithData {
                    error_code,
                    error_message,
                    data,
                } => {
                    if let Ok(mut req) = client
                        .throw_error()
                        .with_job_key(job.key())
                        .with_error_code(error_code.clone())
                        .with_variables(data)
                    {
                        if let Some(error_message) = error_message.clone() {
                            req = req.with_error_message(error_message);
                        }
                        let _ = req.send().await;
                        error!(
                            "Job {} threw error {} {}",
                            error_code,
                            error_message.unwrap_or(String::from("")),
                            job.key(),
                        );
                    }
                }
                WorkerError::ClientError(client_error) => {
                    let _ = client
                        .fail_job()
                        .with_job_key(job.key())
                        .with_retries(job.retries() - 1)
                        .with_error_message(client_error.to_string())
                        .send()
                        .await;

                    error!("Failed job {} with error {}", job.key(), client_error);
                }
            },
        }
    }
}

pub trait JobHandler {
    type Output: WorkerOutputHandler + Send + 'static;
    fn execute(&self, client: Client, job: ActivatedJob) -> impl Future<Output = ()> + Send;
}

impl<F, Fut> JobHandler for F
where
    F: Fn(Client, ActivatedJob) -> Fut + Send + Sync + 'static,
    Fut: Future + Send + 'static,
    Fut::Output: WorkerOutputHandler + Send + 'static,
{
    type Output = Fut::Output;
    async fn execute(&self, client: Client, job: ActivatedJob) {
        let res = (self)(client.clone(), job.clone()).await;
        res.handle_result(client, job).await;
    }
}

impl<F, T, Fut> JobHandler for (F, Arc<SharedState<T>>)
where
    F: Fn(Client, ActivatedJob, Arc<SharedState<T>>) -> Fut + Send + Sync + 'static,
    Fut: Future + Send + 'static,
    Fut::Output: WorkerOutputHandler + Send + 'static,
    T: Send + Sync + 'static,
{
    type Output = Fut::Output;
    async fn execute(&self, client: Client, job: ActivatedJob) {
        let state = self.1.clone();
        let res = (self.0)(client.clone(), job.clone(), state).await;
        res.handle_result(client, job).await;
    }
}

#[derive(Clone)]
pub struct Initial {}

#[derive(Clone)]
pub struct WithJobType {}

#[derive(Clone)]
pub struct WithJobTimeout {}

#[derive(Clone)]
pub struct WithRequestTimeout {}

#[derive(Clone)]
pub struct WithMaxJobs {}

#[derive(Clone)]
pub struct WithConcurrency {}

#[derive(Clone)]
pub struct WithState {}

#[derive(Clone)]
pub struct WithHandler {}

pub trait WorkerBuilderState {}

impl WorkerBuilderState for Initial {}
impl WorkerBuilderState for WithJobType {}
impl WorkerBuilderState for WithJobTimeout {}
impl WorkerBuilderState for WithRequestTimeout {}
impl WorkerBuilderState for WithMaxJobs {}
impl WorkerBuilderState for WithConcurrency {}
impl WorkerBuilderState for WithState {}
impl WorkerBuilderState for WithHandler {}

#[derive(Clone)]
/// `WorkerBuilder` is a builder pattern struct for constructing a `Worker` instance.
///
/// This builder uses the typestate pattern to ensure that all required parameters
/// are set before a Worker can be constructed. The builder enforces proper
/// configuration through its type system.
///
/// # Type Parameters
///
/// * `S` - The current state of the builder (enforces configuration order)
///
/// # Examples
/// ```ignore
/// struct ExampleSharedState {
///     pub increment_me: u32,
/// }
///
/// let state = Arc::new(SharedState(Mutex::new(ExampleSharedState {
///        increment_me: 0,
/// })));
///
/// // Client instantiation
///
/// client
///     .worker()
///     .with_job_timeout(Duration::from_secs(60))
///     .with_request_timeout(Duration::from_secs(10))
///     .with_max_jobs_to_activate(4)
///     .with_concurrency_limit(2)
///     .with_job_type(String::from("demo-service"))
///     .with_state(state)
///     .with_handler(|client, job, state| async move {
///         let mut lock = state.lock().await;
///         lock.increment_me += 1;
///         let _ = client.complete_job().with_job_key(job.key()).send().await;
///      })
///      .build()
/// ```
pub struct WorkerBuilder<S, H = (), T = ()>
where
    S: WorkerBuilderState,
    T: Send + Sync + 'static,
{
    client: Client,
    job_type: String,
    worker_name: String,
    timeout: Duration,
    max_jobs_to_activate: i32,
    concurrency_limit: u32,
    worker_callback: Option<Arc<H>>,
    state: Option<Arc<SharedState<T>>>,
    fetch_variable: Vec<String>,
    request_timeout: Duration,
    tenant_ids: Vec<String>,
    _state: std::marker::PhantomData<S>,
}

impl WorkerBuilder<Initial> {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            job_type: String::new(),
            worker_name: String::new(),
            timeout: Duration::default(),
            max_jobs_to_activate: 0,
            concurrency_limit: 0,
            worker_callback: None,
            state: None,
            fetch_variable: vec![],
            request_timeout: Duration::default(),
            tenant_ids: vec![],
            _state: std::marker::PhantomData,
        }
    }

    /// Sets the request timeout for the worker.
    ///
    /// The request will be completed when at least one job is activated or after the specified `request_timeout`.
    ///
    /// # Arguments
    ///
    /// * `request_timeout` - The duration to wait before the request times out.
    ///
    /// # Returns
    ///
    /// A `WorkerBuilder<WithRequestTimeout>` instance with the request timeout configured.
    pub fn with_request_timeout(
        self,
        request_timeout: Duration,
    ) -> WorkerBuilder<WithRequestTimeout> {
        WorkerBuilder {
            client: self.client,
            job_type: self.job_type,
            worker_name: self.worker_name,
            timeout: self.timeout,
            max_jobs_to_activate: self.max_jobs_to_activate,
            concurrency_limit: self.concurrency_limit,
            worker_callback: self.worker_callback,
            state: self.state,
            fetch_variable: self.fetch_variable,
            request_timeout,
            tenant_ids: self.tenant_ids,
            _state: std::marker::PhantomData,
        }
    }
}

impl WorkerBuilder<WithRequestTimeout> {
    /// Sets the job timeout for the worker.
    ///
    /// A job returned after this call will not be activated by another call until the
    /// specified timeout (in milliseconds) has been reached. This ensures that the job
    /// is not picked up by another worker before the timeout expires.
    ///
    /// # Parameters
    ///
    /// - `timeout`: The duration for which the job should be locked.
    ///
    /// # Returns
    ///
    /// A `WorkerBuilder<WithJobTimeout>` instance with the job timeout configured.
    pub fn with_job_timeout(self, timeout: Duration) -> WorkerBuilder<WithJobTimeout> {
        WorkerBuilder {
            client: self.client,
            job_type: self.job_type,
            worker_name: self.worker_name,
            timeout,
            max_jobs_to_activate: self.max_jobs_to_activate,
            concurrency_limit: self.concurrency_limit,
            worker_callback: self.worker_callback,
            state: self.state,
            fetch_variable: self.fetch_variable,
            request_timeout: self.request_timeout,
            tenant_ids: self.tenant_ids,
            _state: std::marker::PhantomData,
        }
    }
}

impl WorkerBuilder<WithJobTimeout> {
    /// Sets the maximum number of jobs to activate in a single request.
    ///
    /// # Arguments
    ///
    /// * `max_jobs_to_activate` - The maximum number of jobs to activate.
    ///
    /// # Returns
    ///
    /// A `WorkerBuilder<WithMaxJobs>` instance with the `WithMaxJobs` state.
    pub fn with_max_jobs_to_activate(
        self,
        max_jobs_to_activate: i32,
    ) -> WorkerBuilder<WithMaxJobs> {
        WorkerBuilder {
            client: self.client,
            job_type: self.job_type,
            worker_name: self.worker_name,
            timeout: self.timeout,
            max_jobs_to_activate,
            concurrency_limit: self.concurrency_limit,
            worker_callback: self.worker_callback,
            state: self.state,
            fetch_variable: self.fetch_variable,
            request_timeout: self.request_timeout,
            tenant_ids: self.tenant_ids,
            _state: std::marker::PhantomData,
        }
    }
}

impl WorkerBuilder<WithMaxJobs> {
    /// Sets the maximum number of jobs that can be processed concurrently by the worker.
    ///
    /// # Arguments
    ///
    /// * `concurrency_limit` - The maximum number of jobs that the worker can handle at the same time.
    ///
    /// # Returns
    ///
    /// A `WorkerBuilder<WithConcurrency>` instance with the concurrency limit set.
    pub fn with_concurrency_limit(self, concurrency_limit: u32) -> WorkerBuilder<WithConcurrency> {
        WorkerBuilder {
            client: self.client,
            job_type: self.job_type,
            worker_name: self.worker_name,
            timeout: self.timeout,
            max_jobs_to_activate: self.max_jobs_to_activate,
            concurrency_limit,
            worker_callback: self.worker_callback,
            state: self.state,
            fetch_variable: self.fetch_variable,
            request_timeout: self.request_timeout,
            tenant_ids: self.tenant_ids,
            _state: std::marker::PhantomData,
        }
    }
}

impl WorkerBuilder<WithConcurrency> {
    /// Sets the job type for the worker.
    ///
    /// The job type is defined in the BPMN process, for example:
    /// `<zeebe:taskDefinition type="payment-service" />`.
    ///
    /// # Parameters
    ///
    /// - `job_type`: A `String` representing the job type.
    ///
    /// # Returns
    ///
    /// A `WorkerBuilder<WithJobType>` instance with the job type set.
    pub fn with_job_type(self, job_type: String) -> WorkerBuilder<WithJobType> {
        WorkerBuilder {
            client: self.client,
            job_type,
            worker_name: self.worker_name,
            timeout: self.timeout,
            max_jobs_to_activate: self.max_jobs_to_activate,
            concurrency_limit: self.concurrency_limit,
            worker_callback: self.worker_callback,
            state: self.state,
            fetch_variable: self.fetch_variable,
            request_timeout: self.request_timeout,
            tenant_ids: self.tenant_ids,
            _state: std::marker::PhantomData,
        }
    }
}

impl WorkerBuilder<WithJobType> {
    /// Sets the handler function for the worker.
    ///
    /// # Arguments
    ///
    /// * `handler` - A function that takes a `Client` and an `ActivatedJob` as arguments and returns a `Future` that resolves to `()`.
    ///
    /// # Returns
    ///
    /// Returns a `WorkerBuilder` with the `WithHandler` state.
    ///
    /// # Examples
    /// ```ignore
    ///
    /// // You can choose to manually handle returning results from handler functions
    /// async fn example_service(client: Client, job: ActivatedJob) {
    ///     // Your job handling logic here
    ///     // Function has to use the client to return results
    ///     let _ = client.complete_job().with_job_key(job.key()).send().await;
    /// }
    ///
    /// client
    ///    .worker()
    ///    .with_job_timeout(Duration::from_secs(5 * 60))
    ///    .with_request_timeout(Duration::from_secs(10))
    ///    .with_max_jobs_to_activate(4)
    ///    .with_concurrency_limit(2)
    ///     .with_job_type(String::from("example-service"))
    ///    .with_handler(example_service)
    ///    ...
    /// ```
    ///
    /// If the function is defined to return a Result instead the result is used to automatically set the status
    /// of the job.
    ///
    /// ```ignore
    /// async fn example_service_with_result(_client: Client, job: ActivatedJob) -> Result<(), WorkerError<()>> {
    ///     Ok(())
    /// }
    ///
    /// client
    ///    .worker()
    ///    .with_job_timeout(Duration::from_secs(5 * 60))
    ///    .with_request_timeout(Duration::from_secs(10))
    ///    .with_max_jobs_to_activate(4)
    ///    .with_concurrency_limit(2)
    ///    .with_job_type(String::from("example-service"))
    ///    .with_handler(example_service_with_result)
    ///    ...
    /// ```
    /// This works for closures as well but requires them to be type annotated.
    ///
    /// ```ignore
    /// client
    ///     .worker()
    ///     .with_request_timeout(Duration::from_secs(10))
    ///     .with_job_timeout(Duration::from_secs(10))
    ///     .with_max_jobs_to_activate(5)
    ///     .with_concurrency_limit(5)
    ///     .with_job_type(String::from("example_service"))
    ///     .with_handler(|_client, _job| async move { Ok::<(), WorkerError<()>>(()) })
    ///     .build();
    ///
    /// ```
    ///
    /// # Type Parameters
    ///
    /// * `F` - The type of the handler function.
    /// * `Fut` - The return of F
    /// * `Fut::Output` - Fut return value that must implement WorkerOutputHandler
    ///
    /// # Constraints
    ///
    /// * `F` must implement `Fn(Client, ActivatedJob) -> Fut` and must be `Send` and `'static`.
    pub fn with_handler<F, Fut>(self, handler: F) -> WorkerBuilder<WithHandler, F>
    where
        F: Fn(Client, ActivatedJob) -> Fut + Send + 'static,
        Fut: Future + Send + 'static,
        Fut::Output: WorkerOutputHandler + Send + 'static,
    {
        WorkerBuilder {
            client: self.client,
            job_type: self.job_type,
            worker_name: self.worker_name,
            timeout: self.timeout,
            max_jobs_to_activate: self.max_jobs_to_activate,
            concurrency_limit: self.concurrency_limit,
            worker_callback: Some(Arc::new(handler)),
            state: self.state,
            fetch_variable: self.fetch_variable,
            request_timeout: self.request_timeout,
            tenant_ids: self.tenant_ids,
            _state: std::marker::PhantomData,
        }
    }

    /// Sets the state that will be shared across all concurrent instances of the worker.
    ///
    /// # Arguments
    ///
    /// * `shared_state` - An `Arc` containing the shared state.
    ///
    /// # Returns
    ///
    /// Returns a `WorkerStateBuilder` with the provided shared state.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type of the shared state.
    ///
    /// # Constraints
    ///
    /// * `T` must be `Send`, `Sync`, and `'static`.
    ///
    pub fn with_state<T>(self, state: Arc<SharedState<T>>) -> WorkerBuilder<WithState, (), T>
    where
        T: Send + Sync + 'static,
    {
        WorkerBuilder {
            client: self.client,
            job_type: self.job_type,
            worker_name: self.worker_name,
            timeout: self.timeout,
            max_jobs_to_activate: self.max_jobs_to_activate,
            concurrency_limit: self.concurrency_limit,
            worker_callback: None,
            state: Some(state),
            fetch_variable: self.fetch_variable,
            request_timeout: self.request_timeout,
            tenant_ids: self.tenant_ids,
            _state: std::marker::PhantomData,
        }
    }
}

impl<T> WorkerBuilder<WithState, (), T>
where
    T: Send + Sync + 'static,
{
    pub fn with_handler<F, Fut>(
        self,
        handler: F,
    ) -> WorkerBuilder<WithHandler, (F, Arc<SharedState<T>>), T>
    where
        F: Fn(Client, ActivatedJob, Arc<SharedState<T>>) -> Fut + Send + 'static,
        Fut: Future + Send + 'static,
        Fut::Output: WorkerOutputHandler + Send + 'static,
    {
        let state = self.state.unwrap();
        WorkerBuilder {
            client: self.client,
            job_type: self.job_type,
            worker_name: self.worker_name,
            timeout: self.timeout,
            max_jobs_to_activate: self.max_jobs_to_activate,
            concurrency_limit: self.concurrency_limit,
            worker_callback: Some(Arc::new((handler, state))),
            state: None,
            fetch_variable: self.fetch_variable,
            request_timeout: self.request_timeout,
            tenant_ids: self.tenant_ids,
            _state: std::marker::PhantomData,
        }
    }
}

impl<H, T> WorkerBuilder<WithHandler, H, T>
where
    H: JobHandler + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    /// Builds a `Worker` using the collected inputs.
    ///
    /// # Returns
    ///
    /// * `Worker` - The constructed Worker instance
    pub fn build(self) -> Worker<H> {
        let request = proto::ActivateJobsRequest {
            r#type: self.job_type,
            worker: self.worker_name,
            timeout: self.timeout.as_millis() as i64,
            max_jobs_to_activate: self.max_jobs_to_activate,
            fetch_variable: self.fetch_variable,
            request_timeout: self.request_timeout.as_millis() as i64,
            tenant_ids: self.tenant_ids,
        };

        Worker::new(
            self.client,
            request,
            self.request_timeout,
            self.concurrency_limit,
            self.worker_callback
                .expect("Don't transition to build without handler"),
        )
    }

    /// Sets the worker name.
    ///
    /// # Arguments
    ///
    /// * `worker_name` - A `String` representing the name of the worker.
    ///
    /// # Returns
    ///
    /// * `Self` - The updated `WorkerBuilder` instance.
    pub fn with_worker_name(mut self, worker_name: String) -> Self {
        self.worker_name = worker_name;
        self
    }

    /// Adds a single variable to fetch.
    ///
    /// A list of variables to fetch as the job variables; if empty, all visible variables at
    /// the time of activation for the scope of the job will be returned
    ///
    /// # Arguments
    ///
    /// * `fetch_variable` - A `String` representing the variable to fetch.
    ///
    /// # Returns
    ///
    /// * `Self` - The updated `WorkerBuilder` instance.
    pub fn with_fetch_variable(mut self, fetch_variable: String) -> Self {
        self.fetch_variable.push(fetch_variable);
        self
    }

    /// Adds multiple variables to fetch.
    ///
    /// A list of variables to fetch as the job variables; if empty, all visible variables at
    /// the time of activation for the scope of the job will be returned
    ///
    /// # Arguments
    ///
    /// * `fetch_variables` - A `Vec<String>` representing the variables to fetch.
    ///
    /// # Returns
    ///
    /// * `Self` - The updated `WorkerBuilder` instance.
    pub fn with_fetch_variables(mut self, mut fetch_variables: Vec<String>) -> Self {
        self.fetch_variable.append(&mut fetch_variables);
        self
    }

    /// Adds a single tenant ID.
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - A `String` representing the tenant ID.
    ///
    /// # Returns
    ///
    /// * `Self` - The updated `WorkerBuilder` instance.
    pub fn with_tenant_id(mut self, tenant_id: String) -> Self {
        self.tenant_ids.push(tenant_id);
        self
    }

    /// Adds multiple tenant IDs.
    ///
    /// # Arguments
    ///
    /// * `tenant_ids` - A `Vec<String>` representing the tenant IDs.
    ///
    /// # Returns
    ///
    /// * `Self` - The updated `WorkerBuilder` instance.
    pub fn with_tenant_ids(mut self, mut tenant_ids: Vec<String>) -> Self {
        self.tenant_ids.append(&mut tenant_ids);
        self
    }
}

enum PollingMessage {
    FetchJobs,
    JobsFetched(u32),
    FetchJobsComplete,
    JobFinished,
}

struct WorkProducer {
    client: Client,
    job_tx: Sender<ActivatedJob>,
    poll_tx: Sender<PollingMessage>,
    poll_rx: Receiver<PollingMessage>,
    poll_interval: Interval,
    request_timeout: Duration,
    request: proto::ActivateJobsRequest,
    queued_jobs_count: u32,
    max_jobs_to_activate: u32,
}

impl WorkProducer {
    fn fetch_jobs(&mut self) {
        let mut client = self.client.clone();
        let mut request = self.request.clone();
        let poll_tx = self.poll_tx.clone();
        let job_tx = self.job_tx.clone();
        let request_timeout = self.request_timeout;

        request.max_jobs_to_activate = (self.max_jobs_to_activate - self.queued_jobs_count) as i32;

        tokio::spawn(async move {
            if let Err(_err) = timeout(request_timeout, async {
                let res = client
                    .gateway_client
                    .activate_jobs(tonic::Request::new(request))
                    .await
                    .map(|response| {
                        debug!("Response {:?}", response);
                        response.into_inner()
                    });

                let mut jobs_fetched = 0;
                debug!("Res {:?}", res);

                if let Ok(mut stream) = res {
                    while let Ok(Some(activate_job_response)) = stream.message().await {
                        jobs_fetched += activate_job_response.jobs.len() as u32;

                        for job in activate_job_response.jobs {
                            let _ = job_tx.send(job.into()).await;
                        }
                    }
                }

                let _ = poll_tx
                    .send(PollingMessage::JobsFetched(jobs_fetched))
                    .await;
            })
            .await
            {
                error!("{}", _err);
            };

            let _ = poll_tx.send(PollingMessage::FetchJobsComplete).await;
        });
    }

    async fn run(&mut self) {
        let mut fetching_jobs = false;
        loop {
            tokio::select! {
                Some(message) = self.poll_rx.recv() => {
                    match message {
                        PollingMessage::JobsFetched(new_job_count) => {
                            self.queued_jobs_count = self.queued_jobs_count.saturating_add(new_job_count);
                        }
                        PollingMessage::JobFinished => {
                            self.queued_jobs_count = self.queued_jobs_count.saturating_sub(1);
                        }
                        PollingMessage::FetchJobs => {
                            if self.queued_jobs_count <= self.max_jobs_to_activate && !fetching_jobs {
                                fetching_jobs = true;
                                self.fetch_jobs();
                            }
                        }
                        PollingMessage::FetchJobsComplete => {
                            fetching_jobs = false;
                        }
                    }
                },
                _ = self.poll_interval.tick() => {
                    let _ = self.poll_tx.send(PollingMessage::FetchJobs).await;
                },
                else => {
                    break;
                }
            }
        }
    }
}

struct WorkConsumer<H>
where
    H: JobHandler + Send + Sync + 'static,
{
    client: Client,
    job_rx: Receiver<ActivatedJob>,
    poll_tx: Sender<PollingMessage>,
    semaphore: Arc<Semaphore>,
    worker_callback: Arc<H>,
}

impl<H> WorkConsumer<H>
where
    H: JobHandler + Send + Sync + 'static,
{
    async fn run(&mut self) {
        while let Some(job) = self.job_rx.recv().await {
            let permit = self.semaphore.clone().acquire_owned().await.unwrap();
            let poll_tx = self.poll_tx.clone();
            let client = self.client.clone();
            let callback = self.worker_callback.clone();

            tokio::spawn(async move {
                callback.execute(client.clone(), job.clone()).await;
                let _ = poll_tx.send(PollingMessage::JobFinished).await;
                drop(permit);
            });
        }
    }
}

/// The Worker is responsible for fetching jobs from Zeebe and processing them
/// with the associated handler.
/// /// A worker implementation for processing Zeebe jobs with configurable concurrency and state management.
///
/// The `Worker` is responsible for:
/// - Polling for new jobs from the Zeebe broker
/// - Managing job activation and processing
/// - Handling concurrent job execution
/// - Maintaining worker state across job executions
///
/// The worker consists of two main components:
/// - `WorkProducer`: Handles job polling and queue management
/// - `WorkConsumer`: Manages job execution and concurrency
///
/// # Architecture
///
/// The worker uses a producer-consumer pattern where:
/// 1. The producer polls for jobs at regular intervals
/// 2. Jobs are queued in an internal channel
/// 3. The consumer processes jobs concurrently up to the configured limit
///
/// # Concurrency
///
/// Job processing is controlled by:
/// - A semaphore limiting concurrent job executions
/// - Channel-based communication between components
/// - Configurable maximum jobs to activate
///
/// # Example
///
/// ```ignore
/// let worker = client
///     .worker()
///     .with_job_timeout(Duration::from_secs(60))
///     .with_request_timeout(Duration::from_secs(10))
///     .with_max_jobs_to_activate(5)
///     .with_concurrency_limit(3)
///     .with_job_type("example-service")
///     .with_handler(|client, job| async move {
///         // Process job here
///         client.complete_job().with_job_key(job.key()).send().await;
///     })
///     .build();
///
/// // Start the worker
/// worker.run().await?;
///
/// ```
/// # Error Handling
///
/// The worker implements automatic error handling for:
/// - Job activation timeouts
/// - Network errors during polling
/// - Job processing failures
pub struct Worker<H>
where
    H: JobHandler + Send + Sync + 'static,
{
    poller: WorkProducer,
    dispatcher: WorkConsumer<H>,
}

impl<H> Worker<H>
where
    H: JobHandler + Send + Sync + 'static,
{
    fn new(
        client: Client,
        request: proto::ActivateJobsRequest,
        request_timeout: Duration,
        concurrency_limit: u32,
        callback: Arc<H>,
    ) -> Worker<H> {
        let (job_tx, job_rx) = mpsc::channel(32);
        let (poll_tx, poll_rx) = mpsc::channel(32);
        let max_jobs_to_activate = request.max_jobs_to_activate as u32;

        let poller = WorkProducer {
            client: client.clone(),
            job_tx,
            poll_tx: poll_tx.clone(),
            poll_rx,
            poll_interval: interval(DEFAULT_POLL_INTERVAL),
            request,
            request_timeout,
            max_jobs_to_activate,
            queued_jobs_count: 0,
        };

        let dispatcher = WorkConsumer {
            client: client.clone(),
            job_rx,
            poll_tx: poll_tx.clone(),
            semaphore: Arc::new(Semaphore::new(concurrency_limit as usize)),
            worker_callback: callback,
        };

        Worker { poller, dispatcher }
    }

    /// Starts the worker by running both the poller and dispatcher concurrently.
    ///
    /// This method uses `tokio::join!` to run the `poller` and `dispatcher` concurrently.
    /// The poller continuously polls the Zeebe broker for new jobs, while the dispatcher
    /// processes the jobs using the provided callback.
    /// # Example
    /// ```ignore
    /// #[tokio::main]
    /// async fn main() {
    ///     client
    ///         .worker()
    ///         //Worker configuration...
    ///         .build()
    ///         .run()
    ///         .await;
    /// }
    /// ```
    pub async fn run(mut self) {
        tokio::join!(self.poller.run(), self.dispatcher.run());
    }
}
