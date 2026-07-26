use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TfeClientStatus {
    pub address: String,
    pub token_configured: bool,
}
