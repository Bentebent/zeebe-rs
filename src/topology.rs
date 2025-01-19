use crate::{client::Client, proto, ClientError};

/// Request to obtain current cluster topology
#[derive(Debug)]
pub struct TopologyRequest(Client);

impl TopologyRequest {
    /// Creates a new topology request
    pub(crate) fn new(client: Client) -> Self {
        TopologyRequest(client)
    }

    /// Sends request to get current cluster topology
    ///
    /// # Returns
    /// Information about:
    /// - Broker nodes in the cluster
    /// - Partition distribution
    /// - Cluster size and replication factor
    /// - Gateway version
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

/// Information about a partition in the Zeebe cluster
#[derive(Debug)]
pub struct Partition {
    /// Unique identifier for this partition
    partition_id: i32,
    /// Role of the broker for this partition
    /// - 0: LEADER - Handles all requests
    /// - 1: FOLLOWER - Replicates data
    /// - 2: INACTIVE - Not participating
    role: i32,
    /// Health status of this partition
    /// - 0: HEALTHY - Processing normally
    /// - 1: UNHEALTHY - Processing with issues
    /// - 2: DEAD - Not processing
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
    /// Returns the unique identifier for this partition
    pub fn partition_id(&self) -> i32 {
        self.partition_id
    }

    /// Returns the role of this partition
    /// - 0: LEADER - Handles all requests
    /// - 1: FOLLOWER - Replicates data
    /// - 2: INACTIVE - Not participating
    pub fn role(&self) -> i32 {
        self.role
    }

    /// Returns the health status of this partition
    /// - 0: HEALTHY - Processing normally
    /// - 1: UNHEALTHY - Processing with issues
    /// - 2: DEAD - Not processing
    pub fn health(&self) -> i32 {
        self.health
    }
}

/// Information about a broker node in the Zeebe cluster
#[derive(Debug)]
pub struct BrokerInfo {
    /// Unique identifier for this broker within cluster
    node_id: i32,
    /// Network hostname where broker can be reached
    host: String,
    /// Network port where broker accepts connections
    port: i32,
    /// List of partitions this broker manages/replicates
    partitions: Vec<Partition>,
    /// Version of the broker software
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
    /// Returns the unique identifier for this broker within the cluster
    pub fn node_id(&self) -> i32 {
        self.node_id
    }

    /// Returns the network hostname where this broker can be reached
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Returns the network port where this broker accepts connections
    pub fn port(&self) -> i32 {
        self.port
    }

    /// Returns the list of partitions this broker manages/replicates
    pub fn partitions(&self) -> &Vec<Partition> {
        &self.partitions
    }

    /// Returns the version of the broker software
    pub fn version(&self) -> &str {
        &self.version
    }
}

/// Response containing current topology of Zeebe cluster
#[derive(Debug)]
pub struct TopologyResponse {
    /// List of all broker nodes in the cluster
    brokers: Vec<BrokerInfo>,
    /// Total number of nodes in the cluster
    cluster_size: i32,
    /// Total number of partitions distributed across cluster
    partitions_count: i32,
    /// How many copies of each partition are maintained
    replication_factor: i32,
    /// Version of the gateway software
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
    /// Returns the list of all broker nodes in the cluster
    pub fn brokers(&self) -> &Vec<BrokerInfo> {
        &self.brokers
    }

    /// Returns the total number of nodes in the cluster
    pub fn cluster_size(&self) -> i32 {
        self.cluster_size
    }

    /// Returns the total number of partitions distributed across cluster
    pub fn partitions_count(&self) -> i32 {
        self.partitions_count
    }

    /// Returns how many copies of each partition are maintained
    pub fn replication_factor(&self) -> i32 {
        self.replication_factor
    }

    /// Returns the version of the gateway software
    pub fn gateway_version(&self) -> &str {
        &self.gateway_version
    }
}
