use crate::{client::Client, proto, ClientError};
#[derive(Debug)]
pub struct TopologyRequest(Client);

impl TopologyRequest {
    pub fn new(client: Client) -> Self {
        TopologyRequest(client)
    }

    pub async fn send(mut self) -> Result<TopologyResponse, ClientError> {
        let request = proto::TopologyRequest {};

        let result = self
            .0
            .gateway_client
            .topology(tonic::Request::new(request))
            .await?;

        Ok(result.into_inner().into())
    }
}

#[derive(Debug)]
pub struct Partition {
    partition_id: i32,
    role: i32,
    health: i32,
}

impl From<proto::Partition> for Partition {
    fn from(value: proto::Partition) -> Partition {
        Partition {
            partition_id: value.partition_id,
            role: value.role,
            health: value.health,
        }
    }
}

impl Partition {
    pub fn partition_id(&self) -> i32 {
        self.partition_id
    }

    pub fn role(&self) -> i32 {
        self.role
    }

    pub fn health(&self) -> i32 {
        self.health
    }
}

#[derive(Debug)]
pub struct BrokerInfo {
    node_id: i32,
    host: String,
    port: i32,
    partitions: Vec<Partition>,
    version: String,
}

impl From<proto::BrokerInfo> for BrokerInfo {
    fn from(value: proto::BrokerInfo) -> BrokerInfo {
        BrokerInfo {
            node_id: value.node_id,
            host: value.host,
            port: value.port,
            partitions: value.partitions.into_iter().map(|p| p.into()).collect(),
            version: value.version,
        }
    }
}

impl BrokerInfo {
    pub fn node_id(&self) -> i32 {
        self.node_id
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> i32 {
        self.port
    }

    pub fn partitions(&self) -> &Vec<Partition> {
        &self.partitions
    }

    pub fn version(&self) -> &str {
        &self.version
    }
}

#[derive(Debug)]
pub struct TopologyResponse {
    brokers: Vec<BrokerInfo>,
    cluster_size: i32,
    partitions_count: i32,
    replication_factor: i32,
    gateway_version: String,
}

impl From<proto::TopologyResponse> for TopologyResponse {
    fn from(value: proto::TopologyResponse) -> TopologyResponse {
        TopologyResponse {
            brokers: value.brokers.into_iter().map(|b| b.into()).collect(),
            cluster_size: value.cluster_size,
            partitions_count: value.partitions_count,
            replication_factor: value.replication_factor,
            gateway_version: value.gateway_version,
        }
    }
}

impl TopologyResponse {
    pub fn brokers(&self) -> &Vec<BrokerInfo> {
        &self.brokers
    }

    pub fn cluster_size(&self) -> i32 {
        self.cluster_size
    }

    pub fn partitions_count(&self) -> i32 {
        self.partitions_count
    }

    pub fn replication_factor(&self) -> i32 {
        self.replication_factor
    }

    pub fn gateway_version(&self) -> &str {
        &self.gateway_version
    }
}
