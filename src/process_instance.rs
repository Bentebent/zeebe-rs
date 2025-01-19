pub mod cancel;
pub mod create;
pub mod migrate;
pub mod modify;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProcessInstanceError {
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
}
