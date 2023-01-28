use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub api_key: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self { api_key: None }
    }
}
