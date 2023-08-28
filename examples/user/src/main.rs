use ensemble::{
    types::{DateTime, Hashed},
    Model,
};
use std::env;
use validator::Validate;

#[derive(Debug, Model, Validate)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
    #[validate(length(min = 8))]
    pub password: Hashed<String>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[tokio::main]
async fn main() {
    ensemble::setup(&env::var("DATABASE_URL").expect("DATABASE_URL must be set"))
        .await
        .expect("Failed to set up database pool.");

    let users = User::all().await.unwrap();

    dbg!(users);
}
