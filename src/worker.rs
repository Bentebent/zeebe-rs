use crate::{proto, ActivatedJob, Client};
use std::{
    future::Future,
    ops::{Deref, DerefMut},
    pin::Pin,
    sync::Arc,
    time::Duration,
};
use tokio::{
    sync::{
        mpsc::{self, Receiver, Sender},
        Semaphore,
    },
    time::{interval, timeout, Interval},
};

const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(15);
type BoxFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

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

trait JobHandler: Send + Sync {
    fn execute(&self, client: Client, job: ActivatedJob) -> BoxFuture;
}

impl<F> JobHandler for F
where
    F: Fn(Client, ActivatedJob) -> BoxFuture + Send + Sync + 'static,
{
    fn execute(&self, client: Client, job: ActivatedJob) -> BoxFuture {
        (self)(client, job)
    }
}

impl<F, T> JobHandler for (F, Arc<SharedState<T>>)
where
    F: Fn(Client, ActivatedJob, Arc<SharedState<T>>) -> BoxFuture + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    fn execute(&self, client: Client, job: ActivatedJob) -> BoxFuture {
        let state = self.1.clone();
        (self.0)(client, job, state)
    }
}

pub struct Initial {}
pub struct WithJobType {}
pub struct WithTimeout {}
pub struct WithRequestTimeout {}
pub struct WithMaxJobs {}
pub struct WithConcurrency {}
pub struct WithHandler {}

pub trait WorkerBuilderState {}

impl WorkerBuilderState for Initial {}
impl WorkerBuilderState for WithJobType {}
impl WorkerBuilderState for WithTimeout {}
impl WorkerBuilderState for WithRequestTimeout {}
impl WorkerBuilderState for WithMaxJobs {}
impl WorkerBuilderState for WithConcurrency {}
impl WorkerBuilderState for WithHandler {}

/// `WorkerBuilder` is a builder pattern struct for constructing a `Worker` instance.
/// Uses typestate pattern to enforce setting mandatory parameters.
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
///     .with_job_type(String::from("demo-service"))
///     .with_job_timeout(Duration::from_secs(60))
///     .with_request_timeout(Duration::from_secs(10))
///     .with_max_jobs_to_activate(4)
///     .with_concurrency_limit(2)
///     .with_state(state)
///     .with_handler(|client, job, state| async move {
///         let mut lock = state.lock().await;
///         lock.increment_me += 1;
///         let _ = client.complete_job().with_job_key(job.key()).send().await;
///      })
///      .build()
/// ```
pub struct WorkerBuilder<T> {
    client: Client,
    job_type: String,
    worker_name: String,
    timeout: Duration,
    max_jobs_to_activate: i32,
    concurrency_limit: u32,
    worker_callback: Option<Arc<Box<dyn JobHandler>>>,
    fetch_variable: Vec<String>,
    request_timeout: Duration,
    tenant_ids: Vec<String>,
    _state: std::marker::PhantomData<T>,
}

impl<T: WorkerBuilderState> WorkerBuilder<T> {
    pub(crate) fn new(client: Client) -> WorkerBuilder<Initial> {
        WorkerBuilder {
            client,
            job_type: String::new(),
            worker_name: String::new(),
            timeout: Duration::default(),
            max_jobs_to_activate: 0,
            concurrency_limit: 0,
            worker_callback: None,
            fetch_variable: vec![],
            request_timeout: Duration::default(),
            tenant_ids: vec![],
            _state: std::marker::PhantomData,
        }
    }

    fn transition<NewState: WorkerBuilderState>(self) -> WorkerBuilder<NewState> {
        WorkerBuilder {
            client: self.client,
            job_type: self.job_type,
            worker_name: self.worker_name,
            timeout: self.timeout,
            max_jobs_to_activate: self.max_jobs_to_activate,
            concurrency_limit: self.concurrency_limit,
            worker_callback: self.worker_callback,
            fetch_variable: self.fetch_variable,
            request_timeout: self.request_timeout,
            tenant_ids: self.tenant_ids,
            _state: std::marker::PhantomData,
        }
    }
}

impl WorkerBuilder<Initial> {
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
    pub fn with_job_type(mut self, job_type: String) -> WorkerBuilder<WithJobType> {
        self.job_type = job_type;
        self.transition()
    }
}

impl WorkerBuilder<WithJobType> {
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
    /// A `WorkerBuilder<WithTimeout>` instance with the job timeout configured.
    pub fn with_job_timeout(mut self, timeout: Duration) -> WorkerBuilder<WithTimeout> {
        self.timeout = timeout;
        self.transition()
    }
}

impl WorkerBuilder<WithTimeout> {
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
        mut self,
        request_timeout: Duration,
    ) -> WorkerBuilder<WithRequestTimeout> {
        self.request_timeout = request_timeout;
        self.transition()
    }
}

impl WorkerBuilder<WithRequestTimeout> {
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
        mut self,
        max_jobs_to_activate: i32,
    ) -> WorkerBuilder<WithMaxJobs> {
        self.max_jobs_to_activate = max_jobs_to_activate;
        self.transition()
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
    pub fn with_concurrency_limit(
        mut self,
        concurrency_limit: u32,
    ) -> WorkerBuilder<WithConcurrency> {
        self.concurrency_limit = concurrency_limit;
        self.transition()
    }
}

impl WorkerBuilder<WithConcurrency> {
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
    /// async fn example_service(client: Client, job: ActivatedJob) {
    ///     // Your job handling logic here
    /// }
    ///
    /// client
    ///    .worker()
    ///    .with_job_type(String::from("example-service"))
    ///    .with_job_timeout(Duration::from_secs(5 * 60))
    ///    .with_request_timeout(Duration::from_secs(10))
    ///    .with_max_jobs_to_activate(4)
    ///    .with_concurrency_limit(2)
    ///    .with_handler(example_service)
    ///    ...
    /// ```
    ///
    /// # Type Parameters
    ///
    /// * `F` - The type of the handler function.
    /// * `R` - The type of the `Future` returned by the handler function.
    ///
    /// # Constraints
    ///
    /// * `F` must implement `Fn(Client, ActivatedJob) -> R` and must be `Send`, `Sync`, and `'static`.
    /// * `R` must implement `Future<Output = ()>` and must be `Send` and `'static`.
    pub fn with_handler<F, R>(mut self, handler: F) -> WorkerBuilder<WithHandler>
    where
        F: Fn(Client, ActivatedJob) -> R + Send + Sync + 'static,
        R: Future<Output = ()> + Send + 'static,
    {
        self.worker_callback = Some(Arc::new(Box::new(move |client, job| {
            Box::pin(handler(client, job)) as BoxFuture
        })));
        self.transition()
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
    pub fn with_state<T>(self, shared_state: Arc<SharedState<T>>) -> WorkerStateBuilder<T>
    where
        T: Send + Sync + 'static,
    {
        WorkerStateBuilder {
            builder: self,
            state: shared_state,
        }
    }
}

impl WorkerBuilder<WithHandler> {
    /// Builds a `Worker` using the collected inputs.
    ///
    /// # Returns
    ///
    /// * `Worker` - The constructed Worker instance
    pub fn build(self) -> Worker {
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

/// `WorkerStateBuilder` is a builder pattern struct for constructing a `Worker`
/// instance that uses a handler consuming a shared state.
/// This struct leverages the typestate pattern to ensure that all mandatory parameters
/// are set before the `Worker` instance is built.
///
/// # Type Parameters
///
/// * `T` - The type of the shared state.
pub struct WorkerStateBuilder<T> {
    builder: WorkerBuilder<WithConcurrency>,
    state: Arc<SharedState<T>>,
}

impl<T> WorkerStateBuilder<T> {
    /// Sets the handler for the worker.
    ///
    /// This method allows you to specify a handler function that will be called
    /// when a job is activated. The handler function takes a `Client`, an `ActivatedJob`,
    /// and a shared state wrapped in an `Arc`. The handler must return a `Future` that
    /// resolves to `()`.
    ///
    /// # Type Parameters
    ///
    /// * `F` - The type of the handler function.
    /// * `R` - The type of the `Future` returned by the handler function.
    ///
    /// # Parameters
    ///
    /// * `handler` - A function that will be called when a job is activated. It takes
    ///   a `Client`, an `ActivatedJob`, and a shared state wrapped in an `Arc`, and returns
    ///   a `Future` that resolves to `()`.
    ///
    /// # Returns
    ///
    /// Returns a `WorkerBuilder` with the handler set.
    ///
    /// # Examples
    /// ```ignore
    /// struct ExampleSharedState {
    ///     pub increment_me: u32,
    /// }
    /// ...
    /// let state = Arc::new(SharedState(Mutex::new(ExampleSharedState {
    ///     increment_me: 0,
    /// })));
    ///
    /// async fn example_service(client: Client, job: ActivatedJob, state: Arc<Mutex<ExampleSharedState>>) {
    ///     // Your job handling logic here
    /// }
    ///
    /// client
    ///    .worker()
    ///    .with_job_type(String::from("example-service"))
    ///    .with_job_timeout(Duration::from_secs(5 * 60))
    ///    .with_request_timeout(Duration::from_secs(10))
    ///    .with_max_jobs_to_activate(4)
    ///    .with_concurrency_limit(2)
    ///    .with_state(state)
    ///    .with_handler(example_service)
    ///    ...
    /// ```
    ///
    /// # Constraints
    ///
    /// * `T` must implement `Send`, `Sync`, and have a static lifetime.
    /// * `F` must be a function that takes a `Client`, an `ActivatedJob`, and an `Arc<SharedState<T>>`,
    ///   and returns a `Future` that resolves to `()`. It must also implement `Send` and `Sync`, and have a static lifetime.
    /// * `R` must be a `Future` that resolves to `()`, and must implement `Send` and have a static lifetime.
    pub fn with_handler<F, R>(mut self, handler: F) -> WorkerBuilder<WithHandler>
    where
        T: Send + Sync + 'static,
        F: Fn(Client, ActivatedJob, Arc<SharedState<T>>) -> R + Send + Sync + 'static,
        R: Future<Output = ()> + Send + 'static,
    {
        let handler_tuple = (
            move |client: Client, job: ActivatedJob, state: Arc<SharedState<T>>| {
                Box::pin(handler(client, job, state)) as BoxFuture
            },
            self.state.clone(),
        );
        self.builder.worker_callback = Some(Arc::new(Box::new(handler_tuple)));
        self.builder.transition()
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
            if let Err(err) = timeout(request_timeout, async {
                let res = client
                    .gateway_client
                    .activate_jobs(tonic::Request::new(request))
                    .await
                    .map(|response| response.into_inner());

                let mut jobs_fetched = 0;
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
                println!("Error when fetching jobs: {:?}", err);
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

struct WorkConsumer {
    client: Client,
    job_rx: Receiver<ActivatedJob>,
    poll_tx: Sender<PollingMessage>,
    semaphore: Arc<Semaphore>,
    worker_callback: Arc<Box<dyn JobHandler>>,
}

impl WorkConsumer {
    async fn run(&mut self) {
        while let Some(job) = self.job_rx.recv().await {
            let permit = self.semaphore.clone().acquire_owned().await.unwrap();
            let poll_tx = self.poll_tx.clone();
            let client = self.client.clone();
            let callback = self.worker_callback.clone();

            tokio::spawn(async move {
                callback.execute(client, job).await;
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
///     .with_job_type("example-service")
///     .with_job_timeout(Duration::from_secs(60))
///     .with_request_timeout(Duration::from_secs(10))
///     .with_max_jobs_to_activate(5)
///     .with_concurrency_limit(3)
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
pub struct Worker {
    poller: WorkProducer,
    dispatcher: WorkConsumer,
}

impl Worker {
    fn new(
        client: Client,
        request: proto::ActivateJobsRequest,
        request_timeout: Duration,
        callback: Arc<Box<dyn JobHandler>>,
    ) -> Worker {
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
            semaphore: Arc::new(Semaphore::new(1)),
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
