use std::env;
mod migrations;

#[tokio::main]
async fn main() {
    ensemble::setup(&env::var("DATABASE_URL").expect("DATABASE_URL must be set"))
        .await
        .expect("Failed to set up database pool.");

    ensemble::migrate!(migrations::CreateUsersTable, migrations::CreatePostsTable)
        .await
        .expect("Failed to run migrations.");
}
