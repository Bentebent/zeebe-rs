use std::{path::PathBuf, sync::Arc, time::Duration};

use tokio::sync::Mutex;
use zeebe_rs::{ActivatedJob, Client};

#[derive(Debug, Clone)]
struct ExampleSharedState {
    pub increment_me: u32,
}

//ZEEBE_AUTHENTICATION_MODE=identity docker compose up -d
//URL: http://localhost:26500
//Client ID: zeebe
//Client secret: zecret
//OAuth URL: http://localhost:18080/auth/realms/camunda-platform/protocol/openid-connect/token
//Audience: zeebe-api
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("RUST_BACKTRACE", "1");

    let client = zeebe_rs::Client::builder()
        .with_address("http://localhost", 26500)
        .with_oauth(
            String::from("zeebe"),
            String::from("zecret"),
            String::from(
                "http://localhost:18080/auth/realms/camunda-platform/protocol/openid-connect/token",
            ),
            String::from("zeebe-api"),
            Duration::from_secs(30),
        )
        .build()
        .await?;

    let _ = client.auth_initialized().await;

    let res = client
        .deploy_resource()
        .with_resource_file(PathBuf::from("examples/resources/order-process.bpmn"))
        .read_resource_files()?
        .send()
        .await?;
    println!("{:?}", res);

    for _ in 0..10 {
        client
            .create_process_instance()
            .with_bpmn_process_id(String::from("order-process"))
            .without_input()
            .send()
            .await?;
    }

    let state = Arc::new(zeebe_rs::SharedState(Mutex::new(ExampleSharedState {
        increment_me: 0,
    })));

    client
        .worker()
        .with_job_type(String::from("payment-service"))
        .with_job_timeout(Duration::from_secs(5 * 60))
        .with_request_timeout(Duration::from_secs(10))
        .with_max_jobs_to_activate(4)
        .with_concurrency_limit(2)
        //.with_handler(payment_service)
        .with_state(state)
        .with_handler(|client, job, state| async move {
            println!("Hello from closure {:?}", job);
            let mut lock = state.lock().await;
            lock.increment_me += 1;
            println!("State: {:?}", lock);

            payment_service(client, job).await;
        })
        .build()
        .run()
        .await;

    Ok(())
}

async fn payment_service(client: Client, job: ActivatedJob) {
    let _ = client.complete_job().with_job_key(job.key()).send().await;
}
