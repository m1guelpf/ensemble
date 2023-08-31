use std::env;

use ensemble::{
    relationships::{BelongsTo, HasMany, Relationship},
    types::{DateTime, Hashed},
    Model,
};

#[derive(Debug, Model)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
    pub password: Hashed<String>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
    #[model(foreign_key = "author_id")]
    pub posts: HasMany<User, Post>,
}

#[derive(Debug, Model)]
pub struct Post {
    #[model(incrementing)]
    pub id: u64,
    pub created_at: DateTime,
    pub updated_at: DateTime,

    #[model(foreign_key = "author_id")]
    pub user: BelongsTo<Post, User>,
}

#[tokio::main]
async fn main() {
    ensemble::setup(&env::var("DATABASE_URL").expect("DATABASE_URL must be set"))
        .await
        .expect("Failed to set up database pool.");

    let mut user = User::find(1).await.expect("Failed to find user.");
    let posts = user.posts.get().await.expect("Failed to get posts.");
    dbg!(posts);

    let mut post = Post::find(1).await.expect("Failed to find post.");
    let user = post.user.get().await.expect("Failed to get user.");
    dbg!(user);
}
