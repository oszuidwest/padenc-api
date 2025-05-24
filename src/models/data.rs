use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Item {
    pub title: String,
    pub artist: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Track {
    pub item: Item,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Program {
    pub name: String,
    pub expires_at: DateTime<Utc>,
}