#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = zeebe_rs::Client::builder()
        .with_address("http://localhost", 26500)
        .build()
        .await?;

    let topology = client.request_topology().send().await;
    println!("{:?}", topology);

    Ok(())
}
