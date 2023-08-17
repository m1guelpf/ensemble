use chrono::{DateTime, Utc};
use ensemble::Model;

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

fn main() {
    let user = User::find(1).unwrap();
    dbg!(user);
}
