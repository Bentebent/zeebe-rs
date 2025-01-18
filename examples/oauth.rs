use std::time::Duration;

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
    let topology = client.topology().send().await;
    println!("{:?}", topology);

    Ok(())
}
