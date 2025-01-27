# zeebe-rs

[![CI](https://github.com/Bentebent/zeebe-rs/actions/workflows/ci.yml/badge.svg?event=pull_request)](https://github.com/Bentebent/zeebe-rs/actions/workflows/ci.yml)
[![docs.rs](https://img.shields.io/docsrs/zeebe-rs)](https://docs.rs/crate/zeebe-rs/latest)
[![crates.io](https://img.shields.io/crates/v/zeebe-rs.svg)](https://crates.io/crates/zeebe-rs)

A Rust client and worker implementation for interacting with [Camunda Zeebe](https://camunda.com/platform/zeebe/) built using [Tonic](https://github.com/hyperium/tonic), [Tokio](https://github.com/tokio-rs/tokio) and [Serde](https://github.com/serde-rs/serde).

## Usage

Make sure you have [protoc installed.](https://github.com/protocolbuffers/protobuf#protobuf-compiler-installation) Add `zeebe-rs` to your `Cargo.toml` alongside `Tokio` and `Serde`.

```toml
serde = "1.0.217"
tokio = "1.43.0"
zeebe-rs = "0.1.0"
```

`zeebe-rs` uses the builder pattern together with type states extensively to guarantee that requests to Zeebe contain all required information. The client supports type safe conversions of data to and from Zeebe with `Serde`.

### Example

```rust

use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Duration};

#[derive(Debug, Serialize, Deserialize)]
struct HelloWorld {
    hello: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    //Create a client with OAuth
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

    //Block until first OAuth token has been retrieved
    let _ = client.auth_initialized().await;

    //Deploy a BPMN from file
    let result = client
        .deploy_resource()
        .with_resource_file(PathBuf::from("./examples/resources/hello_world.bpmn"))
        .read_resource_files()?
        .send()
        .await?;


    //Create a process instance
    let result = client
        .create_process_instance()
        .with_bpmn_process_id(String::from("hello_world"))
        .with_variables(HelloWorld {
            hello: String::from("world"),
        })?
        .with_result(None)
        .send_with_result::<HelloWorld>()
        .await?;

    Ok(())
}

```

Additional examples available in the documentation and `examples` folder.

## Development

Built using:

- rustc 1.83.0
- cargo 1.85.0-nightly (4c39aaff6 2024-11-25) (fmt and clippy only)
- [protoc v29.2](https://github.com/protocolbuffers/protobuf/releases/tag/v29.2)

## Contributing

We welcome contributions from the community, see our [contribution guidelines](CONTRIBUTING.md).

## License

Copyright (c) 2025 Tim Henriksson

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice (including the next paragraph) shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
