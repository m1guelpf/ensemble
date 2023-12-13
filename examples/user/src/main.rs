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
	pub password: Hashed<String>,
	pub created_at: DateTime,
	pub updated_at: DateTime,
}

#[tokio::main]
async fn main() {
	ensemble::setup(&env::var("DATABASE_URL").expect("DATABASE_URL must be set"))
		.expect("Failed to set up database pool.");

	let users = User::all().await.unwrap();

	dbg!(users);
}
