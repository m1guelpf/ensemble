use chrono::serde::ts_milliseconds;
use chrono::{DateTime, Utc};
use ensemble::Model;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Model, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
    pub password: String,
    #[serde(with = "ts_milliseconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "ts_milliseconds")]
    pub updated_at: DateTime<Utc>,
}

#[tokio::main]
async fn main() {
    ensemble::setup(&env::var("DATABASE_URL").expect("DATABASE_URL must be set"))
        .await
        .expect("Failed to set up database pool.");

    let user = User::all().await.unwrap();
    dbg!(user);
}
