use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

use crate::models::traits::HasId;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Item {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Image {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip)]
    pub path: Option<PathBuf>,
    #[serde(skip_deserializing)]
    pub filename: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Track {
    #[serde(default = "Uuid::new_v4", skip_deserializing)]
    pub id: Uuid,
    pub item: Item,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Image>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Program {
    #[serde(default = "Uuid::new_v4", skip_deserializing)]
    pub id: Uuid,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Image>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Station {
    #[serde(default = "Uuid::new_v4", skip_deserializing)]
    pub id: Uuid,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Image>,
}

impl HasId for Track {
    fn get_id(&self) -> Option<Uuid> {
        Some(self.id)
    }
}

impl HasId for Program {
    fn get_id(&self) -> Option<Uuid> {
        Some(self.id)
    }
}

impl HasId for Station {
    fn get_id(&self) -> Option<Uuid> {
        Some(self.id)
    }
}