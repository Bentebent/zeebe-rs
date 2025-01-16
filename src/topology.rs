use crate::{client::Client, proto};
#[derive(Debug)]
pub struct TopologyRequest(Client);

impl TopologyRequest {
    pub fn new(client: Client) -> Self {
        TopologyRequest(client)
    }

    pub async fn send(mut self) -> TopologyResponse {
        let request = proto::TopologyRequest {};

        let result = self
            .0
            .gateway_client
            .topology(tonic::Request::new(request))
            .await
            .unwrap();

        result.into_inner().into()
    }
}

#[derive(Debug)]
pub struct TopologyResponse {}

impl From<proto::TopologyResponse> for TopologyResponse {
    fn from(_response: proto::TopologyResponse) -> Self {
        TopologyResponse {}
    }
}
