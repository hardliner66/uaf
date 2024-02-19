use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Data {
    pub from: Option<Uuid>,
    pub to: Uuid,
    pub payload: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    Data(Data),
    Spawned { id: Result<Uuid, String>, props: Props },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Props {
    pub executable: std::path::PathBuf,
    #[serde(default)]
    pub args: Vec<String>,
}

//'{"type": "message", "to": "73f31d83-a71e-4e32-a74a-263d3139f54d", "payload": {}}'

#[derive(Debug, Serialize, Deserialize)]
pub struct LogMessage {
    pub level: String,
    pub message: String,
    pub tags: IndexMap<String, serde_json::Value>,
}
