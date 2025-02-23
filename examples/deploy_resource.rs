use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Duration};
use tokio::time::sleep;

#[derive(Debug, Serialize, Deserialize)]
struct HelloWorld {
    hello: String,
}

//ZEEBE_AUTHENTICATION_MODE=identity docker compose up -d
//URL: http://localhost:26500
//Client ID: zeebe
//Client secret: zecret
//OAuth URL: http://localhost:18080/auth/realms/camunda-platform/protocol/openid-connect/token
//Audience: zeebe-api
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    unsafe { std::env::set_var("RUST_BACKTRACE", "1") };

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
            None,
        )
        .build()
        .await?;

    let _ = client.auth_initialized().await;
    let result = client
        .deploy_resource()
        .with_resource_file(PathBuf::from("./examples/resources/hello_world.bpmn"))
        .read_resource_files()?
        .send()
        .await?;

    println!("{:?}", result);

    let result = client
        .publish_message()
        .with_name(String::from("hello_world"))
        .without_correlation_key()
        .with_variables(HelloWorld {
            hello: String::from("foo"),
        })?
        .send()
        .await?;

    println!("{:?}", result);

    sleep(Duration::from_secs(1)).await;

    let result = client
        .publish_message()
        .with_name(String::from("hello_message"))
        .with_correlation_key(String::from("foo"))
        .send()
        .await?;

    println!("{:?}", result);

    Ok(())
}
