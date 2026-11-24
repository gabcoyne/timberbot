use serde::{Deserialize, Serialize};

/// Official Timberborn 1.0+ built-in HTTP Lever state (`GET /api/levers`, `GET /api/levers/{name}`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HttpLeverState {
    pub name: String,
    pub state: bool,
    #[serde(default)]
    pub spring_return: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// Official Timberborn 1.0+ built-in HTTP Adapter state (`GET /api/adapters`, `GET /api/adapters/{name}`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HttpAdapterState {
    pub name: String,
    pub state: bool,
}
