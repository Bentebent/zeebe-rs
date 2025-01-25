use crate::{proto, ActivatedJob, Client, ClientError};
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
    pub fn with_job_type(mut self, job_type: String) -> WorkerBuilder<WithJobType> {
        self.job_type = job_type;
        self.transition()
    }
}

impl WorkerBuilder<WithJobType> {
    pub fn with_job_timeout(mut self, timeout: Duration) -> WorkerBuilder<WithTimeout> {
        self.timeout = timeout;
        self.transition()
    }
}

impl WorkerBuilder<WithTimeout> {
    pub fn with_request_timeout(
        mut self,
        request_timeout: Duration,
    ) -> WorkerBuilder<WithRequestTimeout> {
        self.request_timeout = request_timeout;
        self.transition()
    }
}

impl WorkerBuilder<WithRequestTimeout> {
    pub fn with_max_jobs_to_activate(
        mut self,
        max_jobs_to_activate: i32,
    ) -> WorkerBuilder<WithMaxJobs> {
        self.max_jobs_to_activate = max_jobs_to_activate;
        self.transition()
    }
}

impl WorkerBuilder<WithMaxJobs> {
    pub fn with_concurrency_limit(
        mut self,
        concurrency_limit: u32,
    ) -> WorkerBuilder<WithConcurrency> {
        self.concurrency_limit = concurrency_limit;
        self.transition()
    }
}

impl WorkerBuilder<WithConcurrency> {
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
    pub fn build(self) -> Result<Worker, ClientError> {
        let request = proto::ActivateJobsRequest {
            r#type: self.job_type,
            worker: self.worker_name,
            timeout: self.timeout.as_millis() as i64,
            max_jobs_to_activate: self.max_jobs_to_activate,
            fetch_variable: self.fetch_variable,
            request_timeout: self.request_timeout.as_millis() as i64,
            tenant_ids: self.tenant_ids,
        };

        Ok(Worker::new(
            self.client,
            request,
            self.request_timeout,
            self.worker_callback
                .expect("Don't transition to build without handler"),
        ))
    }

    pub fn with_worker_name(mut self, worker_name: String) -> Self {
        self.worker_name = worker_name;
        self
    }

    pub fn with_fetch_variable(mut self, fetch_variable: String) -> Self {
        self.fetch_variable.push(fetch_variable);
        self
    }

    pub fn with_fetch_variables(mut self, mut fetch_variables: Vec<String>) -> Self {
        self.fetch_variable.append(&mut fetch_variables);
        self
    }

    pub fn with_tenant_id(mut self, tenant_id: String) -> Self {
        self.tenant_ids.push(tenant_id);
        self
    }

    pub fn with_tenant_ids(mut self, mut tenant_ids: Vec<String>) -> Self {
        self.tenant_ids.append(&mut tenant_ids);
        self
    }
}
pub struct WorkerStateBuilder<T> {
    builder: WorkerBuilder<WithConcurrency>,
    state: Arc<SharedState<T>>,
}

impl<T> WorkerStateBuilder<T> {
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

        println!("Requesting jobs {:?}", request);
        tokio::spawn(async move {
            if let Err(err) = timeout(request_timeout, async {
                let res = client
                    .gateway_client
                    .activate_jobs(tonic::Request::new(request))
                    .await
                    .map(|response| response.into_inner());

                println!("{:?}", res);
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
    pub async fn run(mut self) -> Result<(), Box<dyn std::error::Error>> {
        tokio::join!(self.poller.run(), self.dispatcher.run());
        Ok(())
    }
}
