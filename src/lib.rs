#[allow(clippy::all)]
pub(crate) mod proto {
    tonic::include_proto!("gateway_protocol");
}

pub(crate) mod client;
pub(crate) mod deploy_resource;
pub(crate) mod oauth;
pub(crate) mod topology;

pub use client::{Client, ClientBuilder, ClientBuilderError, ClientError};
pub use deploy_resource::DeployResource;
pub use topology::{TopologyRequest, TopologyResponse};

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
