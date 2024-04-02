use chrone::prelude::*;
use uuid::Uuid;

use crate::model::user::User;

#[derive(Debug, Clone)]
pub struct Message {
    pub id: Uuid,
    pub user: User,
    pub body: String,
    pub create_at: DateTime<Utc>,
}

impl Message {
    pub fn new(id: Uuid, user: User, body: &str, create_at: DateTime<Utc>) -> Self {
        Message {
            id, 
            user,
            body: String::from(body),
            create_at
        }
    }
}
