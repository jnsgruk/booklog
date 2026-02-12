use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::ids::UserId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub uuid: String,
    pub is_admin: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewUser {
    pub username: String,
    pub uuid: String,
}

impl User {
    pub fn new(
        id: UserId,
        username: String,
        uuid: String,
        is_admin: bool,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            username,
            uuid,
            is_admin,
            created_at,
        }
    }
}

impl NewUser {
    pub fn new(username: String, uuid: String) -> Self {
        Self { username, uuid }
    }
}
