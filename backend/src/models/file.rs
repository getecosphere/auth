use bson::oid::ObjectId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub filename: String,
    #[serde(rename = "fileType")]
    pub file_type: String,
    #[serde(rename = "fileSize")]
    pub file_size: i64,
    #[serde(rename = "storageType")]
    pub storage_type: String,
    #[serde(rename = "storagePath")]
    pub storage_path: String,
    #[serde(rename = "uploadedBy")]
    pub uploaded_by: String,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
}
