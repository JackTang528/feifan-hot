use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotItem {
    pub rank: u32,
    pub title: String,
    pub url: String,
    pub hot: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardResult {
    pub id: String,
    pub name: String,
    pub items: Vec<HotItem>,
    pub updated_at: i64,
    pub error: Option<String>,
    pub from_cache: bool,
}
