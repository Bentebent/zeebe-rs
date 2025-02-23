use std::{ops::Deref, time::Duration};
use thiserror::Error;
use zeebe_rs::{ActivatedJob, Client, WorkerOutputHandler};

// Custom error type we want to use for auto handler implementation
#[derive(Debug, Error)]
pub enum MyErrorType {
    #[error("")]
    Fail,
}

// We want to use Result<T, E> but can't implement trait WorkerOutputHandler
// because of the orphan rule.
struct MyResult<T>(pub Result<T, MyErrorType>);
impl<T> Deref for MyResult<T> {
    type Target = Result<T, MyErrorType>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Implement the WorkerOutputHandler for our custom result type
impl<T> WorkerOutputHandler for MyResult<T>
where
    T: Send + 'static,
{
    async fn handle_result(self, client: Client, job: ActivatedJob) {
        match *self {
            Ok(_) => unreachable!("This will always fail!"),
            Err(_) => {
                let _ = client.fail_job().with_job_key(job.key()).send().await;
            }
        }
    }
}

async fn always_fail(_client: Client, _job: ActivatedJob) -> MyResult<()> {
    MyResult(Err(MyErrorType::Fail))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("RUST_BACKTRACE", "1");

    // Default configuration using Camunda's docker compose
    // https://github.com/camunda/camunda-platform/blob/5dc74fe71667e18fbb5c8d4694068d662d83ad00/README.md
    let client = Client::builder()
        .with_address("http://localhost", 26500)
        .with_oauth(
            String::from("zeebe"),
            String::from("zecret"),
            String::from(
                "http://localhost:18080/auth/realms/camunda-platform/protocol/openid-connect/token",
            ),
            String::from("zeebe-api"),
            Duration::from_secs(30),
            None,
        )
        .build()
        .await?;

    // Wait until first OAuth token has been retrieved
    client.auth_initialized().await;

    let worker = client
        .worker()
        .with_request_timeout(Duration::from_secs(1))
        .with_job_timeout(Duration::from_secs(1))
        .with_max_jobs_to_activate(1)
        .with_concurrency_limit(1)
        .with_job_type(String::from("placeholder"))
        .with_handler(always_fail)
        .build();

    tokio::join!(worker.run());

    Ok(())
}
