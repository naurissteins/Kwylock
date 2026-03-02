use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct AppConfig {
    pub show_seconds: bool,
    pub clock_24h: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            show_seconds: true,
            clock_24h: true,
        }
    }
}
