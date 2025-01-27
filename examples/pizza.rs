use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};
use tokio::{sync::Mutex, time::sleep};
use zeebe_rs::{ActivatedJob, Client, ClientError, SharedState};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Customer {
    name: String,
    address: String,
    bad_tipper: bool,
    customer_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Order {
    items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Stock {
    items: HashMap<String, i32>,
}

#[derive(Debug, Clone, Serialize)]
struct OrderResult {
    order_accepted: bool,
    message: Option<String>,
}

async fn place_order(
    client: Client,
    process_definition_key: i64,
    name: &str,
    address: &str,
    bad_tipper: bool,
    items: Vec<&str>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let customer = Customer {
        name: name.to_owned(),
        address: address.to_owned(),
        bad_tipper,
        customer_id: format!("{}_{}", name, address),
    };

    let order = Order {
        items: items.into_iter().map(|x| x.to_owned()).collect(),
    };

    let res = client
        .create_process_instance()
        .with_process_definition_key(process_definition_key)
        .with_variables(customer.clone())?
        .send()
        .await?;

    println!("{:?}", res);

    let res = client
        .publish_message()
        .with_name(String::from("order_pizza_msg"))
        .with_correlation_key(customer.customer_id.clone())
        .with_variables(order)?
        .send()
        .await?;

    println!("{:?}", res);

    Ok(())
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
        )
        .build()
        .await?;

    // Wait until first OAuth token has been retrieved
    client.auth_initialized().await;

    // Deploy our pizza ordering process
    // We just discard the result for brevity
    let deploy_resource_response = client
        .deploy_resource()
        .with_resource_file(PathBuf::from("examples/resources/pizza-order.bpmn"))
        .read_resource_files()?
        .send()
        .await?;

    let process_definition_key =
        if let Some(deployment) = deploy_resource_response.deployments().first() {
            match deployment.metadata() {
                Some(metadata) => match metadata {
                    zeebe_rs::Metadata::Process(process_metadata) => {
                        process_metadata.process_definition_key()
                    }
                    _ => {
                        unreachable!("We're only deploying a bpmn here");
                    }
                },
                _ => -1,
            }
        } else {
            -1
        };

    // Let's define our shared state, how much stock we have
    let mut initial_stock = HashMap::new();
    initial_stock.insert(String::from("Pepperoni"), 5);
    initial_stock.insert(String::from("Margherita"), 8);
    initial_stock.insert(String::from("Hawaiian"), 3);
    initial_stock.insert(String::from("Quattro Formaggi"), 0);
    initial_stock.insert(String::from("Vegetarian"), 6);

    let stock = Arc::new(SharedState(Mutex::new(Stock {
        items: initial_stock,
    })));

    let confirm_worker = client
        .worker()
        .with_job_type(String::from("confirm_order"))
        .with_job_timeout(Duration::from_secs(10))
        .with_request_timeout(Duration::from_secs(10))
        .with_max_jobs_to_activate(4)
        .with_concurrency_limit(2)
        .with_state(stock)
        .with_handler(confirm_order)
        .with_fetch_variable(String::from("items"))
        .build();

    let bake_worker = client
        .worker()
        .with_job_type(String::from("bake_pizzas"))
        .with_job_timeout(Duration::from_secs(10))
        .with_request_timeout(Duration::from_secs(10))
        .with_max_jobs_to_activate(10)
        .with_concurrency_limit(10)
        .with_handler(bake_pizzas)
        .build();

    let reject_order_worker = client
        .worker()
        .with_job_type(String::from("reject_order"))
        .with_job_timeout(Duration::from_secs(10))
        .with_request_timeout(Duration::from_secs(10))
        .with_max_jobs_to_activate(5)
        .with_concurrency_limit(5)
        .with_handler(|client, job| async move {
            let _ = client.complete_job().with_job_key(job.key()).send().await;
        })
        .build();

    let deliver_worker = client
        .worker()
        .with_job_type(String::from("deliver_order"))
        .with_job_timeout(Duration::from_secs(10))
        .with_request_timeout(Duration::from_secs(10))
        .with_max_jobs_to_activate(1)
        .with_concurrency_limit(1)
        .with_handler(deliver_order)
        .build();

    let clarify_address_worker = client
        .worker()
        .with_job_type(String::from("call_customer"))
        .with_job_timeout(Duration::from_secs(10))
        .with_request_timeout(Duration::from_secs(10))
        .with_max_jobs_to_activate(5)
        .with_concurrency_limit(5)
        .with_handler(|client, job| async move {
            let mut customer = job.data::<Customer>().unwrap();

            //Same guy keeps leaving out his address for some reason
            customer.address = String::from("1337 Coolsville, USA");
            if let Ok(req) = client
                .complete_job()
                .with_job_key(job.key())
                .with_variables(customer)
            {
                let _ = req.send().await;
            }
        })
        .build();

    tokio::spawn(place_order(
        client.clone(),
        process_definition_key,
        "Jane Doe",
        "100 Coolsville, USA",
        true,
        vec!["Pepperoni"],
    ));

    tokio::spawn(place_order(
        client.clone(),
        process_definition_key,
        "John Doe",
        "999 NotCoolsville, USA",
        false,
        vec!["Hawaiian"; 100],
    ));

    tokio::spawn(place_order(
        client.clone(),
        process_definition_key,
        "Lorem Ipsum",
        "",
        false,
        vec!["Hawaiian", "Pepperoni"],
    ));

    tokio::spawn(place_order(
        client.clone(),
        process_definition_key,
        "George W Bush",
        "White House",
        true,
        vec!["Burger"],
    ));

    let _ = tokio::join!(
        confirm_worker.run(),
        reject_order_worker.run(),
        bake_worker.run(),
        deliver_worker.run(),
        clarify_address_worker.run(),
    );

    Ok(())
}

async fn confirm_order(client: Client, job: ActivatedJob, state: Arc<SharedState<Mutex<Stock>>>) {
    let order = job
        .data::<Order>()
        .expect("For demonstration purposes we're not handling a missing order here");
    let mut stock: tokio::sync::MutexGuard<Stock> = state.lock().await;

    let mut order_accepted = true;
    let mut order_message = None;
    for item in order.items {
        if let Some(quantity) = stock.items.get_mut(&item) {
            if *quantity > 0 {
                *quantity -= 1;
            } else {
                order_message = Some(format!("We're out of stock of {}", item));
                order_accepted = false;
            }
        } else {
            order_message = Some(format!("We don't serve {}", item));
            order_accepted = false;
        }
    }

    println!("Confirmed order");
    if let Ok(req) = client
        .complete_job()
        .with_job_key(job.key())
        .with_variables(OrderResult {
            order_accepted,
            message: order_message,
        })
    {
        let _ = req.send().await;
    }
}

async fn bake_pizzas(client: Client, job: ActivatedJob) {
    let order = job
        .data::<Order>()
        .expect("We shouldn't start baking pizzas if there is no order!");

    println!("Baking pizzas");
    sleep(Duration::from_secs(10 * order.items.len() as u64)).await;
    println!("Finished baking {} pizzas", order.items.len());

    let _ = client.complete_job().with_job_key(job.key()).send().await;
}

async fn deliver_order(client: Client, job: ActivatedJob) {
    let data: Result<Customer, ClientError> = job.data();

    println!("{:?}", data);
    let customer = data.unwrap();

    if customer.address.is_empty() {
        let _ = client
            .throw_error()
            .with_job_key(job.key())
            .with_error_code(String::from("invalid_address"))
            .with_error_message(String::from("Missing address"))
            .send()
            .await;

        let _ = client
            .fail_job()
            .with_job_key(job.key())
            .with_retries(job.retries() - 1)
            .send()
            .await;

        return;
    }

    if customer.bad_tipper && job.retries() > 1 {
        println!("Customer is a bad tipper, let's delay their order");
        sleep(Duration::from_secs(10)).await;

        let builder = client
            .fail_job()
            .with_job_key(job.key())
            .with_retries(job.retries() - 1)
            .with_error_message(String::from("Bad tipper, delaying delivery"))
            .with_variables(json!({"apology": "Oops"}));

        if let Ok(builder) = builder {
            let _ = builder.send().await;
        }
    } else {
        let _ = client.complete_job().with_job_key(job.key()).send().await;
    }
}
