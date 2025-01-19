#[allow(clippy::all)]
pub(crate) mod proto {
    tonic::include_proto!("gateway_protocol");
}

pub(crate) mod client;
pub(crate) mod decision;
pub(crate) mod incident;
pub(crate) mod job;
pub(crate) mod message;
pub(crate) mod oauth;
pub(crate) mod process_instance;
pub(crate) mod resource;
pub(crate) mod set_variables;
pub(crate) mod signal;
pub(crate) mod throw_error;
pub(crate) mod topology;

pub use client::{Client, ClientBuilder, ClientBuilderError, ClientError};

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
