use mongodb::{Database, bson::{Uuid, doc}};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct UserEntity {
    #[serde(rename = "_id")]
    pub id: Uuid,
    pub email: String,
    pub password: String,
}

pub async fn find_by_email(db: Database, email: &str) -> anyhow::Result<Option<UserEntity>> {
    let collection = db.collection("user_entity");
    let entity = collection.find_one(doc!{"email": email}).await?;
    Ok(entity)
}