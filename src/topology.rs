use crate::{client::Client, proto, ClientError};

/// Request to obtain the current cluster topology.
///
/// This request is used to retrieve detailed information about the Zeebe cluster's topology,
/// including broker nodes, partition distribution, cluster size, replication factor, and gateway version.
///
/// # Example
///
/// ```ignore
/// let topology = client.topology().send().await;
/// ```
///
/// # Errors
///
/// Returns a `ClientError` if the request fails.
#[derive(Debug, Clone)]
pub struct TopologyRequest(Client);

impl TopologyRequest {
    pub(crate) fn new(client: Client) -> Self {
        TopologyRequest(client)
    }

    /// Sends a request to get the current cluster topology
    ///
    /// # Returns
    ///
    /// A `Result` containing either:
    /// - `TopologyResponse` with information about:
    ///   - Broker nodes in the cluster
    ///   - Partition distribution
    ///   - Cluster size and replication factor
    ///   - Gateway version
    /// - `ClientError` if the request fails
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
///
/// This struct holds details about a specific partition, including its unique identifier,
/// role, and health status.
#[derive(Debug, Clone)]
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
    /// Returns the unique identifier for this partition.
    ///
    /// # Returns
    ///
    /// An `i32` representing the partition's unique identifier.
    pub fn partition_id(&self) -> i32 {
        self.partition_id
    }

    /// Returns the role of this partition.
    ///
    /// # Returns
    ///
    /// An `i32` representing the role of the partition:
    ///
    /// - `0`: LEADER - Handles all requests.
    /// - `1`: FOLLOWER - Replicates data.
    /// - `2`: INACTIVE - Not participating.
    pub fn role(&self) -> i32 {
        self.role
    }

    /// Returns the health status of this partition.
    ///
    /// # Returns
    ///
    /// An `i32` representing the health status of the partition:
    ///
    /// - `0`: HEALTHY - Processing normally.
    /// - `1`: UNHEALTHY - Processing with issues.
    /// - `2`: DEAD - Not processing.
    pub fn health(&self) -> i32 {
        self.health
    }
}

/// Information about a broker node in the Zeebe cluster
#[derive(Debug, Clone)]
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
    /// Returns the unique identifier for this broker within the cluster.
    ///
    /// # Returns
    ///
    /// An `i32` representing the broker's node ID.
    pub fn node_id(&self) -> i32 {
        self.node_id
    }

    /// Returns the network hostname where this broker can be reached.
    ///
    /// # Returns
    ///
    /// A string slice (`&str`) representing the broker's hostname.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Returns the network port where this broker accepts connections.
    ///
    /// # Returns
    ///
    /// An `i32` representing the broker's port number.
    pub fn port(&self) -> i32 {
        self.port
    }

    /// Returns a reference to the list of partitions managed or replicated by this broker.
    ///
    /// # Returns
    ///
    /// A slice of `Partition` references representing the partitions managed or replicated by this broker.
    pub fn partitions(&self) -> &[Partition] {
        &self.partitions
    }

    /// Returns the version of the broker software.
    ///
    /// # Returns
    ///
    /// A string slice (`&str`) representing the version of the broker software.
    pub fn version(&self) -> &str {
        &self.version
    }
}

/// Response containing current topology of Zeebe cluster
#[derive(Debug, Clone)]
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

/// Represents the response containing the topology information of the Zeebe cluster.
impl TopologyResponse {
    /// Returns a reference to the list of all broker nodes in the cluster.
    ///
    /// # Returns
    ///
    /// A slice of `BrokerInfo` references representing all broker nodes in the cluster.
    pub fn brokers(&self) -> &[BrokerInfo] {
        &self.brokers
    }

    /// Returns the total number of nodes in the cluster.
    ///
    /// # Returns
    ///
    /// An `i32` representing the total number of nodes.
    pub fn cluster_size(&self) -> i32 {
        self.cluster_size
    }

    /// Returns the total number of partitions distributed across the cluster.
    ///
    /// # Returns
    ///
    /// An `i32` representing the total number of partitions.
    pub fn partitions_count(&self) -> i32 {
        self.partitions_count
    }

    /// Returns the number of copies of each partition that are maintained.
    ///
    /// # Returns
    ///
    /// An `i32` representing the replication factor.
    pub fn replication_factor(&self) -> i32 {
        self.replication_factor
    }

    /// Returns the version of the gateway software.
    ///
    /// # Returns
    ///
    /// A string slice (`&str`) representing the version of the gateway software.
    pub fn gateway_version(&self) -> &str {
        &self.gateway_version
    }
}
