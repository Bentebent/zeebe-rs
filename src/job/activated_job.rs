use crate::proto;

#[derive(Debug, Clone)]
pub struct ActivatedJob {}

impl From<proto::ActivatedJob> for ActivatedJob {
    fn from(value: proto::ActivatedJob) -> Self {
        ActivatedJob {}
    }
}
