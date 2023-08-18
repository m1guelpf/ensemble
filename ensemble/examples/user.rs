use chrono::{DateTime, Utc};
use ensemble::Model;
use std::env;

#[derive(Debug, Model)]
#[ensemble(table_name = "users_custom")]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
    pub password: String,
    #[ensemble(created_at)]
    pub created_at: DateTime<Utc>,
    #[ensemble(updated_at)]
    pub updated_at: DateTime<Utc>,
}

#[tokio::main]
async fn main() {
    ensemble::setup(&env::var("DATABASE_URL").expect("DATABASE_URL must be set"))
        .expect("Failed to set up database pool.");

    let user = User::find(1).await.unwrap();
    dbg!(user);
}
